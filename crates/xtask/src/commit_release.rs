//! The commit/tag/push half of a release (task-36).
//!
//! `prepare-release` stamps the tree and records what it changed; this command turns that
//! prepared tree into the release commit, the annotated tag, and the two pushes. It replaces
//! a 40-line shell block in `auto-release.yml` whose guards each encode a real incident:
//! the `.release-touched` staging exists because a hand-kept `git add` list once dropped
//! ~90 documentation fixes the repin step had made, and the unaccounted-worktree guard asks
//! worktree-against-index because `git status --porcelain` and `git diff-index HEAD` also
//! report staged changes and therefore rejected every release while looking correct.
//!
//! Git goes through `pixi-sandbox-git` (D9) — the working-tree primitives on `ShellGit`, not
//! the transport trait — and every guard is tested against a tempdir repository with a real
//! bare remote, never this checkout (D10).

use anyhow::{Context, Result, bail};
use pixi_sandbox_git::ShellGit;
use std::fs;
use std::path::Path;

/// The release commit message; `chore(release):` so convco accepts it on main.
const COMMIT_MESSAGE_PREFIX: &str = "chore(release): ";
/// One release lane, and it is main: the branch the workflow checks out and pushes back to.
const BRANCH_REF: &str = "HEAD:main";

/// Commit, tag, and push the tree `prepare-release` left in `root`.
///
/// Order matters and matches the shell this replaced: the remote-tag refusal fires before
/// anything is staged (a dry-run over an existing tag is a red build, not a diff), and the
/// dry-run stops after the diff, before `.release-touched` is even required.
///
/// # Errors
///
/// Fails when `tag` is not `vX.Y.Z`, when `tag` already exists on `remote`, when the touched-file list cannot be read, or when staging, committing, tagging, or pushing fails.
#[allow(clippy::too_many_arguments)]
pub fn commit_release(root: &Path, tag: &str, dry_run: bool, remote: &str) -> Result<()> {
    let semver = tag.strip_prefix('v').unwrap_or_default();
    let parts: Vec<&str> = semver.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
    {
        bail!("release tag '{tag}' is not shaped vX.Y.Z — a tag flows into refs and URLs");
    }

    let git = ShellGit::new();

    if git.remote_tag_exists(remote, tag)? {
        bail!("tag {tag} already exists on {remote} — nothing released");
    }

    if dry_run {
        print!("{}", git.worktree_diff(root)?);
        eprintln!("::notice::dry-run: prepared {tag} but not committing, tagging, or releasing");
        return Ok(());
    }

    // Stage what prepare-release reported it changed, not a hand-kept list: a file the run
    // modified but failed to record is still a file it should not have touched, and the
    // unaccounted guard below is the backstop for exactly that.
    let touched_name =
        std::env::var("RELEASE_TOUCHED_FILE").unwrap_or_else(|_| ".release-touched".to_string());
    let touched = root.join(&touched_name);
    let listed = fs::read_to_string(&touched).with_context(|| {
        format!("prepare-release did not write {touched_name} — refusing to guess what to stage")
    })?;
    let files: Vec<&str> = listed
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    git.add_files(root, &files)?;

    // The list is a report, not a licence: anything left modified but unrecorded would go
    // uncommitted and be lost on the next checkout. `unstaged_modifications` is
    // worktree-against-index, so the files staged just above are not reported here.
    let unaccounted = git.unstaged_modifications(root)?;
    if !unaccounted.is_empty() {
        bail!(
            "modified files missing from {touched_name}:\n  {}",
            unaccounted.join("\n  ")
        );
    }

    git.commit(root, &format!("{COMMIT_MESSAGE_PREFIX}{tag}"))?;
    git.tag_annotated(root, tag)?;
    git.push_refspec(root, remote, BRANCH_REF)?;
    git.push_refspec(root, remote, &format!("refs/tags/{tag}"))?;
    println!("released {tag}: commit and tag pushed to {remote} ({BRANCH_REF})");
    Ok(())
}
