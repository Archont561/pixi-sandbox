//! The main-integration guard (TASK-111, decision ADR-29): judge how one push reached `main`.
//!
//! `ci`, the coverage upload and the airlock proof each judge one commit — the pushed tip, or a
//! pull request's head — so a change is proven only when it reaches `main` as one judged unit.
//! Two incidents taught it: five commits pushed straight to `main` at once (#144) left four of
//! them without a coverage base and the airlock never ran, and a pull request merged on a red
//! `ci` (#146). The sanctioned shapes are a pull request merged on GitHub (merge commit or
//! squash), the release workflow's version-stamp commit, and a single hand-made commit that
//! touches nothing the airlock proves — the airlock runs on pull requests only.
//!
//! This runs after the push, in `ci` on `main`: it cannot refuse a push, only turn it red with
//! the remedy. Refusing pushes is branch protection's job, which is the maintainer's to enable.
//! The evidence is what git records — the tip's parents and committer — so the depth-2 checkout
//! `ci` makes is enough, and every shape is tested against a throwaway repository.

use crate::commit_release::COMMIT_MESSAGE_PREFIX;
use crate::util::{github_summary, is_strict_semver, read};
use anyhow::{Context, Result, bail};
use pixi_sandbox_git::{BOT_EMAIL, BOT_NAME, CommitFacts, ShellGit};
use std::fmt;
use std::io::BufRead;
use std::path::Path;

/// The workflow whose `pull_request.paths` filter names what the airlock proves.
const AIRLOCK_WORKFLOW: &str = ".github/workflows/airlock.yml";
/// The committer GitHub records when it writes a commit itself: a pull request merged by merge
/// commit or squash — and, the known limit, an edit made in the web editor.
const GITHUB_COMMITTER: (&str, &str) = ("GitHub", "noreply@github.com");

/// How a push reached `main` in a sanctioned shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sanctioned {
    /// The push created the branch: there is no previous tip to judge against.
    BranchCreated,
    /// The push moved nothing.
    NothingPushed,
    /// GitHub wrote the tip: a pull request merged by merge commit or squash.
    PullRequestMerge,
    /// The release workflow's version-stamp commit.
    ReleaseCommit,
    /// One commit made by hand that touches nothing the airlock proves.
    DirectCommit,
}

/// Why a push did not reach `main` in a sanctioned shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// The tip does not sit directly on the previous tip: several commits arrived at once, or
    /// the push rewrote history.
    NotOneCommit {
        /// The previous tip.
        before: String,
        /// The pushed tip.
        after: String,
    },
    /// A merge commit made in a clone and pushed, so the merged commits skipped review and the
    /// pull request checks.
    LocalMerge {
        /// The merge commit.
        merge: String,
    },
    /// A hand-made commit that changes paths only the airlock proves, and the airlock runs on
    /// pull requests only.
    UnprovenAirlockPaths {
        /// The commit.
        commit: String,
        /// The changed paths the airlock's filter matches, in git's order.
        paths: Vec<String>,
    },
}

/// The guard's answer for one push.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// A sanctioned shape.
    Pass(Sanctioned),
    /// Not a sanctioned shape; the message names the remedy.
    Fail(Violation),
}

impl fmt::Display for Sanctioned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Sanctioned::BranchCreated => "the push created main; there is no previous tip to judge",
            Sanctioned::NothingPushed => "the push moved nothing",
            Sanctioned::PullRequestMerge => "a pull request merged on GitHub",
            Sanctioned::ReleaseCommit => "the release workflow's version-stamp commit",
            Sanctioned::DirectCommit => "one direct commit that touches nothing the airlock proves",
        })
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Violation::NotOneCommit { before, after } => write!(
                f,
                "the pushed tip {} does not sit directly on the previous tip {}: several \
                 commits arrived at once or main's history was rewritten, and ci, coverage and \
                 the airlock judge only the tip — integrate through a pull request merged by \
                 squash or merge commit, or squash to one commit before pushing (decision ADR-29)",
                short(after),
                short(before),
            ),
            Violation::LocalMerge { merge } => write!(
                f,
                "{} is a merge commit made in a clone, not by GitHub, so the commits it merged \
                 skipped the pull request checks — open a pull request and merge it on GitHub \
                 (decision ADR-29)",
                short(merge),
            ),
            Violation::UnprovenAirlockPaths { commit, paths } => write!(
                f,
                "{} changes paths the airlock proves ({}), and the airlock runs on pull \
                 requests only — prove this change now with `gh workflow run airlock.yml --ref \
                 main`, and open a pull request for the next one (decision ADR-29)",
                short(commit),
                paths.join(", "),
            ),
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Verdict::Pass(how) => write!(f, "main integration: {how}"),
            Verdict::Fail(why) => write!(f, "main integration: {why}"),
        }
    }
}

/// The first twelve characters of a commit name, as the messages print it.
fn short(sha: &str) -> &str {
    sha.get(..12).unwrap_or(sha)
}

