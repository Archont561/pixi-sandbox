//! The same file-level oracle `crates/pixi-sandbox/tests/fixtures.rs` carries, for this crate:
//! production sources must not grow inline `#[cfg(test)]` modules (Test conventions in
//! AGENTS.md — an oracle must not share a file with the thing it judges).

use std::path::{Path, PathBuf};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn collect_rust(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("src dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn production_sources_carry_no_inline_test_modules() {
    // Pre-rule modules awaiting the same split. Shrink this list; do not extend it.
    const LEGACY: [&str; 24] = [
        "airlock.rs",
        "commit_release.rs",
        "conda_platforms.rs",
        "main.rs",
        "prepare_release.rs",
        "release_assets.rs",
        "release_refs.rs",
        "repo_checks/agents_md.rs",
        "repo_checks/badges.rs",
        "repo_checks/bash32.rs",
        "repo_checks/channel_drift.rs",
        "repo_checks/conda_manifest.rs",
        "repo_checks/mod.rs",
        "repo_checks/mutable_refs.rs",
        "repo_checks/release_tags.rs",
        "repo_checks/relock.rs",
        "repo_checks/stale_refs.rs",
        "repo_checks/workflow_permissions.rs",
        "repo_checks/workflow_shape.rs",
        "smoke.rs",
        "starter.rs",
        "util.rs",
        "version.rs",
        "workflow.rs",
    ];

    let src = crate_dir().join("src");
    let mut sources = Vec::new();
    collect_rust(&src, &mut sources);
    assert!(!sources.is_empty(), "no production sources found");

    let mut offenders = Vec::new();
    let mut legacy_seen = Vec::new();
    for source in sources {
        let relative = source
            .strip_prefix(&src)
            .expect("source sits under src")
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(&source).expect("production source is UTF-8");
        if !text.contains("#[cfg(test)]") {
            continue;
        }
        if LEGACY.contains(&relative.as_str()) {
            legacy_seen.push(relative);
        } else {
            offenders.push(relative);
        }
    }

    assert!(
        offenders.is_empty(),
        "these production sources carry inline `#[cfg(test)]` tests:\n  {}\n\n\
         Move them to `crates/xtask/tests/`: promote the module to `lib.rs` as `pub mod` \
         and test it through its public API, exporting internals a test legitimately needs \
         as `#[doc(hidden)] pub`.",
        offenders.join("\n  ")
    );

    let stale: Vec<&str> = LEGACY
        .iter()
        .copied()
        .filter(|path| !legacy_seen.iter().any(|seen| seen == path))
        .collect();
    assert!(
        stale.is_empty(),
        "these entries no longer carry inline tests — delete them from LEGACY so the list \
         keeps shrinking:\n  {}",
        stale.join("\n  ")
    );
}
