//! `plan` — turn a reviewed `.pixi-sandbox.toml` into native publish jobs.
//!
//! GitHub Actions must know its matrix before runners start. Keeping that plan in the CLI means
//! the schema is tested, source-controlled, and usable locally without a workflow-specific TOML
//! parser.

use crate::cli::PlanArgs;
use crate::commands::support;
use anyhow::{Context, Result, bail};
use pixi_sandbox_core::sandbox_config::{SandboxConfig, plan_override};
use std::path::Path;

pub fn run(args: PlanArgs) -> Result<()> {
    let plan = match &args.envs {
        // Ad-hoc target: the same runner map, validation, and tool-coverage check as a reviewed
        // bundle, so a manual dispatch cannot quietly disagree with the plan it is imitating.
        Some(environments) => {
            let platform = args.platform.as_deref().unwrap_or("linux-64");
            let mut environments = environments.clone();
            environments.retain(|name| !name.is_empty());
            if environments.is_empty() {
                bail!("--envs named no environments");
            }
            plan_override(
                &args.bundle,
                &environments,
                platform,
                &args.branch_prefix,
                args.cargo_vendor,
            )
            .with_context(|| format!("planning an ad-hoc target for platform {platform}"))?
        }
        None => {
            let path = support::absolute(&args.config)?;
            let config = SandboxConfig::load(&path)
                .with_context(|| format!("loading sandbox publish config {}", path.display()))?;
            let mut plan = config.plan().context("planning sandbox publish jobs")?;
            // `push_paths` is surfaced here, not inside `SandboxConfig::plan`, because deriving
            // it needs a repo root to check which optional vendor manifests actually exist
            // (task-53 AC#3) — every other caller of `plan()` stays filesystem-free.
            let repo_root = path.parent().map_or_else(
                || support::absolute(Path::new(".")).unwrap_or_default(),
                Path::to_path_buf,
            );
            let config_display = args.config.to_string_lossy().replace('\\', "/");
            plan.push_paths = config.resolved_push_paths(&repo_root, &config_display);
            plan
        }
    };

    if args.json {
        // Keep stdout a single JSON value: the reusable workflow writes it straight to
        // $GITHUB_OUTPUT for `fromJSON(...)`.
        println!("{}", serde_json::to_string(&plan)?);
        return Ok(());
    }

    println!("schema {} · {} target(s)", plan.schema, plan.include.len());
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
