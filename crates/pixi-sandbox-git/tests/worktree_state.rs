//! What a work tree would commit, answered by `git` rather than by a directory walk.
//!
//! `ShellGit::is_work_tree` and `ShellGit::ls_publishable` exist for one consumer: the starter
//! lane's publisher, which has to tell "bytes the tooling just wrote" apart from "bytes the next
//! revision would carry" (task-85). A directory listing cannot make that distinction, and neither
//! can `git status --porcelain`, which stays silent about tracked-and-unmodified files. So these
//! tests pin the answer against a real `git` in a tempdir — the style `tests/publish.rs` uses for
//! the shell side of the protocol.

use pixi_sandbox_git::ShellGit;
use std::path::Path;
use std::process::Command as StdCommand;

fn run_git(args: &[&str], cwd: &Path) {
    let identity = [
        "-c",
        "user.name=test",
        "-c",
        "user.email=test@example.invalid",
    ];
    let mut argv: Vec<&str> = identity.to_vec();
    argv.extend_from_slice(args);
    let out = StdCommand::new("git")
        .args(&argv)
        .current_dir(cwd)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {argv:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn write(dir: &Path, rel: &str, body: &str) {
    let full = dir.join(rel);
    std::fs::create_dir_all(full.parent().expect("parent")).expect("mkdir");
    std::fs::write(full, body).expect("write");
}

fn commit_count(dir: &Path) -> String {
    let out = StdCommand::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .current_dir(dir)
        .output()
        .expect("git runs");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A repository with one commit: `pixi.toml` tracked, `.gitignore` ignoring `.pixi/`.
fn seeded(dir: &Path) {
    run_git(&["init", "-q"], dir);
    write(dir, ".gitignore", ".pixi/\n");
    write(dir, "pixi.toml", "[workspace]\n");
    run_git(&["add", "-A"], dir);
    run_git(&["commit", "-qm", "seed"], dir);
}

#[test]
fn a_plain_directory_is_not_a_work_tree() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(
        !ShellGit::new()
            .is_work_tree(dir.path())
            .expect("git answers")
    );
}

#[test]
fn a_checked_out_repository_is_a_work_tree() {
    let dir = tempfile::tempdir().expect("tempdir");
    seeded(dir.path());
    assert!(
        ShellGit::new()
            .is_work_tree(dir.path())
            .expect("git answers")
    );
}

/// The whole reason the question is asked of git: runtime state the ignore rules cover is not part
/// of the revision, while a new source file nobody has staged yet is.
#[test]
fn the_answer_holds_the_index_plus_untracked_files_the_ignore_rules_let_through() {
    let dir = tempfile::tempdir().expect("tempdir");
    seeded(dir.path());
    write(dir.path(), ".pixi/envs/default/rg", "stale environment");
    write(dir.path(), "restore.sh", "echo restore\n");

    let mut paths = ShellGit::new()
        .ls_publishable(dir.path())
        .expect("ls-files");
    paths.sort();
    assert_eq!(
        paths,
        vec![
            ".gitignore".to_string(),
            "pixi.toml".to_string(),
            "restore.sh".to_string(),
        ]
    );
}

/// A runtime directory a previous revision committed stays in the answer — the ignore file no
/// longer protects it — and leaves it only once the removal is staged. Reporting the second case
/// as a finding is the false-positive half of the same bug.
#[test]
fn a_tracked_file_survives_until_its_removal_is_staged() {
    let dir = tempfile::tempdir().expect("tempdir");
    seeded(dir.path());
    write(dir.path(), ".pixi/stale", "committed by mistake");
    run_git(&["add", "-f", ".pixi/stale"], dir.path());
    run_git(&["commit", "-qm", "mistake"], dir.path());

    let git = ShellGit::new();
    assert!(
        git.ls_publishable(dir.path())
            .expect("ls-files")
            .contains(&".pixi/stale".to_string()),
        "a committed runtime directory must stay in the answer"
    );

    std::fs::remove_file(dir.path().join(".pixi/stale")).expect("rm");
    assert!(
        git.ls_publishable(dir.path())
            .expect("ls-files")
            .contains(&".pixi/stale".to_string()),
        "an unstaged removal does not change the next commit"
    );

    run_git(&["add", "-A"], dir.path());
    assert!(
        !git.ls_publishable(dir.path())
            .expect("ls-files")
            .contains(&".pixi/stale".to_string()),
        "a staged deletion leaves the next commit, so it must leave the answer too"
    );
}

/// `-z` is not decoration: a path with a space is one entry, not two.
#[test]
fn a_path_holding_a_space_is_one_entry() {
    let dir = tempfile::tempdir().expect("tempdir");
    seeded(dir.path());
    write(dir.path(), "docs/a file.md", "# hello\n");
    let paths = ShellGit::new()
        .ls_publishable(dir.path())
        .expect("ls-files");
    assert!(paths.iter().any(|p| p == "docs/a file.md"), "{paths:?}");
}

/// A fresh clone carries the committed tree and none of the directory's working state, which is
/// what makes it the right thing to run a starter's development task in.
#[test]
fn a_fresh_clone_carries_the_committed_tree_and_nothing_else() {
    let dir = tempfile::tempdir().expect("tempdir");
    let seed = dir.path().join("seed");
    std::fs::create_dir_all(&seed).expect("mkdir");
    seeded(&seed);
    write(&seed, ".pixi/envs/default/rg", "runtime state");
    write(&seed, "only-in-the-work-tree.txt", "uncommitted\n");

    let clone = dir.path().join("clone");
    ShellGit::new()
        .clone_fresh(&seed, &clone)
        .expect("clone runs");

    let paths = ShellGit::new().ls_publishable(&clone).expect("ls-files");
    assert_eq!(
        paths,
        vec![".gitignore".to_string(), "pixi.toml".to_string()]
    );
    assert!(!clone.join(".pixi").exists(), "runtime state leaked");
    assert!(
        !clone.join("only-in-the-work-tree.txt").exists(),
        "an uncommitted file must not reach a clone"
    );
    assert_eq!(commit_count(&clone), "1");
}
