//! `upgrade` — the consumer-upgrade lane as one tested boundary (TASK-76).
//!
//! The generated workflow's upgrade job used to carry this sequence as four embedded Bash
//! blocks: check the owned files against the updated binary, regenerate them, commit the
//! result onto a version-derived branch with a patch artifact, and deliver it as a pull
//! request — or, when the workflow-capable credential is absent, leave a handoff summary
//! and the patch artifact behind instead of failing the scheduled lane. That is the
//! workflow's "how", so it lives here, driven by the updated binary itself: the workflow's
//! step is a single `pixi-sandbox upgrade …` invocation.
//!
//! The flow, in order (decision-4 / D16):
//!
//! 1. **Check** the owned files: spawn `<exe> init --check …` with the exact generation
//!    arguments. Any non-zero exit is drift — the shell's `if ! …; then drift=true`.
//! 2. **Regenerate** on drift: spawn `<exe> init …`. A failure here is a hard error, like
//!    the shell's `set -euo pipefail` step.
//! 3. **Commit** the three owned files onto `pixi-sandbox-upgrade/<version>` through
//!    `GitProtocol::commit_files` (all git access stays behind the trait, D9), and write
//!    the patch plus the regenerated copies into the artifact directory — the fallback a
//!    human applies when delivery is refused.
//! 4. **Deliver**: without the `PIXI_SANDBOX_UPGRADE_TOKEN` secret, or when the push or
//!    the pull request fails, write the handoff summary and `delivery_refused=1` and exit
//!    successfully — a scheduled lane must not turn red over missing credentials. Only a
//!    successful delivery reports the pull request's URL.

mod github;

pub use github::GitHubPullRequestSource;

use anyhow::{Context, Result, bail};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use pixi_sandbox_git::{FileCommit, GitProtocol};

use crate::pipeline::{PhaseCommand, PhaseRunner};

/// Where pull requests are created — the delivery half of the upgrade lane. A trait for
/// the same reason git goes through `GitProtocol` (D9): the decision flow is tested against
/// a fake, and the real HTTP lives in `github.rs`, alone, because it is the one piece no
/// offline test can exercise.
pub trait PullRequestSource {
    /// Open a pull request from `head` onto `base` in `repo` (`owner/name`); `token`
    /// authenticates the call.
    fn create(
        &self,
        repo: &str,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
        token: &str,
    ) -> Result<PullRequest>;
}

/// A created pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequest {
    /// The web URL (`html_url`) — what the step prints and the operator opens.
    pub url: String,
    pub number: u64,
}

/// Everything `upgrade` needs, with every ambient input resolved by the caller (D10): the
/// tests pass tempdirs, a fake runner, `FakeGit` and a fake pull-request source, so no code
/// path can reach a developer's real repository, token or HOME.
#[derive(Debug, Clone)]
pub struct Spec {
    /// The running (updated) binary — the one that spawns `init --check` / `init`.
    pub exe: PathBuf,
    /// The consumer's checked-out working tree; the git operations run here.
    pub repo_root: PathBuf,
    /// The publisher workflow, relative to `repo_root` — `init`'s `--github-workflow-path`.
    pub workflow_path: PathBuf,
    /// The relock workflow, relative to `repo_root` — `init`'s `--relock-workflow-path`.
    pub relock_path: PathBuf,
    /// The airlock launcher, relative to `repo_root` — `init`'s `--script-path`.
    pub script_path: PathBuf,
    /// `init`'s `--relock-ci-workflow`, forwarded verbatim.
    pub relock_ci_workflow: String,
    /// `init`'s `--config`, forwarded verbatim when present (reviewed data, D16).
    pub config: Option<PathBuf>,
    /// `init`'s `--branch`, forwarded verbatim.
    pub branch: String,
    /// The running binary's version — the upgrade target; it names the branch, the commit,
    /// the patch and the pull request.
    pub version: String,
    /// Where the patch and the regenerated copies are written (the artifact fallback).
    pub artifact_dir: PathBuf,
    /// The consumer repository as `owner/name`, for the pull request.
    pub repo: String,
    /// Base branch for the pull request.
    pub base: String,
    /// The optional workflow-capable secret. `None` means delivery must be refused with a
    /// handoff — never a bare failure of the scheduled lane.
    pub upgrade_token: Option<String>,
    /// The token API calls authenticate with (the secret, or `github.token`).
    pub github_token: Option<String>,
    /// `$GITHUB_STEP_SUMMARY`, when the runner provides it; the handoff summary is
    /// appended to it and printed to stdout otherwise.
    pub step_summary: Option<PathBuf>,
    /// `$GITHUB_OUTPUT`, when the runner provides it; the `delivery_refused=1` line is
    /// appended to it and printed to stdout otherwise.
    pub output: Option<PathBuf>,
}

