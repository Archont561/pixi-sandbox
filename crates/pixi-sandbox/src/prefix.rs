//! Shared staged-prefix destination and replacement policy for restore and raw unpack.
//!
//! Callers own installation, final-prefix relocation, oracle verification and markers.
//! This module only checks the destination and promotes a caller-verified stage.

use anyhow::{Context, Result, bail};
use std::{fs, path::Path};

/// Refuse an occupied destination unless replacement was explicitly requested.
/// Empty real directories are accepted without force; symlinks are not empty directories.
///
/// # Errors
/// Returns the existing command diagnostics for an unreadable or occupied destination.
pub fn check_output(output: &Path, force: bool) -> Result<()> {
    match fs::symlink_metadata(output) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).with_context(|| format!("reading {}", output.display())),
        Ok(metadata) => {
            let empty_directory = metadata.file_type().is_dir()
                && !metadata.file_type().is_symlink()
                && fs::read_dir(output)
                    .with_context(|| format!("reading {}", output.display()))?
                    .next()
                    .is_none();
            if !empty_directory && !force {
                bail!(
                    "{} already exists and is not empty — pass --force to replace it",
                    output.display()
                );
            }
        }
    }
    Ok(())
}

/// Do not retire an existing output until installation succeeds. Keep a same-parent
/// backup until the stage rename succeeds, including when --work-dir crosses devices.
///
/// # Errors
/// Returns the promotion error after attempting rollback. If rollback also fails, retains
/// the previous output and reports its recovery path rather than deleting the backup.
pub fn promote_prefix(prefix: &Path, output: &Path, force: bool) -> Result<()> {
    check_output(output, force)?;
    if fs::symlink_metadata(output).is_err() {
        return fs::rename(prefix, output)
            .with_context(|| format!("promoting unpacked prefix to {}", output.display()));
    }
    let parent = output.parent().context("output has no parent")?;
    let backup_dir = tempfile::Builder::new()
        .prefix(".pixi-sandbox-unpack-")
        .tempdir_in(parent)?;
    let backup = backup_dir.path().join("previous");
    fs::rename(output, &backup).context("retiring previous unpack output")?;
    if let Err(error) = fs::rename(prefix, output) {
        if let Err(rollback) = fs::rename(&backup, output) {
            let saved = backup_dir.keep();
            bail!(
                "promotion failed: {error}; rollback failed: {rollback}; previous output retained in {}",
                saved.join("previous").display()
            );
        }
        return Err(error).context("promoting unpacked prefix; previous output restored");
    }
    backup_dir
        .close()
        .context("removing previous unpack output")
}
