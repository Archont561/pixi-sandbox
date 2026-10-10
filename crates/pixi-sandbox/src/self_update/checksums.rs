//! `SHA256SUMS` parsing and verification.
//!
//! The release's `SHA256SUMS` is written by `xtask release-checksums` as `<digest>␣␣<name>`
//! over every standalone binary, and verified there for completeness. This is the consuming
//! half: a missing entry and a digest mismatch are *different* errors on purpose, because they
//! mean different things to an operator — the first says the release is incomplete for this
//! host, the second says the bytes that arrived are not the bytes that were published.

use anyhow::{Result, bail};
use std::collections::BTreeMap;

use crate::release::parse_sha256_manifest;

/// A parsed `SHA256SUMS`, keyed by asset name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checksums {
    entries: BTreeMap<String, String>,
}

impl Checksums {
    /// Parse the file. An unparsable body yields no entries, which `digest_for` then reports
    /// against the asset the caller actually wanted — a better message than "parse error".
    #[must_use]
    pub fn parse(body: &str) -> Self {
        Self {
            entries: parse_sha256_manifest(body),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn names(&self) -> Vec<&str> {
        self.entries.keys().map(String::as_str).collect()
    }

    /// The published digest for `asset`, or an error naming what the file did cover.
    ///
    /// # Errors
    ///
    /// Returns an error if the checksum manifest is empty or unparsable, or does not list
    /// `asset`.
    pub fn digest_for(&self, asset: &str) -> Result<&str> {
        if self.is_empty() {
            bail!(
                "SHA256SUMS for this release is empty or unparsable; refusing to install \
                 unverified bytes"
            );
        }
        match self.entries.get(asset) {
            Some(digest) => Ok(digest.as_str()),
            None => bail!(
                "SHA256SUMS has no entry for {asset}; refusing to install unverified bytes\n\
                 the release covers: {}",
                self.names().join(", ")
            ),
        }
    }

    /// Check `bytes` against the published digest for `asset`.
    ///
    /// Both failure modes are checked here so no caller can accidentally fetch a digest and
    /// forget to compare it (invariant 1, "verify before write", applied to the updater).
    ///
    /// # Errors
    ///
    /// Returns an error if the digest is unavailable or the bytes do not match it.
    pub fn verify(&self, asset: &str, bytes: &[u8]) -> Result<String> {
        let expected = self.digest_for(asset)?;
        let actual = pixi_sandbox_core::shard::sha256_bytes(bytes);
        if actual != expected {
            bail!(
                "checksum mismatch for {asset}\n  expected {expected}\n  actual   {actual}\n\
                 the downloaded bytes are not the published release; nothing was written"
            );
        }
        Ok(actual)
    }
}
