//! The real GitHub HTTP adapters.
//!
//! This module owns the real GitHub HTTP adapters for the release and upgrade traits, but it is
//! not the only socket-touching path in `pixi-sandbox`: `tool_fetch::HttpDownload` directly opens
//! the URLs for pinned helper assets. The traits keep their decision logic in their respective
//! `mod.rs` homes, where offline fakes exercise it. `pixi run coverage` excludes this exact
//! filename for the real GitHub adapter boundary, not for all network I/O. Keep this file thin —
//! a second consumer (such as xtask) or a third GitHub API call is the trigger to reconsider its
//! in-crate home.

use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;

use crate::release::{Asset, PIXI_CHECKSUM_MANIFEST, ReleaseSource, parse_sha256_manifest};
use crate::upgrade::{PullRequest, PullRequestSource};

const GITHUB_ACCEPT: &str = "application/vnd.github+json";
const USER_AGENT: &str = "pixi-sandbox";

/// Apply the request shape every GitHub API call shares. Callers choose the method and where
/// their token came from; this helper only keeps the protocol headers from drifting.
fn github_request<B>(
    request: ureq::RequestBuilder<B>,
    token: Option<&str>,
) -> ureq::RequestBuilder<B> {
    let request = request
        .header("Accept", GITHUB_ACCEPT)
        .header("User-Agent", USER_AGENT);
    match token {
        Some(token) => request.header("Authorization", format!("Bearer {token}")),
        None => request,
    }
}

/// GitHub, for real. `GITHUB_TOKEN` lifts the 60 requests/hour anonymous limit.
pub struct GitHubReleaseSource {
    token: Option<String>,
}

impl GitHubReleaseSource {
    #[must_use]
    pub fn new() -> Self {
        Self {
            token: std::env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty()),
        }
    }

    fn get(&self, url: &str) -> Result<ureq::http::Response<ureq::Body>> {
        github_request(ureq::get(url), self.token.as_deref())
            .call()
            .with_context(|| format!("GET {url}"))
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
            .get(&format!(
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
        let response = self.get(&url)?;
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
        let Ok(mut response) = self.get(&format!(
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

/// GitHub, for real. The token comes from the caller (the workflow's
/// `PIXI_SANDBOX_UPGRADE_TOKEN || github.token`), never from this module's environment.
#[derive(Debug, Default)]
pub struct GitHubPullRequestSource;

impl GitHubPullRequestSource {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl PullRequestSource for GitHubPullRequestSource {
    fn create(
        &self,
        repo: &str,
        head: &str,
        base: &str,
        title: &str,
        body: &str,
        token: &str,
    ) -> Result<PullRequest> {
        let url = format!("https://api.github.com/repos/{repo}/pulls");
        // ureq's `json` feature is not enabled (and enabling it would be a dependency-surface
        // change for one call), so the payload is serialized with the serde_json this crate
        // already depends on.
        let payload = serde_json::to_string(&serde_json::json!({
            "title": title,
            "head": head,
            "base": base,
            "body": body,
        }))
        .context("serializing the pull-request payload")?;
        let mut response = github_request(ureq::post(&url), Some(token))
            .header("Content-Type", "application/json")
            .send(payload)
            .with_context(|| format!("POST {url}"))?;
        let text = response
            .body_mut()
            .read_to_string()
            .with_context(|| format!("reading the pull-request response from {url}"))?;
        let json: serde_json::Value = serde_json::from_str(&text)
            .with_context(|| format!("parsing the pull-request response from {url}"))?;
        let pull_url = json["html_url"]
            .as_str()
            .map(str::to_string)
            .with_context(|| {
                format!("GitHub's pull-request response for {repo} has no html_url")
            })?;
        let number = json["number"]
            .as_u64()
            .with_context(|| format!("GitHub's pull-request response for {repo} has no number"))?;
        Ok(PullRequest {
            url: pull_url,
            number,
        })
    }
}
