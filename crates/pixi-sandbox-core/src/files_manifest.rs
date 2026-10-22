//! `envs/<name>/files.json` — the per-file oracle for a restored environment prefix (ADR-13).
//!
//! `doctor --verify` proves the *branch* is intact and `restore` verifies every blob before
//! writing it, but nothing checked the tree that comes out the other end: a prefix that is
//! structurally plausible but wrong passed every check (task-10, measured against a real
//! transport). This module is the fix: at pack time the environment is unpacked once more,
//! with the native rattler/UV installer (or an explicit legacy adapter), and every file
//! of that tree is recorded here with a digest. `verify::verify_restored` then walks a *restored* prefix and
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
//!    binaries (relocation deliberately skips those, ADR-5's invariant 7).
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
//! * A `*.dist-info/RECORD` keeps its digest like any other file, but two of its row classes
//!   describe bytes its own installer wrote rather than bytes the wheel shipped, so those rows
//!   are collapsed before hashing (see `canonical_file_sha256`). Presence, order, and every
//!   other row survive, and the files the collapsed rows name are still content-verified
//!   entries of the list themselves.

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
///
/// * **1** — digests recorded with uv's regenerated install-metadata rows collapsed (issue
///   #128).
/// * **2** — the same, plus the prefix-stamped entry-point script rows (issue #170).
///
/// Digests are only comparable within one rule, so the document names the one that produced
/// it and a reader applies *that* rule — never the newest it happens to know. A transport
/// packed under schema 1 therefore keeps failing exactly where it already failed (the seven
/// rows issue #170 reported); a transport packed under schema 2 needs a reader that knows
/// schema 2, which is why it carries its own tool.
pub const DOC_SCHEMA: u32 = 2;

/// The document schemas this reader can compare digests under. A document from a newer tool
/// is refused rather than checked under a rule it was not recorded with.
pub const SUPPORTED_DOC_SCHEMAS: [u32; 2] = [1, DOC_SCHEMA];

/// Files `restore` writes into a live prefix, so a staged tree never contains their final
/// content. Excluded from the list; allowed as extras when verifying.
pub const RESTORE_MARKERS: [&str; 5] = [
    "conda-meta/pixi_env_prefix",
    "conda-meta/.pixi-environment-fingerprint",
    "etc/conda/activate.d/pixi-sandbox-cargo-home.sh",
    "etc/conda/activate.d/pixi-sandbox-cargo-home.ps1",
    "etc/conda/activate.d/pixi-sandbox-cargo-home.bat",
];

