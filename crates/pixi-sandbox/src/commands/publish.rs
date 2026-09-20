//! `publish` — put the transport on an orphan branch. See `.knowledge/design.md` §2.
//!
//! Implemented on top of `pixi-sandbox-git`, which is a trait: this command only ever names
//! [`GitProtocol`], so it runs unchanged against the real `git` or the in-memory mock that
//! the tests use. `--dry-run` swaps in [`ShellGit::preview`], i.e. the very same code path
//! with a runner that records commands and executes nothing.
//!
//! Remaining work for the port (the rest is done):
//!   1. `--keep N` rotation: rebuild the branch's history with N snapshots instead of
//!      replacing it. Deferred until there is a retention policy — force-push + `gc` does
//!      *not* shrink an orphan repo (measured, see §2).

use crate::cli::PublishArgs;
use crate::commands::support;
use anyhow::{Context, Result};
use pixi_sandbox_core::manifest::Manifest;
use pixi_sandbox_git::{GitProtocol, ShellGit, Snapshot};
use std::path::Path;

pub fn run(args: PublishArgs) -> Result<()> {
    // ShellGit changes cwd to the work tree while creating its temporary index. Keep the
    // transport absolute so GIT_DIR/GIT_INDEX_FILE never become relative to that new cwd.
    let input = support::existing_dir(&args.input_dir, "--input-dir")?;
    let manifest_path = Manifest::path_in(&input);
    let manifest = Manifest::load(&manifest_path)
        .with_context(|| format!("loading {}", manifest_path.display()))?;
    if args.keep > 0 {
        // Deliberately not silently ignored: an operator asking for retention must hear that
        // this build replaces history.
        return Err(super::not_yet("publish --keep", "2"));
    }

    let remote = args.remote.as_deref().unwrap_or("origin").to_string();
    let message = commit_message(&manifest);

    let git: Box<dyn GitProtocol> = if args.dry_run {
        Box::new(ShellGit::preview())
    } else {
        Box::new(ShellGit::new())
    };
    let published = git
        .publish(&Snapshot {
            dir: &input,
            branch: &args.branch_name,
            remote: &remote,
            message: &message,
        })
        .with_context(|| format!("publishing {} to {remote}", input.display()))?;

    if args.dry_run {
        println!(
            "would publish {} to {remote}:{}",
            input.display(),
            args.branch_name
        );
        println!("  commit: {message}");
        for command in &published.commands {
            println!("  $ {command}");
        }
        println!(
            "  {} file(s), {} MiB — nothing was written (--dry-run)",
            published.files,
            mib(published.bytes)
        );
        return Ok(());
    }

    println!(
        "published {} file(s), {} MiB to {remote}:{}",
        published.files,
        mib(published.bytes),
        args.branch_name
    );
    println!(
        "  commit {}",
        &published.commit[..published.commit.len().min(12)]
    );
    match git.remote_size(&remote, &args.branch_name) {
        // A local remote (or one mounted as a path) can answer this; a URL cannot.
        Ok(Some(bytes)) => println!("  branch stores {} MiB", mib(bytes)),
        Ok(None) => println!("  branch size: not knowable without a local object store"),
        Err(err) => println!("  branch size: {err}"),
    }
    println!(
        "  airlock: git fetch {remote} {}:{}",
        args.branch_name, args.branch_name
    );
    println!("  then:    pixi-sandbox restore --branch-location <dir> --output-path .");
    Ok(())
}

/// The manifest is the payload's identity, so the commit message is a summary of it — the
/// branch reads correctly in `git log` without any tooling.
fn commit_message(manifest: &Manifest) -> String {
    let envs: Vec<&str> = manifest.envs.keys().map(String::as_str).collect();
    let vendored = if manifest.vendor.is_some() {
        " · vendored crates"
    } else {
        ""
    };
    format!(
        "sandbox snapshot {} ({} · {}{vendored} · schema {})",
        manifest.created_at,
        envs.join("+"),
        manifest.platform,
        manifest.schema
    )
}

fn mib(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / (1024.0 * 1024.0))
}

/// Kept next to the command because it is part of the published contract: the branch name is
/// what the airlock types, and the path helpers keep `publish` and `restore` in agreement.
#[allow(dead_code)]
fn manifest_path(input: &Path) -> std::path::PathBuf {
    Manifest::path_in(input)
}
