//! Pack preflight and transport-file assembly, reusable outside the binary target.
//!
//! The command resolves CLI options and orchestrates the phases; this module owns the path
//! and environment-name refusals that must happen before any output directory exists.

use anyhow::{Context, Result, bail};
use pixi_sandbox_core::host_requirements::HostRequirementSet;
use pixi_sandbox_core::manifest::{EnvFiles, MANIFEST_DIR};
use pixi_sandbox_core::sandbox_config::SandboxConfig;
use pixi_sandbox_core::shard;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute, validated paths and the byte shard limit for a fresh transport.
#[derive(Debug)]
pub struct PackLayout {
    pub root: PathBuf,
    pub out: PathBuf,
    pub payload: PathBuf,
    pub shard_limit: u64,
}

/// Resolve pack's paths and refuse missing locks, stale output, and unsafe selections.
/// No directory is created by preflight.
///
/// # Errors
///
/// Returns an error if `repo_root` is not a directory, a required lockfile is missing, the
/// output directory is stale, or a selection is unsafe.
pub fn plan_layout(
    repo_root: &Path,
    output_dir: &Path,
    envs: &[String],
    shard_limit_mib: f64,
) -> Result<PackLayout> {
    let root = support::existing_dir(repo_root, "--repo-root")?;
    let out = support::absolute(output_dir)?;
    let payload = out.join(MANIFEST_DIR);
    let shard_limit = shard_limit_bytes(shard_limit_mib)?;

    if !root.join("pixi.lock").is_file() {
        bail!(
            "{} has no pixi.lock — run `pixi install` before packing",
            root.display()
        );
    }
    if out.exists() {
        bail!(
            "{} already exists — remove it first rather than packing into a stale transport",
            out.display()
        );
    }
    validate_env_names(envs)?;

    Ok(PackLayout {
        root,
        out,
        payload,
        shard_limit,
    })
}

fn shard_limit_bytes(mebibytes: f64) -> Result<u64> {
    if !mebibytes.is_finite() || mebibytes <= 0.0 {
        bail!("--shard-limit-mib must be a positive finite number");
    }
    let bytes = mebibytes * 1024.0 * 1024.0;
    // 2^64 — the same value `u64::MAX as f64` rounds to — spelled as a literal so the
    // bound check itself carries no cast.
    if bytes > 18_446_744_073_709_551_616.0 {
        bail!("--shard-limit-mib is too large");
    }
    // The guards above bound `bytes` to (0, 2^64): the f64->u64 cast saturates rather
    // than wraps, so the limit can never silently truncate.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let bytes = bytes as u64;
    Ok(bytes)
}

fn validate_env_names(envs: &[String]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for name in envs {
        let path = Path::new(name);
        if name.is_empty()
            || name == "."
            || name == ".."
            || path.is_absolute()
            || path.components().count() != 1
        {
            bail!("unsafe environment name {name:?}");
        }
        if !seen.insert(name) {
            bail!("environment {name:?} was requested more than once");
        }
    }
    Ok(())
}

/// Resolve the `[host_requirements]` a manifest for `platform` carries.
///
/// `None` — meaning no section in the manifest at all — when no config was named, when the
/// config declares no table, or when nothing resolves for that platform's host family. That is
/// the backward-compatible half of the contract: a project that declares nothing keeps packing
/// the same bytes earlier releases packed.
///
/// # Errors
///
/// Returns an error if the config cannot be read or parsed, or names a platform the
/// manifest does not carry.
pub fn resolve_host_requirements(
    config: Option<&Path>,
    platform: &str,
) -> Result<Option<HostRequirementSet>> {
    let Some(config) = config else {
        return Ok(None);
    };
    let path = support::absolute(config)?;
    let config = SandboxConfig::load(&path)
        .with_context(|| format!("loading sandbox config {}", path.display()))?;
    Ok(config
        .host_requirements_for(platform)?
        .filter(|set| !set.is_empty()))
}

