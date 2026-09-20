//! `manifest.json` — the wire format of a sandbox branch (schema 1).
//!
//! This file is the *only* source of truth about a transport: what environments it holds,
//! which files make them up, what each file's sha256 is, which tools are embedded, and where
//! the vendored crates came from. `README.md`/`AGENTS.md` on the branch are generated from it.
//!
//! Changing anything here changes a wire format: bump [`SCHEMA_VERSION`], update the fixture
//! in `tests/manifest.rs`, and keep `doctor` able to read the previous version.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Bumped only for incompatible changes; readers refuse anything newer.
pub const SCHEMA_VERSION: u32 = 1;

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
        if self.schema != SCHEMA_VERSION {
            return Err(Error::Invalid(format!(
                "manifest schema {} is not supported by this build (expected {SCHEMA_VERSION})",
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

/// Rules every declared blob must satisfy, wherever it lives (an env or the vendor tree).
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

fn check_rel_path(path: &str, what: &str) -> Result<()> {
    if path.is_empty() {
        return Err(Error::Invalid(format!("empty {what}")));
    }
    if path.starts_with('/') || path.starts_with('\\') || path.contains(':') {
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
