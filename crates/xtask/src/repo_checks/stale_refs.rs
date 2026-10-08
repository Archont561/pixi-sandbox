//! Check 1 (task-6): no reference under `crates/` to the deleted pre-Rust implementation.

use super::Failure;
use super::support::rel;
use crate::util::lines_without_opt_out;
use anyhow::Result;
use std::path::{Path, PathBuf};

// stale-ref-allowed — the check must spell the forbidden words out to forbid them.
const STALE_WORDS: [&str; 3] = ["python", "prototype", "knowledge/research"];

/// task-6: no reference under `crates/` to the implementation deleted in 0.2.0. It lived
/// under `.knowledge/`'s research directory; an airlocked reader has no network to discover
/// that it is gone. A line that must name the thing to forbid it opts out with the marker on
/// (or above) it.
///
/// Generated trees are pruned rather than left to gitignore: `crates/pixi-sandbox/.pixi/bld`
/// holds a vendored third-party registry the moment anyone runs `pixi run package` locally,
/// and this scan has to be as blind to it as git is.
pub(super) fn stale_implementation_references(
    root: &Path,
    failures: &mut Vec<Failure>,
) -> Result<()> {
    let mut hits = Vec::new();
    for file in walk_pruned(&root.join("crates"), &["rs", "md"]) {
        let Ok(text) = crate::util::read(&file) else {
            continue;
        };
        for (number, line) in lines_without_opt_out(&text) {
            let lower = line.to_lowercase();
            if STALE_WORDS.iter().any(|w| lower.contains(w)) {
                hits.push(format!("{}:{number}:{line}", rel(root, &file)));
            }
        }
    }
    if !hits.is_empty() {
        failures.push(Failure::with(
            "crates/ still references the implementation deleted in 0.2.0 (task-6):",
            hits,
            "point the text at .knowledge/design.md instead, or mark the line stale-ref-allowed",
        ));
    }
    Ok(())
}

fn walk_pruned(dir: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    const PRUNED: [&str; 4] = [".pixi", "target", "node_modules", "vendor"];
    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| {
            !(e.file_type().is_dir() && PRUNED.contains(&e.file_name().to_string_lossy().as_ref()))
        })
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| {
            p.extension()
                .is_some_and(|x| extensions.iter().any(|want| x == *want))
        })
        .collect();
    files.sort();
    files
}
