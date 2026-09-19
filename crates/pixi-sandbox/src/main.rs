mod cli;
mod render;

use clap::Parser;
use cli::Command;

fn main() -> std::process::ExitCode {
    let cli = cli::Cli::parse();
    let cwd = cli.cwd.as_deref();
    let manifest_path = cli.manifest_path.as_deref();

    let result = match cli.command {
        Command::Pack(args) => {
            let mut envs = args.envs;
            envs.extend(args.env);
            sandbox_core::pack::run(
                envs,
                args.target,
                args.out.as_deref(),
                args.all_envs,
                args.dry_run,
                cwd,
                manifest_path,
            )
        }
        Command::Inventory(args) => {
            sandbox_core::inventory::run(&args.tier, args.json, cwd, manifest_path)
        }
        Command::Reconstruct(args) => {
            let mode = match args.mode {
                cli::ReconstructMode::Auto => sandbox_core::reconstruct::Mode::Auto,
                cli::ReconstructMode::Cache => sandbox_core::reconstruct::Mode::Cache,
                cli::ReconstructMode::FileChannel => sandbox_core::reconstruct::Mode::FileChannel,
                cli::ReconstructMode::Unpack => sandbox_core::reconstruct::Mode::Unpack,
                cli::ReconstructMode::EnvYml => sandbox_core::reconstruct::Mode::EnvYml,
                cli::ReconstructMode::Tar => sandbox_core::reconstruct::Mode::Tar,
            };
            let pack_format = match args.pack_format {
                cli::PackFormat::Auto => sandbox_core::reconstruct::PackFormat::Auto,
                cli::PackFormat::Pack => sandbox_core::reconstruct::PackFormat::Pack,
                cli::PackFormat::Raw => sandbox_core::reconstruct::PackFormat::Raw,
            };
            let reconstruct_args = sandbox_core::reconstruct::ReconstructArgs {
                from: std::path::PathBuf::from(args.from),
                envs: args.env,
                workspace: args.workspace,
                with_vendor: args.with_vendor,
                mode,
                pack_format,
                print_rung: args.print_rung,
                promote_path: args.promote_path,
                self_test: args.self_test,
            };
            sandbox_core::reconstruct::run(reconstruct_args)
        }
        Command::Plan(args) => sandbox_core::plan::run(
            args.env,
            args.target,
            &args.components,
            args.dry_run,
            args.json,
            cwd,
            manifest_path,
        ),
        Command::Publish(args) => {
            let no_push = args.no_push || !args.push;
            sandbox_core::publish::run(&args.branch, no_push, cwd)
        }
        Command::Verify(args) => sandbox_core::verify::run(&args.dir),
    };

    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            let code = e.code();
            std::process::ExitCode::from(code)
        }
    }
}
