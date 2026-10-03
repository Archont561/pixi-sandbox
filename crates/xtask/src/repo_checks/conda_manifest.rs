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

#[cfg(test)]
mod tests {
    use super::super::test_support::{headlines, valid_fixture};
    use std::fs;

    /// Check 3 fires on a documented reference to a release the manifests do not declare —
    /// and its remedy has to name a task that exists. That hint is the only place a failing
    /// developer is told how to fix the drift, so when the task was renamed to
    /// `pixi run xtask prepare-release`, the string became part of the check's behaviour
    /// rather than decoration around it.
    #[test]
    fn a_documented_release_the_manifests_do_not_declare_fires_check_3_with_a_usable_remedy() {
        let dir = valid_fixture();
        let readme = dir.path().join("README.md");
        let mut text = fs::read_to_string(&readme).expect("read");
        text.push_str(
            "\ncurl -L https://github.com/Archont561/pixi-sandbox/releases/download/v0.9.9/pixi-sandbox-x86_64-unknown-linux-musl\n",
        );
        fs::write(&readme, text).expect("file");

        let failures = super::super::check_repository(dir.path()).expect("checks run");
        let found: Vec<&str> = failures.iter().map(|f| f.headline.as_str()).collect();
        assert_eq!(failures.len(), 1, "{found:?}");
        assert!(
            failures[0]
                .headline
                .contains("documentation pins a version"),
            "{found:?}"
        );
        assert!(
            failures[0].details.iter().any(|d| d.contains("v0.9.9")),
            "the drifted reference must be named: {:?}",
            failures[0].details
        );
        let hint = failures[0].hint.as_deref().unwrap_or_default();
        assert!(
            hint.contains("pixi run xtask prepare-release v1.0.0"),
            "the remedy must name the task that fixes it, at the version that fixes it: {hint}"
        );
    }

    #[test]
    fn a_conda_manifest_version_mismatch_fires_check_3b() {
        let dir = valid_fixture();
        fs::write(
            dir.path().join("crates/pixi-sandbox/pixi.toml"),
            "[package]\nname = \"fixture\"\nversion = \"0.9.0\"\n",
        )
        .expect("manifest");
        let found = headlines(dir.path());
        assert!(
            found
                .iter()
                .any(|h| h.contains("declares 0.9.0 but Cargo.toml declares 1.0.0")),
            "{found:?}"
        );
    }

    #[test]
    fn a_stale_documented_release_reference_fires_check_3() {
        let dir = valid_fixture();
        let readme = fs::read_to_string(dir.path().join("README.md")).expect("readme");
        fs::write(
            dir.path().join("README.md"),
            format!("{readme}\nuses: Archont561/pixi-sandbox/setup@v0.1.0\n"),
        )
        .expect("readme");
        let found = headlines(dir.path());
        assert!(found.iter().any(|h| h.contains("task-2")), "{found:?}");
    }
}
