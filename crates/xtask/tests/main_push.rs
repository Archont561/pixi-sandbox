//! The main-integration guard (TASK-111, decision ADR-29): `judge_push` decides whether one push
//! to `main` reached it in a sanctioned shape, from the commits git holds. Every shape is a
//! throwaway repository here, committed the way GitHub, the release bot or a maintainer each
//! commits it — the committer identity and the parent list are the evidence the guard reads.

use pixi_sandbox_git::{BOT_EMAIL, BOT_NAME};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use xtask::main_push::{Sanctioned, Verdict, Violation, judge_push};

mod support;
use support::run_git;

const DEV: (&str, &str) = (
    "Archont561",
    "142521648+Archont561@users.noreply.github.com",
);
const GITHUB: (&str, &str) = ("GitHub", "noreply@github.com");
const BOT: (&str, &str) = (BOT_NAME, BOT_EMAIL);

/// The airlock trigger as this repository writes it. The guard reads the `pull_request.paths`
/// filter from the checkout, so every fixture repository carries a real one.
const AIRLOCK: &str = r#"name: airlock
on:
  workflow_dispatch:
  schedule:
    - cron: "17 5 * * 1"
  pull_request:
    paths:
      - "crates/**"
      - "pixi.toml"
      - ".github/workflows/airlock.yml"

permissions:
  contents: read
"#;

/// Run `git` as `who`: the committer is the evidence the guard reads, so each commit names its
/// own, overriding the support helper's default identity (git keeps the last `-c`).
fn git_as(who: (&str, &str), dir: &Path, args: &[&str]) -> String {
    let name = format!("user.name={}", who.0);
    let email = format!("user.email={}", who.1);
    let mut full = vec!["-c", name.as_str(), "-c", email.as_str()];
    full.extend_from_slice(args);
    run_git(dir, &full)
}

/// Write `files` and commit them as `who`; returns the new commit's full SHA.
fn commit_as(who: (&str, &str), dir: &Path, files: &[(&str, &str)], message: &str) -> String {
    for (rel, body) in files {
        let full = dir.join(rel);
        std::fs::create_dir_all(full.parent().expect("parent")).expect("mkdir");
        std::fs::write(full, body).expect("write");
        run_git(dir, &["add", "--", rel]);
    }
    git_as(who, dir, &["commit", "-q", "-m", message]);
    run_git(dir, &["rev-parse", "HEAD"])
}

/// A `main` whose tip carries `airlock` as the airlock workflow; returns the repository and
/// that tip — the `before` of the push each test then makes.
fn main_with(airlock: &str) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    run_git(dir.path(), &["init", "-q", "-b", "main"]);
    let tip = commit_as(
        DEV,
        dir.path(),
        &[
            (".github/workflows/airlock.yml", airlock),
            ("README.md", "seed\n"),
        ],
        "chore: seed",
    );
    (dir, tip)
}

fn main_repo() -> (tempfile::TempDir, String) {
    main_with(AIRLOCK)
}

#[test]
fn one_direct_commit_away_from_the_airlock_paths_passes() {
    let (dir, before) = main_repo();
    let after = commit_as(
        DEV,
        dir.path(),
        &[("docs/guide.md", "words\n")],
        "docs: guide",
    );

    let verdict = judge_push(dir.path(), &before, &after).expect("judged");

    assert_eq!(verdict, Verdict::Pass(Sanctioned::DirectCommit));
}

#[test]
fn a_direct_push_of_several_commits_fails_and_names_the_remedy() {
    let (dir, before) = main_repo();
    commit_as(DEV, dir.path(), &[("docs/a.md", "a\n")], "docs: a");
    let after = commit_as(DEV, dir.path(), &[("docs/b.md", "b\n")], "docs: b");

    let verdict = judge_push(dir.path(), &before, &after).expect("judged");

    assert_eq!(
        verdict,
        Verdict::Fail(Violation::NotOneCommit {
            before: before.clone(),
            after: after.clone()
        })
    );
    let message = verdict.to_string();
    assert!(message.contains("pull request"), "{message}");
    assert!(message.contains("squash"), "{message}");
}

