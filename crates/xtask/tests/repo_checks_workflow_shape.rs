//! `repo_checks::workflow_shape` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

mod support;

use std::fs;
use support::{headlines, valid_fixture};
use xtask::repo_checks::workflow_shape::{GENERATED_MARKER, MULTI_RUN_ALLOWED};

#[test]
fn a_multi_line_run_block_fires_check_9() {
    let dir = valid_fixture();
    fs::write(
        dir.path().join(".github/workflows/shape.yml"),
        "jobs:\n  x:\n    steps:\n      - name: two commands\n        run: |\n          pixi run build\n          pixi run test\n",
    )
    .expect("workflow");
    let found = headlines(dir.path());
    assert!(
        found.iter().any(|h| h.contains("multi-line command block")),
        "{found:?}"
    );
}

#[test]
fn every_single_command_spelling_passes_check_9() {
    let dir = valid_fixture();
    // One literal command, a folded scalar (folds to one command however it wraps), a
    // backslash-continued single command, and plain one-line runs: all legal.
    let workflow = concat!(
        "jobs:\n  x:\n    steps:\n",
        "      - name: one literal\n        run: |\n          pixi run test\n",
        "      - name: folded\n        run: >-\n          pixi global install\n          --channel conda-forge\n          pixi-sandbox\n",
        "      - name: continued\n        run: |\n          pixi upload prefix \\\n            --channel example \\\n            dist/*.conda\n",
        "      - run: pixi run lint\n",
        "      - run: rustup target add x86_64-unknown-linux-musl\n",
    );
    fs::write(dir.path().join(".github/workflows/shape.yml"), workflow).expect("workflow");
    let found = headlines(dir.path());
    assert!(
        !found.iter().any(|h| h.contains("multi-line command block")),
        "single commands in every spelling must pass: {found:?}"
    );
}

#[test]
fn comments_and_blank_lines_inside_a_block_are_not_commands() {
    let dir = valid_fixture();
    fs::write(
        dir.path().join(".github/workflows/shape.yml"),
        "jobs:\n  x:\n    steps:\n      - run: |\n          # why this is fine\n\n          pixi run test\n",
    )
    .expect("workflow");
    let found = headlines(dir.path());
    assert!(
        !found.iter().any(|h| h.contains("multi-line command block")),
        "a commented single command is one command: {found:?}"
    );
}

#[test]
fn a_generated_consumer_workflow_is_exempt_but_the_marker_window_is_three_lines() {
    let block = "          pixi run build\n          pixi run test\n";
    // The marker in the header window: an `init` render, exempt.
    let dir = valid_fixture();
    fs::write(
        dir.path().join(".github/workflows/shape.yml"),
        format!(
            "# {GENERATED_MARKER}. Regenerate this file instead of editing it.\n\
             name: publish sandbox\njobs:\n  x:\n    steps:\n      - run: |\n{block}"
        ),
    )
    .expect("workflow");
    let found = headlines(dir.path());
    assert!(
        !found.iter().any(|h| h.contains("multi-line command block")),
        "a generated artifact carries the shape init ships: {found:?}"
    );

    // The same marker on line four: a hand-written workflow cannot borrow the exemption.
    let dir = valid_fixture();
    fs::write(
        dir.path().join(".github/workflows/shape.yml"),
        format!("jobs:\n  x:\n    steps:\n# {GENERATED_MARKER}\n      - run: |\n{block}"),
    )
    .expect("workflow");
    let found = headlines(dir.path());
    assert!(
        found.iter().any(|h| h.contains("multi-line command block")),
        "the marker only counts in the header window: {found:?}"
    );
}

#[test]
fn the_opt_out_marker_silences_a_reviewed_multi_line_step() {
    let block = "          pixi run build\n          pixi run test\n";
    for (name, workflow) in [
        (
            "above",
            "jobs:\n  x:\n    steps:\n      # multiline-run-allowed: generated artifact\n      - run: |\n{block}".to_string(),
        ),
        (
            "inline",
            format!("jobs:\n  x:\n    steps:\n      - run: | # {MULTI_RUN_ALLOWED}\n{{block}}"),
        ),
        (
            "inside",
            format!("jobs:\n  x:\n    steps:\n      - run: |\n          # {MULTI_RUN_ALLOWED}\n{{block}}"),
        ),
    ] {
        let dir = valid_fixture();
        fs::write(
            dir.path().join(".github/workflows/shape.yml"),
            workflow.replace("{block}", block),
        )
        .expect("workflow");
        let found = headlines(dir.path());
        assert!(
            !found.iter().any(|h| h.contains("multi-line command block")),
            "the {name} marker placement must silence the step: {found:?}"
        );
    }
}
