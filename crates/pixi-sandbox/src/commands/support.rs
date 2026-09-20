//! Shared, deliberately small building blocks for commands that touch a transport.
//!
//! The important policy lives here rather than being copied between `pack`, `unpack`, and
//! `restore`: use absolute paths for subprocesses, verify a transport before writing into a
//! project, stage under the destination filesystem, and preserve the useful output from a
//! failed external tool.

use anyhow::{Context, Result, bail};
use pixi_sandbox_core::manifest::Manifest;
use pixi_sandbox_core::verify::{self, Report};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// Return an absolute path without requiring that it exists yet.
pub(crate) fn absolute(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(env::current_dir()
            .context("reading the current directory")?
            .join(path))
    }
}

/// Canonicalise an existing directory and explain errors in terms of the user's path.
pub(crate) fn existing_dir(path: &Path, what: &str) -> Result<PathBuf> {
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
pub(crate) fn run(command: &mut Command) -> Result<String> {
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

/// Locate an executable by name without invoking a shell. Accept a direct path as well, which
/// makes `--unpacker` and test fixtures deterministic.
pub(crate) fn find_executable(name: impl AsRef<OsStr>) -> Option<PathBuf> {
    let name = Path::new(name.as_ref());
    if name.components().count() > 1 || name.is_absolute() {
        return is_executable(name).then(|| name.to_path_buf());
    }

    let path = env::var_os("PATH")?;
    #[cfg(unix)]
    let suffixes = vec![String::new()];
    #[cfg(windows)]
    let suffixes = {
        let pathext = env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
        let mut suffixes = vec![String::new()];
        suffixes.extend(
            pathext
                .split(';')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_ascii_lowercase()),
        );
        suffixes
    };
    #[cfg(not(any(unix, windows)))]
    let suffixes = vec![String::new()];

    for dir in env::split_paths(&path) {
        for suffix in &suffixes {
            let candidate = if suffix.is_empty() {
                dir.join(name)
            } else {
                let file = format!("{}{}", name.to_string_lossy(), suffix);
                dir.join(file)
            };
            if is_executable(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Make a copied tool executable on platforms with Unix permission bits. Windows executable
/// assets retain their extension and need no chmod equivalent.
pub(crate) fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)
            .with_context(|| format!("reading permissions for {}", path.display()))?
            .permissions();
        permissions.set_mode(permissions.mode() | 0o111);
        fs::set_permissions(path, permissions)
            .with_context(|| format!("making {} executable", path.display()))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

/// Remove either a directory tree or a single file if it exists. This is used only for paths
/// controlled by the current command (`work` stages or an explicit `--force` destination).
pub(crate) fn remove_path(path: &Path) -> Result<()> {
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

/// The manifest's environment map is sorted, so the default selection is stable in logs and
/// machine output. An explicit unknown name is always an error rather than a silent skip.
pub(crate) fn select_envs(manifest: &Manifest, requested: &[String]) -> Result<Vec<String>> {
    if requested.is_empty() {
        return Ok(manifest.envs.keys().cloned().collect());
    }
    let missing: Vec<&str> = requested
        .iter()
        .filter(|name| !manifest.envs.contains_key(name.as_str()))
        .map(String::as_str)
        .collect();
    if !missing.is_empty() {
        let available: Vec<&str> = manifest.envs.keys().map(String::as_str).collect();
        bail!(
            "environment(s) {} are not in this transport (available: {})",
            missing.join(", "),
            available.join(", ")
        );
    }
    Ok(requested.to_vec())
}

/// Run every integrity and linkage check before any command writes into a project.
pub(crate) fn verify_transport(
    manifest: &Manifest,
    branch: &Path,
    selected_envs: Option<&[String]>,
) -> Result<Report> {
    let report = verify::verify(manifest, branch, selected_envs);
    if report.ok() {
        return Ok(report);
    }

    let failures = report
        .failures
        .iter()
        .map(|failure| {
            format!(
                "  {}: {}: {}",
                failure.path,
                failure.kind.as_str(),
                failure.detail
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    bail!(
        "transport verification failed with {} failure(s); nothing was written:\n{}",
        report.failures.len(),
        failures
    );
}

/// Make a staging root on the destination filesystem. `pixi-unpack` uses temporary files too,
/// so its TMPDIR/TMP/TEMP children must live here rather than in a small system `/tmp`.
pub(crate) fn work_dir(project_or_parent: &Path, explicit: Option<&Path>) -> Result<PathBuf> {
    let work = match explicit {
        Some(path) => absolute(path)?,
        None => project_or_parent.join(".pixi").join(".restore-work"),
    };
    fs::create_dir_all(work.join("tmp"))
        .with_context(|| format!("creating restore work directory {}", work.display()))?;
    Ok(work)
}

/// Set all conventional temporary-directory variables on a child process.
pub(crate) fn use_work_tmp(command: &mut Command, work: &Path) {
    let temporary = work.join("tmp");
    command
        .env("TMPDIR", &temporary)
        .env("TMP", &temporary)
        .env("TEMP", temporary);
}

/// Check available space before materialising packs, extracted prefixes, and vendored sources.
pub(crate) fn preflight_space(work: &Path, needed: u64) -> Result<()> {
    let available = fs2::available_space(work)
        .with_context(|| format!("checking free space on {}", work.display()))?;
    println!(
        "  work dir {} · need about {} MiB · {} MiB available",
        work.display(),
        mib(needed),
        mib(available)
    );
    if available < needed {
        bail!(
            "not enough free space on {}: need about {} MiB, have {} MiB",
            work.display(),
            mib(needed),
            mib(available)
        );
    }
    Ok(())
}

/// Strip a manifest namespace without ever accepting an absolute or parent-traversal result.
pub(crate) fn relative_after(path: &str, prefix: &str) -> Result<PathBuf> {
    let remainder = path
        .strip_prefix(prefix)
        .ok_or_else(|| anyhow::anyhow!("manifest path {path:?} is not under {prefix:?}"))?;
    let relative = Path::new(remainder);
    if remainder.is_empty()
        || relative.is_absolute()
        || relative.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        bail!("unsafe manifest-relative path {path:?}");
    }
    Ok(relative.to_path_buf())
}

/// A small, dependency-free RFC 3339 UTC timestamp for the manifest's informational field.
pub(crate) fn now_rfc3339() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let days = seconds.div_euclid(86_400);
    let second_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = second_of_day / 3_600;
    let minute = (second_of_day % 3_600) / 60;
    let second = second_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

// Howard Hinnant's public-domain civil-calendar conversion, with 1970-01-01 as day zero.
fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let month_prime = (5 * doy + 2) / 153;
    let day = doy - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year, month, day)
}

pub(crate) fn mib(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / (1024.0 * 1024.0))
}
