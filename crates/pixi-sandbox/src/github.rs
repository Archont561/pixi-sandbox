//! The real GitHub HTTP adapters.
//!
//! This module owns the real GitHub HTTP adapters for the release and upgrade traits, but it is
//! not the only socket-touching path in `pixi-sandbox`: `tool_fetch::HttpDownload` directly opens
//! the URLs for pinned helper assets. The traits keep their decision logic in their respective
//! `mod.rs` homes, where offline fakes exercise it. `pixi run coverage` excludes this exact
//! filename for the real GitHub adapter boundary, not for all network I/O. Keep this file thin —
//! a second consumer (such as xtask) or a third GitHub API call is the trigger to reconsider its
//! in-crate home.
//!
//! Transport (TASK-124 / ADR-27): `latest_tag` uses the `self_update` crate's
//! `backends::github::ReleaseList` for rate-limit aware listing (`Error::RateLimited` with
//! `Retry-After`), and `asset` uses `Download` for the actual file transfer. Checksum parsing
//! itself lives in `self_update::Checksum::from_sums_file` (used in `self_update::mod.rs`).

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
        // Use the self_update crate's GitHub backend for rate-limit aware listing.
        // It returns Error::RateLimited distinct from Unauthorized, correctly classifying
        // GitHub's secondary 403 + Retry-After (ADR-27).
        let (owner, repo_name) = repo
            .split_once('/')
            .ok_or_else(|| anyhow::anyhow!("invalid repo {repo:?}, expected owner/name"))?;
        let mut builder = ::self_update::backends::github::ReleaseList::configure();
        builder.repo_owner(owner);
        builder.repo_name(repo_name);
        if let Some(token) = &self.token {
            builder.auth_token(token);
        } else {
            // Reads GH_TOKEN, GITHUB_TOKEN (gh CLI precedence) – broader than old GITHUB_TOKEN-only,
            // but intentionally so per crate docs.
            builder.auth_token_from_env();
        }
        let releases = builder
            .build()
            .map_err(|e| anyhow::anyhow!(e))
            .context("building GitHub release list")?
            .fetch()
            .map_err(|e| anyhow::anyhow!(e))
            .with_context(|| format!("fetching releases for {repo}"))?;
        let release = releases
            .latest()
            .ok_or_else(|| anyhow::anyhow!("no releases for {repo}"))?;
        // Release::version() is bare semver (no v prefix); reconstruct tag as v{version}
        // to match our historical latest_tag contract (v0.4.4 etc.).
        Ok(format!("v{}", release.version()))
    }

    fn asset(&self, repo: &str, tag: &str, name: &str) -> Result<Asset> {
        // Use the self_update crate's Download engine (ureq with rustls, retries, timeout)
        // per ADR-27, rather than hand-rolled ureq get.
        let url = format!("https://github.com/{repo}/releases/download/{tag}/{name}");
        let mut download = ::self_update::Download::from_url(&url);
        if let Some(token) = &self.token {
            // Private repos need Authorization; for public repos it's harmless.
            // Download::request_header is infallible setter, deferred error surfaced in download_to.
            download.request_header("Authorization", format!("Bearer {token}"));
        }
        let mut bytes = Vec::new();
        download
            .download_to(&mut bytes)
            .map_err(|e| anyhow::anyhow!(e))
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
        // Keep ureq path for this secondary manifest fetch (tools_update) – it already handles 404
        // as Ok(None). Could be migrated to Download as well, but not required for ADR-27.
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
