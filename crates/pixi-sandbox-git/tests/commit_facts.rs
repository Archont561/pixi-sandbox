//! What one commit says about how it reached a branch — its parents, its committer, its subject
//! and the paths it changed — answered by `git`, for the main-integration guard (TASK-111, ADR-29).
//!
//! `xtask check-main-push` decides whether a push to `main` is a sanctioned shape from exactly
//! these facts, so they are pinned here against a real `git` in a tempdir (the style of
//! `tests/worktree_state.rs`), including the depth-2 clone CI runs the guard in.

use pixi_sandbox_git::{CommitFacts, ShellGit};
use std::path::Path;
use std::process::Command as StdCommand;

const DEV: (&str, &str) = ("dev", "dev@example.invalid");
const GITHUB: (&str, &str) = ("GitHub", "noreply@github.com");

/// Run `git` in `cwd` as `committer`. The identity is one of the facts under test, so the fixture
/// sets it per command instead of reading (or needing) the host's git config.
fn git_as(committer: (&str, &str), args: &[&str], cwd: &Path) -> String {
    let out = StdCommand::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", DEV.0)
        .env("GIT_AUTHOR_EMAIL", DEV.1)
        .env("GIT_COMMITTER_NAME", committer.0)
        .env("GIT_COMMITTER_EMAIL", committer.1)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn git(args: &[&str], cwd: &Path) -> String {
    git_as(DEV, args, cwd)
}

/// Write `files` and commit them as `committer`; returns the new commit's full SHA.
fn commit_as(committer: (&str, &str), dir: &Path, files: &[(&str, &str)], message: &str) -> String {
    for (rel, body) in files {
        let full = dir.join(rel);
        std::fs::create_dir_all(full.parent().expect("parent")).expect("mkdir");
        std::fs::write(full, body).expect("write");
        git(&["add", "--", rel], dir);
    }
    git_as(committer, &["commit", "-q", "-m", message], dir);
    git(&["rev-parse", "HEAD"], dir)
}

/// A repository on `main` with one commit; returns its SHA.
fn seeded(dir: &Path) -> String {
    git(&["init", "-q", "-b", "main"], dir);
    commit_as(DEV, dir, &[("README.md", "seed\n")], "seed")
}

#[test]
fn a_commit_reports_its_parent_committer_and_subject() {
    let dir = tempfile::tempdir().expect("tempdir");
    let seed = seeded(dir.path());
    let squash = commit_as(
        GITHUB,
        dir.path(),
        &[("crates/lib.rs", "// one\n")],
        "fix(core): repair the thing (#9)\n\nThe body is not the subject.",
    );

    let facts = ShellGit::new()
        .commit_facts(dir.path(), &squash)
        .expect("git answers");

    assert_eq!(
        facts,
        CommitFacts {
            sha: squash,
            parents: vec![seed],
            committer_name: "GitHub".to_string(),
            committer_email: "noreply@github.com".to_string(),
            subject: "fix(core): repair the thing (#9)".to_string(),
        }
    );
}

/// `feature` adds `docs/a.md` while `main` adds `docs/b.md`; GitHub merges `feature` into
/// `main` with a merge commit. Returns `(main tip before the merge, feature tip, merge)`.
fn merged(dir: &Path) -> (String, String, String) {
    seeded(dir);
    git(&["checkout", "-q", "-b", "feature"], dir);
    let feature = commit_as(DEV, dir, &[("docs/a.md", "a\n")], "docs: a");
    git(&["checkout", "-q", "main"], dir);
    let main_tip = commit_as(DEV, dir, &[("docs/b.md", "b\n")], "docs: b");
    git_as(
        GITHUB,
        &[
            "merge",
            "-q",
            "--no-ff",
            "feature",
            "-m",
            "Merge pull request #1",
        ],
        dir,
    );
    (main_tip, feature, git(&["rev-parse", "HEAD"], dir))
}

#[test]
fn a_merge_lists_both_parents_first_parent_first() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (main_tip, feature, merge) = merged(dir.path());

    let facts = ShellGit::new()
        .commit_facts(dir.path(), &merge)
        .expect("git answers");

    assert_eq!(facts.parents, vec![main_tip, feature]);
}