/// Files that may appear in a restored prefix without being in the list.
///
/// They are pixi's own bookkeeping, written by the first `pixi install` into any prefix
/// that lacks it (measured: `conda-meta/pixi`, `conda-meta/history`), plus the markers and
/// activation hooks `restore` owns. Anything else extra is a failure — that is what
/// rejects a hand-forged record.
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
        if !SUPPORTED_DOC_SCHEMAS.contains(&self.schema) {
            return Err(Error::Invalid(format!(
                "files manifest schema {} is not supported (expected one of {SUPPORTED_DOC_SCHEMAS:?})",
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
            if let Some(digest) = &entry.h
                && (digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()))
            {
                return Err(Error::Invalid(format!(
                    "files manifest {}: not a sha256 digest ({digest})",
                    entry.p
                )));
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

/// What a regenerated `RECORD` row collapses to. A distinct sentinel from [`SENTINEL`],
/// which means "this side's spelling of the prefix", so the two can never be confused for
/// one another in a digest or a diff.
const REGENERATED: &[u8] = b"@pixi-sandbox-regenerated@";

/// What a `RECORD` row collapses to when the bytes it describes were stamped with *this
/// side's* install prefix. A third distinct sentinel: such a row is neither a path spelling
/// ([`SENTINEL`]) nor rewritten on every install ([`REGENERATED`]) — the installer wrote a
/// digest of a file that names the prefix it happened to be installed into.
const PREFIX_STAMPED: &[u8] = b"@pixi-sandbox-prefix-stamped@";

/// sha256 of the canonical form of the file at `rel` — the digest both sides compute, and
/// the only one a regular file is checked by — under the row rule [`DOC_SCHEMA`] records.
#[must_use]
pub fn canonical_file_sha256(rel: &str, bytes: &[u8], candidates: &[Vec<u8>]) -> String {
    canonical_file_sha256_with(RecordRows::for_schema(DOC_SCHEMA), rel, bytes, candidates)
}

/// sha256 of the canonical form of the file at `rel`, under the row rule `rows`.
///
/// `rel` is what makes the form path-aware: a `*.dist-info/RECORD` carries rows describing
/// bytes the *installer* wrote rather than bytes the wheel shipped, so those rows are
/// collapsed before hashing (see [`collapse_installer_rows`]). Everything else about the
/// file is transported content and stays covered.
#[must_use]
pub fn canonical_file_sha256_with(
    rows: RecordRows,
    rel: &str,
    bytes: &[u8],
    candidates: &[Vec<u8>],
) -> String {
    let canonical = canonicalise(bytes, candidates);
    let hashed = if is_dist_info_record(rel) {
        collapse_installer_rows(rows, rel, &canonical)
    } else {
        canonical
    };
    sha256_bytes(&hashed)
}

/// Is this the install metadata uv writes into every `.dist-info` it installs and rewrites
/// on every install: its cache record (a wall-clock timestamp) or the wheel URL it resolved
/// through (`direct_url.json`)?
#[must_use]
pub fn is_uv_install_metadata(rel: &str) -> bool {
    UV_INSTALL_METADATA
        .iter()
        .any(|name| is_under_dist_info(rel, name))
}

/// Is this a wheel's own `RECORD` — the CSV listing the package's installed files with
/// each one's hash and size?
#[must_use]
pub fn is_dist_info_record(rel: &str) -> bool {
    is_under_dist_info(rel, "RECORD")
}

/// The files uv writes as its own install metadata. Both are functions of *when and where*
/// the install ran — the cache record's timestamp and the wheel URL `direct_url.json`
/// records — so both are presence-only and both have their `RECORD` row collapsed. One list,
/// so the presence rule and the `RECORD`-row rule can never drift apart.
const UV_INSTALL_METADATA: [&str; 2] = ["uv_cache.json", "direct_url.json"];

/// Is `rel` a file called `name` sitting directly in a `*.dist-info` directory? The
/// directory qualifier is load-bearing: a file that merely shares the name elsewhere in
/// the prefix is transported content.
fn is_under_dist_info(rel: &str, name: &str) -> bool {
    rel.strip_suffix(name)
        .is_some_and(|dir| dir.ends_with(".dist-info/"))
}

/// May `rel` appear in a restored prefix without being in the oracle's list?
///
/// The bookkeeping files pixi and restore own, plus uv's install metadata (issue #128): uv
/// writes `uv_cache.json` only for installs it routes through its cache, and
/// `direct_url.json` when it reinstalls a wheel. A transport packed by an older tool — or
/// from a different uv — can legitimately produce either file without it being listed.
/// Anything else extra is still a failure, which is what rejects a hand-forged record.
#[must_use]
pub fn is_allowed_extra(rel: &str) -> bool {
    ALLOWED_EXTRAS.contains(&rel) || is_uv_install_metadata(rel)
}

/// Which `*.dist-info/RECORD` rows a document's digests treat as installer-written rather
/// than transported content.
///
/// Selected by [`FilesDoc::schema`], because digests are only comparable within one rule: a
/// reader that applied the newest rule to an older document would call every one of its
/// digests wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordRows {
    /// uv's regenerated install-metadata rows only — schema 1, issue #128.
    Regenerated,
    /// Those rows, plus the entry-point scripts the installer stamps with the install prefix
    /// — schema 2, issue #170.
    RegeneratedAndStamped,
}

impl RecordRows {
    /// The rule a document of `schema` was recorded under.
    ///
    /// Schemas above [`DOC_SCHEMA`] never reach a digest: [`FilesDoc::parse`] refuses them
    /// before a single entry is compared.
    #[must_use]
    pub fn for_schema(schema: u32) -> Self {
        if schema >= 2 {
            Self::RegeneratedAndStamped
        } else {
            Self::Regenerated
        }
    }
}

/// Collapse the rows of a `RECORD` that no honest restore can reproduce byte for byte,
/// leaving line endings, the row order and every other row untouched — so a `RECORD` that
/// differs in anything real still hashes differently.
///
/// Two row classes qualify, and both are written by the *installer* rather than shipped in
/// the wheel:
///
/// * uv's own install metadata ([`UV_INSTALL_METADATA`]) — rewritten on every install, hash
///   and size included (issue #128).
/// * the entry-point scripts the installer writes into the prefix's script directory
///   (`<prefix>/bin/<name>`, and `<prefix>/Scripts/<name>` on Windows) — schema 2 only. Their
///   shebang names the install prefix, so the recorded hash and size are functions of the
///   *staging* prefix that pack's verification unpack and the airlock's restore never share
///   (issue #170). The script file itself is unaffected: it is compared in canonical form,
///   where the prefix spelling is [`SENTINEL`].
///
/// The script rows are recognised by the exact path the installer writes — resolved from the
/// directory holding the `.dist-info`, one `../` per component between it and the prefix — so
/// a row naming a file anywhere else in the tree, or at any other depth, stays transported
/// content.
fn collapse_installer_rows(rows: RecordRows, rel: &str, bytes: &[u8]) -> Vec<u8> {
    let script_prefixes = match rows {
        RecordRows::Regenerated => Vec::new(),
        RecordRows::RegeneratedAndStamped => prefix_script_row_prefixes(rel),
    };
    let mut out = Vec::with_capacity(bytes.len());
    let mut rest = bytes;
    while !rest.is_empty() {
        let (line, tail) = match rest.iter().position(|b| *b == b'\n') {
            Some(at) => rest.split_at(at + 1),
            None => (rest, &[][..]),
        };
        out.extend_from_slice(&collapse_installer_row(line, &script_prefixes));
        rest = tail;
    }
    out
}

/// The spellings that name a script directly inside the prefix's script directory, read from
/// the directory holding this `.dist-info`: one `../` per component between the two, then
/// `bin/` or `Scripts/`.
#[must_use]
fn prefix_script_row_prefixes(rel: &str) -> Vec<Vec<u8>> {
    // `rel` is `<site-packages>/<name>.dist-info/RECORD`. A `RECORD` row resolves from
    // `<site-packages>`, so the climb to the prefix root is one `../` per component of it.
    let site_packages = rel
        .rsplit_once('/')
        .and_then(|(dist_info, _record)| dist_info.rsplit_once('/'))
        .map(|(site_packages, _dist_info_name)| site_packages);
    let Some(site_packages) = site_packages else {
        return Vec::new();
    };
    let depth = site_packages
        .split('/')
        .filter(|component| !component.is_empty())
        .count();
    if depth == 0 {
        return Vec::new();
    }
    let up = "../".repeat(depth);
    ["bin/", "Scripts/"]
        .iter()
        .map(|script_dir| format!("{up}{script_dir}").into_bytes())
        .collect()
}

/// Is `field` a row naming a script directly inside the prefix's script directory — the
/// spelling the installer writes for `<prefix>/bin/<name>` (or `Scripts/<name>`)?
#[must_use]
fn is_prefix_script_row(field: &[u8], prefix: &[u8]) -> bool {
    let Some(name) = field.strip_prefix(prefix) else {
        return false;
    };
    !name.is_empty() && !name.contains(&b'/')
}

/// One `RECORD` line: if it names a row class the installer owns, keep the path field and
/// replace the hash and size that describe those generated bytes.
fn collapse_installer_row(line: &[u8], script_prefixes: &[Vec<u8>]) -> Vec<u8> {
    let (body, ending) = match line.strip_suffix(b"\n") {
        Some(body) => {
            let ending: &[u8] = if body.ends_with(b"\r") {
                b"\r\n"
            } else {
                b"\n"
            };
            (body.strip_suffix(b"\r").unwrap_or(body), ending)
        }
        None => (line, &[][..]),
    };
    let field_len = body.iter().position(|b| *b == b',').unwrap_or(body.len());
    let field = &body[..field_len];
    let name = field.rsplit(|b| *b == b'/').next().unwrap_or(field);
    let sentinel = if UV_INSTALL_METADATA
        .iter()
        .any(|metadata| metadata.as_bytes() == name)
    {
        REGENERATED
    } else if script_prefixes
        .iter()
        .any(|prefix| is_prefix_script_row(field, prefix))
    {
        PREFIX_STAMPED
    } else {
        return line.to_vec();
    };
    let mut out = Vec::with_capacity(line.len());
    out.extend_from_slice(field);
    out.push(b',');
    out.extend_from_slice(sentinel);
    out.push(b',');
    out.extend_from_slice(ending);
    out
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
    // vendored tree and ADR-6 keeps that tree exactly as locked — no new crates for a scan that
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

/// Presence-only entries: conda package records and conda's history, plus uv's own install
/// metadata. A conda record's content embeds the install-time scratch paths *and*
/// `sha256_in_prefix` digests of the relocated files — both differ per host by
/// construction — and uv's `uv_cache.json`/`direct_url.json` are a wall-clock timestamp and
/// the resolved wheel path (issue #128), so only presence is checkable for any of them.
/// Presence is still exact, which is what catching a forged record requires.
fn is_presence_only(rel: &str) -> bool {
    (rel.starts_with("conda-meta/")
        && Path::new(rel)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json")))
        || rel == "conda-meta/history"
        || is_uv_install_metadata(rel)
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
            Some(canonical_file_sha256(rel, &raw, candidates))
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
