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

/// The lines a run prints, built as data so they are testable.
///
/// Formatting lives here rather than in `commands/` for a reason the coverage report made
/// concrete: `commands/` is private to the binary target, so its only test route is spawning
/// the binary — and a separate process earns no coverage credit and cannot assert on
/// structure, only on substrings. Keeping the wording here means the command module stays
/// thin wiring (resolve ambient inputs, print these lines).
impl Plan {
    /// The header every run prints, mutating or not.
    pub fn report(&self) -> Vec<String> {
        vec![
            format!("self-update  {}", self.destination.display()),
            format!("  current  {}", self.current_version),
            format!(
                "  target   {} ({})",
                self.resolved.version,
                self.resolved.selection.describe()
            ),
            format!("  asset    {}", self.asset),
        ]
    }

    /// The verdict `--check` prints. Writes nothing by construction: it is a pure function.
    pub fn check_verdict(&self) -> Vec<String> {
        let verdict = if self.is_up_to_date() {
            "  verdict  up to date — nothing to do".to_string()
        } else {
            format!(
                "  verdict  update available: {} -> {}",
                self.current_version, self.resolved.version
            )
        };
        vec![verdict, "  (--check wrote nothing)".to_string()]
    }

    /// The verdict a mutating run prints when there is nothing to do.
    pub fn up_to_date_verdict(&self) -> String {
        format!("  verdict  already {}; nothing to do", self.current_version)
    }
}

impl Applied {
    /// What a completed update prints.
    pub fn report(&self) -> Vec<String> {
        let mut lines = vec![format!("  sha256   {}", self.digest)];
        for swept in &self.replacement.swept {
            lines.push(format!("  swept    {}", swept.display()));
        }
        if let Some(displaced) = &self.replacement.displaced {
            // Only worth saying when the OS actually kept it: on Unix it never exists, and on
            // Windows it is gone unless the old image is still mapped.
            if displaced.exists() {
                lines.push(format!(
                    "  note     the previous binary is still mapped; {} is removed on the next \
                     update",
                    displaced.display()
                ));
            }
        }
        lines.push(format!(
            "updated  {} -> {}",
            self.plan.current_version, self.plan.resolved.version
        ));
        lines
    }
}

/// The whole command, as a function of its inputs: resolve, report, refuse, and either stop
/// (`--check`) or replace. Returns the lines to print.
///
/// This lives here rather than in `commands/` so the decision flow — which verdict follows
/// which state — is testable. `commands/self_update.rs` is left with the part that genuinely
/// cannot be tested offline: reading the running executable's path, the host triple and the
/// compiled-in version, then printing.
pub fn run(source: &dyn ReleaseSource, request: &Request<'_>, check: bool) -> Result<Vec<String>> {
    let plan = plan(source, request)?;
    let mut lines = plan.report();

    // A refusal is reported identically in both modes: `--check` has to be able to tell CI
    // that this destination would never have been updatable, before it tries.
    if let Some(refusal) = plan.ownership.refusal(&plan.destination) {
        bail!("{}\n{refusal}", lines.join("\n"));
    }

    if check {
        lines.extend(plan.check_verdict());
        return Ok(lines);
    }
    if plan.is_up_to_date() {
        lines.push(plan.up_to_date_verdict());
        return Ok(lines);
    }

    lines.extend(apply(source, request, plan)?.report());
    Ok(lines)
}
