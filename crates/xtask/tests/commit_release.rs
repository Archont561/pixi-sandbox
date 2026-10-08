//! `commit_release` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

use pixi_sandbox_git::ShellGit;
use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;
use xtask::commit_release::*;

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