#[test]
fn a_push_that_rewrote_main_fails() {
    let (dir, _) = main_repo();
    let published = commit_as(DEV, dir.path(), &[("docs/a.md", "a\n")], "docs: a");
    run_git(dir.path(), &["reset", "-q", "--hard", "HEAD~1"]);
    let rewritten = commit_as(DEV, dir.path(), &[("docs/a.md", "a2\n")], "docs: a, again");

    let verdict = judge_push(dir.path(), &published, &rewritten).expect("judged");

    assert_eq!(
        verdict,
        Verdict::Fail(Violation::NotOneCommit {
            before: published,
            after: rewritten
        })
    );
}

#[test]
fn a_pull_request_merge_commit_passes_whatever_it_carries() {
    let (dir, before) = main_repo();
    run_git(dir.path(), &["checkout", "-q", "-b", "feature"]);
    commit_as(DEV, dir.path(), &[("crates/a.rs", "// a\n")], "feat: a");
    commit_as(
        DEV,
        dir.path(),
        &[("pixi.toml", "[workspace]\n")],
        "build: b",
    );
    run_git(dir.path(), &["checkout", "-q", "main"]);
    git_as(
        GITHUB,
        dir.path(),
        &[
            "merge",
            "-q",
            "--no-ff",
            "feature",
            "-m",
            "Merge pull request #7 from o/feature",
        ],
    );
    let after = run_git(dir.path(), &["rev-parse", "HEAD"]);

    let verdict = judge_push(dir.path(), &before, &after).expect("judged");

    assert_eq!(verdict, Verdict::Pass(Sanctioned::PullRequestMerge));
}

#[test]
fn a_squash_merged_pull_request_passes_even_on_airlock_paths() {
    let (dir, before) = main_repo();
    let after = commit_as(
        GITHUB,
        dir.path(),
        &[("crates/a.rs", "// a\n")],
        "feat(core): a (#8)",
    );

    let verdict = judge_push(dir.path(), &before, &after).expect("judged");

    assert_eq!(verdict, Verdict::Pass(Sanctioned::PullRequestMerge));
}

#[test]
fn a_merge_made_in_a_clone_fails() {
    let (dir, before) = main_repo();
    run_git(dir.path(), &["checkout", "-q", "-b", "feature"]);
    commit_as(DEV, dir.path(), &[("docs/a.md", "a\n")], "docs: a");
    run_git(dir.path(), &["checkout", "-q", "main"]);
    run_git(
        dir.path(),
        &[
            "merge",
            "-q",
            "--no-ff",
            "feature",
            "-m",
            "Merge branch 'feature'",
        ],
    );
    let after = run_git(dir.path(), &["rev-parse", "HEAD"]);

    let verdict = judge_push(dir.path(), &before, &after).expect("judged");

    assert_eq!(
        verdict,
        Verdict::Fail(Violation::LocalMerge {
            merge: after.clone()
        })
    );
    let message = verdict.to_string();
    assert!(message.contains("pull request"), "{message}");
}

#[test]
fn a_direct_commit_on_airlock_paths_fails_naming_them() {
    let (dir, before) = main_repo();
    let after = commit_as(
        DEV,
        dir.path(),
        &[
            ("crates/a.rs", "// a\n"),
            ("docs/a.md", "a\n"),
            ("pixi.toml", "[workspace]\n"),
        ],
        "feat: a, by hand",
    );

    let verdict = judge_push(dir.path(), &before, &after).expect("judged");

    assert_eq!(
        verdict,
        Verdict::Fail(Violation::UnprovenAirlockPaths {
            commit: after.clone(),
            paths: vec!["crates/a.rs".to_string(), "pixi.toml".to_string()],
        })
    );
    let message = verdict.to_string();
    assert!(message.contains("crates/a.rs, pixi.toml"), "{message}");
    assert!(message.contains("airlock"), "{message}");
    assert!(message.contains("pull request"), "{message}");
}

#[test]
fn a_sibling_of_a_watched_directory_is_not_watched() {
    let (dir, before) = main_repo();
    let after = commit_as(
        DEV,
        dir.path(),
        &[("crates-notes.md", "n\n"), ("pixi.toml.bak", "b\n")],
        "docs: notes",
    );

    let verdict = judge_push(dir.path(), &before, &after).expect("judged");

    assert_eq!(verdict, Verdict::Pass(Sanctioned::DirectCommit));
}

