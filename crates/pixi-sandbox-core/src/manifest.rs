//! `manifest.json` — the wire format of a sandbox branch (schema 5).
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
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Bumped only for incompatible changes; readers refuse anything newer.
///
/// Schema 2 added `envs.<name>.files` (the per-file oracle for a restored prefix, ADR-13);
/// a schema-1 transport simply has no oracle and `doctor --verify-restored` says so
/// instead of guessing.
///
/// Schema 4 added nested Pixi workspaces (TASK-129): `envs.<id>.workspace` says which nested
/// project an environment belongs to, `workspaces[]` lists the declared set, and a nested
/// environment's id is `<slug>/<env>`. Producers write 4 for **every** transport, root-only
/// bundles included, so there is one writer path rather than two spellings of the same bytes;
/// the accepted cost is that a schema-3-era consumer must upgrade before it can read a transport
/// packed after this change. Readers still accept 1–3 (ADR-24/31).
///
/// Schema 5 adds per-workspace Cargo vendor trees (TASK-131): `workspace_vendors[]` carries one
/// [`Vendor`] tree per nested project that has its own `Cargo.lock`, under
/// `vendor/workspaces/<slug>/` beside the root tree at `vendor/`. The bump is required even
/// though the key is additive: an older binary ignoring it would verify successfully while
/// never checking the nested tree's blobs — a silent partial verification. Producers again
/// write 5 for every transport; readers keep accepting 1–4.
pub const SCHEMA_VERSION: u32 = 5;

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
    /// Explicit executable entries for new producers, e.g. `pixi` and `pixi-sandbox`.
    /// Older manifests may also record a legacy unpacker (ADR-31).
    #[serde(default)]
    pub tools: BTreeMap<String, ToolEntry>,
    /// environment name → packed environment.
    #[serde(default)]
    pub envs: BTreeMap<String, Env>,
    /// Nested Pixi workspaces this transport carries (schema 4, TASK-129). A root-only transport
    /// omits the key entirely, so its manifest stays byte-identical apart from `schema`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workspaces: Vec<WorkspaceEntry>,
    /// Present only when the transport carries `cargo vendor` output (decision ADR-6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor: Option<Vendor>,
    /// One vendored crate tree per nested workspace with its own `Cargo.lock` (schema 5,
    /// TASK-131, ADR-33). Empty for root-only and non-vendored transports, so the key is
    /// omitted and their manifests stay byte-identical apart from `schema`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workspace_vendors: Vec<WorkspaceVendor>,
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
    /// Per-file digests of the unpacked, relocated tree (schema 2, ADR-13). Absent on schema-1
    /// transports, where `doctor --verify-restored` reports the environment as unverifiable
    /// rather than claiming a check it cannot make.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<EnvFiles>,
    /// Set when this environment belongs to a nested workspace (schema 4, TASK-129). The env's
    /// key is then `<slug>/<env>`; everything else about it — the oracle, the blobs, the
    /// fingerprint — is unchanged, because a nested workspace is just another *project root*.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceRef>,
}

/// Which nested project an environment belongs to, as the restore side needs it.
///
/// Repeated per environment rather than looked up in [`Manifest::workspaces`] so that a manifest
/// with one missing list entry cannot silently restore a prefix into the wrong project: the
/// destination is decided by the environment itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRef {
    /// The nested `pixi.toml`, relative to the project root: `subproject/pixi.toml`.
    pub manifest: String,
    /// The nested project's directory, relative to the project root: `subproject`.
    pub directory: String,
}

/// One declared nested workspace, as the transport records it (schema 4, TASK-129).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    /// Transport identity, the first half of every `<slug>/<env>` id it owns.
    pub slug: String,
    /// The nested `pixi.toml`, relative to the project root.
    pub manifest: String,
    /// The nested project's directory, relative to the project root.
    pub directory: String,
    /// Environments this transport carries from it.
    pub environments: Vec<String>,
}

/// The per-file oracle for one environment: `envs/<name>/files.json`.
///
/// It is recorded as a blob so it gets the same verify-before-write treatment as the
/// payload itself (see `files_manifest` for what the digests mean and what they
/// deliberately do not cover).
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

/// Vendored cargo dependencies (`cargo vendor`), keyed by `Cargo.lock` (decision ADR-6).
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

