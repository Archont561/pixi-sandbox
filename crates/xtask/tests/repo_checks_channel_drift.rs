//! `repo_checks::channel_drift` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

mod support;

use std::fs;
use support::{headlines, valid_fixture};

#[test]
fn canonical_channel_drift_fires_check_5() {
    let dir = valid_fixture();
    fs::write(
        dir.path().join("docs/src/content/docs/installation.mdx"),
        "pixi global install -c conda-forge pixi-sandbox\npixi-sandbox init\n",
    )
    .expect("installation docs");
    let found = headlines(dir.path());
    assert!(
        found
            .iter()
            .any(|h| h.contains("canonical channel install")),
        "{found:?}"
    );
}
