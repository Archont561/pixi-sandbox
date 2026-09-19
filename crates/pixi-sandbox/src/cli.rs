use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "pixi-sandbox",
    about = "Pack, publish and reconstruct pixi workspaces (KB spec, D21)",
    version,
    arg_required_else_help = true
)]
pub struct Cli {
    #[arg(long, global = true, help = "Start discovery here")]
    pub cwd: Option<std::path::PathBuf>,

    #[arg(long, global = true, help = "Explicit manifest path")]
    pub manifest_path: Option<std::path::PathBuf>,

    #[arg(long, global = true, help = "Explicit config path")]
    pub config: Option<std::path::PathBuf>,

    #[arg(short = 'v', long, action = clap::ArgAction::Count, global = true, help = "Verbosity")]
    pub verbose: u8,

    #[arg(short = 'q', long, action = clap::ArgAction::Count, global = true, help = "Quiet")]
    pub quiet: u8,

    #[arg(long, global = true, help = "Machine-readable JSON output")]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Pack pixi environments (and optionally the vendored crate graph) into a kit
    Pack(PackArgs),
    /// Print the tiered workspace inventory
    Inventory(InventoryArgs),
    /// Reconstruct a working pixi workspace from a kit artifact
    Reconstruct(ReconstructArgs),
    /// Plan what would be packed (dry-run, JSON)
    Plan(PlanArgs),
    /// Publish kit to dist branch (git plumbing)
    Publish(PublishArgs),
    /// Verify a kit directory
    Verify(VerifyArgs),
}

#[derive(Debug, Parser)]
pub struct PackArgs {
    #[arg(value_name = "ENV", help = "Environments to pack (default: default)")]
    pub envs: Vec<String>,

    #[arg(long, short = 'e', help = "Environments to pack (alternative)")]
    pub env: Vec<String>,

    #[arg(long, help = "Target conda platforms", value_name = "PLATFORM")]
    pub target: Vec<String>,

    #[arg(long, help = "Output directory for packs")]
    pub out: Option<std::path::PathBuf>,

    #[arg(long, help = "Pack all detected environments")]
    pub all_envs: bool,

    #[arg(long, help = "Dry run, print plan only")]
    pub dry_run: bool,
}

#[derive(Debug, Parser)]
pub struct InventoryArgs {
    #[arg(long, help = "Tier cap: L0|L1|L2|L3|auto", default_value = "auto")]
    pub tier: String,

    #[arg(long, help = "Output as JSON")]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct ReconstructArgs {
    #[arg(long, help = "Kit directory or git URL", value_name = "DIR")]
    pub from: String,

    #[arg(
        long,
        short = 'e',
        help = "Environments to reconstruct",
        value_name = "ENV"
    )]
    pub env: Vec<String>,

    #[arg(
        long,
        help = "Workspace directory to materialise into",
        default_value = "."
    )]
    pub workspace: std::path::PathBuf,

    #[arg(long, help = "Include vendored crate graph")]
    pub with_vendor: bool,

    #[arg(long, help = "Reconstruction mode", value_enum, default_value_t = ReconstructMode::Auto)]
    pub mode: ReconstructMode,

    #[arg(long, help = "Pack format detection", value_enum, default_value_t = PackFormat::Auto)]
    pub pack_format: PackFormat,

    #[arg(long, help = "Print rung that was used")]
    pub print_rung: bool,

    #[arg(long, help = "Promote PATH to rc files")]
    pub promote_path: bool,

    #[arg(long, help = "Self-test mode (no SHA256SUMS required)")]
    pub self_test: bool,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ReconstructMode {
    Auto,
    Cache,
    #[value(name = "file-channel")]
    FileChannel,
    Unpack,
    #[value(name = "env-yml")]
    EnvYml,
    Tar,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum PackFormat {
    Auto,
    Pack,
    Raw,
}

#[derive(Debug, Parser)]
pub struct PlanArgs {
    #[arg(long, short = 'e', help = "Environments")]
    pub env: Vec<String>,

    #[arg(long, help = "Target platforms")]
    pub target: Vec<String>,

    #[arg(long, help = "Components: auto or list", default_value = "auto")]
    pub components: String,

    #[arg(long, help = "Dry run")]
    pub dry_run: bool,

    #[arg(long, help = "JSON output")]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct PublishArgs {
    #[arg(
        long,
        help = "Branch to publish onto",
        default_value = "pixi-sandbox-dist"
    )]
    pub branch: String,

    #[arg(long, help = "Dry run, stage only")]
    pub no_push: bool,

    #[arg(
        long,
        help = "Push (default true, set false to stage only)",
        default_value_t = true
    )]
    pub push: bool,
}

#[derive(Debug, Parser)]
pub struct VerifyArgs {
    #[arg(value_name = "DIR", default_value = ".")]
    pub dir: std::path::PathBuf,
}
