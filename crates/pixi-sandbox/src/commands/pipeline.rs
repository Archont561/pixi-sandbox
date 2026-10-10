//! `pipeline` — the CLI half: resolve the ambient inputs once, then run the phases.
//!
//! The ambient inputs are the running executable (which also drives the pack/doctor/publish
//! phases and is the default transport bootstrap), the push token, and
//! `$GITHUB_STEP_SUMMARY` — every one of them runner-provided, so the command reads them
//! here and `pixi_sandbox::pipeline` stays a pure function of its `Spec` (D10). On a phase
//! failure the command prints the annotation, prints the summary when no summary file was
//! provided, and exits with the phase's exit code — the shell's `exit "$exit_code"`.

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::cli::PipelineArgs;
use pixi_sandbox::pipeline::{self, ProcessRunner, Spec};

pub fn run(args: PipelineArgs) -> Result<()> {
    let exe = current_executable()?;
    let spec = Spec {
        envs: args.envs,
        platform: args.platform,
        branch: args.branch,
        remote: args.remote,
        config: args.config,
        self_bin: args.self_bin.unwrap_or_else(|| exe.clone()),
        exe,
        pixi: args.pixi,
        cargo_vendor: args.cargo_vendor,
        transport_dir: args.transport_dir,
        log_dir: args.log_dir,
        push_token: std::env::var("PIXI_SANDBOX_PUSH_TOKEN")
            .ok()
            .filter(|token| !token.is_empty()),
        step_summary: std::env::var_os("GITHUB_STEP_SUMMARY")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from),
    };

    match pipeline::run(&spec, &mut ProcessRunner) {
        Ok(()) => Ok(()),
        Err(failure) => {
            println!("{}", failure.annotation);
            if spec.step_summary.is_none() {
                print!("{}", failure.summary);
            }
            std::process::exit(failure.exit_code);
        }
    }
}

/// The binary that is running, resolved through symlinks — the same executable the phases
/// are spawned as, and the default `--self-bin` bundles into the transport.
fn current_executable() -> Result<PathBuf> {
    let executable =
        std::env::current_exe().context("locating the running pixi-sandbox executable")?;
    Ok(executable.canonicalize().unwrap_or(executable))
}
