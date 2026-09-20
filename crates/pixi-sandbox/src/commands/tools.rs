//! `tools` — inspect the helper-tool pins compiled into the binary or an explicit override.
//!
//! `list` is how a reviewer sees what `pack --fetch-tools` will download. The embedded
//! catalogue makes installed packages self-contained; an external file remains available for a
//! reviewed organisation mirror or a deliberately different release pin.

use crate::cli::{ToolsArgs, ToolsCommand};
use crate::commands::support;
use anyhow::{Context, Result};
use pixi_sandbox_core::tools_lock::{EMBEDDED_SOURCE, ToolsLock};
use std::path::Path;

pub fn run(args: ToolsArgs) -> Result<()> {
    match args.command {
        ToolsCommand::List { tools_lock } => list(tools_lock.as_deref()),
        ToolsCommand::Update { tools_lock } => {
            let target = tools_lock
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "an explicit --tools-lock file".to_string());
            Err(super::not_yet(&format!("tools update ({target})"), "10"))
        }
    }
}

fn list(path: Option<&Path>) -> Result<()> {
    let (lock, label) = match path {
        Some(path) => {
            let path = support::absolute(path)?;
            let lock = ToolsLock::load(&path)
                .with_context(|| format!("reading tool-pin override {}", path.display()))?;
            (lock, path.display().to_string())
        }
        None => (
            ToolsLock::embedded().context("loading embedded helper-tool pins")?,
            EMBEDDED_SOURCE.to_string(),
        ),
    };
    println!("{label} (schema {})", lock.schema);

    for name in lock.names() {
        let tool = &lock.tools[name];
        println!("  {name} {}", tool.version);
        for platform in lock.platforms_of(name) {
            let Some(pin) = lock.pin(name, platform) else {
                continue;
            };
            println!(
                "    {:<14} {:<28} {} {}",
                platform,
                pin.target,
                &pin.sha256[..pin.sha256.len().min(12)],
                pin.linkage,
            );
        }
        println!(
            "    url template   {}",
            tool.url_template.replace("{version}", &tool.version)
        );
    }
    Ok(())
}
