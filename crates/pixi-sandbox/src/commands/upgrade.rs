//! `upgrade` — the CLI half: resolve the ambient inputs once, then run the lane.
//!
//! The ambient inputs are the running (updated) executable and its compiled-in version,
//! the delivery credentials, and the runner's summary/output files — every one of them
//! runner-provided, so the command reads them here and `pixi_sandbox::upgrade` stays a pure
//! function of its `Spec` (D10). Every delivery verdict exits successfully: a refusal is a
//! handoff with a prepared patch, not a red scheduled lane.

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::cli::UpgradeArgs;
use pixi_sandbox::pipeline::ProcessRunner;
use pixi_sandbox::upgrade::{self, GitHubPullRequestSource, Outcome};
use pixi_sandbox_git::ShellGit;

/// The version this binary was built as — the upgrade target. The workflow's upgrade step
/// runs this binary only after self-updating to it, so the version that regenerates the
/// files is always the one the workflow asked for (decision-4 / D16).
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run(args: UpgradeArgs) -> Result<()> {
    let exe = current_executable()?;
    let spec = upgrade::Spec {
        exe,
        repo_root: args.repo_root,
        workflow_path: args.github_workflow_path,
        relock_path: args.relock_workflow_path,
        script_path: args.script_path,
        relock_ci_workflow: args.relock_ci_workflow,
        config: args.config,
        branch: args.branch,
        version: CURRENT_VERSION.to_string(),
        artifact_dir: args.artifact_dir,
        repo: args.repo,
        base: args.base,
        upgrade_token: std::env::var("PIXI_SANDBOX_UPGRADE_TOKEN")
            .ok()
            .filter(|token| !token.is_empty()),
        github_token: std::env::var("GITHUB_TOKEN")
            .ok()
            .filter(|token| !token.is_empty()),
        step_summary: std::env::var_os("GITHUB_STEP_SUMMARY")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from),
        output: std::env::var_os("GITHUB_OUTPUT")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from),
    };

    let git = ShellGit::new();
    let pulls = GitHubPullRequestSource::new();
    let mut runner = ProcessRunner;
    match upgrade::run(&spec, &mut runner, &git, &pulls)? {
        Outcome::NoDrift => {
            println!(
                "no drift: the owned files already match pixi-sandbox {}",
                spec.version
            );
        }
        Outcome::Delivered { url } => {
            println!("opened pull request: {url}");
        }
        Outcome::Refused(refusal) => {
            println!("{}", refusal.annotation);
            if spec.step_summary.is_none() {
                print!("{}", refusal.summary);
            }
            if spec.output.is_none() {
                println!("{}", refusal.output_line);
            }
        }
    }
    Ok(())
}

/// The binary that is running, resolved through symlinks — the updated binary the workflow
/// self-updated to, which spawns `init --check` / `init` as its own phases.
fn current_executable() -> Result<PathBuf> {
    let executable =
        std::env::current_exe().context("locating the running pixi-sandbox executable")?;
    Ok(executable.canonicalize().unwrap_or(executable))
}
