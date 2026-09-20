//! Argument surface. Kept deliberately close to the spec in `.knowledge/design.md` §5.

use anyhow::Result;
use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(
    name = "pixi-sandbox",
    version,
    about = "Offline sandboxes for pixi projects (pack → publish → restore)",
    long_about = None,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Pack one or more pixi environments into a transportable directory.
    Pack(PackArgs),
    /// Force-push a transported directory as an orphan branch.
    Publish(PublishArgs),
    /// Verify and unpack a sandbox branch into a project.
    Restore(RestoreArgs),
    /// Unpack one packed environment into a prefix of your choosing.
    Unpack(UnpackArgs),
    /// Inspect a transport directory or branch: manifest, hashes, tool linkage, sizes.
    Doctor(DoctorArgs),
    /// Validate `.pixi-sandbox.toml` and emit native publish jobs.
    Plan(PlanArgs),
    /// Manage the pinned helper tools (`pixi-pack`, `pixi-unpack`, `pixi`).
    Tools(ToolsArgs),
}

#[derive(Debug, Args)]
pub struct PackArgs {
    /// Project root containing `pixi.lock` (and `Cargo.lock` when vendoring).
    #[arg(long, default_value = ".")]
    pub repo_root: PathBuf,

    /// Comma-separated pixi environment names.
    #[arg(long, value_delimiter = ',', required = true)]
    pub envs: Vec<String>,

    /// Where to write the transport directory.
    #[arg(long)]
    pub output_dir: PathBuf,

    /// Target platform for the packed environments.
    #[arg(long, default_value = "linux-64")]
    pub platform: String,

    /// Split any single file larger than this (MiB). GitHub blocks blobs above 100 MiB.
    #[arg(long, default_value_t = 95.0)]
    pub shard_limit_mib: f64,

    /// Also vendor cargo dependencies (`cargo vendor --locked --versioned-dirs`).
    #[arg(long)]
    pub cargo_vendor: bool,

    /// Storage mode for the vendored crates.
    #[arg(long, value_enum, default_value_t = VendorModeArg::Loose)]
    pub cargo_vendor_mode: VendorModeArg,

    /// Fetch and embed the helper tools pinned into this binary (no system installs).
    #[arg(long)]
    pub fetch_tools: bool,

    /// Use this reviewed pin file instead of the lock embedded in the binary. Relative paths
    /// are resolved against `--repo-root`.
    #[arg(long)]
    pub tools_lock: Option<PathBuf>,

    /// Cache directory for downloaded tools.
    #[arg(long, env = "PIXI_SANDBOX_TOOLS_CACHE")]
    pub tools_cache: Option<PathBuf>,

    /// Bundle this very binary so the branch bootstraps itself.
    #[arg(long)]
    pub self_bin: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum VendorModeArg {
    Loose,
    Tarballs,
}

#[derive(Debug, Args)]
pub struct PublishArgs {
    /// Transport directory produced by `pack`.
    #[arg(long)]
    pub input_dir: PathBuf,

    /// Orphan branch to (force-)push, e.g. `sandbox/dev-linux-64`.
    #[arg(long)]
    pub branch_name: String,

    /// Git remote (URL or path); defaults to `origin`.
    #[arg(long)]
    pub remote: Option<String>,

    /// Keep at most N snapshots on the branch (rotation; 0 = keep everything).
    #[arg(long, default_value_t = 0)]
    pub keep: u32,

    /// Show what would be pushed without touching the remote.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct RestoreArgs {
    /// Extracted branch content, e.g. `git archive <branch> | tar -x -C <dir>`.
    #[arg(long)]
    pub branch_location: PathBuf,

    /// Target project directory the environments should be installed into.
    #[arg(long, alias = "path-to-main-repo-code")]
    pub output_path: PathBuf,

    /// Restore only these envs (default: all).
    #[arg(long, value_delimiter = ',')]
    pub envs: Vec<String>,

    /// Verify hashes and report, then stop without writing anything.
    #[arg(long)]
    pub verify_only: bool,

    /// Replace environments / vendor trees that already exist.
    #[arg(long)]
    pub force: bool,

    /// Skip the vendored cargo dependencies.
    #[arg(long)]
    pub no_vendor: bool,

    /// Scratch space for staging. Must be on the project's filesystem — never a small `/tmp`.
    #[arg(long)]
    pub work_dir: Option<PathBuf>,

