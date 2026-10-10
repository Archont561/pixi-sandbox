//! `commit_files` and `push_branch` — the consumer-checkout operations the upgrade lane
//! needs (TASK-76): commit reviewed file changes onto a fresh branch of the consumer's own
//! checkout, and push that branch so a pull request can be opened against it.
//!
//! The mock tests state the contract (the operation is recorded, the push can be refused).
//! The shell tests prove the same things against a real `git` and a local bare remote — and
//! the recording runner asserts the argv, which is the only way to see "reset the branch at
//! HEAD" (`checkout -B`) and "authored by the bot identity" as facts rather than intentions.

use pixi_sandbox_git::{
    Error, FakeGit, FileCommit, FileCommitted, GitProtocol, Op, RecordingRunner, ShellGit,
};
use rstest::rstest;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::Arc;

const BRANCH: &str = "pixi-sandbox-upgrade/0.6.1";
const OWNED: &str = ".github/workflows/publish-sandbox.yml";
const MESSAGE: &str = "chore(pixi-sandbox): upgrade generated files to 0.6.1";
const BOT_IDENTITY: &str =
    "pixi-sandbox[bot] <41898282+github-actions[bot]@users.noreply.github.com>";

fn run_git(args: &[&str], cwd: &Path) -> String {
    let out = StdCommand::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A checked-out "consumer" repository: one commit on the default branch carrying one
/// owned file, the state the upgrade lane starts from.
fn work_tree() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("repo");
    std::fs::create_dir_all(root.join(".github/workflows")).expect("mkdir");
    std::fs::write(root.join(OWNED), b"version 1\n").expect("write");
    run_git(&["init", "-q"], &root);
    run_git(&["add", "."], &root);
    run_git(
        &[
            "-c",
            "user.name=setup",
            "-c",
            "user.email=setup@example.com",
            "commit",
            "-q",
            "-m",
            "main",
        ],
        &root,
    );
    (dir, root)
}

fn bare_remote() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("remote.git");
    run_git(
        &["init", "-q", "--bare", path.to_str().expect("utf-8 path")],
        dir.path(),
    );
    (dir, path.to_string_lossy().into_owned())
}

fn commit<'a>(root: &'a Path, files: &'a [PathBuf]) -> FileCommit<'a> {
    FileCommit {
        work_tree: root,
        branch: BRANCH,
        files,
        message: MESSAGE,
    }
}

fn owned() -> Vec<PathBuf> {
    vec![PathBuf::from(OWNED)]
}

// ---------------------------------------------------------------- the mock

#[rstest]
fn the_mock_records_commit_files_and_returns_the_scripted_result() {
    let git = FakeGit::new();
    git.script_commit(FileCommitted {
        commit: String::new(),
        changed: false,
        patch: Vec::new(),
    });

    let committed = git
        .commit_files(&commit(Path::new("/work-tree"), &owned()))
        .expect("commit_files");

    assert!(!committed.changed);
    assert_eq!(
        git.ops(),
        vec![Op::CommitFiles {
            branch: BRANCH.to_string(),
            files: vec![OWNED.to_string()],
            message: MESSAGE.to_string(),
        }]
    );
}

#[rstest]
fn the_mock_defaults_to_a_changed_commit_when_nothing_is_scripted() {
    let git = FakeGit::new();
    let committed = git
        .commit_files(&commit(Path::new("/work-tree"), &owned()))
        .expect("commit_files");
    assert!(committed.changed);
    assert!(!committed.commit.is_empty());
    assert!(!committed.patch.is_empty());
}

#[rstest]
fn the_mock_records_a_forced_push_and_can_refuse_it() {
    let git = FakeGit::new();
    git.push_branch(Path::new("/work-tree"), "origin", BRANCH, true)
        .expect("push");
    assert_eq!(
        git.ops(),
        vec![Op::Push {
            remote: "origin".to_string(),
            branch: BRANCH.to_string(),
            forced: true,
        }]
    );

    git.fail_next_push("the token was refused while pushing workflow files");
    let err = git
        .push_branch(Path::new("/work-tree"), "origin", BRANCH, true)
        .unwrap_err();
    assert!(
        matches!(err, Error::Rejected { .. }),
        "a refused push is its own error: {err:?}"
    );
}

// ------------------------------------------------------- the real git

