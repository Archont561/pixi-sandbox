//! `repo_checks::conda_manifest` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

mod support;

use std::fs;
use support::{headlines, valid_fixture};

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

    let failures = xtask::repo_checks::check_repository(dir.path()).expect("checks run");
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