/// How a run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// `init --check` found no drift (or left no staged changes): nothing was regenerated,
    /// nothing was delivered. A notice, not a failure.
    NoDrift,
    /// The upgrade pull request was opened.
    Delivered { url: String },
    /// The patch is prepared but delivery needs credentials or attention. The job still
    /// succeeds: the handoff summary names the reason and the artifact carries the patch.
    Refused(Refusal),
}

/// A refused delivery, with the operator-facing texts already produced (and already
/// appended to `$GITHUB_STEP_SUMMARY` / `$GITHUB_OUTPUT` when the runner provides them).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// Why delivery was refused, as the handoff summary states it.
    pub reason: String,
    /// The `::error::` workflow annotation.
    pub annotation: String,
    /// The handoff markdown.
    pub summary: String,
    /// The `GITHUB_OUTPUT` line (`delivery_refused=1`).
    pub output_line: String,
}

/// Run the upgrade lane. `Ok(Outcome)` covers every delivery verdict; `Err` is reserved
/// for hard failures (the regeneration failing, the commit failing, the artifact not
/// writable) — the cases where the shell's `set -e` also stopped the step.
pub fn run(
    spec: &Spec,
    runner: &mut dyn PhaseRunner,
    git: &dyn GitProtocol,
    pulls: &dyn PullRequestSource,
) -> Result<Outcome> {
    // 1. Check the owned files against the running (updated) binary.
    let check = PhaseCommand {
        phase: "init-check",
        program: spec.exe.display().to_string(),
        args: init_args(spec, true),
        env: Vec::new(),
    };
    let mut sink = io::sink();
    let code = runner
        .run(&check, &mut sink)
        .context("spawning init --check")?;
    if code == 0 {
        return Ok(Outcome::NoDrift);
    }

    // 2. Drift: regenerate. A failure here is a hard error.
    let regenerate = PhaseCommand {
        phase: "init",
        program: spec.exe.display().to_string(),
        args: init_args(spec, false),
        env: Vec::new(),
    };
    let code = runner
        .run(&regenerate, &mut sink)
        .context("spawning init")?;
    if code != 0 {
        bail!("init failed to regenerate the owned files (exit {code})");
    }

    // 3. Commit the three owned files onto the version-derived branch.
    let branch = branch_name(&spec.version);
    let message = format!(
        "chore(pixi-sandbox): upgrade generated files to {}",
        spec.version
    );
    let committed = git.commit_files(&FileCommit {
        work_tree: &spec.repo_root,
        branch: &branch,
        files: &[
            spec.workflow_path.clone(),
            spec.relock_path.clone(),
            spec.script_path.clone(),
        ],
        message: &message,
    })?;
    if !committed.changed {
        // The shell's notice, verbatim: init reported drift but left no staged changes.
        return Ok(Outcome::NoDrift);
    }
    write_artifact(spec, &committed)?;

    // 4. Deliver.
    deliver(spec, git, pulls, &branch)
}

/// The `init` invocation's arguments — the exact generation arguments, forwarded verbatim
/// to both the check and the regeneration (D16: never defaults that could diverge).
fn init_args(spec: &Spec, check: bool) -> Vec<String> {
    let mut args = vec!["init".to_string()];
    if check {
        args.push("--check".to_string());
    }
    args.push("--github-workflow-path".to_string());
    args.push(spec.workflow_path.display().to_string());
    args.push("--relock-workflow-path".to_string());
    args.push(spec.relock_path.display().to_string());
    args.push("--relock-ci-workflow".to_string());
    args.push(spec.relock_ci_workflow.clone());
    args.push("--script-path".to_string());
    args.push(spec.script_path.display().to_string());
    if let Some(config) = &spec.config {
        args.push("--config".to_string());
        args.push(config.display().to_string());
    }
    args.push("--branch".to_string());
    args.push(spec.branch.clone());
    args
}

