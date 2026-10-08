//! `repo_checks` — the check table and the shared fixture (moved from the module's
//! inline `#[cfg(test)]`, TASK-83: names kept, bodies verbatim).

mod support;

use support::{headlines, valid_fixture};
use xtask::repo_checks::workflow_shape::MULTI_RUN_ALLOWED;

#[test]
fn the_valid_fixture_passes_every_check() {
    let dir = valid_fixture();
    assert_eq!(headlines(dir.path()), Vec::<String>::new());
}

/// The render must satisfy check 9 unaided: that is the property that lets this repository
/// commit a generated workflow at all, and it is the reason the relock template is written
/// as one-line steps instead of the publisher's blocks.
#[test]
fn the_committed_render_needs_no_multiline_exemption() {
    let dir = valid_fixture();
    let render =
        std::fs::read_to_string(dir.path().join(xtask::workflow::RELOCK_PATH)).expect("render");
    assert!(!render.contains(MULTI_RUN_ALLOWED), "{render}");
    assert_eq!(headlines(dir.path()), Vec::<String>::new());
}