    /// How to wire `.cargo/config.toml` to the vendored sources.
    #[arg(long, value_enum, default_value_t = CargoConfigArg::Auto)]
    pub cargo_config: CargoConfigArg,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CargoConfigArg {
    /// Write the file only if it does not exist yet.
    Auto,
    /// Overwrite it.
    Write,
    /// Print the snippet for the user to paste.
    Print,
    /// Do nothing; use the `cargo-sandbox` wrapper from `sandbox-env.sh`.
    None,
}

#[derive(Debug, Args)]
pub struct UnpackArgs {
    /// A transport directory, its `.pixi-sandbox/envs/<env>/pack`, or a bare pack dir.
    #[arg(long)]
    pub input_dir: PathBuf,

    /// Directory the environment prefix is written into (the prefix itself, not its parent).
    #[arg(long)]
    pub output_dir: PathBuf,

    /// Environment to unpack. Required when the input holds more than one.
    #[arg(long)]
    pub env: Option<String>,

    /// Use this unpacker instead of the one embedded in the transport.
    #[arg(long)]
    pub unpacker: Option<PathBuf>,

    /// Replace `--output-dir` if it already exists.
    #[arg(long)]
    pub force: bool,

    /// Scratch space for staging; must be on `--output-dir`'s filesystem. Defaults to
    /// a sibling of the output, never `/tmp` (pixi-unpack stages into `$TMPDIR`).
    #[arg(long)]
    pub work_dir: Option<PathBuf>,

    /// Verify hashes and report, then stop without writing anything.
    #[arg(long)]
    pub verify_only: bool,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// A transport directory or an extracted branch.
    #[arg(long)]
    pub branch_location: PathBuf,

    /// Verify every declared blob (sha256 + sizes) instead of only summarising.
    #[arg(long)]
    pub verify: bool,

    /// Limit verification to these envs.
    #[arg(long, value_delimiter = ',')]
    pub envs: Vec<String>,

    /// Machine-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct PlanArgs {
    /// Project declaration listing the environment bundles and native platforms to publish.
    #[arg(long, default_value = ".pixi-sandbox.toml")]
    pub config: PathBuf,

    /// Emit a compact GitHub Actions matrix object on stdout.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ToolsArgs {
    #[command(subcommand)]
    pub command: ToolsCommand,
}

#[derive(Debug, Subcommand)]
pub enum ToolsCommand {
    /// Print the pins embedded in this binary, or an explicit override file.
    List {
        #[arg(long)]
        tools_lock: Option<PathBuf>,
    },
    /// Refresh an explicit pin file from upstream releases (network; review the diff).
    Update {
        #[arg(long)]
        tools_lock: Option<PathBuf>,
    },
}

pub fn run() -> Result<()> {
    use crate::commands;

    // A root bootstrap binary is the first thing an operator sees after extracting an orphan
    // branch. Keep a bare invocation read-only: it verifies the branch and prints the exact
    // restore command, rather than guessing an output directory and writing into the checkout.
    if std::env::args_os().nth(1).is_none() {
        if let Some(branch) = inferred_branch_location() {
            commands::doctor(DoctorArgs {
                branch_location: branch.clone(),
                verify: true,
                envs: Vec::new(),
                json: false,
            })?;
            let binary = if branch.join("pixi-sandbox.exe").is_file() {
                ".\\pixi-sandbox.exe"
            } else {
                "./pixi-sandbox"
            };
            println!(
                "next: {binary} restore --branch-location \"{}\" --output-path <project> --force",
                branch.display()
            );
            println!(
                "shortcut: \"{}/restore.sh\" <project> (or restore.ps1 on Windows)",
                branch.display()
            );
            return Ok(());
        }

        let mut command = Cli::command();
        command.print_help()?;
        return Ok(());
    }

    let cli = Cli::parse();
    match cli.command {
        Command::Pack(args) => commands::pack(args),
        Command::Publish(args) => commands::publish(args),
        Command::Restore(args) => commands::restore(args),
        Command::Unpack(args) => commands::unpack(args),
        Command::Doctor(args) => commands::doctor(args),
        Command::Plan(args) => commands::plan(args),
        Command::Tools(args) => commands::tools(args),
    }
}

fn inferred_branch_location() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    if has_manifest(&cwd) {
        return Some(cwd);
    }

    let executable = std::env::current_exe().ok()?.canonicalize().ok()?;
    let mut directory = executable.parent();
    while let Some(candidate) = directory {
        if has_manifest(candidate) {
            return Some(candidate.to_path_buf());
        }
        directory = candidate.parent();
    }
    None
}

fn has_manifest(directory: &Path) -> bool {
    directory
        .join(".pixi-sandbox")
        .join("manifest.json")
        .is_file()
}
