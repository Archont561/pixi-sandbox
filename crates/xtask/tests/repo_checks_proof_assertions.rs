//! `repo_checks::proof_assertions` (check 13, doc-13 §1.2): the consumer proof may only grep a
//! generated publisher for strings the generator still renders.

mod support;

use std::fs;
use support::{headlines, valid_fixture};

#[test]
fn a_proof_that_greps_a_retired_publisher_string_fires_check_13() {
    let dir = valid_fixture();
    fs::write(
        dir.path().join(".github/workflows/consumer-proof.yml"),
        "name: consumer proof\njobs:\n  consumer-proof:\n    steps:\n      - run: grep -F 'Archont561/pixi-sandbox/releases/download' .github/workflows/publish-sandbox.yml\n",
    )
    .expect("workflow");
    let found = headlines(dir.path());
    assert!(
        found
            .iter()
            .any(|h| h.contains("asserts strings the generated publisher does not contain")),
        "{found:?}"
    );
}

#[test]
fn a_proof_that_greps_only_rendered_strings_is_clean() {
    let found = headlines(valid_fixture().path());
    assert!(
        !found
            .iter()
            .any(|h| h.contains("asserts strings the generated publisher does not contain")),
        "{found:?}"
    );
}

#[test]
fn a_double_quoted_assertion_is_held_to_the_same_contract() {
    let dir = valid_fixture();
    fs::write(
        dir.path().join(".github/workflows/consumer-proof.yml"),
        "name: consumer proof\njobs:\n  consumer-proof:\n    steps:\n      - run: grep -F \"releases/download\" .github/workflows/publish-sandbox.yml\n",
    )
    .expect("workflow");
    let found = headlines(dir.path());
    assert!(
        found
            .iter()
            .any(|h| h.contains("asserts strings the generated publisher does not contain")),
        "{found:?}"
    );
}

#[test]
fn a_grep_aimed_at_another_file_is_none_of_this_checks_business() {
    let dir = valid_fixture();
    fs::write(
        dir.path().join(".github/workflows/consumer-proof.yml"),
        "name: consumer proof\njobs:\n  consumer-proof:\n    steps:\n      - run: grep -F 'releases/download' some/other/file.txt\n",
    )
    .expect("workflow");
    let found = headlines(dir.path());
    assert!(
        !found
            .iter()
            .any(|h| h.contains("asserts strings the generated publisher does not contain")),
        "{found:?}"
    );
}

#[test]
fn a_repository_without_the_proof_has_nothing_to_keep_in_step() {
    let dir = valid_fixture();
    fs::remove_file(dir.path().join(".github/workflows/consumer-proof.yml")).expect("remove");
    let found = headlines(dir.path());
    assert!(
        !found
            .iter()
            .any(|h| h.contains("asserts strings the generated publisher does not contain")),
        "{found:?}"
    );
}

// ----------------------------------------------------------------- the ported home

#[test]
fn a_module_asserting_a_retired_publisher_string_fires_check_13() {
    // TASK-89 moved the proof's two `grep -F` literals out of the workflow and into
    // `PUBLISHER_ASSERTIONS`. The contract is the same one doc-13 §1.2 wrote: an assertion may
    // not outlive the string the generator renders.
    let dir = valid_fixture();
    fs::write(
        dir.path().join("crates/xtask/src/consumer_proof.rs"),
        "pub const PUBLISHER_ASSERTIONS: [&str; 1] = [\"Archont561/pixi-sandbox/releases/download\"];\n",
    )
    .expect("module");
    let found = headlines(dir.path());
    assert!(
        found
            .iter()
            .any(|h| h.contains("asserts strings the generated publisher does not contain")),
        "{found:?}"
    );
}

#[test]
fn a_module_whose_assertions_the_generator_still_renders_is_clean() {
    let found = headlines(valid_fixture().path());
    assert!(
        !found
            .iter()
            .any(|h| h.contains("asserts strings the generated publisher does not contain")),
        "{found:?}"
    );
}

#[test]
fn retiring_a_literal_the_module_asserts_fires_check_13() {
    // The vacuity this check has to survive: the assertions moved, and a check that followed
    // the file name but not the literals would have gone on passing over an empty list. Here
    // the generator stops rendering a string the module still asserts — the exact v0.6.1
    // incident, replayed against the ported home.
    let dir = valid_fixture();
    fs::write(
        dir.path()
            .join("crates/pixi-sandbox/src/generated/github_workflow.rs"),
        "https://prefix.dev/archont561/archont561\npixi global install pixi-sandbox\n",
    )
    .expect("generator");
    let found = headlines(dir.path());
    assert!(
        found
            .iter()
            .any(|h| h.contains("asserts strings the generated publisher does not contain")),
        "a retired literal must fire even though it now lives in Rust: {found:?}"
    );
}

#[test]
fn a_module_that_declares_no_assertions_is_itself_a_finding() {
    // The failure mode doc-13 §1.2 could not see: not a wrong assertion but no assertions, so
    // the check has nothing to compare and passes over an empty list. A proof that stopped
    // checking the publisher is a regression, not a clean result.
    let dir = valid_fixture();
    fs::write(
        dir.path().join("crates/xtask/src/consumer_proof.rs"),
        "pub const CONSUMER_BRANCH: &str = \"sandbox/consumer-proof-linux-64\";\n",
    )
    .expect("module");
    let found = headlines(dir.path());
    assert!(
        found.iter().any(|h| h.contains("asserts nothing")),
        "{found:?}"
    );
}

#[test]
fn a_grep_added_back_to_the_workflow_is_still_policed() {
    // The asset-boundary smoke keeps its own workflow. Scanning both homes means a `grep -F`
    // re-added there cannot escape the contract by moving out of the module.
    let dir = valid_fixture();
    fs::write(
        dir.path().join(".github/workflows/consumer-proof.yml"),
        "name: consumer proof\njobs:\n  consumer-proof:\n    steps:\n      - run: grep -F 'releases/download' .github/workflows/publish-sandbox.yml\n",
    )
    .expect("workflow");
    let found = headlines(dir.path());
    assert!(
        found
            .iter()
            .any(|h| h.contains("asserts strings the generated publisher does not contain")),
        "{found:?}"
    );
}
