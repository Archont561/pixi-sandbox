//! `wrap_payload` — stage a reviewable recipe directory from a prepared payload (TASK-78, ADR-28).
//!
//! Between the pure plan ([`plan_wrap`](pixi_sandbox_core::payload::plan_wrap)) and the build
//! toolchain there is one effectful step that is worth its own name: writing the recipe the
//! build will consume, beside the payload bytes it refers to. The recipe names `$RECIPE_DIR` and
//! `source: path: payload`, so the payload tree must sit next to `recipe.yaml` — this module
//! copies it there, keeping modes, links and empty directories, and writes the provenance
//! document alongside.
//!
//! Keeping the staging here (rather than inside the toolchain) means the exact bytes the build
//! sees are on disk and inspectable before `rattler-build` runs, and an integration test can
//! judge the staged tree without invoking the tool at all.

use std::path::{Path, PathBuf};

use pixi_sandbox_core::payload::{PAYLOAD_DIR, PayloadError, WrapPlan, write_provenance};

/// The recipe file name `rattler-build` looks for at the root of a recipe directory.
pub const RECIPE_FILE: &str = "recipe.yaml";

/// A staged recipe directory: the recipe, the payload it installs and the provenance document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedRecipe {
    /// The directory `rattler-build` is pointed at.
    pub recipe_dir: PathBuf,
    /// `recipe.yaml` inside [`Self::recipe_dir`].
    pub recipe_path: PathBuf,
    /// `pixi-sandbox-payload.json` inside [`Self::recipe_dir`].
    pub provenance_path: PathBuf,
    /// The payload tree inside [`Self::recipe_dir`] (`payload/`).
    pub payload_dir: PathBuf,
}

/// Write `plan`'s recipe and provenance into `out_dir` and copy `payload_root` beside them.
///
/// A stale `payload/` is removed first so a rerun never carries a file the new plan dropped. The
/// recipe and provenance are overwritten in place.
///
/// # Errors
///
/// [`PayloadError::Io`] for any filesystem failure. Symlinks are recreated verbatim on Unix; on
/// hosts without that support a payload containing a link is refused rather than copied into a
/// blob.
pub fn stage_recipe_dir(
    plan: &WrapPlan,
    payload_root: &Path,
    out_dir: &Path,
) -> Result<StagedRecipe, PayloadError> {
    std::fs::create_dir_all(out_dir).map_err(|error| io_error(out_dir, &error))?;
    let payload_dir = out_dir.join(PAYLOAD_DIR);
    remove_dir_if_present(&payload_dir)?;
    copy_tree(payload_root, &payload_dir)?;

    let recipe_path = out_dir.join(RECIPE_FILE);
    std::fs::write(&recipe_path, &plan.recipe).map_err(|error| io_error(&recipe_path, &error))?;
    let provenance_path = write_provenance(out_dir, &plan.provenance)?;

    Ok(StagedRecipe {
        recipe_dir: out_dir.to_path_buf(),
        recipe_path,
        provenance_path,
        payload_dir,
    })
}

/// The `--json` plan report: the declared package identity, the validated payload totals, the
/// provenance document and the rendered recipe, as one serialisable value.
///
/// The shape is a wire contract a consumer scripts against, so it lives here rather than in the
/// command: the command prints it, and this is where it is testable without spawning a binary.
#[must_use]
pub fn plan_report(plan: &WrapPlan) -> serde_json::Value {
    serde_json::json!({
        "schema": 1,
        "package": {
            "name": plan.spec.name,
            "version": plan.spec.version,
            "platform": plan.spec.platform.as_str(),
        },
        "payload": {
            "files": plan.payload.file_count,
            "directories": plan.payload.directory_count,
            "symlinks": plan.payload.symlink_count,
            "bytes": plan.payload.total_bytes,
        },
        "provenance": plan.provenance,
        "recipe": plan.recipe,
    })
}

fn remove_dir_if_present(path: &Path) -> Result<(), PayloadError> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => std::fs::remove_dir_all(path).map_err(|error| io_error(path, &error)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(path, &error)),
    }
}

/// Copy a payload tree, preserving directories, regular files with their mode, and symlinks.
///
/// A single-file payload (AC#2) is copied as the one member of `destination`, so the recipe's
/// `$RECIPE_DIR/payload/` is always a directory.
fn copy_tree(source: &Path, destination: &Path) -> Result<(), PayloadError> {
    std::fs::create_dir_all(destination).map_err(|error| io_error(destination, &error))?;
    let root = std::fs::symlink_metadata(source).map_err(|error| io_error(source, &error))?;
    if !root.is_dir() {
        let name = source.file_name().ok_or_else(|| {
            PayloadError::InvalidConfig(format!(
                "payload path has no file name: {}",
                source.display()
            ))
        })?;
        std::fs::copy(source, destination.join(name)).map_err(|error| io_error(source, &error))?;
        return Ok(());
    }
    let entries = std::fs::read_dir(source).map_err(|error| io_error(source, &error))?;
    for entry in entries {
        let entry = entry.map_err(|error| io_error(source, &error))?;
        let from = entry.path();
        let to = destination.join(entry.file_name());
        let metadata = std::fs::symlink_metadata(&from).map_err(|error| io_error(&from, &error))?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            recreate_symlink(&from, &to)?;
        } else if file_type.is_dir() {
            copy_tree(&from, &to)?;
        } else {
            std::fs::copy(&from, &to).map_err(|error| io_error(&from, &error))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn recreate_symlink(from: &Path, to: &Path) -> Result<(), PayloadError> {
    let target = std::fs::read_link(from).map_err(|error| io_error(from, &error))?;
    std::os::unix::fs::symlink(target, to).map_err(|error| io_error(to, &error))
}

#[cfg(not(unix))]
fn recreate_symlink(from: &Path, to: &Path) -> Result<(), PayloadError> {
    let _ = (from, to);
    Err(PayloadError::InvalidConfig(
        "staging a payload that contains a symbolic link is not supported on this host".to_string(),
    ))
}

fn io_error(path: &Path, error: &std::io::Error) -> PayloadError {
    PayloadError::Io {
        path: path.display().to_string(),
        message: error.to_string(),
    }
}
