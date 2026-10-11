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
