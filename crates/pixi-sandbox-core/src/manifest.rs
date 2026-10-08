//! `manifest.json` — the wire format of a sandbox branch (schema 2).
//!
//! This file is the *only* source of truth about a transport: what environments it holds,
//! which files make them up, what each file's sha256 is, which tools are embedded, and where
//! the vendored crates came from. `README.md`/`AGENTS.md` on the branch are generated from it.
//!
//! Changing anything here changes a wire format: bump [`SCHEMA_VERSION`], update the fixture
//! in `tests/manifest.rs`, and keep `doctor` able to read the previous version — a published
//! branch outlives the binary that packed it, so readers accept every schema they understand
//! and refuse only newer ones.

use crate::error::{Error, Result};
use crate::host_requirements::HostRequirementSet;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Bumped only for incompatible changes; readers refuse anything newer. Schema 2 added
/// `envs.<name>.files` (the per-file oracle for a restored prefix, D13); a schema-1
/// transport simply has no oracle and `doctor --verify-restored` says so instead of guessing.
pub const SCHEMA_VERSION: u32 = 2;

/// Directory that holds the payload inside a transport / on a branch.
pub const MANIFEST_DIR: &str = ".pixi-sandbox";

/// File name of the manifest itself, relative to [`MANIFEST_DIR`].
pub const MANIFEST_FILE: &str = "manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    /// Which `pixi-sandbox` wrote this file (name + version, not a sha).
    pub tool: ToolInfo,
    /// RFC 3339, UTC. Informational only — never used for decisions.
    pub created_at: String,
    pub platform: String,
    /// Files above this size were split into `.partNNN` (GitHub blocks blobs > 100 MiB).
    pub shard_limit_bytes: u64,
    pub source: Source,
    /// name → tool, e.g. `pixi`, `pixi-unpack`, `pixi-sandbox` (decision D4).
    #[serde(default)]
    pub tools: BTreeMap<String, ToolEntry>,
    /// environment name → packed environment.
    #[serde(default)]
    pub envs: BTreeMap<String, Env>,
    /// Present only when the transport carries `cargo vendor` output (decision D6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor: Option<Vendor>,
    /// What this transport's workloads need from the *host*, resolved for [`Manifest::platform`]
    /// (issue #109, TASK-75): a libc floor, host packages, services, capabilities and headless
    /// display providers. Additive within schema 2 and written only when the project declared a
    /// `[host_requirements]` table, so a transport without the section stays byte-identical to
    /// what earlier releases packed, and a reader that predates the field ignores it rather than
    /// refusing the manifest. Nothing here is installed by a restore; the section exists so a
    /// restored branch can still say what it expects from the machine that runs it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_requirements: Option<HostRequirementSet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Source {
    /// Commit the payload was built from, when the packer could see one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// sha256 of `pixi.lock` — the real identity of the environment set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lock_sha256: Option<String>,
}

/// A tool embedded in the branch, ready to run with no network and no install step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEntry {
    pub version: String,
    /// Where it came from (recorded for provenance; the airlock never fetches it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// sha256 from the embedded or explicitly overridden reviewed tool pins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_sha256: Option<String>,
    /// `static` | `dynamic` | `script` | `system` — `dynamic` is a bug (see `verify.rs`).
    pub linkage: String,
    pub size_bytes: u64,
    /// Path inside the transport, relative to [`MANIFEST_DIR`]: `tools/linux-64/pixi`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Env {
    pub platform: String,
    /// Directory holding the local conda channel, e.g. `.pixi-sandbox/envs/dev/pack`.
    pub pack_path: String,
    pub packed_size_bytes: u64,
    pub unpacked_size_bytes: u64,
    /// pixi's `conda-meta/.pixi-environment-fingerprint` value (16 hex chars), if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pixi_environment_fingerprint: Option<String>,
    pub blobs: Vec<Blob>,
    /// Per-file digests of the unpacked, relocated tree (schema 2, D13). Absent on schema-1
    /// transports, where `doctor --verify-restored` reports the environment as unverifiable
    /// rather than claiming a check it cannot make.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<EnvFiles>,
}