/// One nested workspace's vendored crate tree (schema 5, TASK-131).
///
/// The identity fields mirror [`WorkspaceEntry`] so a tree can never be restored into the wrong
/// project, and the payload reuses [`Vendor`] unchanged: the same mode, lock hash, size and
/// per-file blob rules apply, only the transport directory is workspace-qualified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceVendor {
    /// Transport identity, identical to the workspace's [`WorkspaceEntry::slug`].
    pub slug: String,
    /// The nested `pixi.toml`, relative to the project root: `subproject/pixi.toml`.
    pub manifest: String,
    /// The nested project's directory, relative to the project root: `subproject`.
    pub directory: String,
    /// The vendored tree, whose `directory` is `.pixi-sandbox/vendor/workspaces/<slug>`.
    pub vendor: Vendor,
}

impl WorkspaceVendor {
    /// Identity paths must stay inside the project root, and every blob must live under this
    /// tree's qualified directory, so one workspace's crates can never be verified or restored
    /// as another project's tree. The one-segment directory rule mirrors the check on an
    /// environment's embedded [`WorkspaceRef`]: every declared workspace carries environments,
    /// so transports already enforce this shape by the time a vendor tree can reference one.
    fn validate(&self) -> Result<()> {
        if self.slug.is_empty() {
            return Err(Error::Invalid(
                "workspace vendor with an empty slug".to_string(),
            ));
        }
        check_rel_path(&self.manifest, "workspace vendor manifest")?;
        check_rel_path(&self.directory, "workspace vendor directory")?;
        if self.directory.contains('/') {
            return Err(Error::Invalid(format!(
                "workspace vendor {}: directory must be one path segment: {}",
                self.slug, self.directory
            )));
        }
        let prefix = format!("vendor/workspaces/{}/", self.slug);
        for blob in &self.vendor.blobs {
            check_blob(&format!("vendor/workspaces/{}", self.slug), blob)?;
            if !blob.path.starts_with(&prefix) {
                return Err(Error::Invalid(format!(
                    "workspace vendor {}: blob {} must stay under {prefix}",
                    self.slug, blob.path
                )));
            }
        }
        Ok(())
    }
}

impl Manifest {
    /// Path of the manifest inside a transport directory / extracted branch.
    #[must_use]
    pub fn path_in(branch_location: &Path) -> PathBuf {
        branch_location.join(MANIFEST_DIR).join(MANIFEST_FILE)
    }