#[test]
fn the_release_commit_passes_although_it_touches_airlock_paths() {
    let (dir, before) = main_repo();
    let after = commit_as(
        BOT,
        dir.path(),
        &[
            ("pixi.toml", "version = \"1.2.3\"\n"),
            ("crates/x/Cargo.toml", "v\n"),
        ],
        "chore(release): v1.2.3",
    );

    let verdict = judge_push(dir.path(), &before, &after).expect("judged");

    assert_eq!(verdict, Verdict::Pass(Sanctioned::ReleaseCommit));
}

#[test]
fn a_bot_commit_that_is_not_a_release_still_needs_a_pull_request() {
    let (dir, _) = main_repo();
    for subject in [
        "chore: tidy",
        "chore(release): next",
        "chore(release): v1.2",
    ] {
        let before = run_git(dir.path(), &["rev-parse", "HEAD"]);
        let after = commit_as(BOT, dir.path(), &[("crates/a.rs", subject)], subject);

        let verdict = judge_push(dir.path(), &before, &after).expect("judged");

        assert!(
            matches!(
                verdict,
                Verdict::Fail(Violation::UnprovenAirlockPaths { .. })
            ),
            "{subject}: {verdict:?}"
        );
    }
}

#[test]
fn creating_the_branch_passes_with_nothing_to_compare() {
    let (dir, after) = main_repo();

    let verdict = judge_push(dir.path(), &"0".repeat(40), &after).expect("judged");

    assert_eq!(verdict, Verdict::Pass(Sanctioned::BranchCreated));
}

#[test]
fn a_push_that_moved_nothing_passes() {
    let (dir, _) = main_repo();

    // The local default: `before` is the remote branch, `after` is HEAD, and they agree.
    let verdict = judge_push(dir.path(), "main", "HEAD").expect("judged");

    assert_eq!(verdict, Verdict::Pass(Sanctioned::NothingPushed));
}

#[test]
fn an_airlock_filter_the_guard_cannot_read_is_an_error_not_a_pass() {
    let cases = [
        (
            "a glob beyond `dir/**`",
            AIRLOCK.replace("\"crates/**\"", "\"crates/*.rs\""),
            "crates/*.rs",
        ),
        (
            "no paths filter",
            AIRLOCK.replace(
                "    paths:\n      - \"crates/**\"\n      - \"pixi.toml\"\n      - \".github/workflows/airlock.yml\"\n",
                "",
            ),
            "paths",
        ),
        (
            "a flow-style list",
            AIRLOCK.replace(
                "    paths:\n      - \"crates/**\"\n      - \"pixi.toml\"\n      - \".github/workflows/airlock.yml\"\n",
                "    paths: [\"crates/**\"]\n",
            ),
            "paths",
        ),
        (
            "an ignore list",
            AIRLOCK.replace("    paths:\n", "    paths-ignore:\n      - \"docs/**\"\n    paths:\n"),
            "paths-ignore",
        ),
        ("no pull_request trigger", "name: airlock\non:\n  workflow_dispatch:\n".to_string(), "pull_request"),
        (
            "a quoted item with a trailing comment",
            AIRLOCK.replace("\"pixi.toml\"", "\"pixi.toml\" # the manifest"),
            "pixi.toml",
        ),
        (
            "a bare item with a trailing comment",
            AIRLOCK.replace("\"pixi.toml\"", "pixi.toml # the manifest"),
            "pixi.toml",
        ),
        (
            "an empty paths list",
            AIRLOCK.replace(
                "      - \"crates/**\"\n      - \"pixi.toml\"\n      - \".github/workflows/airlock.yml\"\n",
                "",
            ),
            "paths",
        ),
        (
            "a mapping under paths",
            AIRLOCK.replace("      - \"pixi.toml\"\n", "      manifest: pixi.toml\n"),
            "manifest: pixi.toml",
        ),
    ];
    for (case, airlock, named) in cases {
        let (dir, before) = main_with(&airlock);
        let after = commit_as(DEV, dir.path(), &[("docs/a.md", "a\n")], "docs: a");

        let error = judge_push(dir.path(), &before, &after).expect_err(case);

        let message = format!("{error:#}");
        assert!(message.contains(named), "{case}: {message}");
        assert!(message.contains("airlock.yml"), "{case}: {message}");
    }
}

