//! `pixi-sandbox` — pack pixi environments, publish them as an orphan branch, restore them
//! on a machine with no network.
//!
//! pixi discovers `pixi-<command>` binaries on `PATH`, so every verb below also works as
//! `pixi sandbox <verb>` without patching pixi (see `.knowledge/design.md` §5).

mod cli;
mod commands;

use std::process::ExitCode;

fn main() -> ExitCode {
    commands::install_tracing();
    match cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // `{:?}` keeps anyhow's context chain — the "Caused by:" block is the useful part
            // when something fails on a machine nobody can log into.
            eprintln!("Error: {err:?}");
            ExitCode::FAILURE
        }
    }
}
