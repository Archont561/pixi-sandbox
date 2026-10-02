//! The real GitHub implementation of `ReleaseSource`.
//!
//! Isolated in its own file because it is the only part of `release` that needs a socket, and
//! therefore the only part no offline test can reach. `pixi run coverage` excludes this file by
//! name (see the `coverage` task in `pixi.toml`): counting it would either depress the number
//! permanently or invite a mock-the-HTTP-client test that proves nothing about GitHub. Keep it
//! thin — anything with a decision in it belongs in `mod.rs`, where tests can reach it.

use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;

use super::{Asset, ReleaseSource, parse_sha256_manifest};

/// Release asset name for a project that publishes `<name>.sha256` next to each asset.
pub const PIXI_CHECKSUM_MANIFEST: &str = "sha256.sum";

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
