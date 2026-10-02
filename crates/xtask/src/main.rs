//! Typed entry point for repository automation.
//!
//! Everything that used to be a repo-targeting shell script lives here as a tested Rust
//! subcommand, so one runtime covers all five release runners — the v0.3.6 release died on
//! macOS's /bin/bash 3.2 over a single Bash-4 builtin (run 36865921206), a class of failure
//! an xtask cannot have. The only shell that remains under `scripts/` is `restore.sh`, the
//! one-command bootstrap that must run where no toolchain can be assumed.
//!
//! Commands take an explicit `--root` (default: the working directory, which is the project
//! root under `pixi run`); the policy logic lives in per-module pure functions that tests
//! drive against tempdir fixtures, never against this checkout (D10).

mod airlock;
mod commit_release;
mod conda_platforms;
mod prepare_release;
mod release_assets;
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
    /// Rewrite this repository's committed render of the generated relock workflow.
    RenderRelock,
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
    /// Copy the built release binary to its `pixi-sandbox-<target>[.exe]` asset name.
    StageReleaseBinary {
        /// Rust target triple; defaults to the host triple from `rustc -vV`. Positional, so
        /// the release workflow's one-liner carries no `--flag value` pair (pixi task args
        /// take one token per placeholder; anything flag-shaped goes after a `--`).
        target: Option<String>,
        /// Cargo target directory the binary was built into.
        #[arg(long, default_value = "target")]
        target_dir: PathBuf,
        /// Directory the asset is staged into.
        #[arg(long, default_value = ".")]
        out_dir: PathBuf,
        /// Strip executable; best-effort, the release profile already strips.
        #[arg(long, default_value = "strip")]
        strip: String,
    },
    /// Write SHA256SUMS over the standalone release binaries and verify it covers every one.
    ReleaseChecksums {
        /// Directory holding the downloaded binary artifacts.
        #[arg(default_value = "dist")]
        dir: PathBuf,
    },
    /// Emit the airlock proof matrix from the plan binary and validate its shape.
    AirlockMatrix {
        /// Plan binary to invoke (the plan job builds it from this commit).
        #[arg(long, default_value = "target/release/pixi-sandbox")]
        bin: String,
        /// Sandbox config for config mode.
        #[arg(long, default_value = ".pixi-sandbox.toml")]
        config: String,
        /// Override mode: comma-separated environments. Empty means config mode; the four
        /// flags below mirror the workflow inputs, where absent arrives as an empty string.
        #[arg(long, default_value = "")]
        envs: String,
        #[arg(long, default_value = "")]
        platform: String,
        #[arg(long, default_value = "")]
        bundle: String,
        #[arg(long, default_value = "")]
        branch_prefix: String,
        /// true | false; empty means true, like the `${CARGO_VENDOR:-true}` shell default.
        #[arg(long, default_value = "")]
        cargo_vendor: String,
    },
    /// Resolve which published release the airlock proof should exercise.
    ResolveReleaseTag {
        /// Repository (`owner/name`) for the releases API.
        #[arg(long)]
        repo: String,
        /// Explicit tag to prove (`vars.SANDBOX_RELEASE_VERSION`); empty means derive.
        #[arg(long, default_value = "")]
        r#override: String,
    },
    /// Download the static release binary the airlock transport should embed.
    AirlockSelfBin {
        /// Repository (`owner/name`) to download the release asset from.
        #[arg(long)]
        repo: String,
        /// Release tag resolved by the plan job.
        #[arg(long)]
        tag: String,
        /// Pixi platform from the matrix.
        #[arg(long)]
        platform: String,
        /// Output path for the executable.
        #[arg(long)]
        out: PathBuf,
    },
    /// Install the matrix environments (frozen) and pack with the released binary.
    AirlockPack {
        /// Comma-separated environments from the matrix.
        #[arg(long)]
        envs: String,
        /// Transport output directory.
        #[arg(long)]
        out: PathBuf,
        /// true | false | empty (= true): the matrix cargo-vendor policy.
        #[arg(long, default_value = "")]
        cargo_vendor: String,
        /// Released binary to pack (and embed) with; defaults to `pixi-sandbox` on PATH.
        #[arg(long)]
        self_bin: Option<PathBuf>,
    },
    /// Fetch a published sandbox branch into a worktree the way a developer would.
    AirlockFetch {
        /// Remote (URL or path) that carries the branch.
        #[arg(long)]
        remote: String,
        /// Branch to fetch.
        #[arg(long)]
        branch: String,
        /// Host repository directory (created fresh).
        #[arg(long)]
        host_dir: PathBuf,
        /// Worktree directory the branch is checked out into.
        #[arg(long)]
        worktree: PathBuf,
    },
    /// Re-exec a command with network egress denied (unshare -n / sandbox-exec).
    DenyEgress {
        /// Command (with arguments) to run under egress denial; everything after `--`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// Turn a prepared tree into the release commit, tag, and pushes (auto-release's half).
    CommitRelease {
        /// Release tag to cut, `vX.Y.Z`. Positional, so the workflow's one-liner carries no
        /// `--flag value` pair (pixi task args take one token per placeholder).
        tag: String,
        /// Print the diff and stop: no staging, commit, tag, or push.
        #[arg(long)]
        dry_run: bool,
        /// Remote to push the branch and tag to.
        #[arg(long, default_value = "origin")]
        remote: String,
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
    run_args(Args::parse())
}

fn run_args(args: Args) -> Result<()> {
    let root = args.root;
    match args.command {
        Command::LintGeneratedWorkflow { actionlint } => {
            workflow::lint_generated_workflow(&actionlint)
        }
        Command::RenderRelock => workflow::render_relock(&root),
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
            // The workflow consumes the version as `steps.<id>.outputs.version`; locally the
            // tag remains the last line of stdout, which is what a command substitution reads
            // (task-36 AC#4).
            util::github_output("version", &tag);
            if let Ok(stat) = pixi_sandbox_git::ShellGit::new().worktree_diff_stat(&root) {
                util::github_summary(&format!("## Prepared `{tag}`\n\n```\n{stat}\n```"));
            }
            println!("{tag}");
            Ok(())
        }
        Command::CommitRelease {
            tag,
            dry_run,
            remote,
        } => commit_release::commit_release(&root, &tag, dry_run, &remote),
        Command::AirlockMatrix {
            bin,
            config,
            envs,
            platform,
            bundle,
            branch_prefix,
            cargo_vendor,
        } => airlock::airlock_matrix(
            &bin,
            &config,
            &envs,
            &platform,
            &bundle,
            &branch_prefix,
            &cargo_vendor,
        ),
        Command::ResolveReleaseTag { repo, r#override } => {
            airlock::resolve_release_tag(&root, &repo, &r#override)
        }
        Command::AirlockSelfBin {
            repo,
            tag,
            platform,
            out,
        } => airlock::airlock_self_bin(&repo, &tag, &platform, &root.join(out)),
        Command::AirlockPack {
            envs,
            out,
            cargo_vendor,
            self_bin,
        } => airlock::airlock_pack(
            &root,
            &envs,
            &root.join(out),
            &cargo_vendor,
            self_bin.as_deref(),
        ),
        Command::AirlockFetch {
            remote,
            branch,
            host_dir,
            worktree,
        } => airlock::airlock_fetch(&remote, &branch, &root.join(host_dir), &root.join(worktree)),
        Command::DenyEgress { command } => airlock::deny_egress(&command),
        Command::StageReleaseBinary {
            target,
            target_dir,
            out_dir,
            strip,
        } => release_assets::stage_release_binary(
            target.as_deref(),
            &root.join(target_dir),
            &root.join(out_dir),
            &strip,
        )
        .map(|_| ()),
        Command::ReleaseChecksums { dir } => {
            release_assets::release_checksums(&root.join(dir)).map(|_| ())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Args, Command, run_args};

    #[test]
    fn airlock_self_bin_subcommand_reaches_the_downloader_validation() {
        let root = tempfile::tempdir().expect("tempdir");
        let error = run_args(Args {
            root: root.path().to_path_buf(),
            command: Command::AirlockSelfBin {
                repo: "owner/repo".to_string(),
                tag: "v0.4.0".to_string(),
                platform: "freebsd-64".to_string(),
                out: "pixi-sandbox".into(),
            },
        })
        .expect_err("unknown platform");

        assert!(
            format!("{error:#}").contains("no static pixi-sandbox release asset"),
            "{error:#}"
        );
    }
}
