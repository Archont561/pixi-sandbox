mod cli;
mod render;

use clap::Parser;
use cli::Command;

fn main() -> std::process::ExitCode {
    match cli::Cli::parse().command {
        Command::Pack => render::ok("pack: not yet implemented"),
        Command::Inventory => render::ok("inventory: not yet implemented"),
        Command::Reconstruct => render::ok("reconstruct: not yet implemented"),
    }
    std::process::ExitCode::SUCCESS
}
