//! `self-update` — replace a standalone pixi-sandbox binary with a published release.
//!
//! The trust boundary, in the order it is enforced (ADR-26 / ADR-16):
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
//!
//! Transport (TASK-124 / ADR-27): release lookup via `self_update` crate's
//! `backends::github::ReleaseList` (rate-limit aware), download via `Download`, and
//! checksum parsing via `Checksum::from_sums_file`. Ownership ladder and replace dance stay
//! hand-rolled; `update()` is never called.

pub mod assets;
pub mod ownership;
pub mod replace;

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

use crate::release::ReleaseSource;
use ownership::Ownership;
use replace::{ReplaceStrategy, Replacement};

/// The release that publishes pixi-sandbox's standalone binaries.
///
/// Re-exported from `release::PIXI_SANDBOX_REPO` so this default and the generated
/// workflow's release-download URL are the same constant, not two literals that can drift
/// apart (issue #80).
pub const DEFAULT_REPO: &str = crate::release::PIXI_SANDBOX_REPO;

/// The checksum manifest `xtask release-checksums` writes for every release.
pub const SUMS_ASSET: &str = "SHA256SUMS";

/// Everything the command needs, with every ambient input resolved by the caller.
///
/// No field here is read from the environment inside this module — the same rule `user_tools`
/// follows (ADR-10): tests pass tempdirs and explicit hosts, so no code path can reach a
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
    #[must_use]
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

/// The release this run will install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The tag as GitHub spells it, e.g. `v0.4.4`.
    pub tag: String,
    /// The bare version, e.g. `0.4.4`.
    pub version: String,
    /// How the tag was chosen.
    pub selection: Selection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// No `--version`: whatever the project currently calls latest.
    Latest,
    /// An exact version the caller named.
    Pinned,
}

impl Selection {
    #[must_use]
    pub fn describe(self) -> &'static str {
        match self {
            Selection::Latest => "latest",
            Selection::Pinned => "pinned",
        }
    }
}

/// Accept `X.Y.Z` or `vX.Y.Z` and return the bare `X.Y.Z`.
///
/// Strict on purpose: a loose parser turns a typo into a 404 from the download step, where the
/// error no longer says which input was wrong.
///
/// # Errors
///
/// Returns an error naming the malformed input when it is not `X.Y.Z` or `vX.Y.Z`.
pub fn normalise_version(input: &str) -> Result<String> {
    let bare = input.strip_prefix('v').unwrap_or(input);
    let parts: Vec<&str> = bare.split('.').collect();
    let valid = parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part.len() == 1 || !part.starts_with('0'))
        });
    if !valid {
        bail!("{input:?} is not an exact version; expected X.Y.Z (for example 0.4.4)");
    }
    Ok(bare.to_string())
}

/// Resolve `requested` (or latest) against `source`.
///
/// # Errors
///
/// Returns an error if the requested version does not parse or does not exist upstream.
pub fn resolve(
    source: &dyn ReleaseSource,
    repo: &str,
    requested: Option<&str>,
) -> Result<Resolved> {
    if let Some(requested) = requested {
        let version = normalise_version(requested)?;
        Ok(Resolved {
            tag: format!("v{version}"),
            version,
            selection: Selection::Pinned,
        })
    } else {
        let tag = source
            .latest_tag(repo)
            .with_context(|| format!("resolving the latest pixi-sandbox release of {repo}"))?;
        let version = normalise_version(&tag).with_context(|| {
            format!("{repo}'s latest release tag {tag:?} is not a vX.Y.Z release")
        })?;
        Ok(Resolved {
            tag,
            version,
            selection: Selection::Latest,
        })
    }
}

/// Steps 1 and 2: resolve the release and classify the destination. Writes nothing, downloads
/// nothing but the release metadata.
///
/// # Errors
///
/// Returns an error if the host has no release asset or the requested version cannot be
/// resolved.
pub fn plan(source: &dyn ReleaseSource, request: &Request<'_>) -> Result<Plan> {
    let asset = assets::asset_for(request.host_os, request.host_arch)?;
    let resolved = resolve(source, request.repo, request.requested_version)?;
    let ownership = ownership::classify(request.destination);
    Ok(Plan {
        resolved,
        asset: asset.to_string(),
        destination: request.destination.to_path_buf(),
        current_version: request.current_version.to_string(),
        ownership,
    })
}

