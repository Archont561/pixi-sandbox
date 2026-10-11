//! The three working-tree primitives the release lane's consumer proof needs (TASK-89): a bare
//! remote to publish into, a work tree whose branch is named rather than inherited, and a
//! detached checkout of a fetched revision.
//!
//! They exist because the proof is a *consumer's* journey — `git init --bare`, `git clone`,
//! `git fetch <branch>`, `git checkout --detach FETCH_HEAD` — and invariant 8 keeps every one
//! of those inside this crate. `GitProtocol::fetch_into` is deliberately not reused: it
//! materialises a tree with plumbing and no work tree, so a proof built on it would be a proof
//! about `pixi-sandbox-git`'s own materialisation instead of about what a consumer's `git`
//! actually does. These tests pin the primitives against a real `git` in tempdirs (D10), the
//! style `tests/publish.rs` uses for the shell side of the protocol.

use pixi_sandbox_git::{PreviewRunner, ShellGit};
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::Arc;

const BRANCH: &str = "sandbox/consumer-proof-linux-64";

fn run_git(args: &[&str], cwd: &Path) -> String {
    let outcome = try_git(args, cwd);
    assert!(outcome.0, "git {args:?} failed: {}", outcome.2);
    outcome.1
}

/// `git` without the success assertion, for the commands whose *failure* is the fact under
/// test — `symbolic-ref -q HEAD` exits 1 precisely when HEAD is detached. Returns
/// (succeeded, stdout, stderr).
fn try_git(args: &[&str], cwd: &Path) -> (bool, String, String) {
    let identity = [
        "-c",
        "user.name=test",
        "-c",
        "user.email=test@example.invalid",
    ];
    let mut full_argv: Vec<&str> = identity.to_vec();
    full_argv.extend_from_slice(args);
    let out = StdCommand::new("git")
        .args(&full_argv)
        .current_dir(cwd)
        .output()
        .expect("git runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
        String::from_utf8_lossy(&out.stderr).trim().to_string(),
    )
}

fn write(dir: &Path, rel: &str, body: &str) {
    let full = dir.join(rel);
    std::fs::create_dir_all(full.parent().expect("parent")).expect("mkdir");
    std::fs::write(full, body).expect("write");
}

/// The consumer's committed fixture: one revision on a named branch, nothing else.
fn committed_consumer(root: &Path) -> String {
    write(root, "README.md", "consumer fixture\n");
    write(
        root,
        "pixi.toml",
        "[workspace]\nname = \"consumer-proof\"\n",
    );
    let git = ShellGit::new().identity("consumer-proof", "consumer-proof@example.invalid");
    git.init_repo_branch(root, "main").expect("init");
    git.add_files(root, &["README.md", "pixi.toml"])
        .expect("add");
    git.commit(root, "test: consumer fixture").expect("commit");
    run_git(&["rev-parse", "HEAD"], root)
}

/// A consumer repository that has fetched `BRANCH` but not checked anything out: the state
/// `git clone` of an empty bare remote leaves behind.
fn fetched_checkout() -> (tempfile::TempDir, PathBuf, String) {
    let remote_dir = tempfile::tempdir().expect("tempdir");
    let remote = remote_dir.path().join("consumer-proof.git");
    let git = ShellGit::new();
    git.init_bare(&remote).expect("init_bare");

    let source_dir = tempfile::tempdir().expect("tempdir");
    let source = source_dir.path().join("consumer");
    std::fs::create_dir_all(&source).expect("mkdir");
    let published = committed_consumer(&source);
    git.push_refspec(
        &source,
        remote.to_str().expect("utf-8"),
        &format!("HEAD:{BRANCH}"),
    )
    .expect("push");

    let checkout_dir = tempfile::tempdir().expect("tempdir");
    let checkout = checkout_dir.path().join("fresh-checkout");
    git.clone_fresh(&remote, &checkout).expect("clone");
    git.fetch_shallow(&checkout, "origin", BRANCH)
        .expect("fetch");
    (checkout_dir, checkout, published)
}

#[test]
fn a_bare_repository_accepts_a_push_and_clones_back_what_was_pushed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let remote = dir.path().join("remote.git");
    ShellGit::new().init_bare(&remote).expect("init_bare");

    // The proof publishes into this directory and then clones from it, so "bare" is the
    // contract: no work tree to desynchronise, and a clone that sees the pushed branch.
    assert!(
        !remote.join("README.md").exists(),
        "a bare repository has no work tree"
    );
    assert!(
        run_git(&["rev-parse", "--is-bare-repository"], &remote) == "true",
        "the repository is bare"
    );

    let source_dir = tempfile::tempdir().expect("tempdir");
    let source = source_dir.path().join("consumer");
    std::fs::create_dir_all(&source).expect("mkdir");
    committed_consumer(&source);
    let git = ShellGit::new();
    git.push_refspec(
        &source,
        remote.to_str().expect("utf-8"),
        &format!("HEAD:{BRANCH}"),
    )
    .expect("a bare repository accepts a push");

    let out = dir.path().join("clone");
    git.clone_fresh(&remote, &out).expect("clone");
    assert_eq!(
        run_git(&["rev-list", "--count", &format!("origin/{BRANCH}")], &out),
        "1",
        "the clone carries the pushed revision"
    );
}

