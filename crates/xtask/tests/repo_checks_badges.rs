//! `repo_checks::badges` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

mod support;

use std::fs;
use support::{headlines, valid_fixture};

#[test]
fn a_badge_that_disagrees_with_the_workspace_fires_check_2() {
    let dir = valid_fixture();
    let readme = fs::read_to_string(dir.path().join("README.md")).expect("readme");
    fs::write(
        dir.path().join("README.md"),
        readme.replace("osx--arm64", "win--64"),
    )
    .expect("readme");
    let found = headlines(dir.path());
    assert!(
        found
            .iter()
            .any(|h| h.contains("Platforms badge advertises")),
        "{found:?}"
    );
    assert!(
        found.iter().any(|h| h.contains("claims win-64")),
        "{found:?}"
    );
}

#[test]
fn a_published_platform_the_workspace_lacks_fires_check_2() {
    let dir = valid_fixture();
    fs::write(
        dir.path().join(".pixi-sandbox.toml"),
        "schema = 1\n\n[[bundle]]\nname = \"developer\"\nplatforms = [\"linux-aarch64\"]\n",
    )
    .expect("plan");
    let found = headlines(dir.path());
    assert!(
        found.iter().any(|h| h.contains("publishes linux-aarch64")),
        "{found:?}"
    );
}