#[rstest]
fn commit_files_resets_the_branch_stages_and_commits_under_the_bot_identity() {
    let (_dir, root) = work_tree();
    // The regeneration: the owned file changed on disk.
    std::fs::write(root.join(OWNED), b"version 2\n").expect("write");

    let git = ShellGit::new();
    let committed = git
        .commit_files(&commit(&root, &owned()))
        .expect("commit_files");

    assert!(committed.changed);
    assert!(!committed.commit.is_empty());
    assert_eq!(
        run_git(&["rev-parse", "--abbrev-ref", "HEAD"], &root),
        BRANCH
    );
    assert_eq!(run_git(&["log", "--format=%s", "-1"], &root), MESSAGE);
    assert_eq!(
        run_git(&["log", "--format=%an <%ae>", "-1"], &root),
        BOT_IDENTITY
    );
    assert_eq!(run_git(&["rev-parse", "HEAD"], &root), committed.commit);
    // The patch artifact is the commit's binary diff, naming the owned file.
    let patch = String::from_utf8_lossy(&committed.patch);
    assert!(patch.contains("publish-sandbox.yml"), "{patch}");
    assert!(patch.contains("version 1"), "{patch}");
    assert!(patch.contains("version 2"), "{patch}");
}

#[rstest]
fn commit_files_reports_nothing_to_propose_when_the_files_match_head() {
    let (_dir, root) = work_tree();
    let git = ShellGit::new();

    let committed = git
        .commit_files(&commit(&root, &owned()))
        .expect("commit_files");

    assert!(!committed.changed);
    assert!(committed.commit.is_empty());
    assert!(committed.patch.is_empty());
    assert_eq!(run_git(&["rev-list", "--count", "HEAD"], &root), "1");
}

#[rstest]
fn commit_files_runs_the_expected_argv_in_order() {
    let (_dir, root) = work_tree();
    std::fs::write(root.join(OWNED), b"version 2\n").expect("write");
    let runner = Arc::new(RecordingRunner::new());
    let git = ShellGit::with_runner(Box::new(runner.clone()));

    git.commit_files(&commit(&root, &owned()))
        .expect("commit_files");

    let rendered = runner.rendered();
    // Every command carries the identity pair (`-c user.name=… -c user.email=…`) as its
    // first arguments; strip `git` plus those four words to see the subcommand and its args.
    let tail: Vec<String> = rendered
        .iter()
        .map(|line| {
            let mut words = line.split_whitespace();
            for _ in 0..5 {
                words.next();
            }
            words.collect::<Vec<_>>().join(" ")
        })
        .collect();
    assert_eq!(
        tail,
        vec![
            format!("checkout -B {BRANCH}"),
            format!("add -- {OWNED}"),
            "diff --cached --quiet".to_string(),
            format!("commit -m {MESSAGE}"),
            "rev-parse HEAD".to_string(),
            "diff HEAD^ HEAD --binary".to_string(),
        ]
    );
}

#[rstest]
fn push_branch_pushes_the_branch_and_force_replaces_it() {
    let (_dir, root) = work_tree();
    let (_remote_dir, remote) = bare_remote();
    let git = ShellGit::new();

    // First proposal: the branch does not exist on the remote yet.
    std::fs::write(root.join(OWNED), b"version 2\n").expect("write");
    git.commit_files(&commit(&root, &owned()))
        .expect("commit_files");
    git.push_branch(&root, &remote, BRANCH, true).expect("push");
    let first_tip = run_git(&["rev-parse", BRANCH], &root);
    assert_eq!(
        run_git(&["rev-parse", &format!("refs/heads/{BRANCH}")], &root),
        first_tip
    );

    // A second proposal onto the same version-derived branch name replaces the remote branch.
    std::fs::write(root.join(OWNED), b"version 3\n").expect("write");
    git.commit_files(&commit(&root, &owned()))
        .expect("commit_files");
    git.push_branch(&root, &remote, BRANCH, true)
        .expect("force push");
    let second_tip = run_git(&["rev-parse", BRANCH], &root);
    assert_ne!(first_tip, second_tip);
    let remote_tip = StdCommand::new("git")
        .args([
            "--git-dir",
            &remote,
            "rev-parse",
            &format!("refs/heads/{BRANCH}"),
        ])
        .output()
        .expect("git runs");
    assert!(remote_tip.status.success());
    assert_eq!(
        String::from_utf8_lossy(&remote_tip.stdout).trim(),
        second_tip
    );
}

#[rstest]
fn a_refused_push_branch_reports_the_remote_rejection() {
    let (_dir, root) = work_tree();
    let git = ShellGit::new();
    let err = git
        .push_branch(&root, "/nonexistent/remote.git", BRANCH, true)
        .unwrap_err();
    assert!(
        matches!(err, Error::Rejected { .. }),
        "a refused push is its own error: {err:?}"
    );
}
