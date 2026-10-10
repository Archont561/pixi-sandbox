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
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes are not valid JSON or fail validation: a non-relative
    /// or unsafe path, or a malformed digest.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let doc: FilesDoc = serde_json::from_slice(bytes)
            .map_err(|e| Error::Invalid(format!("files manifest: {e}")))?;
        doc.validate()?;
        Ok(doc)
    }

    /// The manifest's path and digest rules, applied to this document.
    ///
    /// # Errors
    ///
    /// Returns an error naming the first entry that violates the rules (a non-relative or
    /// unsafe path, or a malformed digest).
    pub fn validate(&self) -> Result<()> {
        self.validate_labelled("files manifest entry")
    }

    /// The same rules, worded for the side that produced the entries: at pack time the
    /// list is being scanned off disk and no manifest exists yet, so a path rejection must
    /// name the scanned file rather than the document it was going to be written into
    /// (issue #95's diagnosis detour).
    fn validate_labelled(&self, what: &str) -> Result<()> {
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
            check_rel_path(&entry.p, what)?;
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
    ///
    /// # Errors
    ///
    /// Returns an error if the document cannot be serialised (a fixed shape, so this is
    /// not expected in practice — the signature stays honest rather than infallible).
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| Error::Other(format!("serialising files manifest: {e}")))?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    #[must_use]
    pub fn entries(&self) -> usize {
        self.files.len()
    }
}

/// Path of an environment's file list inside the transport payload, relative to
/// [`crate::manifest::MANIFEST_DIR`].
#[must_use]
pub fn list_rel_path(env: &str) -> String {
    format!("envs/{env}/{FILE_NAME}")
}

/// The canonical form in which pack-time and restore-time trees hash equally: NUL runs
/// collapsed, then every known prefix-path spelling replaced by [`SENTINEL`].
#[must_use]
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
#[must_use]
pub fn canonical_sha256(bytes: &[u8], candidates: &[Vec<u8>]) -> String {
    sha256_bytes(&canonicalise(bytes, candidates))
}

#[doc(hidden)] // test boundary: the NUL-run rule the canonical form is built on (tests/files_manifest.rs)
#[must_use]
pub fn collapse_nul_runs(bytes: &[u8]) -> Vec<u8> {
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

#[doc(hidden)] // test boundary: the neutralisation step both entry points share (tests/files_manifest.rs)
#[must_use]
pub fn replace_all(haystack: &[u8], needle: &[u8], replacement: &[u8]) -> Vec<u8> {
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

/// Walk `prefix` into the plain files and symlinks it contains, relative paths, sorted.
///
/// # Errors
///
/// Returns an error if the tree cannot be walked (a missing or unreadable prefix).
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
#[must_use]
pub fn is_restore_marker(rel: &str) -> bool {
    RESTORE_MARKERS.contains(&rel)
}

/// Presence-only entries: conda package records and conda's history. A record's content
/// embeds the install-time scratch paths *and* `sha256_in_prefix` digests of the relocated
/// files — both differ per host by construction, so only presence is checkable. Presence is
/// still exact, which is what catching a forged record requires.
fn is_presence_only(rel: &str) -> bool {
    (rel.starts_with("conda-meta/")
        && Path::new(rel)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json")))
        || rel == "conda-meta/history"
}

/// Scan a staged prefix into a [`FilesDoc`] plus the tree's size in bytes (the honest
/// `unpacked_size_bytes` for the manifest: measured on what restore will actually produce).
///
/// `candidates` are the prefix-path spellings this side must neutralise — at pack time the
/// verification stage's prefix and the pack copy pixi-unpack read from.
///
/// # Errors
///
/// Returns an error if the prefix cannot be walked, a file cannot be read or hashed, or a
/// scanned path violates the manifest's path rules.
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
        excluded: RESTORE_MARKERS
            .iter()
            .map(std::string::ToString::to_string)
            .collect(),
        files,
    };
    doc.validate_labelled("scanned environment file")?;
    Ok((doc, bytes))
}

/// The executable bit of `path` (false on platforms without one).
///
/// # Errors
///
/// Returns an error if the path's metadata cannot be read.
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
