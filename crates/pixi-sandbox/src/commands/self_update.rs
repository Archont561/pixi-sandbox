//! `self-update` — the CLI half: resolve the ambient inputs once, then report.
//!
//! Every environment read lives here (the running executable, the host triple, the compiled-in
//! version), so `crate::self_update` itself stays a pure function of its `Request` and the
//! tests never reach a developer's real binary (D10).

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::cli::SelfUpdateArgs;
use pixi_sandbox::release::GitHubReleaseSource;
use pixi_sandbox::self_update::{self, Request, replace::ReplaceStrategy};

/// The version this binary was built as — what a self-update is updating *from*.
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run(args: SelfUpdateArgs) -> Result<()> {
    let destination = match args.dest {
        Some(dest) => dest,
        None => current_executable()?,
    };
    let (host_os, host_arch) = self_update::assets::current_host();
    let source = GitHubReleaseSource::new();

    let request = Request {
        repo: &args.repo,
        requested_version: args.version.as_deref(),
        destination: &destination,
        current_version: CURRENT_VERSION,
        host_os,
        host_arch,
        strategy: ReplaceStrategy::current(),
    };

    let plan = self_update::plan(&source, &request)?;

    println!("self-update  {}", plan.destination.display());
    println!(
        "  current  {CURRENT_VERSION}\n  target   {} ({})\n  asset    {}",
        plan.resolved.version,
        plan.resolved.selection.describe(),
        plan.asset
    );

    if let Some(refusal) = plan.ownership.refusal(&plan.destination) {
        // Report the refusal the same way in both modes: `--check` must be able to tell CI
        // that this destination would never have been updatable, before it tries.
        anyhow::bail!("{refusal}");
    }

    if args.check {
        if plan.is_up_to_date() {
            println!("  verdict  up to date — nothing to do");
        } else {
            println!(
                "  verdict  update available: {CURRENT_VERSION} -> {}",
                plan.resolved.version
            );
        }
        println!("  (--check wrote nothing)");
        return Ok(());
    }

    if plan.is_up_to_date() {
        println!("  verdict  already {CURRENT_VERSION}; nothing to do");
        return Ok(());
    }

    let applied = self_update::apply(&source, &request, plan)?;
    println!("  sha256   {}", applied.digest);
    for swept in &applied.replacement.swept {
        println!("  swept    {}", swept.display());
    }
    if let Some(displaced) = &applied.replacement.displaced {
        if displaced.exists() {
            println!(
                "  note     the previous binary is still mapped; {} is removed on the next update",
                displaced.display()
            );
        }
    }
    println!(
        "updated  {} -> {}",
        applied.plan.current_version, applied.plan.resolved.version
    );
    Ok(())
}

/// The binary that is running, resolved through symlinks so the ownership ladder judges the
/// real file rather than a link into a Pixi prefix.
fn current_executable() -> Result<PathBuf> {
    let executable =
        std::env::current_exe().context("locating the running pixi-sandbox executable")?;
    Ok(executable.canonicalize().unwrap_or(executable))
}
