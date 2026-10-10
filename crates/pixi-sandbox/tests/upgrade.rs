//! Tests for `pixi_sandbox::upgrade` — the consumer-upgrade lane the generated workflow
//! runs as a single `pixi-sandbox upgrade …` step (TASK-76).
//!
//! The decision flow — drift detection, regeneration, the version-derived commit, the
//! artifact fallback, and every delivery verdict — is asserted against a fake runner,
//! `FakeGit` and a fake pull-request source. No phase executes, no git remote is touched,
//! no socket is opened, and no test points at this checkout or a real HOME (D10).

use anyhow::Result;
use pixi_sandbox::pipeline::{PhaseCommand, PhaseRunner};
use pixi_sandbox::upgrade::{self, Outcome, PullRequest, PullRequestSource, Spec};
use pixi_sandbox_git::{FakeGit, Op};
use rstest::{fixture, rstest};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;

const VERSION: &str = "0.6.1";
const WORKFLOW: &str = ".github/workflows/publish-sandbox.yml";
const RELOCK: &str = ".github/workflows/relock.yml";
const SCRIPT: &str = "restore.sh";
const BRANCH_ARG: &str = "sandbox/developer-linux-64";
const EXPECTED_BRANCH: &str = "pixi-sandbox-upgrade/0.6.1";

/// A scripted child-process boundary for the `init` phases.
#[derive(Debug, Default)]
struct FakeRunner {
    script: VecDeque<i32>,
    seen: Vec<PhaseCommand>,
}

impl FakeRunner {
    fn script(mut self, codes: &[i32]) -> Self {
        self.script = codes.iter().copied().collect();
        self
    }
}

impl PhaseRunner for FakeRunner {
    fn run(&mut self, command: &PhaseCommand, _log: &mut (dyn Write + Send)) -> Result<i32> {
        self.seen.push(command.clone());
        Ok(self.script.pop_front().unwrap_or(0))
    }
}

/// One recorded pull-request creation: (repo, head, base, title, body, token).
type PullCall = (String, String, String, String, String, String);

/// A scripted pull-request boundary.
#[derive(Debug, Default)]
struct FakePulls {
    fail: bool,
    calls: RefCell<Vec<PullCall>>,
}

impl FakePulls {
    fn calls(&self) -> Vec<PullCall> {
        self.calls.borrow().clone()
    }
}

impl PullRequestSource for FakePulls {
    fn create(
        &self,
        repo: &str,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
        token: &str,
    ) -> Result<PullRequest> {
        self.calls.borrow_mut().push((
            repo.to_string(),
            head.to_string(),
            base.to_string(),
            title.to_string(),
            body.to_string(),
            token.to_string(),
        ));
        if self.fail {
            anyhow::bail!("422: pull request could not be created");
        }
        Ok(PullRequest {
            url: format!("https://example.invalid/{repo}/pull/13"),
            number: 13,
        })
    }
}

struct Ctx {
    _dir: tempfile::TempDir,
    root: PathBuf,
    spec: Spec,
}

#[fixture]
fn ctx() -> Ctx {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("repo");
    // The checked-out consumer tree: the three owned files exist, as init wrote them.
    for file in [WORKFLOW, RELOCK] {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, b"generated v1\n").expect("write");
    }
    std::fs::write(root.join(SCRIPT), b"generated v1\n").expect("write");
    let spec = Spec {
        exe: dir.path().join("bin/pixi-sandbox"),
        repo_root: root.clone(),
        workflow_path: PathBuf::from(WORKFLOW),
        relock_path: PathBuf::from(RELOCK),
        script_path: PathBuf::from(SCRIPT),
        relock_ci_workflow: "ci.yml".to_string(),
        config: Some(PathBuf::from("config/pixi-sandbox.toml")),
        branch: BRANCH_ARG.to_string(),
        version: VERSION.to_string(),
        artifact_dir: dir.path().join("artifacts"),
        repo: "consumer/project".to_string(),
        base: "main".to_string(),
        upgrade_token: Some("secret".to_string()),
        github_token: Some("secret".to_string()),
        step_summary: None,
        output: None,
    };
    Ctx {
        _dir: dir,
        root,
        spec,
    }
}

fn outcome_of(
    spec: &Spec,
    runner: &mut FakeRunner,
    git: &FakeGit,
    pulls: &FakePulls,
) -> Result<Outcome> {
    upgrade::run(spec, runner, git, pulls)
}

#[rstest]
fn no_drift_regenerates_and_delivers_nothing(ctx: Ctx) {
    let mut runner = FakeRunner::default().script(&[0]);
    let git = FakeGit::new();
    let pulls = FakePulls::default();

    let outcome = outcome_of(&ctx.spec, &mut runner, &git, &pulls).expect("the lane succeeds");

    assert_eq!(outcome, Outcome::NoDrift);
    // Only the check ran.
    assert_eq!(runner.seen.len(), 1);
    assert!(git.ops().is_empty(), "no git operation without drift");
    assert!(pulls.calls().is_empty());
    assert!(!ctx.spec.artifact_dir.exists(), "no patch without drift");
}