/// The per-file oracle for one environment: `envs/<name>/files.json`, recorded as a blob so
/// it gets the same verify-before-write treatment as the payload itself (see
/// `files_manifest` for what the digests mean and what they deliberately do not cover).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvFiles {
    /// The list itself, shipped inside the transport.
    pub blob: Blob,
    /// Number of per-file entries, so a summary can be checked without reading the list.
    pub entries: u64,
}

/// One file of the payload. `parts` is empty unless the file exceeded the shard limit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blob {
    /// Path relative to [`MANIFEST_DIR`] (e.g. `envs/dev/pack/channel/noarch/x.conda`).
    pub path: String,
    pub size: u64,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    /// Path of the part relative to [`MANIFEST_DIR`], e.g. `<blob>.part000`.
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

/// Vendored cargo dependencies (`cargo vendor`), keyed by `Cargo.lock` (decision D6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vendor {
    /// `loose` (default: git dedups files) or `tarballs` (one archive per crate).
    pub mode: String,
    pub crates: u64,
    pub size_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_lock_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    /// One entry per vendored file, so the tree gets the same verify-before-write
    /// treatment as the environments (invariant 1 — the vendor tree is the largest
    /// contributor of *files* to a branch, and a half-verified tree would be the
    /// easiest place for a corrupted transport to hide).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blobs: Vec<Blob>,
}

impl Manifest {
    /// Path of the manifest inside a transport directory / extracted branch.
    pub fn path_in(branch_location: &Path) -> PathBuf {
        branch_location.join(MANIFEST_DIR).join(MANIFEST_FILE)
    }

