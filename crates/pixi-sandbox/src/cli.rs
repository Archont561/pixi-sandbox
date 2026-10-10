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
    /// Generate a publishing workflow and this platform's offline restore launcher.
    Init(InitArgs),
    /// Validate `.pixi-sandbox.toml` and emit native publish jobs.
    Plan(PlanArgs),
    /// Manage the pinned helper tools (`pixi-pack`, `pixi-unpack`, `pixi`).
    Tools(ToolsArgs),
    /// Replace this standalone binary with a published release (decision-4).
    SelfUpdate(SelfUpdateArgs),
    /// Download the checksum-verified standalone release binary for this host (workflow bootstrap).
    FetchRelease(FetchReleaseArgs),
    /// Install, pack, verify, and publish a transport — the generated publisher's pipeline.
    Pipeline(PipelineArgs),
    /// Regenerate the owned files with this binary and deliver the upgrade as a pull request.
    ///
    /// Reads its ambient inputs from the runner's environment: `PIXI_SANDBOX_UPGRADE_TOKEN`
    /// (the optional workflow-capable secret — when absent, delivery is refused with a
    /// handoff and the prepared patch, never a red scheduled lane), `GITHUB_TOKEN` (what the
    /// pull request authenticates with), and `$GITHUB_STEP_SUMMARY` / `$GITHUB_OUTPUT`.
    Upgrade(UpgradeArgs),
}

#[derive(Debug, Args)]
pub struct UpgradeArgs {
    /// The consumer repository's checked-out working tree; the git operations run here.
    #[arg(long, default_value = ".")]
    pub repo_root: PathBuf,

    /// GitHub Actions workflow path, relative to the project root — `init`'s flag name, so
    /// the upgrade forwards the exact generation arguments (D16).
    #[arg(long, value_name = "PATH")]
    pub github_workflow_path: PathBuf,

    /// Lockfile-refresh workflow path, relative to the project root.
    #[arg(long, value_name = "PATH")]
    pub relock_workflow_path: PathBuf,

    /// Workflow the relock bot dispatches after pushing a lock commit.
    #[arg(long)]
    pub relock_ci_workflow: String,

    /// Airlock launcher path, relative to the project root.
    #[arg(long, value_name = "PATH")]
    pub script_path: PathBuf,

    /// Reviewed sandbox plan path (`init`'s `--config`), forwarded to both `init` calls.
    /// The config itself is reviewed data and is never regenerated or staged (D16).
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Default local sandbox branch archived by the launcher (`init`'s `--branch`).
    #[arg(long)]
    pub branch: String,

    /// Where the upgrade patch and the regenerated copies are written — the artifact
    /// fallback a human applies when delivery is refused.
    #[arg(long, value_name = "PATH")]
    pub artifact_dir: PathBuf,

    /// The consumer repository as `owner/name`, for the pull request.
    #[arg(long, value_name = "OWNER/NAME")]
    pub repo: String,

    /// Base branch for the pull request.
    #[arg(long, default_value = "main")]
    pub base: String,
}

#[derive(Debug, Args)]
pub struct PipelineArgs {
    /// Comma-separated pixi environment names to install and pack (the plan matrix's value).
    #[arg(long, value_name = "LIST")]
    pub envs: String,

    /// Target platform for the packed environments.
    #[arg(long, default_value = "linux-64")]
    pub platform: String,

    /// Orphan branch to publish the transport to.
    #[arg(long)]
    pub branch: String,

    /// Git remote (URL) to publish to. The publish phase authenticates with the
    /// `PIXI_SANDBOX_PUSH_TOKEN` environment variable (the generated workflow sets it to
    /// `github.token`); without it, the operator's own git credentials apply.
    #[arg(long)]
    pub remote: String,

    /// Reviewed sandbox config: `pack --config` records host requirements from it, and
    /// `doctor --budget-config` enforces its budgets before anything is published.
    #[arg(long, value_name = "PATH")]
    pub config: PathBuf,

    /// The standalone binary to bundle as the transport bootstrap. The generated workflow
    /// runs the pipeline as that very binary, so the default is the running executable.
    #[arg(long, value_name = "PATH")]
    pub self_bin: Option<PathBuf>,

    /// Also vendor cargo dependencies (`--cargo-vendor`). Value-taking as well as bare, so
    /// the workflow can pass the matrix's `true`/`false` through one spelling (the same shape
    /// `plan --cargo-vendor` already uses).
    #[arg(long, num_args = 0..=1, default_missing_value = "true", default_value_t = false)]
    pub cargo_vendor: bool,

