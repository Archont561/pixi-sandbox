//! Cargo vendoring policy (D6): refuse collisions before creating a transport.

use crate::pack::{sum_files, support};
use anyhow::{Context, Result, bail};
use pixi_sandbox_core::manifest::{MANIFEST_DIR, Vendor};
use pixi_sandbox_core::shard;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;

/// A `--cargo-vendor` preflight: the lockfile must exist, and no crate+version may be
/// reachable from two sources.
///
/// # Errors
///
/// Returns an error if the lockfile is missing, unreadable, or names a crate+version
/// reachable from two sources.
pub fn validate_vendorable_lockfile(root: &Path) -> Result<()> {
    let cargo_lock = root.join("Cargo.lock");
    if !cargo_lock.is_file() {
        bail!(
            "--cargo-vendor needs a Cargo.lock at {}",
            cargo_lock.display()
        );
    }
    reject_duplicate_crate_sources(&cargo_lock)
}

/// Refuse a lockfile in which one crate+version is reachable from two different sources.
///
/// `cargo vendor` cannot represent this: it maps each crate to a `<name>-<version>` directory
/// under the vendor root, so two entries for the same pair collide and cargo aborts with
/// "found duplicate version of package ... vendored from two sources" and no remedy (design.md
/// §11, known upstream). Left alone that surfaces as a pack failure on the connected side at
/// best, and at worst as a transport that packs "successfully" around a missing crate and only
/// breaks on the airlock, where cargo reports nothing useful.
///
/// The remedy is a maintainer decision, not something to guess at, so the error names the
/// crates and the two sources and states the two real ways out. Path (workspace) members have
/// no `source` in the lockfile and are not vendored, so they are ignored; two entries for the
/// same crate+version from the *same* source are a normal lockfile artefact, not this bug.
fn reject_duplicate_crate_sources(cargo_lock: &Path) -> Result<()> {
    let text = fs::read_to_string(cargo_lock)
        .with_context(|| format!("reading {}", cargo_lock.display()))?;
    let lock: toml::Value =
        toml::from_str(&text).with_context(|| format!("parsing {}", cargo_lock.display()))?;

    let Some(packages) = lock.get("package").and_then(toml::Value::as_array) else {
        // A lockfile with no [[package]] entries cannot have duplicates.
        return Ok(());
    };

    // (name, version) -> the distinct sources it is reachable from, in first-seen order.
    let mut origins: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for package in packages {
        let (Some(name), Some(version)) = (
            package.get("name").and_then(toml::Value::as_str),
            package.get("version").and_then(toml::Value::as_str),
        ) else {
            continue;
        };
        // No `source` means a path/workspace member: not vendored, so not a duplicate source.
        let Some(source) = package.get("source").and_then(toml::Value::as_str) else {
            continue;
        };
        let seen = origins
            .entry((name.to_string(), version.to_string()))
            .or_default();
        if !seen.iter().any(|existing| existing == source) {
            seen.push(source.to_string());
        }
    }

    let duplicates = origins
        .into_iter()
        .filter(|(_, sources)| sources.len() > 1)
        .collect::<Vec<_>>();
    if duplicates.is_empty() {
        return Ok(());
    }

    let detail = duplicates
        .iter()
        .map(|((name, version), sources)| {
            let list = sources
                .iter()
                .map(|source| format!("\n      - {source}"))
                .collect::<String>();
            format!(
                "    {name} {version} is reachable from {} sources:{list}",
                sources.len()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    bail!(
        "cannot vendor: {} crate(s) are reachable from more than one source in {}:\n{detail}\n\n\
         `cargo vendor` stores every crate as <name>-<version> under one vendor root, so two \
         sources for the same pair collide and it aborts with no remedy. An airlock would then \
         find a crate missing with nothing pointing at the cause.\n\
         Fix it on the connected side, before packing:\n  \
           - make the versions differ, so each source provides a distinct pair; or\n  \
           - drop one of the two dependencies, if the crate is reachable from the other \
         source anyway.\n\
         Patching the vendored tree is not a fix: the next `cargo update` reintroduces the \
         collision.",
        duplicates.len(),
        cargo_lock.display()
    );
}

/// Cargo trees stay loose for deduplication, or use one archive per crate (D6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VendorMode {
    Loose,
    Tarballs,
}

/// Toolchain provenance recorded in the branch's human-readable guide.
#[derive(Debug)]
pub struct VendorInfo {
    pub cargo: String,
    pub rustc: String,
}

/// External programs used for vendoring. Supplying paths keeps tests independent of PATH
/// without mocking the file policy; the CLI uses the same cargo/rustc names as before.
#[derive(Debug)]
pub struct Toolchain {
    pub cargo: PathBuf,
    pub rustc: PathBuf,
}

impl Default for Toolchain {
    fn default() -> Self {
        Self {
            cargo: PathBuf::from("cargo"),
            rustc: PathBuf::from("rustc"),
        }
    }
}

/// Vendor a validated project using cargo and rustc from the caller's environment.
///
/// # Errors
///
/// Returns an error if cargo or rustc cannot be run, or the vendored tree cannot be
/// written.
pub fn vendor_tree(
    root: &Path,
    out: &Path,
    payload: &Path,
    mode: VendorMode,
) -> Result<(Option<Vendor>, Option<VendorInfo>)> {
    vendor_tree_with_toolchain(root, out, payload, mode, &Toolchain::default())
}

/// The same vendoring operation with explicit external-program paths.
///
/// # Errors
///
/// Returns an error if the named cargo or rustc cannot be run, or the vendored tree
/// cannot be written.
pub fn vendor_tree_with_toolchain(
    root: &Path,
    out: &Path,
    payload: &Path,
    mode: VendorMode,
    toolchain: &Toolchain,
) -> Result<(Option<Vendor>, Option<VendorInfo>)> {
    let cargo_lock = root.join("Cargo.lock");
    // Already validated in `run` before the output tree was created; re-checked here so this
    // function stays correct if it is ever called on its own.
    validate_vendorable_lockfile(root)?;

    let vendor_dir = payload.join("vendor");
    let source_dir = match mode {
        VendorMode::Loose => vendor_dir.clone(),
        VendorMode::Tarballs => out.join(".cargo-vendor-tmp"),
    };
    support::remove_path(&source_dir)?;
    let mut command = Command::new(&toolchain.cargo);
    command
        .current_dir(root)
        .arg("vendor")
        .arg("--locked")
        .arg("--versioned-dirs")
        .arg(&source_dir);
    support::run(&mut command).context("running cargo vendor")?;

    let crates = crate_directories(&source_dir)?;
    if crates.is_empty() {
        bail!("cargo vendor produced no crate directories");
    }
    if matches!(mode, VendorMode::Tarballs) {
        fs::create_dir_all(&vendor_dir)
            .with_context(|| format!("creating {}", vendor_dir.display()))?;
        for crate_dir in &crates {
            let name = crate_dir.file_name().ok_or_else(|| {
                anyhow::anyhow!("vendor directory without a name: {}", crate_dir.display())
            })?;
            let archive_path = vendor_dir.join(format!("{}.tar", name.to_string_lossy()));
            let archive = File::create(&archive_path)
                .with_context(|| format!("creating {}", archive_path.display()))?;
            let mut builder = tar::Builder::new(archive);
            builder
                .append_dir_all(name, crate_dir)
                .with_context(|| format!("archiving {}", crate_dir.display()))?;
            builder
                .finish()
                .with_context(|| format!("finishing {}", archive_path.display()))?;
        }
        support::remove_path(&source_dir)?;
    }

    let files = shard::files_under(&vendor_dir)?;
    let size = sum_files(&files)?;
    let mode_name = match mode {
        VendorMode::Loose => "loose",
        VendorMode::Tarballs => "tarballs",
    };
    println!(
        "  {} crates · {} files · {} MiB · {mode_name}",
        crates.len(),
        files.len(),
        support::mib(size)
    );
    Ok((
        Some(Vendor {
            mode: mode_name.to_string(),
            crates: crates.len() as u64,
            size_bytes: size,
            cargo_lock_sha256: Some(shard::sha256_file(&cargo_lock)?),
            directory: Some(format!("{MANIFEST_DIR}/vendor")),
            blobs: Vec::new(),
        }),
        Some(VendorInfo {
            cargo: command_version(&toolchain.cargo)
                .unwrap_or_else(|_| "cargo (unknown)".to_string()),
            rustc: command_version(&toolchain.rustc)
                .unwrap_or_else(|_| "rustc (unknown)".to_string()),
        }),
    ))
}

fn crate_directories(root: &Path) -> Result<Vec<PathBuf>> {
    let mut directories = fs::read_dir(root)
        .with_context(|| format!("reading cargo vendor output {}", root.display()))?
        .filter_map(std::result::Result::ok)
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(std::fs::FileType::is_dir)
                .map(|_| entry.path())
        })
        .collect::<Vec<_>>();
    directories.sort();
    Ok(directories)
}

fn command_version(program: &Path) -> Result<String> {
    let mut command = Command::new(program);
    command.arg("--version");
    Ok(support::run(&mut command)?.trim().to_string())
}
