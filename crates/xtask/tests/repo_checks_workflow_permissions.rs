//! `repo_checks::workflow_permissions` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

mod support;

use proptest::prelude::*;
use std::fmt::Write as _;
use std::fs;
use support::{headlines, valid_fixture};
use xtask::repo_checks::workflow_permissions::{LEVELS, SCOPES};

const HEADLINE: &str = "permission scope GitHub does not accept";

fn fires(workflow: &str) -> bool {
    let dir = valid_fixture();
    fs::write(dir.path().join(".github/workflows/perms.yml"), workflow).expect("workflow");
    headlines(dir.path()).iter().any(|h| h.contains(HEADLINE))
}

/// The exact line that bricked `auto-release.yml`, at both levels a `permissions:` block
/// can appear — a job-level block takes the file down just as a top-level one does.
#[test]
fn the_workflows_scope_fires_check_11_at_either_level() {
    assert!(fires(
        "permissions:\n  contents: write\n  actions: write\n  workflows: write\njobs:\n  x:\n    steps: []\n"
    ));
    assert!(fires(
        "jobs:\n  x:\n    permissions:\n      contents: read\n      workflows: write\n    steps: []\n"
    ));
}

/// Every scope GitHub documents, at every level, plus the whole-block shorthands.
#[test]
fn every_real_scope_and_shorthand_passes_check_11() {
    for level in LEVELS {
        let mut block = String::new();
        for scope in SCOPES {
            // A String sink cannot fail, so the fmt::Result carries no information.
            let _ = writeln!(block, "  {scope}: {level}");
        }
        assert!(
            !fires(&format!("permissions:\n{block}")),
            "the documented scopes must pass at {level}"
        );
    }
    for shorthand in ["read-all", "write-all", "{}"] {
        assert!(
            !fires(&format!("permissions: {shorthand}\njobs: {{}}\n")),
            "`permissions: {shorthand}` is legal"
        );
    }
}

/// A bogus whole-block value is the same class of fatal typo as a bogus scope, and so is
/// an access level that is not read/write/none.
#[test]
fn a_bogus_shorthand_or_access_level_fires_check_11() {
    assert!(fires("permissions: all-the-things\njobs: {}\n"));
    assert!(fires("permissions:\n  contents: writeable\n"));
    assert!(fires("permissions:\n  contents: true\n"));
}

/// The parser must read YAML shape, not the substring `permissions:`. Prose comments
/// discussing a permissions block (relock.yml has one) and shell inside a `run:` that
/// names `permissions.id-token` (release.yml has one) are not declarations.
#[test]
fn prose_and_shell_that_merely_mention_permissions_are_not_declarations() {
    assert!(!fires(
        "jobs:\n  x:\n    # A job-level `permissions:` block is a replacement, not an addition\n    permissions:\n      contents: write\n",
    ));
    assert!(!fires(
        "jobs:\n  x:\n    steps:\n      - run: ': \"${URL:?grant permissions.id-token: write}\"'\n",
    ));
}

/// Block framing: comments, blank lines, quoted values and trailing comments live inside
/// a block, and the block ends at the first line that dedents back to the key — a sibling
/// top-level key is not a scope.
#[test]
fn check_11_reads_the_block_body_and_stops_at_the_dedent() {
    assert!(!fires(
        "permissions:\n  # why this is needed\n\n  contents: \"write\"\n  actions: write # dispatch release.yml\n",
    ));
    assert!(!fires(
        "permissions:\n  contents: read\nconcurrency:\n  group: ci\n  cancel-in-progress: true\n",
    ));
    // …and a scope hiding after the comments is still found.
    assert!(fires(
        "permissions:\n  # why this is needed\n\n  contents: write\n  workflows: write\n"
    ));
}

/// The opt-out exists for one case: GitHub ships a scope newer than `SCOPES`. Without it
/// a correct workflow would be unfixable except by editing this crate.
#[test]
fn the_house_opt_out_marker_silences_a_scope_newer_than_this_list() {
    assert!(!fires(
        "permissions:\n  contents: read\n  code-quality: write # stale-ref-allowed\n"
    ));
    assert!(!fires(
        "permissions:\n  contents: read\n  # stale-ref-allowed: GitHub shipped this after our pin\n  code-quality: write\n",
    ));
}

/// Every declaration is reported, not just the first: a red run names every problem.
#[test]
fn check_11_reports_every_bad_scope_in_one_pass() {
    let dir = valid_fixture();
    fs::write(
        dir.path().join(".github/workflows/perms.yml"),
        "permissions:\n  workflows: read\njobs:\n  a:\n    permissions:\n      contents: read\n  b:\n    permissions:\n      bogus: write\n",
    )
    .expect("workflow");
    let failures = xtask::repo_checks::check_repository(dir.path()).expect("checks");
    let found = failures
        .iter()
        .find(|f| f.headline.contains(HEADLINE))
        .expect("scope failure");
    assert_eq!(found.details.len(), 2, "{:?}", found.details);
    assert!(
        found.details[0].contains("workflows"),
        "{:?}",
        found.details
    );
    assert!(found.details[1].contains("bogus"), "{:?}", found.details);
}

/// The list is read by humans against actionlint's diagnostic, which prints its own set
/// alphabetically. Keep it sorted and unique so the two can be diffed by eye — an
/// out-of-order insert is how `artifact-metadata` went missing in the first place.
#[test]
fn the_scope_list_is_sorted_and_unique() {
    let mut sorted = SCOPES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted, SCOPES,
        "SCOPES must stay sorted and free of duplicates"
    );
}

/// A scope name that is not on the list is always rejected, however it is spelled — the
/// property the one-line `workflows: write` regression needed and did not have.
fn unknown_scope() -> impl Strategy<Value = String> {
    "[a-z][a-z-]{0,20}".prop_filter("must not be a real scope", |candidate| {
        !SCOPES.contains(&candidate.as_str())
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn any_scope_outside_the_documented_set_always_fires_check_11(
        scope in unknown_scope(),
        level in prop::sample::select(LEVELS),
    ) {
        prop_assert!(
            fires(&format!("permissions:\n  contents: read\n  {scope}: {level}\n")),
            "scope={scope} level={level}",
        );
    }

    #[test]
    fn a_documented_scope_never_fires_check_11(
        scope in prop::sample::select(SCOPES),
        level in prop::sample::select(LEVELS),
    ) {
        prop_assert!(
            !fires(&format!("permissions:\n  {scope}: {level}\n")),
            "scope={scope} level={level}",
        );
    }
}
