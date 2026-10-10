use pixi_sandbox::generated::{RelockWorkflowOptions, render_relock_workflow};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn render(cargo: bool) -> String {
    render_relock_workflow(RelockWorkflowOptions {
        cli_version: "9.8.7",
        pixi_version: "0.81.0",
        cargo,
        relock_workflow: "relock.yml",
        ci_workflow: "ci.yml",
        publisher_workflow: "publish-sandbox.yml",
    })
}

fn directives(workflow: &str) -> String {
    workflow
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn vendoring_projects_guard_both_lockfiles_before_the_repair_job() {
    let workflow = render(true);
    let guard = workflow
        .split_once("  relock:")
        .expect("rendered workflow has a relock job")
        .0;

    assert!(guard.contains("- run: pixi lock --check"), "{guard}");
    assert!(guard.contains("- run: cargo fetch --locked"), "{guard}");
    assert_eq!(
        directives(&workflow)
            .matches("cargo fetch --locked")
            .count(),
        2,
        "the stale-head guard and repaired-head validation are both locked; the repair itself still refreshes Cargo.lock\n{workflow}"
    );
    assert!(
        workflow.contains("- run: cargo fetch\n"),
        "the repair job must still refresh Cargo.lock\n{workflow}"
    );
}

#[test]
fn conda_only_projects_have_no_cargo_guard_or_repair_step() {
    let workflow = directives(&render(false));
    assert!(!workflow.contains("cargo fetch"), "{workflow}");
    assert!(!workflow.contains("Cargo.lock"), "{workflow}");
}

/// The rendered `actions/github-script` pin, split into its reference and its trailing label.
///
/// The pin must stay immutable, but the SHA itself legitimately moves: a Dependabot bump is
/// reviewed in `generated/relock_workflow.rs` and re-rendered, and asserting one literal SHA here
/// made every such PR red for a reason no test could fix. Assert the *shape* instead — the
/// property `check-repository` check 4 already enforces over the committed render — so a reviewed
/// bump needs no edit to this file and a mutable tag still cannot slip through.
fn github_script_pin(workflow: &str) -> (&str, &str) {
    let line = workflow
        .lines()
        .find(|line| {
            line.trim_start()
                .starts_with("uses: actions/github-script@")
        })
        .unwrap_or_else(|| panic!("the Checks-API client is gone from the render:\n{workflow}"));
    line.trim_start()
        .trim_start_matches("uses: actions/github-script@")
        .split_once(char::is_whitespace)
        .unwrap_or_else(|| panic!("the pin carries no trailing release label: {line}"))
}

/// A repaired lock commit is a new PR head. The original guard belongs to the stale head and
/// must not be the last verdict a reviewer sees: start one named Check Run directly on the bot
/// commit before launching its downstream validation work.
#[test]
fn a_repaired_commit_starts_an_authoritative_pr_visible_check_run() {
    let workflow = render(true);

    assert!(
        workflow.contains("checks: write"),
        "the repair job needs only the Checks API scope to attach its verdict: {workflow}"
    );
    assert!(
        workflow.contains("name: Start repaired-head validation"),
        "{workflow}"
    );
    let (reference, label) = github_script_pin(&workflow);
    assert_eq!(
        reference.len(),
        40,
        "the static API client must be pinned to a commit, not a tag: {reference}"
    );
    assert!(
        reference
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
        "a pin that is not 40 lowercase hex characters is not a commit SHA: {reference}"
    );
    assert!(
        label.starts_with("# v"),
        "an unlabelled SHA leaves the next reader unable to tell what it is: {label}"
    );
    assert!(
        workflow.contains("head_sha: process.env.REPAIRED_SHA"),
        "the Check Run must target the bot-created commit, not the stale event SHA: {workflow}"
    );
    assert!(
        workflow.contains("REPAIRED_SHA: ${{ steps.commit.outputs.commit_hash }}"),
        "git-auto-commit's actual pushed SHA is the only safe check target: {workflow}"
    );
    assert!(
        workflow.contains("name: 'pixi-sandbox relock validation'"),
        "reviewers need one stable, recognisable verdict: {workflow}"
    );
}

/// The check must stay in progress until both detached validation workflows have completed,
/// then record the combined result on the repaired SHA. The bot dispatches one input-marked
/// relock validation run; it validates its own guard first, then observes CI and publishing
/// without checking out or executing any contributor-controlled code.
#[test]
fn validation_observer_aggregates_only_the_repaired_head_ci_and_publish_verdicts() {
    let workflow = render(true);

    assert!(
        workflow.contains("inputs:\n      repaired_sha:"),
        "the normal manual dispatch remains available, while only an explicit repaired SHA starts the verdict: {workflow}"
    );
    assert!(
        workflow.contains("  verdict:\n    name: relock verdict"),
        "{workflow}"
    );
    assert!(
        workflow.contains("inputs.repaired_sha != ''"),
        "only the bot's repaired-head validation dispatch may affect the check: {workflow}"
    );
    assert!(
        workflow.contains("    permissions:\n      actions: read\n      checks: write"),
        "the observer needs no contents write or checkout: {workflow}"
    );
    assert!(
        workflow.contains("github.rest.actions.listWorkflowRuns"),
        "the final verdict must inspect both dispatched workflow results: {workflow}"
    );
    assert!(
        workflow.contains("run.status !== 'completed'"),
        "a pending CI or publisher run must keep the check in progress: {workflow}"
    );
    assert!(
        workflow.contains("run.conclusion !== 'success'"),
        "a completed non-successful validation must fail the authoritative check: {workflow}"
    );
    assert!(
        workflow.contains("'Repaired-head validation passed'"),
        "the check turns green only after both required validations succeed: {workflow}"
    );
    assert!(
        !directives(&workflow).contains("gh workflow run relock.yml"),
        "the observer must not recreate the relock-dispatch loop: {workflow}"
    );
}

/// A fork cannot receive a write-capable token, and every failed local, dispatch, guard, or
/// downstream validation must complete the same repaired-head Check Run as a failure instead of
/// leaving an ambiguous green or permanently pending result.
#[test]
fn fork_and_failed_validation_paths_are_explicit_and_fail_closed() {
    let workflow = render(true);

    assert!(
        workflow.contains("if: ${{ github.event.pull_request.head.repo.fork }}"),
        "{workflow}"
    );
    assert!(
        workflow.contains("::error::relock cannot push to a fork's branch"),
        "{workflow}"
    );
    assert!(
        workflow.matches("'failure'").count() >= 5,
        "local guard, dispatch, and observer failures must all conclude the Check Run: {workflow}"
    );
    assert!(
        workflow.contains("Repaired lock guard failed"),
        "the observer must fail the Check Run when its own repaired-head guard fails: {workflow}"
    );
    assert!(
        workflow.contains("Repaired-head validation timed out"),
        "a missing downstream result must fail closed instead of leaving an in-progress check forever: {workflow}"
    );
}

#[test]
fn rendered_cargo_guard_accepts_a_clean_lock_and_rejects_a_stale_one_without_writing() {
    let scratch = tempdir().expect("temporary consumer");
    let project = scratch.path().join("consumer");
    let dependency = scratch.path().join("dependency");
    let home = scratch.path().join("home");
    let cargo_home = scratch.path().join("cargo-home");
    fs::create_dir_all(project.join("src")).expect("consumer src");
    fs::create_dir_all(dependency.join("src")).expect("dependency src");
    fs::create_dir_all(&home).expect("isolated HOME");
    fs::create_dir_all(&cargo_home).expect("isolated CARGO_HOME");
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("consumer manifest");
    fs::write(project.join("src/lib.rs"), "pub fn consumer() {}\n").expect("consumer source");
    fs::write(
        dependency.join("Cargo.toml"),
        "[package]\nname = \"dependency\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("dependency manifest");
    fs::write(dependency.join("src/lib.rs"), "pub fn dependency() {}\n")
        .expect("dependency source");

    let cargo = |args: &[&str]| {
        Command::new("cargo")
            .args(args)
            .current_dir(&project)
            .env("HOME", &home)
            .env("CARGO_HOME", &cargo_home)
            .env("CARGO_NET_OFFLINE", "true")
            .output()
            .expect("run Cargo from the restored Pixi environment")
    };
    let generated = cargo(&["generate-lockfile", "--offline"]);
    assert!(
        generated.status.success(),
        "generate lock: {}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let clean_lock = fs::read(project.join("Cargo.lock")).expect("clean lock");

    let guard = cargo(&["fetch", "--locked"]);
    assert!(
        guard.status.success(),
        "clean guard: {}",
        String::from_utf8_lossy(&guard.stderr)
    );
    assert_eq!(
        fs::read(project.join("Cargo.lock")).expect("lock after clean guard"),
        clean_lock,
        "the rendered guard must not rewrite a clean lock"
    );

    fs::write(
        project.join("Cargo.toml"),
        format!(
            "[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\ndependency = {{ path = {dependency:?} }}\n"
        ),
    )
    .expect("stale consumer manifest");
    let stale = cargo(&["fetch", "--locked"]);
    assert!(!stale.status.success(), "a stale Cargo.lock must fail");
    let stderr = String::from_utf8_lossy(&stale.stderr);
    assert!(
        stderr.contains("lock file") && stderr.contains("--locked"),
        "{stderr}"
    );
    assert_eq!(
        fs::read(project.join("Cargo.lock")).expect("lock after stale guard"),
        clean_lock,
        "--locked must leave the stale lock untouched"
    );
}