/// One entry of the airlock's `pull_request.paths` filter: an exact path, or a directory written
/// `dir/**` (every path below it). The two forms the filter uses; any other glob is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Watched {
    Exact(String),
    Below(String),
}

impl Watched {
    fn parse(pattern: &str) -> Result<Self> {
        let (form, base) = match pattern.strip_suffix("/**") {
            Some(dir) => (Watched::Below(format!("{dir}/")), dir),
            None => (Watched::Exact(pattern.to_string()), pattern),
        };
        if base.is_empty() || base.contains(['*', '?', '[', ']', '!', '{', '}']) {
            bail!(
                "{AIRLOCK_WORKFLOW}: the guard reads exact paths and `dir/**` only, not \
                 `{pattern}` — widen the guard (crates/xtask/src/main_push.rs) with the filter"
            );
        }
        Ok(form)
    }

    fn matches(&self, path: &str) -> bool {
        match self {
            Watched::Exact(exact) => path == exact,
            Watched::Below(dir) => path.starts_with(dir.as_str()),
        }
    }
}

/// The airlock workflow's `on.pull_request.paths` filter, read from its text.
///
/// Read as text, like the other workflow checks, so it accepts exactly the shape the workflow
/// uses — a block list under `on:` → `pull_request:` → `paths:` — and refuses any other with a
/// named error. A guard that cannot read its policy must not pass a push by default.
fn airlock_filter(text: &str) -> Result<Vec<Watched>> {
    let lines: Vec<(usize, &str)> = text
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .map(|line| (line.len() - line.trim_start().len(), line.trim()))
        .collect();
    let on = child(&lines, None, "on:")
        .with_context(|| format!("{AIRLOCK_WORKFLOW} has no top-level `on:`"))?;
    let pull_request = child(&lines, Some(on), "pull_request:").with_context(|| {
        format!("{AIRLOCK_WORKFLOW} has no `pull_request:` trigger for the guard to read")
    })?;
    let block = children(&lines, pull_request);
    if block
        .iter()
        .any(|(_, text)| text.starts_with("paths-ignore:"))
    {
        bail!("{AIRLOCK_WORKFLOW}: the guard reads `paths`, not `paths-ignore`");
    }
    let items = child(&lines, Some(pull_request), "paths:")
        .map(|paths| children(&lines, paths))
        .filter(|items| !items.is_empty())
        .with_context(|| {
            format!(
                "{AIRLOCK_WORKFLOW}: `pull_request` has no `paths:` block list for the guard to \
                 read"
            )
        })?;
    items
        .iter()
        .map(|(_, text)| {
            let pattern = text
                .strip_prefix("- ")
                .and_then(|item| unquote(item.trim()))
                .with_context(|| {
                    format!(
                        "{AIRLOCK_WORKFLOW}: `paths` holds `{text}`, which the guard does not \
                         read as one path pattern"
                    )
                })?;
            Watched::parse(pattern)
        })
        .collect()
}

/// A list item's pattern: wholly inside one pair of quotes, or bare. Anything else — a trailing
/// comment, a stray quote — is refused rather than read as part of a path.
fn unquote(item: &str) -> Option<&str> {
    for quote in ['"', '\''] {
        if let Some(inner) = item.strip_prefix(quote) {
            return inner.strip_suffix(quote).filter(|i| !i.contains(quote));
        }
    }
    (!item.contains(['"', '\'', '#', ' '])).then_some(item)
}

/// The index in `lines` of `key` among the direct children of line `parent` — the lines nested
/// under it at the depth of the first one — or among the top-level lines for `None`.
fn child(lines: &[(usize, &str)], parent: Option<usize>, key: &str) -> Option<usize> {
    let (start, scope) = match parent {
        None => (0, lines),
        Some(parent) => (parent + 1, children(lines, parent)),
    };
    let depth = scope.first()?.0;
    scope
        .iter()
        .position(|&(indent, text)| indent == depth && text == key)
        .map(|offset| start + offset)
}

