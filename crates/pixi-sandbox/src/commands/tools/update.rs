//! `tools update` — refresh the helper-tool pins from the same upstream the pins name.
//!
//! The catalogue is reviewed data (D4), so this command never *installs* anything and never
//! decides that a newer build is better. It produces a candidate lock for a human to diff.
//! What it does insist on is that every hash it writes is earned:
//!
//! * **Cross-check against the upstream manifest when the project publishes one.** pixi ships a
//!   `sha256.sum` per release, so a corrupted download is caught against a second, independently
//!   published source. Measured: Quantco/pixi-pack publishes **no** checksum manifest at all, so
//!   there the hash is trust-on-first-use and the lock records that fact in `note` rather than
//!   pretending to a guarantee it does not have.
//! * **Verify linkage from the bytes, not from the previous lock.** A pin is allowed to claim
//!   `static`; this command reads the ELF and refuses if the asset is not. That is the check that
//!   catches a `~/.pixi/bin` trampoline (a 766 KiB dynamic shim that works on the build machine
//!   and dies in the airlock) at pin time instead of at restore time.
//!
//! The write is atomic and happens only after every asset in the catalogue has been checked, so a
//! failure anywhere leaves the operator's lock byte-for-byte unchanged (AC#2).

use crate::cli::ToolsUpdateArgs;
use crate::commands::support;
use anyhow::{Context, Result};
use pixi_sandbox::release::GitHubReleaseSource;
use pixi_sandbox::tools_update::{refresh, render, report_check, write_atomic};
use pixi_sandbox_core::tools_lock::ToolsLock;

/// `generated_at` and the `note` on every pin we touch are the audit trail: the next reader must
/// be able to tell a hash that was cross-checked from one that was merely observed.
pub fn run(args: &ToolsUpdateArgs) -> Result<()> {
    let target = match &args.tools_lock {
        Some(path) => {
            let path = support::absolute(path)?;
            let lock = ToolsLock::load(&path)
                .with_context(|| format!("reading tool pins {}", path.display()))?;
            (lock, Some(path))
        }
        None => (
            ToolsLock::embedded().context("loading embedded helper-tool pins")?,
            None,
        ),
    };
    let (lock, path) = target;

    let source = GitHubReleaseSource::new();
    let (updated, report) = refresh(&lock, &source, &args.tool, &support::now_rfc3339())?;

    if args.check {
        return report_check(&report);
    }

    let rendered = render(&updated)?;
    match path {
        // No `--tools-lock`: print the candidate so a reviewer can redirect it into the asset
        // file and diff it. The embedded copy is compiled into the binary, so there is nothing
        // beside the executable to write — and nothing to corrupt by accident.
        None => {
            print!("{rendered}");
            eprintln!(
                "\n{} tool(s) checked. Redirect this into crates/pixi-sandbox-core/assets/tools.lock.json and review the diff.",
                report.checked
            );
            Ok(())
        }
        Some(path) => write_atomic(&path, &rendered).context("writing the updated tools lock"),
    }
}