#[rstest]
fn the_check_forwards_the_exact_generation_arguments(ctx: Ctx) {
    let mut runner = FakeRunner::default().script(&[0]);
    let git = FakeGit::new();
    let pulls = FakePulls::default();

    outcome_of(&ctx.spec, &mut runner, &git, &pulls).expect("the lane succeeds");

    let check = &runner.seen[0];
    assert_eq!(check.phase, "init-check");
    assert_eq!(check.program, ctx.spec.exe.display().to_string());
    assert_eq!(
        check.args,
        vec![
            "init",
            "--check",
            "--github-workflow-path",
            WORKFLOW,
            "--relock-workflow-path",
            RELOCK,
            "--relock-ci-workflow",
            "ci.yml",
            "--script-path",
            SCRIPT,
            "--config",
            "config/pixi-sandbox.toml",
            "--branch",
            BRANCH_ARG,
        ]
    );
}

#[rstest]
fn drift_regenerates_then_commits_the_three_owned_files_onto_the_version_branch(ctx: Ctx) {
    let mut runner = FakeRunner::default().script(&[1, 0]);
    let git = FakeGit::new();
    let pulls = FakePulls::default();

    let outcome = outcome_of(&ctx.spec, &mut runner, &git, &pulls).expect("the lane succeeds");

    // Both init calls ran, check first.
    let phases: Vec<&str> = runner.seen.iter().map(|phase| phase.phase).collect();
    assert_eq!(phases, vec!["init-check", "init"]);
    assert!(runner.seen[1].args.starts_with(&["init".to_string()]));
    assert!(!runner.seen[1].args.iter().any(|arg| arg == "--check"));

    // The commit carries exactly the three owned files — the config is never staged.
    let ops = git.ops();
    assert_eq!(
        ops[0],
        Op::CommitFiles {
            branch: EXPECTED_BRANCH.to_string(),
            files: vec![WORKFLOW.to_string(), RELOCK.to_string(), SCRIPT.to_string()],
            message: format!("chore(pixi-sandbox): upgrade generated files to {VERSION}"),
        }
    );

    // The artifact fallback: the patch and the regenerated copies at their relative paths.
    let patch = ctx
        .spec
        .artifact_dir
        .join(format!("pixi-sandbox-upgrade-{VERSION}.patch"));
    assert!(patch.exists(), "the patch artifact is written");
    for file in [WORKFLOW, RELOCK, SCRIPT] {
        let copy = ctx.spec.artifact_dir.join(file);
        assert!(
            copy.exists(),
            "{} is copied into the artifact dir",
            copy.display()
        );
        assert_eq!(
            std::fs::read(&copy).expect("read the copy"),
            b"generated v1\n"
        );
    }

    assert!(matches!(outcome, Outcome::Delivered { .. }));
}

#[rstest]
fn a_regenerate_failure_is_a_hard_error(ctx: Ctx) {
    let mut runner = FakeRunner::default().script(&[1, 2]);
    let git = FakeGit::new();
    let pulls = FakePulls::default();

    let err = outcome_of(&ctx.spec, &mut runner, &git, &pulls).unwrap_err();

    assert!(
        format!("{err:?}").contains("init failed to regenerate"),
        "{err:?}"
    );
    assert!(
        git.ops().is_empty(),
        "a failed regeneration commits nothing"
    );
    assert!(pulls.calls().is_empty());
}

#[rstest]
fn no_staged_changes_is_a_notice_not_a_failure(ctx: Ctx) {
    let mut runner = FakeRunner::default().script(&[1, 0]);
    let git = FakeGit::new();
    git.script_commit(pixi_sandbox_git::FileCommitted {
        commit: String::new(),
        changed: false,
        patch: Vec::new(),
    });
    let pulls = FakePulls::default();

    let outcome = outcome_of(&ctx.spec, &mut runner, &git, &pulls).expect("the lane succeeds");

    assert_eq!(outcome, Outcome::NoDrift);
    assert!(
        !ctx.spec.artifact_dir.exists(),
        "no patch when nothing changed"
    );
    assert_eq!(git.pushes(), 0);
    assert!(pulls.calls().is_empty());
}