    /// Where `pack` writes the transport directory. Reset before packing: `pack` refuses a
    /// stale output directory, and a rerun must not inherit one.
    #[arg(long, value_name = "PATH", default_value = "pixi-sandbox-transport")]
    pub transport_dir: PathBuf,

    /// Where the pipeline log and the per-phase diagnostic logs are written. Reset at the
    /// start of every run.
    #[arg(long, value_name = "PATH", default_value = "pixi-sandbox-logs")]
    pub log_dir: PathBuf,

    /// The `pixi` executable the install phase runs (default: `pixi` from PATH).
    #[arg(long, default_value = "pixi")]
    pub pixi: String,
}

#[derive(Debug, Args)]
// The root sets `propagate_version`, which hands every subcommand an auto-generated
// `--version`. Here that name belongs to the pinned release being fetched, so the inherited
// flag is turned off rather than renaming the argument an operator would reach for first —
// the same reason `SelfUpdateArgs` disables it.
#[command(disable_version_flag = true)]
pub struct FetchReleaseArgs {
    /// Where to install the verified standalone binary.
    ///
    /// The form the generated workflows use: a disposable path in runner scratch, installed
    /// by the package-managed CLI so the released standalone binary can drive the rest of
    /// the job. The destination is classified by the same ownership ladder `self-update`
    /// uses — a Pixi-managed path is refused before anything is downloaded.
    #[arg(long, value_name = "PATH")]
    pub dest: PathBuf,

    /// Exact release to fetch (`X.Y.Z` or `vX.Y.Z`). The generated workflows always pin —
    /// the workflow's own `PIXI_SANDBOX_VERSION` stamp — never float.
    #[arg(long, value_name = "X.Y.Z")]
    pub version: String,

    /// Repository publishing the standalone release assets.
    #[arg(long, default_value = pixi_sandbox::self_update::DEFAULT_REPO)]
    pub repo: String,
}

#[derive(Debug, Args)]
// The root sets `propagate_version`, which hands every subcommand an auto-generated
// `--version`. Here that name belongs to the release being requested, so the inherited flag
// is turned off rather than renaming the argument an operator would reach for first.
#[command(disable_version_flag = true)]
pub struct SelfUpdateArgs {
    /// Install this exact release instead of the latest one (`X.Y.Z` or `vX.Y.Z`).
    ///
    /// Every committed workflow and published transport names an exact version; `latest` is
    /// discovery for a reviewed upgrade, and this is also how a rollback selects an older
    /// release (decision-4).
    #[arg(long, value_name = "X.Y.Z")]
    pub version: Option<String>,

    /// Report the current and resolved versions and exit without downloading or writing.
    #[arg(long)]
    pub check: bool,

    /// Update this standalone binary instead of the running one.
    ///
    /// The form CI uses: a disposable, checksum-verified binary in runner scratch. A
    /// package-manager-owned path is refused with its remedy either way.
    #[arg(long, value_name = "PATH")]
    pub dest: Option<PathBuf>,

    /// Repository publishing the standalone release assets.
    #[arg(long, default_value = pixi_sandbox::self_update::DEFAULT_REPO)]
    pub repo: String,
}

#[derive(Debug, Args, Default)]
pub struct DiagnosticsArgs {
    /// Increase diagnostic detail on stderr (`-v` for phases, `-vv` for reviewed context).
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Create a durable diagnostic log, flushed after every phase and never overwriting a file.
    #[arg(long, value_name = "PATH")]
    pub log_file: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct PackArgs {
    #[command(flatten)]
    pub diagnostics: DiagnosticsArgs,

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

    /// Bundle this very binary so the branch bootstraps itself. Must run *standalone*:
    /// use the checksum-verified release asset (`pixi-sandbox-<target>`), never a
    /// `pixi global install` trampoline — pack executes `--version` under an empty
    /// environment and refuses a binary that breaks without its prefix (issue #81).
    #[arg(long)]
    pub self_bin: Option<PathBuf>,

    /// Optional sandbox config (`.pixi-sandbox.toml` / `pixi-sandbox.toml`, relative to the
    /// current directory like `plan --config`). When given, the `[host_requirements]` table
    /// resolved for `--platform` is recorded in the transport manifest, so a restored branch
    /// still says what its workloads need from the machine (issue #109, TASK-75). Without the
    /// flag, no host-requirement section is written and the manifest is byte-identical to what
    /// earlier releases packed.
    #[arg(long)]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum VendorModeArg {
    Loose,
    Tarballs,
}

#[derive(Debug, Args)]
pub struct PublishArgs {
    #[command(flatten)]
    pub diagnostics: DiagnosticsArgs,