    /// Read + validate. `path` must be the manifest itself (see [`Manifest::path_in`]).
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
        let manifest: Manifest =
            serde_json::from_slice(&bytes).map_err(|e| Error::InvalidManifest {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// The rules that make a manifest safe to act on. Everything here is cheap; the
    /// expensive checks (hashing 250 MiB) live in [`crate::verify`].
    pub fn validate(&self) -> Result<()> {
        // A published branch outlives the binary that wrote it: an airlock may restore a
        // transport packed by an older release (which simply carries fewer fields), but a
        // newer schema than this build understands is refused — it could mean anything.
        if !schema_supported(self.schema) {
            return Err(Error::Invalid(format!(
                "manifest schema {} is not supported by this build (understands 1..={SCHEMA_VERSION})",
                self.schema
            )));
        }
        if self.envs.is_empty() && self.tools.is_empty() {
            return Err(Error::Invalid(
                "manifest declares no environments and no tools".to_string(),
            ));
        }
        for (name, env) in &self.envs {
            if name.is_empty() {
                return Err(Error::Invalid("environment with an empty name".to_string()));
            }
            check_rel_path(&env.pack_path, "pack_path")?;
            if env.blobs.is_empty() {
                return Err(Error::Invalid(format!("env {name}: no blobs declared")));
            }
            for blob in &env.blobs {
                check_blob(&format!("env {name}"), blob)?;
            }
            if let Some(files) = &env.files {
                check_blob(&format!("env {name} files"), &files.blob)?;
                if files.entries == 0 {
                    return Err(Error::Invalid(format!(
                        "env {name}: files manifest declares no entries"
                    )));
                }
            }
        }
        if let Some(vendor) = &self.vendor {
            for blob in &vendor.blobs {
                check_blob("vendor", blob)?;
            }
        }
        for (name, tool) in &self.tools {
            if let Some(path) = &tool.path {
                check_rel_path(path, "tool path")?;
            }
            if let Some(digest) = &tool.pinned_sha256 {
                check_digest(digest, name)?;
            }
        }
        Ok(())
    }

    /// Every blob of every (selected) environment, plus the vendored tree when present.
    /// The vendor blobs are not filtered by `only`: a partial `--envs` restore still
    /// needs the crates, so they are verified and materialised either way.
    pub fn blobs(&self, only: Option<&[String]>) -> Vec<(&str, &Blob)> {
        let mut out = Vec::new();
        for (name, env) in &self.envs {
            if let Some(sel) = only {
                if !sel.iter().any(|s| s == name) {
                    continue;
                }
            }
            for blob in &env.blobs {
                out.push((name.as_str(), blob));
            }
        }
        if let Some(vendor) = &self.vendor {
            for blob in &vendor.blobs {
                out.push(("vendor", blob));
            }
        }
        out
    }

    /// Bytes the payload occupies inside the branch (packed environments, not unpacked).
    pub fn payload_bytes(&self) -> u64 {
        let envs: u64 = self.envs.values().map(|e| e.packed_size_bytes).sum();
        let tools: u64 = self.tools.values().map(|t| t.size_bytes).sum();
        let vendor = self.vendor.as_ref().map(|v| v.size_bytes).unwrap_or(0);
        envs + tools + vendor
    }

    /// (envs, tools, vendor) split of [`Manifest::payload_bytes`], in bytes.
    pub fn payload_split(&self) -> (u64, u64, u64) {
        (
            self.envs.values().map(|e| e.packed_size_bytes).sum(),
            self.tools.values().map(|t| t.size_bytes).sum(),
            self.vendor.as_ref().map(|v| v.size_bytes).unwrap_or(0),
        )
    }

    /// Where a blob lives inside the transport directory.
    pub fn blob_abs_path(&self, branch_location: &Path, blob: &Blob) -> PathBuf {
        branch_location.join(MANIFEST_DIR).join(&blob.path)
    }
}

/// Rules every declared blob must satisfy, wherever it lives (an env, its files manifest,
/// or the vendor tree).
fn check_blob(context: &str, blob: &Blob) -> Result<()> {
    check_rel_path(&blob.path, "blob path")?;
    check_digest(&blob.sha256, &blob.path)?;
    if !blob.parts.is_empty() {
        let part_total: u64 = blob.parts.iter().map(|p| p.size).sum();
        if part_total != blob.size {
            return Err(Error::Invalid(format!(
                "{context}: {}: parts sum to {part_total} bytes but the blob is {}",
                blob.path, blob.size
            )));
        }
        for part in &blob.parts {
            check_rel_path(&part.path, "part path")?;
            check_digest(&part.sha256, &part.path)?;
        }
    }
    Ok(())
}

/// True when this build can act on the manifest's schema (see [`SCHEMA_VERSION`]).
pub fn schema_supported(schema: u32) -> bool {
    (1..=SCHEMA_VERSION).contains(&schema)
}

pub(crate) fn check_rel_path(path: &str, what: &str) -> Result<()> {
    if path.is_empty() {
        return Err(Error::Invalid(format!("empty {what}")));
    }
    // A Windows drive prefix (`C:…`, any ASCII letter) is absolute in disguise; a colon
    // anywhere else is a legal POSIX byte — perl's module man pages are
    // `man/man3/App::Cpan.3` — so only the drive shape rejects, never the bare colon
    // (issue #95: an environment that resolved perl could not pack).
    let bytes = path.as_bytes();
    let drive_prefix = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    if path.starts_with('/') || path.starts_with('\\') || drive_prefix {
        return Err(Error::Invalid(format!("{what} must be relative: {path}")));
    }
    if Path::new(path)
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(Error::Invalid(format!(
            "{what} escapes the transport directory: {path}"
        )));
    }
    Ok(())
}

fn check_digest(digest: &str, what: &str) -> Result<()> {
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::Invalid(format!(
            "{what}: not a sha256 digest ({digest})"
        )));
    }
    Ok(())
}