#[rstest]
fn delivery_without_the_secret_is_refused_with_a_handoff_before_any_push(ctx: Ctx) {
    let mut spec = ctx.spec.clone();
    spec.upgrade_token = None;
    spec.step_summary = Some(ctx.root.join("summary.md"));
    spec.output = Some(ctx.root.join("output.txt"));
    let mut runner = FakeRunner::default().script(&[1, 0]);
    let git = FakeGit::new();
    let pulls = FakePulls::default();

    let outcome = outcome_of(&spec, &mut runner, &git, &pulls).expect("a refusal is not a failure");

    let Outcome::Refused(refusal) = outcome else {
        panic!("expected a refusal, got {outcome:?}");
    };
    assert_eq!(
        refusal.reason,
        "PIXI_SANDBOX_UPGRADE_TOKEN is not configured"
    );
    assert_eq!(
        refusal.annotation,
        "::error::upgrade PR delivery unavailable: PIXI_SANDBOX_UPGRADE_TOKEN is not \
         configured; configure PIXI_SANDBOX_UPGRADE_TOKEN with Workflows: write or apply the \
         uploaded patch artifact"
    );
    assert_eq!(refusal.output_line, "delivery_refused=1");

    // The handoff summary: the version, the reason, the remedy, the artifact.
    let summary = &refusal.summary;
    assert!(
        summary.contains("### pixi-sandbox upgrade needs delivery credentials"),
        "{summary}"
    );
    assert!(summary.contains("\"0.6.1\""), "{summary}");
    assert!(
        summary.contains("PIXI_SANDBOX_UPGRADE_TOKEN is not configured"),
        "{summary}"
    );
    assert!(summary.contains("Workflows: write"), "{summary}");
    assert!(
        summary.contains("pixi-sandbox-upgrade-artifacts"),
        "{summary}"
    );
    assert!(summary.contains("git apply"), "{summary}");

    // The files were written where the runner reads them.
    let written = std::fs::read_to_string(ctx.root.join("summary.md")).expect("summary file");
    assert_eq!(written, refusal.summary);
    let output = std::fs::read_to_string(ctx.root.join("output.txt")).expect("output file");
    assert_eq!(output, "delivery_refused=1\n");

    // Nothing was pushed, no PR was attempted, but the patch is still there.
    assert_eq!(git.pushes(), 0, "a refusal happens before the push");
    assert!(pulls.calls().is_empty());
    assert!(
        ctx.spec
            .artifact_dir
            .join(format!("pixi-sandbox-upgrade-{VERSION}.patch"))
            .exists()
    );
}

#[rstest]
fn a_refused_push_is_a_handoff_not_a_failure(ctx: Ctx) {
    let mut runner = FakeRunner::default().script(&[1, 0]);
    let git = FakeGit::new();
    git.fail_next_push("the token was refused while pushing workflow files");
    let pulls = FakePulls::default();

    let outcome =
        outcome_of(&ctx.spec, &mut runner, &git, &pulls).expect("a refusal is not a failure");

    let Outcome::Refused(refusal) = outcome else {
        panic!("expected a refusal, got {outcome:?}");
    };
    assert_eq!(
        refusal.reason,
        "the configured token was refused while pushing workflow files; it needs Workflows: write"
    );
    assert!(pulls.calls().is_empty(), "no PR after a refused push");
    // The push was attempted, forced, onto the version-derived branch.
    assert_eq!(
        git.ops().last(),
        Some(&Op::Push {
            remote: "origin".to_string(),
            branch: EXPECTED_BRANCH.to_string(),
            forced: true,
        })
    );
}

#[rstest]
fn a_failed_pull_request_is_a_handoff_not_a_failure(ctx: Ctx) {
    let mut runner = FakeRunner::default().script(&[1, 0]);
    let git = FakeGit::new();
    let pulls = FakePulls {
        fail: true,
        ..FakePulls::default()
    };

    let outcome =
        outcome_of(&ctx.spec, &mut runner, &git, &pulls).expect("a refusal is not a failure");

    let Outcome::Refused(refusal) = outcome else {
        panic!("expected a refusal, got {outcome:?}");
    };
    assert_eq!(
        refusal.reason,
        "the branch was pushed but the pull request could not be opened"
    );
    assert_eq!(
        git.pushes(),
        1,
        "the branch was pushed before the PR failed"
    );
    assert_eq!(pulls.calls().len(), 1);
}

#[rstest]
fn a_delivered_upgrade_opens_the_pull_request_with_the_review_shape(ctx: Ctx) {
    let mut runner = FakeRunner::default().script(&[1, 0]);
    let git = FakeGit::new();
    let pulls = FakePulls::default();

    let outcome = outcome_of(&ctx.spec, &mut runner, &git, &pulls).expect("the lane succeeds");

    let Outcome::Delivered { url } = outcome else {
        panic!("expected delivery, got {outcome:?}");
    };
    assert_eq!(url, "https://example.invalid/consumer/project/pull/13");

    assert_eq!(pulls.calls().len(), 1);
    let (repo, head, base, title, body, token) = &pulls.calls()[0];
    assert_eq!(repo, "consumer/project");
    assert_eq!(head, EXPECTED_BRANCH);
    assert_eq!(base, "main");
    assert_eq!(
        title,
        "chore(pixi-sandbox): upgrade generated files to 0.6.1"
    );
    // The body names the explicit post-merge dispatch (task-44's lesson restated).
    assert!(
        body.contains("gh workflow run .github/workflows/publish-sandbox.yml --ref main"),
        "{body}"
    );
    assert_eq!(token, "secret");
}

#[rstest]
fn the_branch_name_is_derived_from_the_version() {
    assert_eq!(upgrade::branch_name("1.2.3"), "pixi-sandbox-upgrade/1.2.3");
}
