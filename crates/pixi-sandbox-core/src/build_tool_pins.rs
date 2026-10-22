//! Caller-supplied pins for the optional connected-side package builder. No embedded catalogue.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Pin-file schema this build understands.
pub const LOCK_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildToolPins {
    pub schema: u32,
    /// Optional provenance timestamp supplied by the builder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
    pub tools: BTreeMap<String, Tool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub version: String,
    /// `{version}` and `{target}` are substituted; no other templating.
    pub url_template: String,
    /// pixi platform name → release asset for that platform.
    pub platforms: BTreeMap<String, PlatformPin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformPin {
    /// Release-asset target triple (plus `.exe` on Windows).
    pub target: String,
    pub sha256: String,
    /// `static` (musl / fully linked), `system` (OS libraries), or `dynamic` (a bug).
    pub linkage: String,
    /// Free-form note, e.g. how a pin was cross-checked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl BuildToolPins {
    /// The pin for a tool on a platform, if this catalogue has one.
    #[must_use]
    pub fn pin(&self, tool: &str, platform: &str) -> Option<&PlatformPin> {
        self.tools.get(tool)?.platforms.get(platform)
    }

    /// Fully substituted download URL for a tool on a platform.
    ///
    /// Returns `None` when either the tool or the platform is not pinned — callers must treat
    /// that as "cannot proceed", never as "fall back to whatever is on PATH".
    #[must_use]
    pub fn url(&self, tool: &str, platform: &str) -> Option<String> {
        let entry = self.tools.get(tool)?;
        let pin = entry.platforms.get(platform)?;
        Some(
            entry
                .url_template
                .replace("{version}", &entry.version)
                .replace("{target}", &pin.target),
        )
    }

    /// Platforms pinned for a tool, in a stable order.
    #[must_use]
    pub fn platforms_of(&self, tool: &str) -> Vec<&str> {
        self.tools
            .get(tool)
            .map(|tool| tool.platforms.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }
}