    /// Read + validate. `path` must be the manifest itself (see [`Manifest::path_in`]).
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, is not valid JSON, or fails validation.
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
    ///
    /// # Errors
    ///
    /// Returns an error naming the first rule violated: an unsupported schema, a missing
    /// field, an unknown platform, or a malformed blob or tool entry.
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
            if let Some(workspace) = &env.workspace {
                check_rel_path(&workspace.manifest, "workspace manifest")?;
                check_rel_path(&workspace.directory, "workspace directory")?;
                if workspace.directory.contains('/') {
                    return Err(Error::Invalid(format!(
                        "env {name}: workspace directory must be one path segment: {}",
                        workspace.directory
                    )));
                }
                let expected = format!("{}/", workspace.directory.replace('/', "-"));
                if !name.starts_with(&expected) {
                    return Err(Error::Invalid(format!(
                        "env {name}: a nested environment id must start with `{expected}`"
                    )));
                }
            } else if name.strip_suffix(' ').is_some() || name.contains('/') {
                return Err(Error::Invalid(format!(
                    "env {name}: a `/` in an environment id means a nested workspace, but no \
                     workspace descriptor is recorded"
                )));
            }
        }
        for workspace in &self.workspaces {
            if workspace.slug.is_empty() {
                return Err(Error::Invalid("workspace with an empty slug".to_string()));
            }
            check_rel_path(&workspace.manifest, "workspace manifest")?;
            check_rel_path(&workspace.directory, "workspace directory")?;
            if workspace.environments.is_empty() {
                return Err(Error::Invalid(format!(
                    "workspace {}: no environments declared",
                    workspace.slug
                )));
            }
        }
        if let Some(vendor) = &self.vendor {
            for blob in &vendor.blobs {
                check_blob("vendor", blob)?;
            }
        }
        self.validate_workspace_vendors()?;
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

    /// Per-tree shape checks plus the transport-wide identity rules: the restore destination
    /// is slug-derived, so one slug may own exactly one tree, and a tree without a declared
    /// workspace has no legitimate destination. Identity must match the TASK-129 descriptor
    /// exactly — the packer builds both records from the same config entry.
    fn validate_workspace_vendors(&self) -> Result<()> {
        let mut slugs = BTreeSet::new();
        for tree in &self.workspace_vendors {
            tree.validate()?;
            if !slugs.insert(tree.slug.clone()) {
                return Err(Error::Invalid(format!(
                    "workspace vendor {}: duplicate vendor tree for one workspace",
                    tree.slug
                )));
            }
            let known = self.workspaces.iter().any(|workspace| {
                workspace.slug == tree.slug
                    && workspace.manifest == tree.manifest
                    && workspace.directory == tree.directory
            });
            if !known {
                return Err(Error::Invalid(format!(
                    "workspace vendor {}: no declared workspace matches its slug, manifest and \
                     directory",
                    tree.slug
                )));
            }
        }
        Ok(())
    }

    /// Every blob of every (selected) environment, plus the vendored trees when present.
    /// The vendor blobs are not filtered by `only`: a partial `--envs` restore still
    /// needs the crates, so they are verified and materialised either way. The context is
    /// borrowed for the static cases and owned only for a workspace-qualified vendor path.
    #[must_use]
    pub fn blobs(&self, only: Option<&[String]>) -> Vec<(Cow<'_, str>, &Blob)> {
        let mut out = Vec::new();
        for (name, env) in &self.envs {
            if let Some(sel) = only
                && !sel.iter().any(|s| s == name)
            {
                continue;
            }
            for blob in &env.blobs {
                out.push((Cow::Borrowed(name.as_str()), blob));
            }
        }
        if let Some(vendor) = &self.vendor {
            for blob in &vendor.blobs {
                out.push((Cow::Borrowed("vendor"), blob));
            }
        }
        for tree in &self.workspace_vendors {
            for blob in &tree.vendor.blobs {
                out.push((Cow::Owned(format!("vendor/workspaces/{}", tree.slug)), blob));
            }
        }
        out
    }

    /// Where this environment's prefix is materialised inside `project`.
    ///
    /// The single place both `restore` and `doctor --verify-restored` ask, so the tree one
    /// command writes is the tree the other one checks. A root environment lands at
    /// `<project>/.pixi/envs/<env>` exactly as before; a nested one lands in its own project's
    /// Pixi env directory — `<project>/<directory>/.pixi/envs/<env>` — which is the layout
    /// `pixi run --manifest-path …` reads, and the reason a delegated task needs no solve.
    ///
    /// # Errors
    /// Returns an error when a nested environment's id does not carry its workspace slug, which
    /// [`Manifest::validate`] already refuses on a loaded manifest.
    pub fn env_destination(&self, project: &Path, id: &str) -> Result<PathBuf> {
        let env = self.envs.get(id).ok_or_else(|| {
            Error::Invalid(format!("environment {id:?} is not in this transport"))
        })?;
        let name = env_name_of(id, env)?;
        Ok(match &env.workspace {
            None => project.join(".pixi").join("envs").join(name),
            Some(workspace) => project
                .join(&workspace.directory)
                .join(".pixi")
                .join("envs")
                .join(name),
        })
    }

    /// The environment's own name, without the `<slug>/` prefix a nested id carries.
    ///
    /// # Errors
    /// Returns an error for an id this transport does not hold, or one whose prefix does not
    /// match its own workspace descriptor.
    pub fn env_name<'a>(&self, id: &'a str) -> Result<&'a str> {
        let env = self.envs.get(id).ok_or_else(|| {
            Error::Invalid(format!("environment {id:?} is not in this transport"))
        })?;
        env_name_of(id, env)
    }

    /// The restore scratch directory for an environment id, workspace-qualified.
    ///
    /// Root `default` and a nested `subproject/default` must not share a staging directory: the
    /// first failure of a restore leaves the staged packs in place as evidence, and evidence
    /// filed under the wrong name is worse than none.
    #[must_use]
    pub fn staging_component(&self, id: &str) -> String {
        id.replace('/', "--")
    }

    /// The transport directory an environment's blobs are recorded under, with its trailing
    /// slash: `envs/<id>/pack/` for the root, `envs/workspaces/<slug>/<env>/pack/` for a nested
    /// one. Both `pack` records blobs under it and `restore` strips it, so it lives here once.
    ///
    /// # Errors
    /// Returns an error for an id this transport does not hold.
    pub fn pack_prefix(&self, id: &str) -> Result<String> {
        let env = self.envs.get(id).ok_or_else(|| {
            Error::Invalid(format!("environment {id:?} is not in this transport"))
        })?;
        let name = env_name_of(id, env)?;
        Ok(match &env.workspace {
            None => format!("envs/{name}/pack/"),
            Some(workspace) => format!(
                "envs/{}/{}/{}/pack/",
                crate::sandbox_config::WORKSPACES_SEGMENT,
                workspace.directory.replace('/', "-"),
                name
            ),
        })
    }

    /// Bytes the payload occupies inside the branch (packed environments, not unpacked).
    #[must_use]
    pub fn payload_bytes(&self) -> u64 {
        let envs: u64 = self.envs.values().map(|e| e.packed_size_bytes).sum();
        let tools: u64 = self.tools.values().map(|t| t.size_bytes).sum();
        let vendor = self.root_and_workspace_vendor_bytes();
        envs + tools + vendor
    }

    /// (envs, tools, vendor) split of [`Manifest::payload_bytes`], in bytes. The vendor total
    /// is the root tree plus every per-workspace tree (TASK-131): branch bytes are counted
    /// once per tree even when two lockfiles share a crate — git storage dedups, verification
    /// and checkout do not.
    #[must_use]
    pub fn payload_split(&self) -> (u64, u64, u64) {
        (
            self.envs.values().map(|e| e.packed_size_bytes).sum(),
            self.tools.values().map(|t| t.size_bytes).sum(),
            self.root_and_workspace_vendor_bytes(),
        )
    }

    /// Recorded size of the root vendor tree and every per-workspace tree.
    #[must_use]
    fn root_and_workspace_vendor_bytes(&self) -> u64 {
        let root = self.vendor.as_ref().map_or(0, |v| v.size_bytes);
        self.workspace_vendors
            .iter()
            .fold(root, |sum, tree| sum + tree.vendor.size_bytes)
    }

    /// Where a blob lives inside the transport directory.
    #[must_use]
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
#[must_use]
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
    // Every reader of a declared path ends up taking its final component: the shard code
    // joins `file_name()` onto a directory rather than replaying the path as written, so a
    // relative path with no *normal* final component (`.`, `./`) is not a curiosity but a
    // panic on data the branch supplies. Refuse it here, where the rest of the shape rules
    // live, instead of at each of those readers (TASK-80). `a/.` is deliberately still
    // accepted: its components normalise to `a`, which has a name.
    if Path::new(path).file_name().is_none() {
        return Err(Error::Invalid(format!("{what} has no file name: {path}")));
    }
    Ok(())
}

