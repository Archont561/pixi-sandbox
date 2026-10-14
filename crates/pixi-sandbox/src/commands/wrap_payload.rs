//! `wrap-payload` — fetch a prebuilt payload and stage its conda package recipe (TASK-78, ADR-28).
//!
//! Thin wiring, as every command module is: it resolves the ambient inputs (the config path and
//! its directory, the budget source) and hands the work to the library. The config-declared
//! `prepare` command does the network (`crate::prepare`), the pure rules decide and render
//! (`pixi_sandbox_core::payload`), and staging writes the recipe directory the build tool will
//! consume (`crate::wrap_payload`). pixi-sandbox never downloads the payload itself: it runs a
//! command the consumer reviewed and checks the declared SHA256 against what that command left
//! (AC#13).

use crate::cli::WrapPayloadArgs;
use crate::commands::support;
use anyhow::{Context, Result};
use pixi_sandbox::prepare::{ShellRunner, prepare, read_prepared};
use pixi_sandbox::rattler_build::{RattlerBuildToolchain, RealProcessRunner, resolve_tool};
use pixi_sandbox::tool_fetch::{HttpDownload, tools_cache};
use pixi_sandbox::wrap_payload::{plan_report, stage_recipe_dir};
use pixi_sandbox_core::payload::{
    PayloadToolchain, WrapPlan, WrapRequest, check_built_package_blob_budget, plan_wrap,
};
use pixi_sandbox_core::payload_config::PayloadConfig;
use pixi_sandbox_core::sandbox_config::SandboxConfig;
use pixi_sandbox_core::transport_budget::TransportBudgets;
use std::path::Path;

pub fn run(args: &WrapPayloadArgs) -> Result<()> {
    let config_path = support::absolute(&args.config)?;
    let config = PayloadConfig::load(&config_path)
        .with_context(|| format!("loading payload config {}", config_path.display()))?;
    let config_dir = config_path
        .parent()
        .map_or_else(|| std::path::PathBuf::from("."), Path::to_path_buf);

    let budgets = resolve_budgets(args.budget_config.as_deref())?;

    let prepared = if args.skip_prepare {
        read_prepared(&config, &config_dir).context("reading the prepared payload")?
    } else {
        prepare(&config, &config_dir, &ShellRunner).context("preparing the payload")?
    };

    let request = WrapRequest {
        spec: config.spec()?,
        entries: prepared.entries,
        source_url: config.source.url.clone(),
        source_sha256: config.source.sha256.clone(),
        budgets,
    };
    let plan = plan_wrap(&request).context("planning the payload wrap")?;

    if args.json {
        println!("{}", serde_json::to_string(&plan_report(&plan))?);
        return Ok(());
    }
    if args.plan {
        print_plan(&plan);
        return Ok(());
    }

    let out_dir = args
        .out
        .clone()
        .unwrap_or_else(|| config_dir.join(".pixi/payload-build"));
    let staged = stage_recipe_dir(&plan, &prepared.payload_root, &out_dir)
        .with_context(|| format!("staging the recipe directory {}", out_dir.display()))?;
    println!(
        "staged {} {} for {} at {}",
        plan.spec.name,
        plan.spec.version,
        plan.spec.platform.as_str(),
        staged.recipe_dir.display()
    );

    if args.build {
        let _package = build_and_publish(
            args,
            &plan,
            &staged.recipe_dir,
            &config_dir,
            budgets.max_blob_bytes,
        )?;
    }
    Ok(())
}

/// Build the staged recipe with `rattler-build`, check its actual size, and publish it.
///
/// The tool is fetched from its reviewed pins into the tools cache unless `--build-tool` names an
/// existing one. This is the only step of `wrap-payload` that may touch the network, and it is the
/// tool that is fetched — never the payload (AC#12, AC#13). The built `.conda` is checked against
/// `max_blob_bytes` before the channel is indexed.
fn build_and_publish(
    args: &WrapPayloadArgs,
    plan: &WrapPlan,
    work_dir: &Path,
    config_dir: &Path,
    max_blob_bytes: u64,
) -> Result<std::path::PathBuf> {
    let program = if let Some(path) = &args.build_tool {
        support::absolute(path)?
    } else {
        let cache = tools_cache(
            args.tools_cache.as_deref(),
            std::env::var_os("HOME").as_deref(),
        )
        .context("choosing the tools cache")?;
        resolve_tool(plan.spec.platform, &cache, &HttpDownload)?
    };
    let channel_dir = match &args.channel {
        Some(path) => support::absolute(path)?,
        None => config_dir.join(".pixi/payload-channel"),
    };

    // Do not pre-create the channel: `rattler-build publish --to file://…` initialises a channel
    // that does not exist yet, but refuses an existing directory whose index it cannot find. The
    // path is created by the tool, so a first wrap works and later wraps append.
    let toolchain = RattlerBuildToolchain::new(program, &RealProcessRunner);
    let package = toolchain
        .build(plan, work_dir)
        .context("building the payload package")?;
    check_built_package_blob_budget(&package, max_blob_bytes)?;
    toolchain
        .index(&channel_dir, &package)
        .context("publishing the package into the channel")?;
    println!(
        "built {} and published it to {}",
        package.display(),
        channel_dir.display()
    );
    Ok(package)
}

fn resolve_budgets(budget_config: Option<&Path>) -> Result<TransportBudgets> {
    match budget_config {
        Some(path) => {
            let path = support::absolute(path)?;
            let sandbox = SandboxConfig::load(&path)
                .with_context(|| format!("loading budget config {}", path.display()))?;
            sandbox
                .budgets
                .to_transport_budgets()
                .context("reading budgets from the sandbox config")
        }
        None => Ok(TransportBudgets::default()),
    }
}

fn print_plan(plan: &WrapPlan) {
    println!(
        "{} {} for {}",
        plan.spec.name,
        plan.spec.version,
        plan.spec.platform.as_str()
    );
    println!(
        "  payload: {} file(s), {} dir(s), {} link(s), {} byte(s)",
        plan.payload.file_count,
        plan.payload.directory_count,
        plan.payload.symlink_count,
        plan.payload.total_bytes
    );
    println!("  source:  {}", plan.provenance.source.url);
    println!("  sha256:  {}", plan.provenance.source.sha256);
    println!("  recipe:");
    for line in plan.recipe.lines() {
        println!("    {line}");
    }
}
