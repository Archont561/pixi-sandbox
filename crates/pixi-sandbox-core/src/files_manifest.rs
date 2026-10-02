//! `envs/<name>/files.json` — the per-file oracle for a restored environment prefix (D13).
//!
//! `doctor --verify` proves the *branch* is intact and `restore` verifies every blob before
//! writing it, but nothing checked the tree that comes out the other end: a prefix that is
//! structurally plausible but wrong passed every check (task-10, measured against a real
//! transport). This module is the fix: at pack time the environment is unpacked once more,
//! with the same pinned `pixi-unpack` the airlock will use, and every file of that tree is
//! recorded here with a digest. `verify::verify_restored` then walks a *restored* prefix and
//! compares it against this list — presence, content, symlink targets and the executable bit.
//!
//! ## Why digests need a canonical form
//!
//! A prefix is not byte-portable: it names itself, in text files and in fixed-width binary
//! fields, and the name changes three times between pack and airlock (build placeholder →
//! staging prefix → final prefix). Two rules make both sides comparable anyway, both measured
//! against a real `pixi-pack`/`pixi-unpack` 0.7.11 transport:
//!
//! 1. **NUL runs collapse to one NUL.** Conda's binary prefix replacement writes the install
//!    prefix into fixed-width, NUL-padded fields, so the padding *length* is a function of the
//!    prefix-path length, not of the content. Collapsing the runs removes that dependency
//!    (and only that: it also means a padding-only corruption of an inert region is not
//!    caught, which is the honest price).
//! 2. **Every known spelling of the prefix path becomes [`SENTINEL`].** Each side replaces
//!    only the paths it knows: pack replaces its verification-scratch paths, the airlock
//!    replaces the final prefix and the restore-scratch paths that survive inside NUL-fixed
//!    binaries (relocation deliberately skips those, D5's invariant 7).
//!
//! ## What is content-verified and what is not
//!
//! * `conda-meta/*.json` package records and `conda-meta/history` are **presence-only**: a
//!   record embeds install-scratch paths *and* `sha256_in_prefix` values that are hashes of
//!   the relocated files — functions of the host's paths, not normalisable. Their presence is
//!   still exact, which is what the forged-record attack needs to be caught by.
//! * `conda-meta/pixi_env_prefix`, `conda-meta/.pixi-environment-fingerprint`, and the
//!   `etc/conda/activate.d/pixi-sandbox-cargo-home.*` hooks are written by `restore` itself,
//!   so they are excluded from the list and allowed as extras on disk; the fingerprint is
//!   checked separately against the manifest's recorded value.

use crate::error::{Error, Result};
use crate::manifest::check_rel_path;
use crate::shard::sha256_bytes;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// File name of the per-env file list, relative to the environment directory
/// (i.e. `envs/<name>/files.json` inside the transport payload).
pub const FILE_NAME: &str = "files.json";

/// What each side replaces its known prefix paths with before hashing.
pub const SENTINEL: &[u8] = b"@pixi-sandbox-prefix@";

/// Schema of the list document itself (independent of the manifest's schema).
pub const DOC_SCHEMA: u32 = 1;

/// Files `restore` writes into a live prefix, so a staged tree never contains their final
/// content. Excluded from the list; allowed as extras when verifying.
pub const RESTORE_MARKERS: [&str; 5] = [
    "conda-meta/pixi_env_prefix",
    "conda-meta/.pixi-environment-fingerprint",
    "etc/conda/activate.d/pixi-sandbox-cargo-home.sh",
    "etc/conda/activate.d/pixi-sandbox-cargo-home.ps1",
    "etc/conda/activate.d/pixi-sandbox-cargo-home.bat",
];

/// Files that may appear in a restored prefix without being in the list: pixi's own
/// bookkeeping, written by the first `pixi install` into any prefix that lacks it (measured:
/// `conda-meta/pixi`, `conda-meta/history`), plus the markers and activation hooks `restore`
/// owns. Anything else extra is a failure — that is what rejects a hand-forged record.
pub const ALLOWED_EXTRAS: [&str; 7] = [
    "conda-meta/pixi",
    "conda-meta/history",
    "conda-meta/pixi_env_prefix",
    "conda-meta/.pixi-environment-fingerprint",
    "etc/conda/activate.d/pixi-sandbox-cargo-home.sh",
    "etc/conda/activate.d/pixi-sandbox-cargo-home.ps1",
    "etc/conda/activate.d/pixi-sandbox-cargo-home.bat",
];

