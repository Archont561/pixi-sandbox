//! Helpers shared by more than one check. `crate::util::lines_without_opt_out` is the other
//! cross-check shared primitive; it already lived outside this subsystem before the split and
//! each check module imports it directly from there instead of through here.

use std::path::{Path, PathBuf};

/// Render `path` relative to `root` for a failure message, falling back to the absolute path
/// if it is not actually inside `root` (synthetic fixtures never hit that branch).
pub(super) fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Every workflow file under `.github/workflows`, sorted for stable failure ordering. Shared
/// by the checks that scan workflows for a textual pattern (`mutable_refs`, `release_tags`,
/// `workflow_shape`); checks that read a handful of named files (`channel_drift`, relock) do not
/// need it.
pub(super) fn workflow_files(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(root.join(".github/workflows"))
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(walkdir::DirEntry::into_path)
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .collect();
    files.sort();
    files
}
