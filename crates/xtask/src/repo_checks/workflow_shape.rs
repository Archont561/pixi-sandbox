//! Check 9 (task-36): every workflow `run:` is a single command line.

use super::Failure;
use super::support::{rel, workflow_files};
use anyhow::Result;
use regex::Regex;
use std::path::Path;

/// The reviewed exception marker: a step that deliberately carries a multi-line `run:` block
/// states so with this on the step, the line above it, or a comment inside the block — and
/// the marker text is where the reason lives. TASK-76 removed the blanket exemption this
/// check used to grant every `init`-marked artifact: the generated publisher no longer
/// carries a multi-line block at all, so the one-command-per-step rule now applies to
/// rendered consumer workflows unaided, and only a named, justified exception (this marker,
/// with its reason) survives.
#[doc(hidden)] // test boundary: the opt-out marker (tests/repo_checks_workflow_shape.rs)
pub const MULTI_RUN_ALLOWED: &str = "multiline-run-allowed";

/// task-36: no workflow step carries more than one command line. The house rule is that a
/// step is `uses:`, a single `pixi run <task>` line, or a one-line host bootstrap; anything
/// longer is a pixi task, and anything with logic in it is an xtask subcommand. Without a
/// guard the workflows re-grow embedded shell — they carried ~280 lines of it, in three
/// dialects, when this task started — and per-platform shell is exactly what killed the
/// v0.3.6 release on macOS.
///
/// Text-based on purpose (like every check here): a YAML parser would be a new dependency
/// the airlock cannot take until a transport carries it, and the shape being policed — a
/// `run:` block scalar — is visible without one.
///
/// What counts as one command: a folded scalar (`run: >-`) folds its lines into a single
/// command and is never flagged; inside a literal block (`run: |`), blank lines and `#`
/// comments are not commands, and a line ending in `\\` continues onto the next — so a
/// wrapped single command stays legal. Only two or more logical commands in one literal
/// block fire the check.
pub(super) fn workflow_shape(root: &Path, failures: &mut Vec<Failure>) -> Result<()> {
    let block_intro =
        Regex::new(r"^(\s*)(?:-\s+)?run:\s*([|>])[+-]?\s*(#.*)?$").expect("static regex");
    let mut findings = Vec::new();
    for file in workflow_files(root) {
        let text = crate::util::read(&file)?;
        let lines: Vec<&str> = text.lines().collect();
        let mut index = 0;
        while index < lines.len() {
            let Some(caps) = block_intro.captures(lines[index]) else {
                index += 1;
                continue;
            };
            let intro = index;
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let folded = &caps[2] == ">";

            // The block body: every following line that is blank or more indented than the
            // `run:` key. A block scalar must be the last key of its step, so the first
            // less-indented non-blank line ends it.
            let mut end = intro + 1;
            while end < lines.len() {
                let line = lines[end];
                let line_indent = line.len() - line.trim_start().len();
                if !line.trim().is_empty() && line_indent <= indent {
                    break;
                }
                end += 1;
            }

            // The reviewed exception: the marker on the `run:` line, the line above it, or
            // any comment inside the block.
            let opted_out = lines[intro].contains(MULTI_RUN_ALLOWED)
                || intro > 0 && lines[intro - 1].contains(MULTI_RUN_ALLOWED)
                || lines[intro + 1..end].iter().any(|line| {
                    line.trim_start().starts_with('#') && line.contains(MULTI_RUN_ALLOWED)
                });

            if !folded && !opted_out {
                let body = &lines[intro + 1..end];
                let commands = count_commands(body);
                if commands > 1 {
                    findings.push(format!(
                        "{}:{}: `run:` block holds {} command lines",
                        rel(root, &file),
                        intro + 1,
                        commands
                    ));
                }
            }
            index = end;
        }
    }
    if !findings.is_empty() {
        failures.push(Failure::with(
            "a workflow step runs a multi-line command block, which the house rule keeps out of workflows:",
            findings,
            "make the step a single `pixi run <task>` line (a task in pixi.toml, or an xtask subcommand when it carries logic); a reviewed exception carries `multiline-run-allowed` on the step",
        ));
    }
    Ok(())
}

/// Logical command count of a literal block body: comments and blanks are not commands, and
/// a trailing backslash joins a line to the next one.
fn count_commands(body: &[&str]) -> usize {
    let mut commands = 0;
    let mut continuing = false;
    for line in body {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if !continuing {
            commands += 1;
        }
        continuing = trimmed.ends_with('\\');
    }
    commands
}