/// One file (or symlink) of the unpacked environment tree, relative to the prefix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    /// Path relative to the prefix, `/`-separated.
    pub p: String,
    /// sha256 of the canonicalised content. Absent for presence-only entries (conda-meta
    /// records and history), whose content is a function of the installing host's paths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<String>,
    /// Executable bit (unix). A prefix whose modes were lost is a broken prefix.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub x: bool,
    /// Present iff this entry is a symlink: the (canonicalised) link target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub l: Option<String>,
}

/// The parsed `files.json` of one environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesDoc {
    pub schema: u32,
    /// The exclusions this list was built with, so a reader can tell an old list that
    /// excluded less apart from a tree that genuinely carries an extra file.
    pub excluded: Vec<String>,
    pub files: Vec<FileEntry>,
}

impl FilesDoc {
    /// Parse and validate. The rules are the manifest's rules: relative safe paths and
    /// well-formed digests, because this document is trusted the same way.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let doc: FilesDoc = serde_json::from_slice(bytes)
            .map_err(|e| Error::Invalid(format!("files manifest: {e}")))?;
        doc.validate()?;
        Ok(doc)
    }

    pub fn validate(&self) -> Result<()> {
        if self.schema != DOC_SCHEMA {
            return Err(Error::Invalid(format!(
                "files manifest schema {} is not supported (expected {DOC_SCHEMA})",
                self.schema
            )));
        }
        if self.files.is_empty() {
            return Err(Error::Invalid("files manifest lists no files".to_string()));
        }
        let mut seen = std::collections::BTreeSet::new();
        for entry in &self.files {
            check_rel_path(&entry.p, "files manifest entry")?;
            if !seen.insert(entry.p.clone()) {
                return Err(Error::Invalid(format!(
                    "files manifest lists {} twice",
                    entry.p
                )));
            }
            if let Some(digest) = &entry.h {
                if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(Error::Invalid(format!(
                        "files manifest {}: not a sha256 digest ({digest})",
                        entry.p
                    )));
                }
            }
            if entry.l.is_some() && entry.h.is_some() {
                return Err(Error::Invalid(format!(
                    "files manifest {}: a symlink cannot also carry a content digest",
                    entry.p
                )));
            }
        }
        Ok(())
    }

    /// Serialise the way `pack` writes the file: pretty JSON with a final newline, so the
    /// bytes (and therefore the manifest's blob digest) are stable.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| Error::Other(format!("serialising files manifest: {e}")))?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    pub fn entries(&self) -> usize {
        self.files.len()
    }
}

/// Path of an environment's file list inside the transport payload, relative to
/// [`crate::manifest::MANIFEST_DIR`].
pub fn list_rel_path(env: &str) -> String {
    format!("envs/{env}/{FILE_NAME}")
}

/// The canonical form in which pack-time and restore-time trees hash equally: NUL runs
/// collapsed, then every known prefix-path spelling replaced by [`SENTINEL`].
pub fn canonicalise(bytes: &[u8], candidates: &[Vec<u8>]) -> Vec<u8> {
    let mut out = collapse_nul_runs(bytes);
    for candidate in candidates {
        if !candidate.is_empty() && contains(&out, candidate) {
            out = replace_all(&out, candidate, SENTINEL);
        }
    }
    out
}

/// sha256 of the canonicalised content — the digest a [`FileEntry`] records.
pub fn canonical_sha256(bytes: &[u8], candidates: &[Vec<u8>]) -> String {
    sha256_bytes(&canonicalise(bytes, candidates))
}

fn collapse_nul_runs(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == 0 {
            out.push(0);
            while at < bytes.len() && bytes[at] == 0 {
                at += 1;
            }
        } else {
            let start = at;
            while at < bytes.len() && bytes[at] != 0 {
                at += 1;
            }
            out.extend_from_slice(&bytes[start..at]);
        }
    }
    out
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    find_at(haystack, needle, 0).is_some()
}

fn find_at(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    let first = needle[0];
    let mut at = from;
    while at + needle.len() <= haystack.len() {
        if haystack[at] == first && &haystack[at..at + needle.len()] == needle {
            return Some(at);
        }
        at += 1;
    }
    None
}