#[test]
fn a_work_tree_created_with_an_explicit_branch_names_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("consumer");
    std::fs::create_dir_all(&root).expect("mkdir");

    ShellGit::new()
        .init_repo_branch(&root, "main")
        .expect("init");

    // The proof never relies on the ambient `init.defaultBranch`: a runner and a developer's
    // machine may disagree, and the fixture must be the same tree on both.
    assert_eq!(
        run_git(&["symbolic-ref", "--short", "HEAD"], &root),
        "main",
        "the unborn HEAD names the requested branch"
    );

    // And the name is the one asked for, not merely one this machine's default happens to
    // differ from: a developer whose `init.defaultBranch` is `main` must see the same result.
    let distinctive = dir.path().join("named");
    std::fs::create_dir_all(&distinctive).expect("mkdir");
    ShellGit::new()
        .init_repo_branch(&distinctive, "consumer-fixture")
        .expect("init");
    assert_eq!(
        run_git(&["symbolic-ref", "--short", "HEAD"], &distinctive),
        "consumer-fixture",
        "no default branch produces this name"
    );
}

#[test]
fn a_detached_checkout_materialises_the_fetched_revision() {
    let (_dir, checkout, published) = fetched_checkout();

    // An empty bare remote clones to an unborn HEAD: the bytes only arrive with the fetch, and
    // the proof detaches onto them rather than tracking a branch the clone never had.
    assert!(
        !checkout.join("pixi.toml").exists(),
        "cloning an empty remote materialises nothing"
    );

    ShellGit::new()
        .checkout_detached(&checkout, "FETCH_HEAD")
        .expect("checkout");

    assert_eq!(
        run_git(&["rev-parse", "HEAD"], &checkout),
        published,
        "HEAD is the revision the fetch brought"
    );
    assert!(
        !try_git(&["symbolic-ref", "-q", "HEAD"], &checkout).0,
        "HEAD is detached, not on a branch"
    );
    assert_eq!(
        std::fs::read_to_string(checkout.join("pixi.toml")).expect("work tree"),
        "[workspace]\nname = \"consumer-proof\"\n",
        "the work tree carries the fetched revision's files"
    );
}

#[test]
fn a_detached_checkout_of_a_branch_name_leaves_head_off_that_branch() {
    let (_dir, checkout, _published) = fetched_checkout();
    // A branch the checkout can actually be on: this is the case `--detach` exists for. With
    // `FETCH_HEAD` git detaches on its own, so without a named branch the flag proves nothing.
    run_git(&["branch", "transport", "FETCH_HEAD"], &checkout);

    ShellGit::new()
        .checkout_detached(&checkout, "transport")
        .expect("checkout");

    assert!(
        !try_git(&["symbolic-ref", "-q", "HEAD"], &checkout).0,
        "HEAD is detached even though the revision names a branch"
    );
    assert_eq!(
        run_git(&["rev-parse", "--short", "HEAD"], &checkout),
        run_git(&["rev-parse", "--short", "transport"], &checkout),
        "the work tree is at the branch's revision"
    );
    assert_eq!(
        run_git(&["status", "--porcelain"], &checkout),
        "",
        "checking out detached left the branch itself where it was"
    );
}

#[test]
fn a_detached_checkout_of_an_unknown_revision_is_an_error_naming_the_command() {
    let (_dir, checkout, _published) = fetched_checkout();

    let error = ShellGit::new()
        .checkout_detached(&checkout, "no-such-revision")
        .expect_err("an unknown revision cannot be checked out");

    // The airlock operator reads this in a CI log with no shell to experiment in, so the
    // message carries the command and git's own complaint rather than a bare exit status.
    let text = error.to_string();
    assert!(
        text.contains("checkout") && text.contains("no-such-revision"),
        "{text}"
    );
}

#[test]
fn the_new_primitives_run_the_expected_argv() {
    // `PreviewRunner` records and executes nothing, which is what an argv assertion wants: the
    // three primitives are being asked what command they *are*, not to perform it.
    let dir = tempfile::tempdir().expect("tempdir");
    let runner = Arc::new(PreviewRunner::new());
    let git = ShellGit::with_runner(Box::new(runner.clone()));

    git.init_bare(&dir.path().join("remote.git"))
        .expect("init_bare");
    git.init_repo_branch(&dir.path().join("consumer"), "main")
        .expect("init");
    git.checkout_detached(&dir.path().join("checkout"), "FETCH_HEAD")
        .expect("checkout");

    // Strip `git` and the four identity words to see each subcommand and its arguments.
    let tail: Vec<String> = runner
        .rendered()
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
            // The bare repository is created *at* its path, not inside a cwd, so the path is
            // an argument where the other two primitives are `cwd`-scoped.
            format!("init -q --bare {}", dir.path().join("remote.git").display()),
            "init -q --initial-branch main".to_string(),
            "checkout -q --detach FETCH_HEAD".to_string(),
        ]
    );
}