    /// Transport directory produced by `pack`.
    #[arg(long)]
    pub input_dir: PathBuf,

    /// Orphan branch to (force-)push, e.g. `sandbox/dev-linux-64`.
    #[arg(long)]
    pub branch_name: String,

    /// Git remote (URL or path); defaults to `origin`.
    #[arg(long)]
    pub remote: Option<String>,

    /// Keep at most N snapshots on the branch (rotation; 0/1 = one orphan snapshot).
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

    /// Replace environments / vendor trees that already exist. Also replaces unmanaged
    /// `pixi` / `pixi-sandbox` entries in the user bin directory (task-33 collisions).
    #[arg(long)]
    pub force: bool,

    /// Skip the vendored cargo dependencies.
    #[arg(long)]
    pub no_vendor: bool,

    /// Scratch space for staging. Must be on the project's filesystem — never a small `/tmp`.
    #[arg(long)]
    pub work_dir: Option<PathBuf>,

    /// How to wire Cargo to the vendored sources.
    #[arg(long, value_enum, default_value_t = CargoConfigArg::Auto)]
    pub cargo_config: CargoConfigArg,

    /// Register the restored `pixi` and `pixi-sandbox` tools in a per-user bin directory
    /// (default `~/.local/bin`, `%USERPROFILE%\.pixi-sandbox\bin` on Windows) and add that
    /// directory to the shell's persistent PATH — only after the restored tree has been
    /// verified against the manifest. `skip` makes restore touch nothing outside the project
    /// (CI, shared accounts, locked-down airlocks).
    ///
    /// `overrides_with` itself: the generated launchers pass this flag explicitly *and*
    /// forward their own arguments, so a later `--user-tools skip` from the operator must win
    /// over the launcher's choice rather than error out.
    #[arg(
        long,
        value_enum,
        default_value_t = UserToolsPolicy::Register,
        env = "PIXI_SANDBOX_USER_TOOLS",
        overrides_with = "user_tools"
    )]
    pub user_tools: UserToolsPolicy,

    /// Per-user bin directory for the registered launchers (default: `~/.local/bin` on Unix,
    /// `%USERPROFILE%\.pixi-sandbox\bin` on Windows). Configurable for airlock policy.
    #[arg(long, env = "PIXI_SANDBOX_USER_BIN")]
    pub user_bin: Option<PathBuf>,
}

