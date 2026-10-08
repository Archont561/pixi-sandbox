//! `repo_checks::relock` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

mod support;

use std::fs;
use support::{headlines, valid_fixture};

#[test]
fn a_stale_committed_relock_workflow_fires_check_10() {
    let dir = valid_fixture();
    let path = dir.path().join(xtask::workflow::RELOCK_PATH);
    let current = fs::read_to_string(&path).expect("render");
    // The drift that matters in practice: someone edits the workflow instead of the
    // template it is rendered from.
    fs::write(
        &path,
        current.replace("- run: pixi lock\n", "- run: pixi lock --json\n"),
    )
    .expect("sabotage");

    let found = headlines(dir.path());
    assert!(
        found
            .iter()
            .any(|h| h.contains("not the generator's current render")),
        "{found:?}"
    );
}

#[test]
fn a_missing_relock_workflow_names_the_command_that_writes_it() {
    let dir = valid_fixture();
    fs::remove_file(dir.path().join(xtask::workflow::RELOCK_PATH)).expect("remove");

    let failures = xtask::repo_checks::check_repository(dir.path()).expect("checks run");
    let failure = failures
        .iter()
        .find(|f| f.headline.contains("committed relock workflow is missing"))
        .expect("check 10 fires");
    assert!(
        failure
            .hint
            .as_ref()
            .is_some_and(|hint| hint.contains("xtask render-relock")),
        "{}",
        failure.headline
    );
}