/// Build the per-file oracle for one environment (D13).
///
/// The unpack uses the pinned unpacker — exactly what `restore` will do on the airlock —
/// and records the tree it produces, with every prefix-path spelling canonicalised away
/// (see `files_manifest`).
///
/// The unpack runs on a *copy* of the pack: pixi-unpack writes its extraction cache into the
/// pack directory it reads from, and the payload tree must stay exactly what pixi-pack
/// produced. The scratch lives inside `out` (never `/tmp`, invariant 3) and is removed before
/// returning, so a successful pack leaves no trace of it.
///
/// # Errors
///
/// Returns an error if the verification unpack cannot be created or run, or the oracle
/// cannot be recorded.
pub fn build_files_oracle(
    out: &Path,
    payload: &Path,
    env: &str,
    pack: &Path,
    unpacker: &Path,
    shard_limit: u64,
) -> Result<(EnvFiles, u64)> {
    let scratch = out.join(format!(".pixi-sandbox-verify-{env}"));
    support::remove_path(&scratch)?;
    let pack_copy = scratch.join("pack");
    let stage = scratch.join("stage");
    fs::create_dir_all(scratch.join("tmp"))
        .with_context(|| format!("creating {}", scratch.join("tmp").display()))?;
    fs::create_dir_all(&pack_copy).with_context(|| format!("creating {}", pack_copy.display()))?;
    fs::create_dir_all(&stage).with_context(|| format!("creating {}", stage.display()))?;

    for file in shard::files_under(pack)? {
        let relative = file
            .strip_prefix(pack)
            .with_context(|| format!("{} is outside {}", file.display(), pack.display()))?;
        let destination = pack_copy.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::copy(&file, &destination)
            .with_context(|| format!("copying {} for the verification unpack", file.display()))?;
    }

    let mut command = Command::new(unpacker);
    command
        .arg(&pack_copy)
        .arg("-o")
        .arg(&stage)
        .arg("-e")
        .arg(env);
    support::use_work_tmp(&mut command, &scratch);
    support::run(&mut command).with_context(|| {
        format!(
            "verification-unpacking environment {env} with {}",
            unpacker.display()
        )
    })?;

    let prefix = stage.join(env);
    if !prefix.is_dir() {
        bail!(
            "the verification unpack completed but did not create {}",
            prefix.display()
        );
    }

    // The paths this side must neutralise: the stage prefix the unpacker stamped in, and the
    // pack copy it installed from — in both literal and canonical form, so a symlinked
    // scratch directory cannot defeat the canonicalisation.
    let mut candidates: Vec<Vec<u8>> = Vec::new();
    let mut push_candidate = |path: &Path| {
        let bytes = path.to_string_lossy().into_owned().into_bytes();
        if !bytes.is_empty() && !candidates.contains(&bytes) {
            candidates.push(bytes);
        }
    };
    push_candidate(&prefix);
    if let Ok(canonical) = prefix.canonicalize() {
        push_candidate(&canonical);
    }
    push_candidate(&pack_copy);
    if let Ok(canonical) = pack_copy.canonicalize() {
        push_candidate(&canonical);
    }

    let (doc, unpacked_bytes) =
        pixi_sandbox_core::files_manifest::scan_prefix(&prefix, &candidates)
            .context("scanning the verification-unpacked environment")?;
    support::remove_path(&scratch)?;

    let relative = pixi_sandbox_core::files_manifest::list_rel_path(env);
    let list_path = payload.join(&relative);
    let encoded = doc.to_bytes()?;
    fs::write(&list_path, &encoded).with_context(|| format!("writing {}", list_path.display()))?;
    let blob = shard::record_file(payload, &relative, shard_limit)
        .with_context(|| format!("recording {}", list_path.display()))?;
    Ok((
        EnvFiles {
            blob,
            entries: doc.entries() as u64,
        },
        unpacked_bytes,
    ))
}