/// The branch an upgrade proposes onto: a name derived from the version, so a re-proposal
/// reuses it (and the push force-replaces).
#[must_use]
pub fn branch_name(version: &str) -> String {
    format!("pixi-sandbox-upgrade/{version}")
}

/// The artifact fallback: the binary patch and the regenerated copies at their
/// repository-relative paths — `git diff HEAD^ HEAD --binary` plus `cp --parents`.
fn write_artifact(spec: &Spec, committed: &pixi_sandbox_git::FileCommitted) -> Result<()> {
    fs::create_dir_all(&spec.artifact_dir).with_context(|| {
        format!(
            "creating artifact directory {}",
            spec.artifact_dir.display()
        )
    })?;
    let patch = spec
        .artifact_dir
        .join(format!("pixi-sandbox-upgrade-{}.patch", spec.version));
    fs::write(&patch, &committed.patch)
        .with_context(|| format!("writing patch artifact {}", patch.display()))?;
    for file in [&spec.workflow_path, &spec.relock_path, &spec.script_path] {
        let source = spec.repo_root.join(file);
        let target = spec.artifact_dir.join(file);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating artifact directory {}", parent.display()))?;
        }
        fs::copy(&source, &target)
            .with_context(|| format!("copying {} into the artifact directory", source.display()))?;
    }
    Ok(())
}

/// Delivery: push the branch, then open the pull request. A missing secret, a refused
/// push and a failed pull request are all refusals with a handoff — the scheduled lane
/// must not turn red over credentials it was not given.
fn deliver(
    spec: &Spec,
    git: &dyn GitProtocol,
    pulls: &dyn PullRequestSource,
    branch: &str,
) -> Result<Outcome> {
    if spec.upgrade_token.is_none() {
        return Ok(refuse(spec, "PIXI_SANDBOX_UPGRADE_TOKEN is not configured"));
    }
    if git
        .push_branch(&spec.repo_root, "origin", branch, true)
        .is_err()
    {
        return Ok(refuse(
            spec,
            "the configured token was refused while pushing workflow files; it needs Workflows: write",
        ));
    }

    let title = format!(
        "chore(pixi-sandbox): upgrade generated files to {}",
        spec.version
    );
    let body = format!(
        "Regenerated by the scheduled pixi-sandbox upgrade job. A `github.token` merge starts \
         no `on: push` workflow, so after merging run: gh workflow run {} --ref {}",
        spec.workflow_path.display(),
        spec.base
    );
    let token = spec.github_token.as_deref().unwrap_or_default();
    match pulls.create(&spec.repo, branch, &spec.base, &title, &body, token) {
        Ok(pull) => Ok(Outcome::Delivered { url: pull.url }),
        Err(_) => Ok(refuse(
            spec,
            "the branch was pushed but the pull request could not be opened",
        )),
    }
}

/// A refused delivery: the handoff summary, the annotation and the output line, with the
/// file writes already done when the runner provides the paths.
fn refuse(spec: &Spec, reason: &str) -> Outcome {
    let summary = format!(
        "### pixi-sandbox upgrade needs delivery credentials\n\n\
         The generated files for version \"{version}\" are ready, but the upgrade PR was not \
         opened.\n\
         Reason: {reason}\n\n\
         Configure the consumer secret \"PIXI_SANDBOX_UPGRADE_TOKEN\" with Workflows: write \
         (PAT or GitHub App), then rerun this upgrade.\n\
         The patch is attached as the \"pixi-sandbox-upgrade-artifacts\" artifact and can be \
         applied with git apply.\n",
        version = spec.version,
    );
    let annotation = format!(
        "::error::upgrade PR delivery unavailable: {reason}; configure \
         PIXI_SANDBOX_UPGRADE_TOKEN with Workflows: write or apply the uploaded patch artifact"
    );
    let output_line = "delivery_refused=1".to_string();

    if let Some(path) = &spec.step_summary {
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = std::io::Write::write_all(&mut file, summary.as_bytes());
        }
    }
    if let Some(path) = &spec.output {
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{output_line}");
        }
    }

    Outcome::Refused(Refusal {
        reason: reason.to_string(),
        annotation,
        summary,
        output_line,
    })
}
