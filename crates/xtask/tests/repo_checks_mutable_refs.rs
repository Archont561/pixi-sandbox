//! `repo_checks::mutable_refs` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

mod support;

use std::fs;
use support::valid_fixture;

#[test]
fn a_mutable_tag_or_an_unlabelled_sha_fires_check_4() {
    let dir = valid_fixture();
    fs::write(
        dir.path().join(".github/workflows/bad.yml"),
        "jobs:\n  x:\n    steps:\n      - uses: actions/checkout@v7\n      - uses: actions/cache@3d3c42e5aac5ba805825da76410c181273ba90b1\n",
    )
    .expect("workflow");
    let failures = xtask::repo_checks::check_repository(dir.path()).expect("checks");
    let pins = failures
        .iter()
        .find(|f| f.headline.contains("not pinned to a full commit SHA"))
        .expect("pin failure");
    assert_eq!(pins.details.len(), 2, "{:?}", pins.details);
    assert!(pins.details[0].contains("mutable ref"));
    assert!(pins.details[1].contains("no trailing release label"));
}
