//! Where release bytes come from, for every command that needs them.
//!
//! This is a trait for the same reason git goes through `GitProtocol` (D9): the risk lives in
//! what a command *does* with downloaded bytes — verification, pin rewriting, replacing an
//! executable — so that logic has to be exercisable without a network and without GitHub.
//! `tools update` was the first consumer; `self-update` (decision-4) is the second, and a
//! second near-identical trait would have meant two fakes drifting apart.

use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;

/// Where a release's bytes come from.
pub trait ReleaseSource {
    /// Newest release tag for `owner/repo`, exactly as GitHub spells it (e.g. `v0.81.0`).
    fn latest_tag(&self, repo: &str) -> Result<String>;

    /// Download one release asset by name.
    fn asset(&self, repo: &str, tag: &str, name: &str) -> Result<Asset>;

    /// Upstream-published checksums for a release, when the project publishes one.
    ///
    /// `None` means "this project ships no manifest", which is a weaker guarantee rather than a
    /// failure — recorded in the pin so a later reader can tell the two cases apart.
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
    pub fn sha256(&self) -> String {
        pixi_sandbox_core::shard::sha256_bytes(&self.bytes)
    }
}

/// GitHub, for real. `GITHUB_TOKEN` lifts the 60 requests/hour anonymous limit.
pub struct GitHubReleaseSource {
    token: Option<String>,
}

impl GitHubReleaseSource {
    pub fn new() -> Self {
        Self {
            token: std::env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty()),
        }
    }

    fn request(&self, url: &str) -> Result<ureq::http::Response<ureq::Body>> {
        let mut get = ureq::get(url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "pixi-sandbox");
        if let Some(token) = &self.token {
            get = get.header("Authorization", format!("Bearer {token}"));
        }
        get.call().with_context(|| format!("GET {url}"))
    }
}

impl Default for GitHubReleaseSource {
    fn default() -> Self {
        Self::new()
    }
}

/// Release asset name for a project that publishes `<name>.sha256` next to each asset.
pub const PIXI_CHECKSUM_MANIFEST: &str = "sha256.sum";

impl ReleaseSource for GitHubReleaseSource {
    fn latest_tag(&self, repo: &str) -> Result<String> {
        let body = self
            .request(&format!(
                "https://api.github.com/repos/{repo}/releases/latest"
            ))?
            .body_mut()
            .read_to_string()
            .with_context(|| format!("reading the latest release of {repo}"))?;
        let json: serde_json::Value = serde_json::from_str(&body)
            .with_context(|| format!("parsing the latest release of {repo}"))?;
        json["tag_name"]
            .as_str()
            .map(str::to_string)
            .with_context(|| format!("GitHub's latest-release response for {repo} has no tag_name"))
    }

    fn asset(&self, repo: &str, tag: &str, name: &str) -> Result<Asset> {
        let url = format!("https://github.com/{repo}/releases/download/{tag}/{name}");
        let mut bytes = Vec::new();
        let response = self.request(&url)?;
        std::io::copy(&mut response.into_body().into_reader(), &mut bytes)
            .with_context(|| format!("downloading {name} from {url}"))?;
        if bytes.is_empty() {
            bail!("{url} returned an empty body; refusing to record a hash for nothing");
        }
        Ok(Asset { bytes })
    }

    fn published_checksums(
        &self,
        repo: &str,
        tag: &str,
    ) -> Result<Option<BTreeMap<String, String>>> {
        // A project with no manifest answers 404; that is a fact about the project, not an error.
        let Ok(mut response) = self.request(&format!(
            "https://github.com/{repo}/releases/download/{tag}/{PIXI_CHECKSUM_MANIFEST}"
        )) else {
            return Ok(None);
        };
        let body = response
            .body_mut()
            .read_to_string()
            .with_context(|| format!("reading {PIXI_CHECKSUM_MANIFEST} for {repo} {tag}"))?;
        Ok(Some(parse_sha256_manifest(&body)))
    }
}

/// Parse the two spellings of a checksum manifest: `<hex>  <name>` (coreutils) and `<name> <hex>`.
///
/// pixi's `sha256.sum` is coreutils-shaped, but accepting both costs one branch and means a
/// mirror that emits the other order is not silently treated as "no manifest".
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

pub fn is_sha256(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit())
}
