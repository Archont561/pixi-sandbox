//! Where release bytes come from, for every command that needs them.
//!
//! This is a trait for the same reason git goes through `GitProtocol` (ADR-9): the risk lives in
//! what a command *does* with downloaded bytes — verification, pin rewriting, replacing an
//! executable — so that logic has to be exercisable without a network and without GitHub.
//! `tools update` was the first consumer; `self-update` (ADR-26) is the second, and a
//! second near-identical trait would have meant two fakes drifting apart.
//!
//! The real HTTP implementation lives in [`crate::github`], alongside the upgrade adapter:
//! both are the only pieces that cannot be exercised offline, so isolating them keeps that
//! gap out of the trait and verification logic this module exposes.

use anyhow::Result;
use std::collections::BTreeMap;

/// The repository that publishes pixi-sandbox's own release assets (standalone binaries
/// and `SHA256SUMS`) — never the consumer's.
///
/// `self_update::DEFAULT_REPO` re-exports this constant so the CLI's `--repo` default and
/// the generated workflow's release-download URL cannot name two different repositories
/// (issue #80: the generated workflow once resolved the download against
/// `$GITHUB_REPOSITORY`, the *consumer's* repository, which publishes no such assets).
pub const PIXI_SANDBOX_REPO: &str = "Archont561/pixi-sandbox";

/// Release asset name for a project that publishes `<name>.sha256` next to each asset.
pub const PIXI_CHECKSUM_MANIFEST: &str = "sha256.sum";

/// Where a release's bytes come from.
pub trait ReleaseSource {
    /// Newest release tag for `owner/repo`, exactly as GitHub spells it (e.g. `v0.81.0`).
    ///
    /// # Errors
    ///
    /// Returns an error if the repository cannot be queried or carries no release.
    fn latest_tag(&self, repo: &str) -> Result<String>;

    /// Download one release asset by name.
    ///
    /// # Errors
    ///
    /// Returns an error if the asset cannot be downloaded (a missing tag or asset, a failed
    /// request).
    fn asset(&self, repo: &str, tag: &str, name: &str) -> Result<Asset>;

    /// Upstream-published checksums for a release, when the project publishes one.
    ///
    /// `None` means "this project ships no manifest", which is a weaker guarantee rather than a
    /// failure — recorded in the pin so a later reader can tell the two cases apart.
    ///
    /// # Errors
    ///
    /// Returns an error if the checksum manifest exists but cannot be read.
    fn published_checksums(
        &self,
        repo: &str,
        tag: &str,
    ) -> Result<Option<BTreeMap<String, String>>>;
}

pub struct Asset {
    pub bytes: Vec<u8>,
}

impl Asset {
    #[must_use]
    pub fn sha256(&self) -> String {
        pixi_sandbox_core::shard::sha256_bytes(&self.bytes)
    }
}

/// Parse the two spellings of a checksum manifest: `<hex>  <name>` (coreutils) and `<name> <hex>`.
///
/// pixi's `sha256.sum` is coreutils-shaped, but accepting both costs one branch and means a
/// mirror that emits the other order is not silently treated as "no manifest".
#[must_use]
pub fn parse_sha256_manifest(body: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in body.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        match fields.as_slice() {
            [hex, name, ..] if is_sha256(hex) => {
                map.insert((*name).to_string(), (*hex).to_string());
            }
            [name, hex, ..] if is_sha256(hex) => {
                map.insert((*name).to_string(), (*hex).to_string());
            }
            _ => {}
        }
    }
    map
}

#[must_use]
pub fn is_sha256(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit())
}
