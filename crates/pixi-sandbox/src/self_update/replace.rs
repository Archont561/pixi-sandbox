//! Stage beside the destination, then replace it — including when the destination is the
//! binary currently executing this code.
//!
//! The Unix half is the rule `shard::join_parts` already follows (task-37 AC#3): rename onto
//! the destination, which is atomic *and* ETXTBSY-proof, because it swaps the directory entry
//! and leaves the busy inode to the running process. Removing first would open a window in
//! which the tool does not exist at all.
//!
//! Windows cannot rename onto an existing file, and it refuses to delete or write-open a
//! running image — but it *does* permit renaming one within the same volume. So the running
//! executable is renamed aside to a `.old-<version>` sibling first, the new file takes its
//! name, and the corpse is swept on a later run (deleting it now fails by design while the
//! old image is still mapped). That sweep is step one of every update, so the leftovers of the
//! previous update are what get reaped, not this one's.
//!
//! The strategy is a parameter rather than a `cfg!` read so both paths are exercised on
//! whatever machine runs the tests — the `LauncherKind` precedent in `user_tools`.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

/// Which replacement dance to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplaceStrategy {
    /// Rename straight over the destination.
    Unix,
    /// Rename the destination aside first, because it may be a running image.
    Windows,
}

impl ReplaceStrategy {
    #[must_use]
    pub fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else {
            Self::Unix
        }
    }
}

/// What the replacement did, so the caller can report it precisely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replacement {
    /// The file that now holds the new bytes.
    pub destination: PathBuf,
    /// The displaced previous binary, when one was kept aside (Windows).
    pub displaced: Option<PathBuf>,
    /// Stale `.old-*` siblings removed before this update ran.
    pub swept: Vec<PathBuf>,
}

/// Prefix of a displaced previous binary. Distinctive enough that the sweep cannot match a
/// file that is not ours.
#[doc(hidden)]
pub const DISPLACED_INFIX: &str = ".pixi-sandbox-old-";
/// Prefix of the staged download, hidden so a half-written update is not mistaken for a tool.
#[doc(hidden)]
pub const STAGING_INFIX: &str = ".pixi-sandbox-update-";

/// Write `bytes` over `destination`, atomically, staging in the destination's own directory.
///
/// Staging in that directory — not `$TMPDIR` (invariant 3) — is what makes the final rename a
/// same-filesystem operation and therefore atomic; a cross-device rename would silently become
/// a copy with a torn-write window.
///
/// # Errors
///
/// Returns an error if the bytes cannot be staged or the destination cannot be replaced.
pub fn install(
    strategy: ReplaceStrategy,
    destination: &Path,
    bytes: &[u8],
    current_version: &str,
) -> Result<Replacement> {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;

    // Step 1: reap the previous update's corpse, if the OS has let go of it by now.
    let swept = sweep_displaced(parent);

    // Step 2: stage the verified bytes beside the destination.
    let staged = staging_path(destination);
    std::fs::write(&staged, bytes)
        .with_context(|| format!("staging the new binary at {}", staged.display()))?;
    if let Err(err) = make_executable(&staged) {
        let _ = std::fs::remove_file(&staged);
        return Err(err);
    }

    // Step 3: swap it in.
    let displaced = match strategy {
        ReplaceStrategy::Unix => {
            if let Err(err) = std::fs::rename(&staged, destination) {
                let _ = std::fs::remove_file(&staged);
                return Err(err).with_context(|| {
                    format!("replacing {} with the new binary", destination.display())
                });
            }
            None
        }
        ReplaceStrategy::Windows => windows_swap(&staged, destination, current_version)?,
    };

    Ok(Replacement {
        destination: destination.to_path_buf(),
        displaced,
        swept,
    })
}

/// Rename the (possibly running) destination aside, then move the staged file into its place.
///
/// If the second rename fails the first is undone, so the window in which the tool is missing
/// is one syscall wide and self-healing.
#[doc(hidden)]
pub fn windows_swap(
    staged: &Path,
    destination: &Path,
    current_version: &str,
) -> Result<Option<PathBuf>> {
    if !destination.exists() {
        std::fs::rename(staged, destination)
            .with_context(|| format!("installing the new binary at {}", destination.display()))?;
        return Ok(None);
    }

    let aside = displaced_path(destination, current_version);
    let _ = std::fs::remove_file(&aside);
    std::fs::rename(destination, &aside).with_context(|| {
        format!(
            "moving the running binary {} aside to {}",
            destination.display(),
            aside.display()
        )
    })?;

    if let Err(err) = std::fs::rename(staged, destination) {
        // Put it back: better a failed update than a missing tool.
        if std::fs::rename(&aside, destination).is_err() {
            let _ = std::fs::remove_file(staged);
            bail!(
                "failed to install {} ({err}), and the previous binary could not be restored \
                 from {} — move it back by hand",
                destination.display(),
                aside.display()
            );
        }
        let _ = std::fs::remove_file(staged);
        return Err(err)
            .with_context(|| format!("installing the new binary at {}", destination.display()));
    }

    // Expected to fail while the old image is still mapped; step 1 of the next run reaps it.
    let _ = std::fs::remove_file(&aside);
    Ok(Some(aside))
}

/// Delete leftover `.old-*` siblings. Best effort by construction: a file still held open by a
/// running process simply stays until next time.
fn sweep_displaced(parent: &Path) -> Vec<PathBuf> {
    let mut swept = Vec::new();
    let Ok(entries) = std::fs::read_dir(parent) else {
        return swept;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.contains(DISPLACED_INFIX) && std::fs::remove_file(&path).is_ok() {
            swept.push(path);
        }
    }
    swept.sort();
    swept
}

fn file_name_of(path: &Path) -> String {
    path.file_name().map_or_else(
        || "pixi-sandbox".to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

#[doc(hidden)]
#[must_use]
pub fn staging_path(destination: &Path) -> PathBuf {
    destination.with_file_name(format!(
        "{STAGING_INFIX}{}-{}",
        std::process::id(),
        file_name_of(destination)
    ))
}

fn displaced_path(destination: &Path, current_version: &str) -> PathBuf {
    destination.with_file_name(format!(
        "{}{DISPLACED_INFIX}{current_version}",
        file_name_of(destination)
    ))
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .with_context(|| format!("making {} executable", path.display()))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    // Windows derives executability from the extension, not a permission bit.
    Ok(())
}
