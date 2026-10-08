//! `repo_checks::release_tags` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

mod support;

use std::fs;
use support::{headlines, valid_fixture};

#[test]
fn a_literal_release_tag_in_a_workflow_fires_check_6() {
    let dir = valid_fixture();
    fs::write(
        dir.path().join(".github/workflows/proof.yml"),
        "jobs:\n  x:\n    steps:\n      - with:\n          version: ${{ vars.RELEASE || 'v0.3.0' }}\n",
    )
    .expect("workflow");
    let found = headlines(dir.path());
    assert!(
        found
            .iter()
            .any(|h| h.contains("pins a literal release tag")),
        "{found:?}"
    );
}
