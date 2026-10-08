//! Check 3 (task-2 follow-up): version references cannot drift (`release_refs::scan`), and the
//! conda package manifest (the one file whose format demands a restated literal) equals
//! Cargo.toml.

use super::Failure;
use anyhow::Result;
use std::path::Path;

/// task-2 follow-up: two regimes, one predicate (`release_refs`, shared with the fix in
/// `prepare-release`, so the reporter and the fixer cannot disagree). Plus 3b: the conda
/// package manifest is the one file whose format demands a restated literal version, so it
/// must equal Cargo.toml's — it sat at 0.2.0 for two releases while the workspace said 0.3.2,
/// and a `pixi publish` would have shipped a wrongly-versioned .conda with no red build.
pub(super) fn version_references(root: &Path, failures: &mut Vec<Failure>) -> Result<()> {
    let workspace_version = crate::version::workspace_version(root)?;
    match crate::release_refs::scan(root) {
        Err(error) => failures.push(Failure::new(format!(
            "the release-reference scan failed: {error:#}"
        ))),
        Ok(findings) if !findings.is_empty() => failures.push(Failure::with(
            "documentation pins a version the manifests do not declare (task-2):",
            findings,
            format!(
                "README-family drift: run 'pixi run xtask prepare-release v{workspace_version}' (or mark the line stale-ref-allowed); docs/ literals: replace the tag with v__VERSION__ — the site derives the version at build time"
            ),
        )),
        Ok(_) => {}
    }

    let conda_manifest = root.join("crates/pixi-sandbox/pixi.toml");
    let conda_version = crate::util::read(&conda_manifest)?.lines().find_map(|l| {
        l.strip_prefix("version = \"")
            .and_then(|rest| rest.split('"').next())
            .map(str::to_string)
    });
    match conda_version {
        None => failures.push(Failure::new(
            "crates/pixi-sandbox/pixi.toml has no parsable [package] version — 'pixi publish' would refuse or guess",
        )),
        Some(conda_version) if conda_version != workspace_version => failures.push(Failure::with(
            format!(
                "crates/pixi-sandbox/pixi.toml declares {conda_version} but Cargo.toml declares {workspace_version} — the published .conda would carry the wrong version"
            ),
            Vec::new(),
            format!(
                "prepare-release stamps this file; for a manual fix set version = \"{workspace_version}\" in crates/pixi-sandbox/pixi.toml"
            ),
        )),
        Some(_) => {}
    }
    Ok(())
}
