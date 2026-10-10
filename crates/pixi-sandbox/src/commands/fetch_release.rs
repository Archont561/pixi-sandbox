//! `fetch-release` — the CLI half: resolve the ambient inputs once, then report.
//!
//! Every environment read lives here (the host triple, the compiled-in version), so
//! `self_update::fetch_release` stays a pure function of its `Request` and the tests never
//! reach a developer's real binary or a socket (D10). The command is deliberately thin:
//! unlike `self-update` it has no `--check` and no up-to-date shortcut — the bootstrap
//! download always fetches (see the function's doc comment).

use anyhow::Result;

use crate::cli::FetchReleaseArgs;
use pixi_sandbox::release::GitHubReleaseSource;
use pixi_sandbox::self_update::{self, Request, replace::ReplaceStrategy};

/// The version this binary was built as — informational only: a fetch never compares it,
/// because the destination is not this binary.
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run(args: FetchReleaseArgs) -> Result<()> {
    let (host_os, host_arch) = self_update::assets::current_host();
    let source = GitHubReleaseSource::new();

    let request = Request {
        repo: &args.repo,
        requested_version: Some(&args.version),
        destination: &args.dest,
        current_version: CURRENT_VERSION,
        host_os,
        host_arch,
        strategy: ReplaceStrategy::current(),
    };

    let applied = self_update::fetch_release(&source, &request)?;
    for line in applied.fetch_report() {
        println!("{line}");
    }
    Ok(())
}
