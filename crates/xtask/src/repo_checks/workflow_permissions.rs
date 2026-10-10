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
#[doc(hidden)] // test boundary: the accepted scopes and levels (tests/repo_checks_workflow_permissions.rs)
pub const SCOPES: &[&str] = &[
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
#[doc(hidden)] // test boundary: the accepted scopes and levels (tests/repo_checks_workflow_permissions.rs)
pub const LEVELS: &[&str] = &["read", "write", "none"];

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
/// only tunes `GITHUB_TOKEN`, which GitHub bars from workflow files no matter what it says, so
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