/// Record every file under `root` (relative to `payload`) as transport blobs.
///
/// # Errors
///
/// Returns an error if the payload tree cannot be walked or a file cannot be recorded.
pub fn record_tree(
    payload: &Path,
    root: &Path,
    shard_limit: u64,
) -> Result<Vec<pixi_sandbox_core::manifest::Blob>> {
    let mut blobs = Vec::new();
    for path in shard::files_under(root)? {
        let relative = path
            .strip_prefix(payload)
            .with_context(|| format!("{} is outside {}", path.display(), payload.display()))?
            .to_string_lossy()
            .replace('\\', "/");
        blobs.push(shard::record_file(payload, &relative, shard_limit)?);
    }
    Ok(blobs)
}

/// Total size of `paths` in bytes.
///
/// # Errors
///
/// Returns an error if any path's metadata cannot be read, or the total overflows u64.
pub fn sum_files(paths: &[PathBuf]) -> Result<u64> {
    paths.iter().try_fold(0u64, |sum, path| {
        let size = fs::metadata(path)
            .with_context(|| format!("reading metadata for {}", path.display()))?
            .len();
        sum.checked_add(size)
            .ok_or_else(|| anyhow::anyhow!("size overflow while reading {}", path.display()))
    })
}

#[must_use]
pub fn fingerprint_of(root: &Path, env: &str) -> Option<String> {
    let marker = root
        .join(".pixi")
        .join("envs")
        .join(env)
        .join("conda-meta")
        .join(".pixi-environment-fingerprint");
    fs::read_to_string(marker)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Shared command primitives. The binary re-exports these instead of keeping a second
/// implementation, so extraction cannot change subprocess diagnostics or scratch handling.
#[doc(hidden)]
pub mod support {
    use anyhow::{Context, Result, bail};
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    /// Return an absolute path without requiring that it exists yet.
    pub fn absolute(path: &Path) -> Result<PathBuf> {
        if path.is_absolute() {
            Ok(path.to_path_buf())
        } else {
            Ok(env::current_dir()
                .context("reading the current directory")?
                .join(path))
        }
    }

    /// Canonicalise an existing directory and explain errors in terms of the user's path.
    pub fn existing_dir(path: &Path, what: &str) -> Result<PathBuf> {
        let absolute = absolute(path)?;
        if !absolute.is_dir() {
            bail!("{} is not a directory: {}", what, absolute.display());
        }
        absolute
            .canonicalize()
            .with_context(|| format!("canonicalising {}", absolute.display()))
    }

    /// Run a child process, returning combined textual output. On failure the command and both
    /// streams are kept in the error — a bare exit code is not actionable on an airlock.
    pub fn run(command: &mut Command) -> Result<String> {
        let shown = format!("{command:?}");
        command.stdin(Stdio::null());
        let output = command
            .output()
            .with_context(|| format!("starting {shown}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut combined = stdout.into_owned();
        if !stderr.is_empty() {
            if !combined.is_empty() && !combined.ends_with('\n') {
                combined.push('\n');
            }
            combined.push_str(&stderr);
        }

        if !output.status.success() {
            let rendered = combined.trim();
            if rendered.is_empty() {
                bail!("command failed ({}) : {shown}", output.status);
            }
            bail!("command failed ({}) : {shown}\n{rendered}", output.status);
        }
        Ok(combined)
    }

    /// Remove either a directory tree or a single file if it exists. This is used only for paths
    /// controlled by the current command (`work` stages or an explicit `--force` destination).
    pub fn remove_path(path: &Path) -> Result<()> {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
        };
        if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
            fs::remove_dir_all(path).with_context(|| format!("removing {}", path.display()))?;
        } else {
            fs::remove_file(path).with_context(|| format!("removing {}", path.display()))?;
        }
        Ok(())
    }

    /// Set all conventional temporary-directory variables on a child process.
    pub fn use_work_tmp(command: &mut Command, work: &Path) {
        let temporary = work.join("tmp");
        command
            .env("TMPDIR", &temporary)
            .env("TMP", &temporary)
            .env("TEMP", temporary);
    }

    /// A MiB rendering for logs. The u64->f64 cast can lose the low bits of huge byte
    /// counts; that is inherent to a display conversion and harmless here.
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn mib(bytes: u64) -> String {
        format!("{:.1}", bytes as f64 / (1024.0 * 1024.0))
    }
}