/// The final component of a manifest-declared path, or [`Error::Invalid`] naming the path.
///
/// The fallible twin of [`Path::file_name`] for branch data: [`check_rel_path`] has already
/// refused every path without a final component, so a manifest that loaded through
/// [`Manifest::load`] cannot reach the error branch. Call sites inside the crate still route
/// through here rather than `expect`, because a path is untrusted input and the cost of
/// being wrong is a panic mid-restore (TASK-80 AC#3).
pub(crate) fn rel_path_file_name<'a>(path: &'a str, what: &str) -> Result<&'a std::ffi::OsStr> {
    Path::new(path)
        .file_name()
        .ok_or_else(|| Error::Invalid(format!("{what} has no file name: {path}")))
}

fn check_digest(digest: &str, what: &str) -> Result<()> {
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::Invalid(format!(
            "{what}: not a sha256 digest ({digest})"
        )));
    }
    Ok(())
}

/// The environment's own name, without the `<slug>/` prefix a nested id carries.
fn env_name_of<'a>(id: &'a str, env: &Env) -> Result<&'a str> {
    match &env.workspace {
        None => Ok(id),
        Some(workspace) => {
            let slug = workspace.directory.replace('/', "-");
            id.strip_prefix(&format!("{slug}/")).ok_or_else(|| {
                Error::Invalid(format!(
                    "env {id}: a nested environment id must start with `{slug}/`"
                ))
            })
        }
    }
}