/// task-33: whether a successful, verified restore also registers the bundled tools for the
/// user. The default is what a person restoring an airlock wants; `skip` is what CI wants.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum UserToolsPolicy {
    /// Write managed launchers and a managed PATH block (or Windows user PATH entry).
    Register,
    /// Touch nothing outside the project: no HOME, no profile, no registry.
    Skip,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CargoConfigArg {
    /// Write sandbox-owned Cargo config under `.pixi-sandbox/cargo-home` and activate it via Pixi.
    Auto,
    /// Overwrite the project `.cargo/config.toml`.
    Write,
    /// Print the Cargo source-replacement snippet for the user to paste.
    Print,
    /// Do nothing; callers must provide their own Cargo source configuration before using
    /// `pixi run -- cargo ...` offline.
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
    #[command(flatten)]
    pub diagnostics: DiagnosticsArgs,

    /// A transport directory or an extracted branch.
    #[arg(long)]
    pub branch_location: PathBuf,

    /// Verify every declared blob (sha256 + sizes) instead of only summarising.
    #[arg(long)]
    pub verify: bool,

    /// Verify a *restored project* against the manifest's per-file digests: every file,
    /// symlink target and executable bit of `.pixi/envs/<name>`, plus the fingerprint
    /// marker. Implies --verify (an oracle is only as good as its own bytes). Writes nothing.
    #[arg(long, value_name = "PROJECT")]
    pub verify_restored: Option<PathBuf>,

    /// The restore work directory, when the restore used a non-default one. The
    /// restored-tree check neutralises the scratch paths a restore embeds; with a custom
    /// `--work-dir` it needs the same one named here.
    #[arg(long, requires = "verify_restored")]
    pub work_dir: Option<PathBuf>,

    /// Limit verification to these envs.
    #[arg(long, value_delimiter = ',')]
    pub envs: Vec<String>,

    /// Enforce reviewed transport size budgets from a sandbox config before reporting success.
    #[arg(long)]
    pub budget_config: Option<PathBuf>,

    /// Exit non-zero when a host requirement the transport declares is *missing* on this host.
    /// Without it, host findings are reported and never change the exit code: the generated
    /// publisher runs `doctor` on a CI runner that is not where the workload runs. `unknown`
    /// results — a host without the queries to answer, or a transport for another OS family —
    /// never fail either way; enforcement must not punish a machine it cannot inspect.
    #[arg(long)]
    pub require_host_requirements: bool,

    /// Machine-readable output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Project root where generated files are written.
    #[arg(long, default_value = ".")]
    pub project_root: PathBuf,

    /// GitHub Actions workflow path, relative to the project root.
    #[arg(long, default_value = ".github/workflows/publish-sandbox.yml")]
    pub github_workflow_path: PathBuf,

    /// Lockfile-refresh workflow path, relative to the project root. The bot it generates
    /// solves on the connected side for manifests edited where the network is not.
    #[arg(long, default_value = ".github/workflows/relock.yml")]
    pub relock_workflow_path: PathBuf,

    /// Workflow the relock bot dispatches after pushing a lock commit, relative to
    /// .github/workflows/. A `GITHUB_TOKEN` push triggers nothing, so this dispatch is the only
    /// verdict the lock commit gets.
    #[arg(long, default_value = "ci.yml")]
    pub relock_ci_workflow: String,

    /// Airlock launcher path, relative to the project root. Defaults to restore.sh on Unix and
    /// restore.ps1 on Windows.
    #[arg(long)]
    pub script_path: Option<PathBuf>,

    /// Reviewed sandbox plan path. Without this override, init prefers pixi-sandbox.toml and
    /// falls back to the legacy .pixi-sandbox.toml when it already exists.
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Default local sandbox branch archived by the launcher.
    #[arg(long, default_value = "sandbox/developer-linux-64")]
    pub branch: String,

    /// Replace a user-owned file at a selected generated path. Files carrying pixi-sandbox's
    /// generation marker are safely regenerated without this flag.
    #[arg(long, conflicts_with = "check")]
    pub force: bool,

    /// Render every owned file fresh and compare it to what is on disk; write nothing. Exits
    /// non-zero naming each drifted file (remedy: `pixi-sandbox init`) and each foreign-owned
    /// file separately (remedy: `--force`); a clean tree exits 0 (task-47 AC#4 / decision D16).
    #[arg(long)]
    pub check: bool,
}

#[derive(Debug, Args)]
pub struct PlanArgs {
    /// Project declaration listing the environment bundles and native platforms to publish.
    #[arg(long, default_value = ".pixi-sandbox.toml")]
    pub config: PathBuf,

    /// Emit a compact GitHub Actions matrix object on stdout.
    #[arg(long)]
    pub json: bool,

    /// Plan one ad-hoc target instead of the reviewed bundles, for a manual dispatch. Passing
    /// this switches off config mode; the runner label and helper-pin coverage are still
    /// resolved and validated exactly as they are for a reviewed bundle.
    #[arg(long, value_name = "NAME", value_delimiter = ',')]
    pub envs: Option<Vec<String>>,

    /// Platform for the ad-hoc target. Only valid together with `--envs`.
    #[arg(long, requires = "envs")]
    pub platform: Option<String>,

    /// Bundle name for the ad-hoc target, which becomes part of the branch name.
    #[arg(long, requires = "envs", default_value = "custom")]
    pub bundle: String,

    /// Branch prefix for the ad-hoc target.
    #[arg(long, requires = "envs", default_value = "sandbox")]
    pub branch_prefix: String,

    /// Cargo-vendor policy for the ad-hoc target. Value-taking (`--cargo-vendor false`) as
    /// well as bare (`--cargo-vendor`, = true), because a `default_value_t = true` bool is
    /// otherwise a `SetTrue` flag that can never be false — the airlock workflow's
    /// `cargo-vendor: false` input was unreachable through this CLI until task-36 fixed it.
    #[arg(
        long,
        requires = "envs",
        default_value_t = true,
        num_args = 0..=1,
        default_missing_value = "true"
    )]
    pub cargo_vendor: bool,
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
    Update(ToolsUpdateArgs),
}