#[test]
fn a_root_commit_has_no_parents() {
    let dir = tempfile::tempdir().expect("tempdir");
    let seed = seeded(dir.path());

    let facts = ShellGit::new()
        .commit_facts(dir.path(), &seed)
        .expect("git answers");

    assert!(facts.parents.is_empty(), "{facts:?}");
}

#[test]
fn changed_paths_name_both_sides_of_a_rename_and_survive_odd_names() {
    let dir = tempfile::tempdir().expect("tempdir");
    git(&["init", "-q", "-b", "main"], dir.path());
    commit_as(
        DEV,
        dir.path(),
        &[
            ("crates/old.rs", "// old\n"),
            ("pixi.toml", "[workspace]\n"),
            ("gone.txt", "x\n"),
        ],
        "seed",
    );
    git(&["rm", "-q", "gone.txt"], dir.path());
    std::fs::create_dir(dir.path().join("docs")).expect("mkdir");
    git(&["mv", "crates/old.rs", "docs/moved.rs"], dir.path());
    let tip = commit_as(
        DEV,
        dir.path(),
        &[
            ("pixi.toml", "[workspace]\nname = \"x\"\n"),
            ("notes/a b.md", "spaced\n"),
        ],
        "chore: shuffle",
    );

    let mut paths = ShellGit::new()
        .changed_paths(dir.path(), &tip)
        .expect("git answers");
    paths.sort();

    // A move out of `crates/` must still name `crates/old.rs`: a guard keyed on paths cannot
    // be dodged by renaming a file away from the paths it watches.
    assert_eq!(
        paths,
        [
            "crates/old.rs",
            "docs/moved.rs",
            "gone.txt",
            "notes/a b.md",
            "pixi.toml"
        ]
    );
}

#[test]
fn a_root_commit_changed_every_path_it_holds() {
    let dir = tempfile::tempdir().expect("tempdir");
    git(&["init", "-q", "-b", "main"], dir.path());
    let root = commit_as(
        DEV,
        dir.path(),
        &[("a.txt", "a\n"), ("crates/b.rs", "// b\n")],
        "root",
    );

    let paths = ShellGit::new()
        .changed_paths(dir.path(), &root)
        .expect("git answers");

    assert_eq!(paths, ["a.txt", "crates/b.rs"]);
}

#[test]
fn a_merge_changed_what_it_brought_into_its_first_parent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (_, _, merge) = merged(dir.path());

    let paths = ShellGit::new()
        .changed_paths(dir.path(), &merge)
        .expect("git answers");

    assert_eq!(paths, ["docs/a.md"]);
}

#[test]
fn rev_parse_resolves_names_and_passes_a_full_object_name_through() {
    let dir = tempfile::tempdir().expect("tempdir");
    let seed = seeded(dir.path());
    let shell = ShellGit::new();

    assert_eq!(shell.rev_parse(dir.path(), "main").expect("a branch"), seed);
    assert_eq!(shell.rev_parse(dir.path(), "HEAD").expect("HEAD"), seed);
    // A full object name is not looked up: the guard compares the `before` of a push that
    // rewrote history, whose commit the depth-2 checkout does not hold.
    let absent = "d".repeat(40);
    assert_eq!(
        shell
            .rev_parse(dir.path(), &absent)
            .expect("passed through"),
        absent
    );
    assert!(shell.rev_parse(dir.path(), "no-such-branch").is_err());
}

#[test]
fn the_facts_survive_the_depth_two_clone_ci_checks_out() {
    let origin = tempfile::tempdir().expect("tempdir");
    seeded(origin.path());
    let previous = commit_as(DEV, origin.path(), &[("a.txt", "a\n")], "second");
    let tip = commit_as(DEV, origin.path(), &[("crates/x.rs", "// x\n")], "third");
    let clone = tempfile::tempdir().expect("tempdir");
    let url = format!("file://{}", origin.path().display());
    git(&["clone", "-q", "--depth", "2", &url, "."], clone.path());
    let shell = ShellGit::new();

    let facts = shell
        .commit_facts(clone.path(), &tip)
        .expect("the tip is fetched");
    let paths = shell
        .changed_paths(clone.path(), &tip)
        .expect("so is its parent");

    assert_eq!(facts.parents, vec![previous]);
    assert_eq!(paths, ["crates/x.rs"]);
}