fn replace_all(haystack: &[u8], needle: &[u8], replacement: &[u8]) -> Vec<u8> {
    // Deliberately dependency-free (memmem would be faster, but the airlock builds from the
    // vendored tree and D6 keeps that tree exactly as locked — no new crates for a scan that
    // runs once per pack and once per verify).
    let mut out = Vec::with_capacity(haystack.len());
    let mut at = 0;
    while at < haystack.len() {
        if let Some(found) = find_at(haystack, needle, at) {
            out.extend_from_slice(&haystack[at..found]);
            out.extend_from_slice(replacement);
            at = found + needle.len();
        } else {
            out.extend_from_slice(&haystack[at..]);
            break;
        }
    }
    out
}

/// A walked prefix: regular files and symlinks (never followed), both keyed by their
/// `/`-separated path relative to the prefix, sorted for deterministic output.
#[derive(Debug, Default)]
pub struct Walked {
    pub files: BTreeMap<String, PathBuf>,
    pub symlinks: BTreeMap<String, PathBuf>,
}

pub fn walk_prefix(prefix: &Path) -> Result<Walked> {
    let mut walked = Walked::default();
    for entry in walkdir::WalkDir::new(prefix).sort_by_file_name() {
        let entry = entry.map_err(|e| Error::Other(e.to_string()))?;
        if entry.depth() == 0 {
            continue;
        }
        let rel = rel_of(prefix, entry.path())?;
        if entry.file_type().is_symlink() {
            walked.symlinks.insert(rel, entry.path().to_path_buf());
        } else if entry.file_type().is_file() {
            walked.files.insert(rel, entry.path().to_path_buf());
        }
    }
    Ok(walked)
}

fn rel_of(prefix: &Path, path: &Path) -> Result<String> {
    let rel = path
        .strip_prefix(prefix)
        .map_err(|_| Error::Invalid(format!("{} escapes {}", path.display(), prefix.display())))?;
    let mut out = String::new();
    for component in rel.components() {
        match component {
            std::path::Component::Normal(part) => {
                let part = part.to_str().ok_or_else(|| {
                    Error::Invalid(format!(
                        "{}: a file name in the prefix is not valid UTF-8",
                        path.display()
                    ))
                })?;
                if !out.is_empty() {
                    out.push('/');
                }
                out.push_str(part);
            }
            // walkdir under a prefix cannot produce these; refusing keeps the stored paths
            // trustworthy even if that assumption ever changes.
            _ => {
                return Err(Error::Invalid(format!(
                    "{}: unsafe component under {}",
                    path.display(),
                    prefix.display()
                )));
            }
        }
    }
    Ok(out)
}

/// Is this path one of the files `restore` writes itself? Its content is a function of the
/// restoring host, never of the transport, so it cannot be part of the oracle.
pub fn is_restore_marker(rel: &str) -> bool {
    RESTORE_MARKERS.contains(&rel)
}

/// Presence-only entries: conda package records and conda's history. A record's content
/// embeds the install-time scratch paths *and* `sha256_in_prefix` digests of the relocated
/// files — both differ per host by construction, so only presence is checkable. Presence is
/// still exact, which is what catching a forged record requires.
fn is_presence_only(rel: &str) -> bool {
    (rel.starts_with("conda-meta/") && rel.ends_with(".json")) || rel == "conda-meta/history"
}

