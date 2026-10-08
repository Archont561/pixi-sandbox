//! `repo_checks::stale_refs` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

mod support;

use std::fs;
use support::headlines;

#[test]
fn a_stale_reference_under_crates_fires_check_1_and_the_marker_silences_it() {
    let dir = support::valid_fixture();
    fs::write(
        dir.path().join("crates/pixi-sandbox/src/old.rs"),
        // stale-ref-allowed — fixture content must name a forbidden word to test the check.
        "// see the Python prototype\n",
    )
    .expect("file");
    let found = headlines(dir.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("task-6"), "{found:?}");

    fs::write(
        dir.path().join("crates/pixi-sandbox/src/old.rs"),
        "// stale-ref-allowed — documents history\n// see the Python prototype\n",
    )
    .expect("file");
    assert_eq!(headlines(dir.path()), Vec::<String>::new());
}