/// Verify `binary_bytes` against `sums_body` for `asset` using the `self_update` crate's
/// SHA256SUMS parser (ADR-27). Returns the actual digest on success.
fn verify_with_crate(asset: &str, sums_body: &str, binary_bytes: &[u8]) -> Result<String> {
    let checksum = ::self_update::Checksum::from_sums_file(sums_body, asset)
        .map_err(|e| anyhow::anyhow!(e))
        .with_context(|| format!("parsing {SUMS_ASSET} for {asset}"))?;

    let expected = match &checksum {
        ::self_update::Checksum::Sha256(hex) | ::self_update::Checksum::Sha512(hex) => hex.clone(),
        _ => unreachable!("Checksum is non_exhaustive but only Sha256/Sha512 exist"),
    };

    let actual = match &checksum {
        ::self_update::Checksum::Sha256(_) => pixi_sandbox_core::shard::sha256_bytes(binary_bytes),
        ::self_update::Checksum::Sha512(_) => {
            use sha2::{Digest, Sha512};
            let mut hasher = Sha512::new();
            hasher.update(binary_bytes);
            let result = hasher.finalize();
            let mut hex = String::with_capacity(result.len() * 2);
            for byte in result {
                use std::fmt::Write as _;
                let _ = write!(&mut hex, "{byte:02x}");
            }
            hex
        }
        _ => unreachable!("Checksum is non_exhaustive but only Sha256/Sha512 exist"),
    };

    if !actual.eq_ignore_ascii_case(expected.trim()) {
        bail!(
            "checksum mismatch for {asset}\n  expected {expected}\n  actual   {actual}\n\
             the downloaded bytes are not the published release; nothing was written"
        );
    }
    Ok(actual)
}

/// Steps 3 to 5: download, verify, replace. Refuses a non-standalone destination first.
///
/// # Errors
///
/// Returns an error if the destination is not standalone, the download fails, the checksum
/// does not match, or the replacement cannot be written.
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

    let binary = source
        .asset(request.repo, tag, &plan.asset)
        .with_context(|| format!("downloading {} for {tag}", plan.asset))?;

    // Verify before write (invariant 1). The digest is computed from what arrived, never
    // carried over from the manifest that is supposed to be judging it.
    // Uses the self_update crate's parser (Checksum::from_sums_file) per ADR-27.
    let sums_str = String::from_utf8_lossy(&sums.bytes);
    let digest = verify_with_crate(&plan.asset, &sums_str, &binary.bytes)
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

/// Fetch the pinned standalone release asset for this host and install it at
/// `request.destination`, checksum-verified.
///
/// This is the bootstrap download the generated workflows run before any pixi-sandbox
/// binary of their own exists on the runner (TASK-76).
///
/// Unlike [`run`] there is no up-to-date shortcut: the destination is not the running
/// binary, so the running binary's version says nothing about what the destination holds.
/// Every call resolves, downloads, verifies and installs (ADR-26 / ADR-16 — the bootstrap
/// is the pinned, checksum-verified binary, always).
///
/// # Errors
///
/// Returns an error if the destination is not standalone, the download fails, the checksum
/// does not match, or the installation cannot be written.
pub fn fetch_release(source: &dyn ReleaseSource, request: &Request<'_>) -> Result<Applied> {
    let plan = plan(source, request)?;
    if let Some(refusal) = plan.ownership.refusal(&plan.destination) {
        bail!("{refusal}");
    }
    apply(source, request, plan)
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn up_to_date_verdict(&self) -> String {
        format!("  verdict  already {}; nothing to do", self.current_version)
    }
}

impl Applied {
    /// What a completed update prints.
    #[must_use]
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

    /// What a `fetch-release` run prints: the verified bytes now at the destination. The
    /// wording deliberately differs from [`Applied::report`] — nothing was updated, the
    /// destination was provisioned.
    #[must_use]
    pub fn fetch_report(&self) -> Vec<String> {
        vec![
            format!("fetch-release  {}", self.plan.destination.display()),
            format!(
                "  version  {} ({})",
                self.plan.resolved.version, self.plan.resolved.tag
            ),
            format!("  asset    {}", self.plan.asset),
            format!("  sha256   {}", self.digest),
        ]
    }
}

/// The whole command, as a function of its inputs: resolve, report, refuse, and either stop
/// (`--check`) or replace. Returns the lines to print.
///
/// This lives here rather than in `commands/` so the decision flow — which verdict follows
/// which state — is testable. `commands/self_update.rs` is left with the part that genuinely
/// cannot be tested offline: reading the running executable's path, the host triple and the
/// compiled-in version, then printing.
///
/// # Errors
///
/// Returns an error if the release cannot be resolved or the destination is not standalone;
/// an up-to-date destination is a report, not an error.
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
