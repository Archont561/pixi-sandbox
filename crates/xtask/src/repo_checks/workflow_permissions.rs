//! Check 11: every `permissions:` scope is one GitHub actually accepts.

use super::Failure;
use super::support::{rel, workflow_files};
use anyhow::Result;
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;

/// The scopes GitHub accepts in a `permissions:` block. Anything else is not a tighter or
/// looser permission — it is a parse error that takes the whole workflow file down with it.
///
/// This must stay exactly the set the pinned actionlint knows (`actionlint = "=1.7.12"` in
/// `pixi.toml`; it prints the full list in its own diagnostic). Both gates run in the same
/// `pixi run lint`, so a disagreement is worse than either gate alone: a scope this list omits
/// but GitHub accepts — `artifact-metadata` was exactly that during review — becomes a failure
/// no edit to the workflow can clear.
const SCOPES: &[&str] = &[
    "actions",
    "artifact-metadata",
    "attestations",
    "checks",
    "contents",
    "deployments",
    "discussions",
    "id-token",
    "issues",
    "models",
    "packages",
    "pages",
    "pull-requests",
    "repository-projects",
    "security-events",
    "statuses",
];

/// The whole-block shorthands, the only legal values of an inline `permissions:`.
const SHORTHANDS: &[&str] = &["read-all", "write-all", "{}"];

/// The per-scope access levels.
const LEVELS: &[&str] = &["read", "write", "none"];

/// An invalid `permissions:` scope does not weaken a job, it deletes the workflow: GitHub
/// refuses to parse the file, so every run ends in 0s with no jobs and the only diagnostic is
/// "This run likely failed because of a workflow file issue" on the Actions tab — nothing in
/// the diff, the PR, or any job log names the line. `auto-release.yml` was bricked exactly
/// that way by a one-line `workflows: write` (run 37501052919): a reviewer reading
/// `contents: write` / `actions: write` / `workflows: write` sees three plausible scopes, and
/// only the third does not exist.
///
/// The trap is specifically `workflows`, because the thing people want when they write it is
/// real: pushing a commit that touches `.github/workflows/**` does need a `workflows` right.
/// But that right lives on the *token*, not in this block — a PAT's `workflow` scope, a
/// fine-grained token's `Workflows: read and write`, or a GitHub App permission. `permissions:`
/// only tunes GITHUB_TOKEN, which GitHub bars from workflow files no matter what it says, so
/// the scope can never appear here however badly a workflow needs the capability.
///
/// Unlike check 9 this does not exempt a generated render: an invalid scope bricks a
/// `pixi-sandbox init` artifact just as hard, and that one ships to consumers.
///
/// actionlint catches this too, and did — but only as a red `pixi run lint`, after the change
/// is written. This check puts the same verdict in `check-repository`, which is what
/// `auto-release.yml` itself runs (the "Validate prepared repository" step) before it is
/// allowed to commit a release.
///
/// Text-based for the same reason as check 9: a YAML parser is a dependency the airlock
/// cannot take, and a `scope: level` pair is visible without one.
///
/// The house opt-out applies. It is not an escape hatch for a typo — it is for the one case
/// this list can be wrong: GitHub ships a new scope and the constant above has not caught up.
pub(super) fn workflow_permissions(root: &Path, failures: &mut Vec<Failure>) -> Result<()> {
    let intro = Regex::new(r"^(\s*)permissions:\s*(.*)$").expect("static regex");
    let entry = Regex::new(r"^\s*([A-Za-z0-9_.-]+)\s*:\s*(.*)$").expect("static regex");
    let mut findings = Vec::new();
    for file in workflow_files(root) {
        let text = crate::util::read(&file)?;
        let lines: Vec<&str> = text.lines().collect();
        let kept: HashSet<usize> = crate::util::lines_without_opt_out(&text)
            .map(|(number, _)| number)
            .collect();
        let name = rel(root, &file);
        let mut index = 0;
        while index < lines.len() {
            if !kept.contains(&(index + 1)) {
                index += 1;
                continue;
            }
            let Some(caps) = intro.captures(lines[index]) else {
                index += 1;
                continue;
            };
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let inline = scrub(&caps[2]);

            // `permissions: read-all` and friends: the value sits on the key's own line, and
            // there is no block to walk.
            if !inline.is_empty() {
                if !SHORTHANDS.contains(&inline.as_str()) {
                    findings.push(format!(
                        "{name}:{}: `permissions: {inline}` is not a whole-block shorthand",
                        index + 1
                    ));
                }
                index += 1;
                continue;
            }

            // The block body: every following line more indented than the key. A blank or
            // comment line is neither an entry nor the end of the block.
            let mut end = index + 1;
            while end < lines.len() {
                let line = lines[end];
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    end += 1;
                    continue;
                }
                if line.len() - line.trim_start().len() <= indent {
                    break;
                }
                // An opted-out line is still part of the block; it is just not judged.
                if !kept.contains(&(end + 1)) {
                    end += 1;
                    continue;
                }
                if let Some(pair) = entry.captures(line) {
                    let scope = &pair[1];
                    let level = scrub(&pair[2]);
                    if !SCOPES.contains(&scope) {
                        findings.push(format!(
                            "{name}:{}: unknown permission scope `{scope}`",
                            end + 1
                        ));
                    } else if !level.is_empty() && !LEVELS.contains(&level.as_str()) {
                        findings.push(format!(
                            "{name}:{}: scope `{scope}` is set to `{level}`, not read/write/none",
                            end + 1
                        ));
                    }
                }
                end += 1;
            }
            index = end;
        }
    }
    if !findings.is_empty() {
        failures.push(Failure::with(
            "a workflow declares a permission scope GitHub does not accept, which makes the whole file unparseable (run 37501052919):", // stale-ref-allowed
            findings,
            "use one of actions, artifact-metadata, attestations, checks, contents, deployments, discussions, id-token, issues, models, packages, pages, pull-requests, repository-projects, security-events, statuses — note `workflows` is not one: pushing .github/workflows/** needs a token carrying the PAT `workflow` scope (or a GitHub App's Workflows permission), not a permissions: entry",
        ));
    }
    Ok(())
}

/// A declared value with any inline comment and surrounding quotes removed.
fn scrub(value: &str) -> String {
    let bare = value.split('#').next().unwrap_or("").trim();
    let unquoted = bare
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .or_else(|| {
            bare.strip_prefix('\'')
                .and_then(|rest| rest.strip_suffix('\''))
        });
    unquoted.unwrap_or(bare).to_string()
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{headlines, valid_fixture};
    use super::{LEVELS, SCOPES};
    use proptest::prelude::*;
    use std::fs;

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
            let block: String = SCOPES
                .iter()
                .map(|scope| format!("  {scope}: {level}\n"))
                .collect();
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
        let failures = super::super::check_repository(dir.path()).expect("checks");
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
}