/// Scan a staged prefix into a [`FilesDoc`] plus the tree's size in bytes (the honest
/// `unpacked_size_bytes` for the manifest: measured on what restore will actually produce).
///
/// `candidates` are the prefix-path spellings this side must neutralise — at pack time the
/// verification stage's prefix and the pack copy pixi-unpack read from.
pub fn scan_prefix(prefix: &Path, candidates: &[Vec<u8>]) -> Result<(FilesDoc, u64)> {
    let walked = walk_prefix(prefix)?;
    let mut files = Vec::with_capacity(walked.files.len() + walked.symlinks.len());
    let mut bytes = 0u64;

    for (rel, path) in &walked.files {
        if is_restore_marker(rel) {
            continue;
        }
        bytes += fs::metadata(path).map_err(|e| Error::io(path, e))?.len();
        let digest = if is_presence_only(rel) {
            None
        } else {
            let raw = fs::read(path).map_err(|e| Error::io(path, e))?;
            Some(canonical_sha256(&raw, candidates))
        };
        files.push(FileEntry {
            p: rel.clone(),
            h: digest,
            x: is_executable(path)?,
            l: None,
        });
    }

    for (rel, path) in &walked.symlinks {
        if is_restore_marker(rel) {
            continue;
        }
        let target = fs::read_link(path).map_err(|e| Error::io(path, e))?;
        // Symlink targets are relative in a conda prefix, but an absolute one would name the
        // prefix and is canonicalised the same way file content is.
        let canonical = canonicalise(target.to_string_lossy().as_bytes(), candidates);
        files.push(FileEntry {
            p: rel.clone(),
            h: None,
            x: false,
            l: Some(String::from_utf8_lossy(&canonical).into_owned()),
        });
    }

    let doc = FilesDoc {
        schema: DOC_SCHEMA,
        excluded: RESTORE_MARKERS.iter().map(|s| s.to_string()).collect(),
        files,
    };
    doc.validate()?;
    Ok((doc, bytes))
}