/// Run the command as `ci` runs it (`pixi run xtask check-main-push -- <before> <after>`), with
/// the run summary pointed at `summary` so the test never writes into a real job's summary.
fn check_main_push(dir: &Path, args: &[&str], summary: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("--root")
        .arg(dir)
        .arg("check-main-push")
        .args(args)
        .env("GITHUB_STEP_SUMMARY", summary)
        .output()
        .expect("run xtask")
}

#[test]
fn the_command_fails_a_violation_with_an_annotated_remedy_and_a_summary() {
    let (dir, before) = main_repo();
    commit_as(DEV, dir.path(), &[("docs/a.md", "a\n")], "docs: a");
    let after = commit_as(DEV, dir.path(), &[("docs/b.md", "b\n")], "docs: b");
    let scratch = tempfile::tempdir().expect("tempdir");
    let summary = scratch.path().join("summary.md");

    let output = check_main_push(dir.path(), &[&before, &after], &summary);

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("::error::main integration: "),
        "{stderr}"
    );
    assert!(stderr.contains("squash"), "{stderr}");
    let summary = std::fs::read_to_string(&summary).expect("summary written on failure too");
    assert!(summary.contains("## Main integration"), "{summary}");
}

#[test]
fn the_command_judges_head_against_origin_main_by_default() {
    let (dir, _) = main_repo();
    let remote = tempfile::tempdir().expect("tempdir");
    run_git(remote.path(), &["init", "-q", "--bare"]);
    let url = remote.path().to_str().expect("utf-8 path");
    run_git(dir.path(), &["remote", "add", "origin", url]);
    run_git(dir.path(), &["push", "-q", "origin", "main"]);
    let scratch = tempfile::tempdir().expect("tempdir");

    let output = check_main_push(dir.path(), &[], &scratch.path().join("summary.md"));

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("the push moved nothing"), "{stdout}");
}

fn check_main_push_pre_push(dir: &Path, args: &[&str], input: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("--root")
        .arg(dir)
        .arg("check-main-push")
        .arg("--pre-push")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn xtask");
    child
        .stdin
        .take()
        .expect("stdin piped")
        .write_all(input.as_bytes())
        .expect("write pre-push refs");
    child.wait_with_output().expect("wait for xtask")
}

#[test]
fn a_main_pre_push_violation_warns_without_blocking_the_push() {
    let (dir, before) = main_repo();
    run_git(
        dir.path(),
        &["update-ref", "refs/remotes/origin/main", &before],
    );
    commit_as(DEV, dir.path(), &[("docs/a.md", "a\n")], "docs: a");
    let after = commit_as(DEV, dir.path(), &[("docs/b.md", "b\n")], "docs: b");
    let input = format!("refs/heads/main {after} refs/heads/main {before}\n");

    let output = check_main_push_pre_push(dir.path(), &["origin/main", "HEAD"], &input);

    assert!(output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("warning: main push preflight"), "{stderr}");
    assert!(stderr.contains("squash"), "{stderr}");
    assert!(!stderr.contains("::error::"), "{stderr}");
}

#[test]
fn a_pre_push_to_a_non_main_ref_skips_the_main_diagnostic() {
    let (dir, _) = main_repo();
    let local_oid = run_git(dir.path(), &["rev-parse", "HEAD"]);
    let input = format!(
        "refs/heads/main {local_oid} refs/heads/feature 0000000000000000000000000000000000000000\n"
    );

    let output = check_main_push_pre_push(dir.path(), &["origin/main", "HEAD"], &input);

    assert!(output.status.success(), "{output:?}");
    assert!(
        output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn a_main_pre_push_without_a_remote_tracking_tip_warns_but_does_not_block() {
    let (dir, _) = main_repo();
    let local_oid = run_git(dir.path(), &["rev-parse", "HEAD"]);
    let input = format!("refs/heads/main {local_oid} refs/heads/main {local_oid}\n");

    let output = check_main_push_pre_push(dir.path(), &["origin/main", "HEAD"], &input);

    assert!(output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("warning: main push preflight could not judge"),
        "{stderr}"
    );
    assert!(stderr.contains("origin/main"), "{stderr}");
}
