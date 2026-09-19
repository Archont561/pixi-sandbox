use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "pixi-sandbox",
    about = "Pack, publish and reconstruct pixi workspaces (KB spec, D21)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Pack pixi environments (and optionally the vendored crate graph) into a kit
    Pack,
    /// Print the tiered workspace inventory
    Inventory,
    /// Reconstruct a working pixi workspace from a kit artifact
    Reconstruct,
}
