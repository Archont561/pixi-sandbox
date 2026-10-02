//! `self-update` — replace a standalone pixi-sandbox binary with a published release.
//!
//! The trust boundary, in the order it is enforced (decision-4 / D16):
//!
//! 1. **Resolve** a tag — `latest` only as discovery for a reviewed upgrade, otherwise exact.
//! 2. **Classify the destination.** A Pixi-managed executable, a `pixi global` trampoline, a
//!    managed launcher or a manifest-owned transport tool is refused *before* anything is
//!    downloaded, so a refused run costs no bytes and changes nothing.
//! 3. **Download** the canonical asset for this host, plus the release's `SHA256SUMS`.
//! 4. **Verify** the asset against that file. A missing entry and a mismatch are distinct
//!    refusals, and neither reaches step 5.
//! 5. **Stage and replace**, atomically, beside the destination.
//!
//! Nothing between steps 3 and 5 ever executes the downloaded bytes, and `--check` stops after
//! step 2 — that is what makes it safe to run from CI against an arbitrary release.

pub mod assets;
pub mod checksums;
pub mod ownership;
pub mod replace;
pub mod resolver;

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

use crate::release::ReleaseSource;
use checksums::Checksums;
use ownership::Ownership;
use replace::{ReplaceStrategy, Replacement};
use resolver::Resolved;

/// The release that publishes pixi-sandbox's standalone binaries.
pub const DEFAULT_REPO: &str = "Archont561/pixi-sandbox";

/// The checksum manifest `xtask release-checksums` writes for every release.
pub const SUMS_ASSET: &str = "SHA256SUMS";

/// Everything the command needs, with every ambient input resolved by the caller.
///
/// No field here is read from the environment inside this module — the same rule `user_tools`
/// follows (D10): tests pass tempdirs and explicit hosts, so no code path can reach a
/// developer's real binary.
pub struct Request<'a> {
    pub repo: &'a str,
    pub requested_version: Option<&'a str>,
    pub destination: &'a Path,
    pub current_version: &'a str,
    pub host_os: &'a str,
    pub host_arch: &'a str,
    pub strategy: ReplaceStrategy,
}

/// What a run decided, before any bytes move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub resolved: Resolved,
    pub asset: String,
    pub destination: PathBuf,
    pub current_version: String,
    pub ownership: Ownership,
}

impl Plan {
    /// True when the destination already holds the version we would install.
    pub fn is_up_to_date(&self) -> bool {
        self.current_version == self.resolved.version
    }
}

/// What a completed update did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    pub plan: Plan,
    pub digest: String,
    pub replacement: Replacement,
}

/// Steps 1 and 2: resolve the release and classify the destination. Writes nothing, downloads
/// nothing but the release metadata.
pub fn plan(source: &dyn ReleaseSource, request: &Request<'_>) -> Result<Plan> {
    let asset = assets::asset_for(request.host_os, request.host_arch)?;
    let resolved = resolver::resolve(source, request.repo, request.requested_version)?;
    let ownership = ownership::classify(request.destination);
    Ok(Plan {
        resolved,
        asset: asset.to_string(),
        destination: request.destination.to_path_buf(),
        current_version: request.current_version.to_string(),
        ownership,
    })
}

/// Steps 3 to 5: download, verify, replace. Refuses a non-standalone destination first.
pub fn apply(source: &dyn ReleaseSource, request: &Request<'_>, plan: Plan) -> Result<Applied> {
    if !plan.ownership.is_standalone() {
        // The ladder's verdict is final and carries its own remedy; every refusal happens
        // before a single byte is downloaded.
        match plan.ownership.refusal(&plan.destination) {
            Some(refusal) => bail!("{refusal}"),
            None => bail!(
                "{} is not a standalone binary and cannot be self-updated",
                plan.destination.display()
            ),
        }
    }

    let tag = &plan.resolved.tag;
    let sums = source
        .asset(request.repo, tag, SUMS_ASSET)
        .with_context(|| format!("downloading {SUMS_ASSET} for {tag}"))?;
    let sums = Checksums::parse(&String::from_utf8_lossy(&sums.bytes));

    let binary = source
        .asset(request.repo, tag, &plan.asset)
        .with_context(|| format!("downloading {} for {tag}", plan.asset))?;

    // Verify before write (invariant 1). The digest is computed from what arrived, never
    // carried over from the manifest that is supposed to be judging it.
    let digest = sums
        .verify(&plan.asset, &binary.bytes)
        .with_context(|| format!("verifying {} against {SUMS_ASSET} for {tag}", plan.asset))?;

    let replacement = replace::install(
        request.strategy,
        &plan.destination,
        &binary.bytes,
        &plan.current_version,
    )?;

    Ok(Applied {
        plan,
        digest,
        replacement,
    })
}
