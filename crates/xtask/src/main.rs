//! Typed entry point for repository automation.
//!
//! Everything that used to be a repo-targeting shell script lives here as a tested Rust
//! subcommand, so one runtime covers all five release runners — the v0.3.6 release died on
//! macOS's /bin/bash 3.2 over a single Bash-4 builtin (run 36865921206), a class of failure
//! an xtask cannot have. The only shell that remains under `scripts/` is the bootstrap pair
//! that must run where no toolchain can be assumed (`restore.sh`, `airlock-gate.sh`).
//!
//! Commands take an explicit `--root` (default: the working directory, which is the project
//! root under `pixi run`); the policy logic lives in per-module pure functions that tests
//! drive against tempdir fixtures, never against this checkout (D10).

mod conda_platforms;
mod prepare_release;
mod release_refs;
mod repo_checks;
mod smoke;
mod util;
mod version;
mod workflow;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(about = "Repository automation for pixi-sandbox")]
struct Args {
    /// Repository root the repo-targeting commands operate on.
    #[arg(long, default_value = ".")]
    root: PathBuf,

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
    /// Print the workspace version (the single source of truth in Cargo.toml).
    Version,
    /// Run every repository consistency check and report all failures in one pass.
    CheckRepository,
    /// Report every documented release reference that disagrees with the declared version.
    CheckReleaseRefs,
    /// Fail unless every supported platform contributed exactly one conda package.
    CheckCondaPlatforms {
        /// Artifact root holding the per-platform `conda-<platform>/` directories.
        #[arg(default_value = "dist/conda")]
        dir: PathBuf,
    },
    /// Install the just-built conda package(s) and run the packaged binary on this machine.
    SmokeCondaPackage {
        /// Directory the `package` task wrote the `.conda` file(s) into.
        #[arg(default_value = ".publish/out")]
        out_dir: PathBuf,
    },
    /// Resolve the next release version and stamp it into the repository (no git actions).
    PrepareRelease {
        /// auto | major | minor | patch | vX.Y.Z | X.Y.Z (default: $PIXI_SANDBOX_RELEASE or auto).
        selector: Option<String>,
    },
}

fn main() {
    if let Err(error) = run() {
        // GitHub-annotated, like the scripts this replaced, so a workflow failure carries
        // the reason into the run summary instead of only the raw log.
        eprintln!("::error::{error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    let root = args.root;
    match args.command {
        Command::LintGeneratedWorkflow { actionlint } => {
            workflow::lint_generated_workflow(&actionlint)
        }
        Command::Version => version::print_version(&root),
        Command::CheckRepository => repo_checks::run(&root),
        Command::CheckReleaseRefs => {
            let findings = release_refs::scan(&root)?;
            if !findings.is_empty() {
                for finding in &findings {
                    eprintln!("::error::{finding}");
                }
                anyhow::bail!("{} stale release reference(s)", findings.len());
            }
            eprintln!("release references agree with the declared version");
            Ok(())
        }
        Command::CheckCondaPlatforms { dir } => conda_platforms::check(&root.join(dir)),
        Command::SmokeCondaPackage { out_dir } => {
            smoke::smoke_conda_package(&root, &root.join(out_dir))
        }
        Command::PrepareRelease { selector } => {
            let selector = selector
                .or_else(|| std::env::var("PIXI_SANDBOX_RELEASE").ok())
                .unwrap_or_else(|| "auto".to_string());
            let tag = prepare_release::run(&root, &selector)?;
            // The only value on stdout, so `$(… | tail -n1)` in the release workflow stays a
            // plain command substitution.
            println!("{tag}");
            Ok(())
        }
    }
}
