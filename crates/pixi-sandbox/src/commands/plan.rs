//! `plan` — turn a reviewed `.pixi-sandbox.toml` into native publish jobs.
//!
//! GitHub Actions must know its matrix before runners start. Keeping that plan in the CLI means
//! the schema is tested, source-controlled, and usable locally without a workflow-specific TOML
//! parser.

use crate::cli::PlanArgs;
use crate::commands::support;
use anyhow::{Context, Result};
use pixi_sandbox_core::sandbox_config::SandboxConfig;

pub fn run(args: PlanArgs) -> Result<()> {
    let path = support::absolute(&args.config)?;
    let config = SandboxConfig::load(&path)
        .with_context(|| format!("loading sandbox publish config {}", path.display()))?;
    let plan = config.plan().context("planning sandbox publish jobs")?;

    if args.json {
        // Keep stdout a single JSON value: the reusable workflow writes it straight to
        // $GITHUB_OUTPUT for `fromJSON(...)`.
        println!("{}", serde_json::to_string(&plan)?);
        return Ok(());
    }

    println!("{} (schema {})", path.display(), plan.schema);
    for target in plan.include {
        let vendor = if target.cargo_vendor {
            "with vendored Cargo crates"
        } else {
            "without vendored Cargo crates"
        };
        println!(
            "  {}: {} · {} · runner {} · {} · {vendor}",
            target.bundle, target.environments, target.platform, target.runner, target.branch
        );
    }
    Ok(())
}
