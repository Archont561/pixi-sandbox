//! The real GitHub implementation of `PullRequestSource`.
//!
//! Isolated in its own file for the same reason `release/github.rs` is: it is the only part
//! of the upgrade lane that needs a socket, and therefore the only part no offline test can
//! reach. `pixi run coverage` excludes this file by name (see the `coverage` task in
//! `crates/package.json`); counting it would either depress the number permanently or invite
//! a mock-the-HTTP-client test that proves nothing about GitHub. Keep it thin — anything
//! with a decision in it belongs in `mod.rs`, where tests can reach it.

use anyhow::{Context, Result};

use super::{PullRequest, PullRequestSource};

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
        let mut response = ureq::post(&url)
            .header("Accept", "application/vnd.github+json")
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .header("User-Agent", "pixi-sandbox")
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