/// The lines nested under line `parent`: those after it indented deeper than it.
fn children<'a>(lines: &'a [(usize, &'a str)], parent: usize) -> &'a [(usize, &'a str)] {
    let indent = lines[parent].0;
    let rest = &lines[parent + 1..];
    let end = rest
        .iter()
        .position(|&(deeper, _)| deeper <= indent)
        .unwrap_or(rest.len());
    &rest[..end]
}

/// Judge the push that moved `main` from `before` to `after` in the repository at `root`.
///
/// `before` may be all zeros (the push created the branch) or a full SHA the checkout does not
/// hold (the push rewrote history); `after` must be present. The airlock filter is read first,
/// on every push, so a filter the guard cannot read fails the first push after it changed.
///
/// # Errors
///
/// Fails when the airlock filter cannot be read or uses a shape the guard does not read, when
/// `before` names nothing, or when git cannot describe `after`.
pub fn judge_push(root: &Path, before: &str, after: &str) -> Result<Verdict> {
    let watched = airlock_filter(&read(&root.join(AIRLOCK_WORKFLOW))?)?;
    if !before.is_empty() && before.bytes().all(|b| b == b'0') {
        return Ok(Verdict::Pass(Sanctioned::BranchCreated));
    }
    let git = ShellGit::new();
    let previous = git
        .rev_parse(root, before)
        .with_context(|| format!("the previous tip `{before}` names no commit"))?;
    let tip = git
        .commit_facts(root, after)
        .with_context(|| format!("the pushed tip `{after}` names no commit"))?;
    if tip.sha == previous {
        return Ok(Verdict::Pass(Sanctioned::NothingPushed));
    }
    if tip.parents.first() != Some(&previous) {
        return Ok(Verdict::Fail(Violation::NotOneCommit {
            before: previous,
            after: tip.sha,
        }));
    }
    let by_github = committed_by(&tip, GITHUB_COMMITTER);
    if tip.parents.len() > 1 {
        return Ok(if by_github {
            Verdict::Pass(Sanctioned::PullRequestMerge)
        } else {
            Verdict::Fail(Violation::LocalMerge { merge: tip.sha })
        });
    }
    if by_github {
        return Ok(Verdict::Pass(Sanctioned::PullRequestMerge));
    }
    if is_release_commit(&tip) {
        return Ok(Verdict::Pass(Sanctioned::ReleaseCommit));
    }
    let paths: Vec<String> = git
        .changed_paths(root, &tip.sha)?
        .into_iter()
        .filter(|path| watched.iter().any(|w| w.matches(path)))
        .collect();
    Ok(if paths.is_empty() {
        Verdict::Pass(Sanctioned::DirectCommit)
    } else {
        Verdict::Fail(Violation::UnprovenAirlockPaths {
            commit: tip.sha,
            paths,
        })
    })
}

fn committed_by(tip: &CommitFacts, (name, email): (&str, &str)) -> bool {
    tip.committer_name == name && tip.committer_email == email
}

/// The release workflow's commit: the bot identity `commit-release` commits under, and the
/// subject it writes. Both, so neither a hand commit titled like a release nor another bot
/// commit is exempt.
fn is_release_commit(tip: &CommitFacts) -> bool {
    committed_by(tip, (BOT_NAME, BOT_EMAIL))
        && tip
            .subject
            .strip_prefix(COMMIT_MESSAGE_PREFIX)
            .and_then(|tag| tag.strip_prefix('v'))
            .is_some_and(is_strict_semver)
}

/// Judge the push and report it: a run-summary line either way, and an error naming the remedy
/// when the push is not a sanctioned shape.
///
/// # Errors
///
/// Fails when the push is not a sanctioned shape, or when [`judge_push`] cannot judge it.
pub fn check_main_push(root: &Path, before: &str, after: &str) -> Result<()> {
    let verdict = judge_push(root, before, after)?;
    github_summary(&format!("## Main integration\n\n- {verdict}\n"));
    match verdict {
        Verdict::Pass(_) => {
            println!("{verdict}");
            Ok(())
        }
        Verdict::Fail(why) => bail!("main integration: {why}"),
    }
}

/// Check Git's pre-push ref updates, applying the main-integration guard only to `main`.
///
/// Git supplies one whitespace-separated line per ref: local ref, local object ID, remote ref,
/// and remote object ID. Other branches remain untouched by this optional local convenience.
/// The CI check remains the repository-wide authority (ADR-29).
///
/// # Errors
///
/// Fails when a nonempty input line is malformed, a main deletion is attempted, or a main
/// update is not in a sanctioned shape.
pub fn check_main_push_hook(root: &Path, input: impl BufRead) -> Result<()> {
    let mut saw_update = false;
    let mut saw_main_update = false;
    for (index, line) in input.lines().enumerate() {
        let line =
            line.with_context(|| format!("reading Git pre-push update line {}", index + 1))?;
        if line.trim().is_empty() {
            continue;
        }
        saw_update = true;
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 4 {
            bail!(
                "Git pre-push update line {} has {} field(s), expected 4",
                index + 1,
                fields.len()
            );
        }
        let local_sha = fields[1];
        let remote_ref = fields[2];
        let remote_sha = fields[3];
        if remote_ref != "refs/heads/main" {
            continue;
        }
        saw_main_update = true;
        if local_sha.bytes().all(|byte| byte == b'0') {
            bail!(
                "Git pre-push update deletes refs/heads/main; the ADR-29 guard can only judge a commit update"
            );
        }
        check_main_push(root, remote_sha, local_sha)?;
    }
    if !saw_update {
        println!("main integration: no ref updates; skipped");
    } else if !saw_main_update {
        println!("main integration: push does not target refs/heads/main; skipped");
    }
    Ok(())
}