pub fn is_executable(path: &Path) -> Result<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        Ok(fs::metadata(path)
            .map_err(|e| Error::io(path, e))?
            .permissions()
            .mode()
            & 0o111
            != 0)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::{ALLOWED_EXTRAS, SENTINEL, canonicalise, collapse_nul_runs, replace_all};
    use std::fs;
    use std::path::Path;

    #[test]
    fn nul_runs_collapse_to_a_single_nul() {
        assert_eq!(collapse_nul_runs(b"ab"), b"ab");
        assert_eq!(collapse_nul_runs(b"a\0\0\0\0b"), b"a\0b");
        assert_eq!(collapse_nul_runs(b"\0\0"), b"\0");
        assert_eq!(collapse_nul_runs(b"\0a\0\0b\0"), b"\0a\0b\0");
        // A single NUL is a single NUL: integers in binaries keep their meaning.
        assert_eq!(collapse_nul_runs(b"a\0b"), b"a\0b");
    }

    #[test]
    fn replacements_cover_text_and_padded_binary_fields() {
        let stage = b"/scratch/stage/env".to_vec();
        let final_path = b"/home/user/project/.pixi/envs/env".to_vec();

        // A text file names its prefix once.
        let text_stage = format!("prefix={}\n", String::from_utf8_lossy(&stage));
        let text_final = format!("prefix={}\n", String::from_utf8_lossy(&final_path));
        assert_eq!(
            canonicalise(text_stage.as_bytes(), std::slice::from_ref(&stage)),
            canonicalise(text_final.as_bytes(), std::slice::from_ref(&final_path)),
            "text: both sides must hash the same canonical bytes"
        );

        // A fixed-width binary field: the prefix is NUL-padded to the field width, so the
        // padding length follows the path length. Collapsing NULs makes both sides equal.
        let field_stage = [stage.as_slice(), b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"].concat();
        let field_final = [final_path.as_slice(), b"\0"].concat();
        assert_eq!(
            canonicalise(&field_stage, std::slice::from_ref(&stage)),
            canonicalise(&field_final, std::slice::from_ref(&final_path)),
            "binary: padding must not leak the path length"
        );

        // The sentinel itself is what both canonical forms contain, in place of the path.
        let canonical = canonicalise(text_stage.as_bytes(), std::slice::from_ref(&stage));
        assert_eq!(canonical, [b"prefix=", SENTINEL, b"\n"].concat());
    }

    #[test]
    fn multiple_candidates_and_repeated_occurrences_all_neutralise() {
        let stage = b"/s/env".to_vec();
        let pack = b"/s/pack".to_vec();
        let text = b"/s/env/bin:/s/env/lib:/s/pack/cache/x:/other";
        let canonical = replace_all(&replace_all(text, &stage, SENTINEL), &pack, SENTINEL);
        assert_eq!(
            canonical,
            [
                SENTINEL,
                b"/bin:",
                SENTINEL,
                b"/lib:",
                SENTINEL,
                b"/cache/x:/other"
            ]
            .concat()
        );
    }

    #[test]
    fn allowed_extras_are_only_restore_or_pixi_bookkeeping() {
        // The allowlist is the attack surface of the oracle: everything in it escapes content
        // verification, so it must stay exactly the files pixi and restore write themselves.
        for rel in ALLOWED_EXTRAS {
            assert!(
                rel.starts_with("conda-meta/")
                    || rel.starts_with("etc/conda/activate.d/pixi-sandbox-cargo-home."),
                "{rel}: the allowlist must not reach outside pixi/restore-owned bookkeeping"
            );
        }
    }

    #[test]
    fn scanning_a_staged_prefix_produces_a_verifiable_list() {
        let temp = tempfile::tempdir().unwrap();
        let prefix = temp.path().join("env");
        fs::create_dir_all(prefix.join("bin")).unwrap();
        fs::create_dir_all(prefix.join("conda-meta")).unwrap();

        let staged = prefix.to_string_lossy().into_owned().into_bytes();
        fs::create_dir_all(prefix.join("lib")).unwrap();
        fs::write(prefix.join("bin/tool"), b"#!/bin/sh\necho hi\n").unwrap();
        fs::write(
            prefix.join("lib/pkgconfig.pc"),
            format!("prefix={}\n", String::from_utf8_lossy(&staged)),
        )
        .unwrap();
        fs::write(
            prefix.join("conda-meta/records.json"),
            format!(
                "{{\"url\": \"file://{}/channel/x.conda\"}}",
                String::from_utf8_lossy(&staged)
            ),
        )
        .unwrap();
        fs::write(prefix.join("conda-meta/history"), "// log\n").unwrap();
        // restore's own markers: excluded, never listed
        fs::write(prefix.join("conda-meta/pixi_env_prefix"), "irrelevant").unwrap();
        fs::write(
            prefix.join("conda-meta/.pixi-environment-fingerprint"),
            "0123456789abcdef",
        )
        .unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(prefix.join("bin/tool"), fs::Permissions::from_mode(0o755))
                .unwrap();
        }

        let (doc, bytes) = super::scan_prefix(&prefix, std::slice::from_ref(&staged)).unwrap();
        let by_path: std::collections::BTreeMap<&str, &super::FileEntry> = doc
            .files
            .iter()
            .map(|entry| (entry.p.as_str(), entry))
            .collect();
        assert_eq!(
            doc.excluded,
            super::RESTORE_MARKERS
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(by_path.len(), 4, "markers excluded: {:?}", by_path.keys());
        assert!(by_path["bin/tool"].x);
        assert!(by_path["bin/tool"].h.is_some());
        // the .pc file's digest is over the sentinel form
        let pc = fs::read(prefix.join("lib/pkgconfig.pc")).unwrap();
        assert_eq!(
            by_path["lib/pkgconfig.pc"].h.as_deref(),
            Some(super::canonical_sha256(&pc, &[staged]).as_str()),
        );
        // records and history are presence-only
        assert!(by_path["conda-meta/records.json"].h.is_none());
        assert!(by_path["conda-meta/history"].h.is_none());
        assert!(bytes > 0);
        doc.validate().unwrap();
    }

    #[test]
    fn parsing_rejects_a_forged_or_malformed_list() {
        let bad = [
            r#"{"schema": 2, "excluded": [], "files": [{"p": "a"}]}"#, // future schema
            r#"{"schema": 1, "excluded": [], "files": []}"#,           // nothing listed
            r#"{"schema": 1, "excluded": [], "files": [{"p": "../escape"}]}"#,
            r#"{"schema": 1, "excluded": [], "files": [{"p": "a"}, {"p": "a"}]}"#,
            r#"{"schema": 1, "excluded": [], "files": [{"p": "a", "h": "nope"}]}"#,
            r#"{"schema": 1, "excluded": [], "files": [{"p": "a", "h": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08", "l": "../x"}]}"#,
        ];
        for text in bad {
            assert!(
                super::FilesDoc::parse(text.as_bytes()).is_err(),
                "must be refused: {text}"
            );
        }
        let good = r#"{"schema": 1, "excluded": ["conda-meta/pixi_env_prefix"], "files": [{"p": "bin/tool", "h": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08", "x": true}]}"#;
        super::FilesDoc::parse(good.as_bytes()).unwrap();
    }

    #[test]
    fn the_list_path_is_inside_the_environment_directory() {
        assert_eq!(super::list_rel_path("demo"), "envs/demo/files.json");
        assert!(!Path::new(&super::list_rel_path("demo")).is_absolute());
    }
}
