//! Typed entry point for repository automation.

mod workflow;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(about = "Repository automation for pixi-sandbox")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate the consumer workflow and validate it with actionlint.
    LintGeneratedWorkflow {
        /// actionlint executable to invoke.
        #[arg(long, default_value = "actionlint")]
        actionlint: PathBuf,
    },
}

fn main() -> Result<()> {
    match Args::parse().command {
        Command::LintGeneratedWorkflow { actionlint } => {
            workflow::lint_generated_workflow(&actionlint)
        }
    }
}
