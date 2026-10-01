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

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as StdCommand;

    /// A real repository with a real bare remote, in tempdirs. The guards this module exists
    /// for are about git's actual index semantics, so an in-memory mock would prove nothing.
    fn repo_with_remote() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().expect("tempdir");
        let remote_path = dir.path().join("remote.git");
        run(
            &["init", "-q", "--bare"],
            dir.path(),
            &remote_path.to_string_lossy(),
        );
        run(&["init", "-q", "-b", "main"], dir.path(), "");
        // Tracked from the start: the unaccounted-worktree guard asks about *modified*
        // files, and an untracked file is invisible to `git diff` — prepare-release always
        // modifies files the repository already carries.
        fs::write(dir.path().join("CHANGELOG.md"), "# changelog\n").expect("write");
        fs::write(dir.path().join("Cargo.toml"), "version = \"0.3.6\"\n").expect("write");
        run(&["add", "."], dir.path(), "");
        run(&["commit", "-q", "-m", "initial"], dir.path(), "");
        run(
            &["remote", "add", "origin", &remote_path.to_string_lossy()],
            dir.path(),
            "",
        );
        run(&["push", "-q", "origin", "main"], dir.path(), "");
        (dir, remote_path.to_string_lossy().into_owned())
    }

    fn run(args: &[&str], cwd: &Path, extra: &str) -> String {
        let mut command = StdCommand::new("git");
        command.args(args);
        if !extra.is_empty() {
            command.arg(extra);
        }
        // Fixture identity, per command: the fixture must not read (or need) the
        // host's git config — only the code under test carries an identity, via ShellGit.
        command
            .env("GIT_AUTHOR_NAME", "fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
            .env("GIT_COMMITTER_NAME", "fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid");
        command.current_dir(cwd);
        let out = command.output().expect("git runs");
        assert!(
            out.status.success(),
            "git {args:?} {extra} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn dirty(root: &Path, touched: &[&str]) {
        fs::write(root.join("CHANGELOG.md"), "new entry\n").expect("write");
        fs::write(root.join("Cargo.toml"), "version = \"0.3.7\"\n").expect("write");
        fs::write(root.join(".release-touched"), touched.join("\n")).expect("write");
    }

    fn head_count(root: &Path) -> usize {
        run(&["rev-list", "--count", "HEAD"], root, "")
            .parse()
            .expect("number")
    }

    fn remote_tags(root: &Path, remote: &str) -> String {
        run(&["ls-remote", "--tags"], root, remote)
    }

    #[test]
    fn a_prepared_tree_commits_tags_and_pushes_exactly_what_touched_lists() {
        let (dir, remote) = repo_with_remote();
        let root = dir.path();
        dirty(root, &["CHANGELOG.md", "Cargo.toml"]);

        commit_release(root, "v0.3.7", false, &remote).expect("release");

        assert_eq!(
            run(&["log", "-1", "--pretty=%s"], root, ""),
            "chore(release): v0.3.7"
        );
        assert_eq!(
            run(&["log", "-1", "--pretty=%an"], root, ""),
            "pixi-sandbox[bot]",
            "the release commit is automation, and carries the bot identity"
        );
        for file in ["CHANGELOG.md", "Cargo.toml"] {
            assert!(
                run(
                    &["show", "--name-only", "--pretty=format:", "HEAD"],
                    root,
                    ""
                )
                .contains(file),
                "{file} must be in the release commit"
            );
        }
        assert_eq!(head_count(root), 2, "exactly one new commit");
        assert!(run(&["tag", "-l"], root, "").contains("v0.3.7"));
        assert!(
            remote_tags(root, &remote).contains("v0.3.7"),
            "the tag reached the remote"
        );
        // The remote is addressed by path (no `origin` name in the fixture), so git keeps no
        // remote-tracking ref — ask the remote itself what its main points at.
        let local_main = run(&["rev-parse", "main"], root, "");
        assert!(
            run(&["ls-remote"], root, &remote).contains(&format!("{local_main}\trefs/heads/main")),
            "main reached the remote"
        );
        assert!(
            ShellGit::new()
                .unstaged_modifications(root)
                .expect("clean")
                .is_empty(),
            "nothing modified is left behind"
        );
    }

    #[test]
    fn a_missing_release_touched_file_refuses_to_guess() {
        let (dir, remote) = repo_with_remote();
        let root = dir.path();
        fs::write(root.join("Cargo.toml"), "version = \"0.3.7\"\n").expect("write");

        let error = commit_release(root, "v0.3.7", false, &remote).expect_err("no report");
        let message = format!("{error:#}");
        assert!(
            message.contains("refusing to guess"),
            "must refuse: {message}"
        );
        assert_eq!(head_count(root), 1, "nothing was committed");
    }

    #[test]
    fn an_unaccounted_modification_fails_the_release_and_commits_nothing() {
        let (dir, remote) = repo_with_remote();
        let root = dir.path();
        // The report lists one of the two modified files.
        dirty(root, &["CHANGELOG.md"]);

        let error = commit_release(root, "v0.3.7", false, &remote).expect_err("unaccounted");
        let message = format!("{error:#}");
        assert!(
            message.contains("Cargo.toml"),
            "must name the unaccounted file: {message}"
        );
        assert_eq!(head_count(root), 1, "nothing was committed");
        assert!(remote_tags(root, &remote).is_empty(), "no tag was pushed");
    }

    #[test]
    fn an_existing_remote_tag_refuses_before_anything_is_staged() {
        let (dir, remote) = repo_with_remote();
        let root = dir.path();
        run(&["tag", "v0.3.7"], root, "");
        run(&["push", "-q", "origin", "refs/tags/v0.3.7"], root, "");
        dirty(root, &["CHANGELOG.md", "Cargo.toml"]);

        let error = commit_release(root, "v0.3.7", false, &remote).expect_err("tag exists");
        let message = format!("{error:#}");
        assert!(
            message.contains("already exists"),
            "must say the tag is taken: {message}"
        );
        assert_eq!(head_count(root), 1, "nothing was committed");
        assert!(
            !run(&["diff", "--cached", "--name-only"], root, "").contains("CHANGELOG.md"),
            "the refusal fires before staging"
        );
    }

    #[test]
    fn dry_run_touches_nothing() {
        let (dir, remote) = repo_with_remote();
        let root = dir.path();
        dirty(root, &["CHANGELOG.md", "Cargo.toml"]);

        commit_release(root, "v0.3.7", true, &remote).expect("dry run");

        assert_eq!(head_count(root), 1, "no commit");
        assert!(run(&["tag", "-l"], root, "").is_empty(), "no tag");
        assert!(remote_tags(root, &remote).is_empty(), "nothing pushed");
        assert!(
            run(&["diff", "--cached", "--name-only"], root, "").is_empty(),
            "not even staged"
        );
    }

    #[test]
    fn a_tag_not_shaped_like_a_version_is_refused() {
        let (dir, remote) = repo_with_remote();
        for bad in ["0.3.7", "v1.2", "v1.2.3-rc1", "latest"] {
            let error = commit_release(dir.path(), bad, false, &remote).expect_err("shape");
            assert!(
                format!("{error:#}").contains("not shaped"),
                "{bad} must be refused"
            );
        }
        assert_eq!(head_count(dir.path()), 1, "nothing was committed");
    }
}