#[derive(Debug, Args)]
pub struct ToolsUpdateArgs {
    /// Update this file in place. Without it the candidate catalogue is printed, so it can be
    /// redirected into `crates/pixi-sandbox-core/assets/tools.lock.json` and reviewed as a diff.
    #[arg(long)]
    pub tools_lock: Option<PathBuf>,

    /// Report whether anything is newer and exit non-zero if so, without downloading or writing.
    /// This is the form a scheduled workflow keys on.
    #[arg(long)]
    pub check: bool,

    /// Limit the run to these tool names (repeatable). Everything else is reported as skipped.
    #[arg(long = "tool", value_name = "NAME")]
    pub tool: Vec<String>,
}

pub fn run() -> Result<()> {
    use crate::commands;

    // A root bootstrap binary is the first thing an operator sees after extracting an orphan
    // branch. Keep a bare invocation read-only: it verifies the branch and prints the exact
    // restore command, rather than guessing an output directory and writing into the checkout.
    if std::env::args_os().nth(1).is_none() {
        if let Some(branch) = inferred_branch_location() {
            commands::doctor(&DoctorArgs {
                diagnostics: DiagnosticsArgs::default(),
                branch_location: branch.clone(),
                verify: true,
                verify_restored: None,
                work_dir: None,
                envs: Vec::new(),
                budget_config: None,
                require_host_requirements: false,
                json: false,
            })?;
            let manifest = pixi_sandbox_core::manifest::Manifest::load(
                &pixi_sandbox_core::manifest::Manifest::path_in(&branch),
            )?;
            let executable = pixi_sandbox_core::tools_lock::executable_filename(
                "pixi-sandbox",
                &manifest.platform,
            );
            let binary = branch
                .join(".pixi-sandbox")
                .join("tools")
                .join(&manifest.platform)
                .join(executable);
            println!(
                "next: \"{}\" restore --branch-location \"{}\" --output-path <project> --force",
                binary.display(),
                branch.display()
            );
            return Ok(());
        }

        let mut command = Cli::command();
        command.print_help()?;
        return Ok(());
    }

    let cli = Cli::parse();
    let diagnostics = diagnostics_for(&cli.command)?;
    let result = match cli.command {
        Command::Pack(args) => commands::pack(args),
        Command::Publish(args) => commands::publish(&args),
        Command::Restore(args) => commands::restore(&args),
        Command::Unpack(args) => commands::unpack(&args),
        Command::Doctor(args) => commands::doctor(&args),
        Command::Init(args) => commands::init(&args),
        Command::Plan(args) => commands::plan(&args),
        Command::Tools(args) => commands::tools(args),
        Command::SelfUpdate(args) => commands::self_update(args),
        Command::FetchRelease(args) => commands::fetch_release(&args),
        Command::Pipeline(args) => commands::pipeline(args),
        Command::Upgrade(args) => commands::upgrade(args),
    };

    match result {
        Ok(()) => {
            if let Some(session) = diagnostics {
                session.finish_success()?;
            }
            Ok(())
        }
        Err(error) => {
            if let Some(session) = diagnostics {
                session.finish_failure(&error)?;
            }
            Err(error)
        }
    }
}

fn diagnostics_for(command: &Command) -> Result<Option<crate::diagnostics::Session>> {
    let (name, options, context) = match command {
        Command::Pack(args) => (
            "pack",
            &args.diagnostics,
            format!(
                "repo={} output={} platform={} environments={} cargo-vendor={}",
                args.repo_root.display(),
                args.output_dir.display(),
                args.platform,
                args.envs.len(),
                args.cargo_vendor
            ),
        ),
        Command::Doctor(args) => (
            "doctor",
            &args.diagnostics,
            format!(
                "branch={} verify={} verify-restored={} environments={} json={}",
                args.branch_location.display(),
                args.verify,
                args.verify_restored.is_some(),
                args.envs.len(),
                args.json
            ),
        ),
        Command::Publish(args) => (
            "publish",
            &args.diagnostics,
            format!(
                "input={} branch={} remote={} keep={} dry-run={}",
                args.input_dir.display(),
                args.branch_name,
                if args.remote.is_some() {
                    "<configured>"
                } else {
                    "<default>"
                },
                args.keep,
                args.dry_run
            ),
        ),
        _ => return Ok(None),
    };
    crate::diagnostics::Session::start(name, options, &context).map(Some)
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
