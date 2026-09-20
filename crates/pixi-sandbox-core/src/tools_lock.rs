//! The pinned helper-tool catalogue (decision D4).
//!
//! The canonical JSON is compiled into `pixi-sandbox-core`, so an installed
//! `pixi-sandbox` binary can safely run `pack --fetch-tools` without requiring a loose file
//! beside it. Callers may supply an external `--tools-lock` only as an explicit, reviewable
//! override (for example for an organisation mirror or an emergency pin update).
//!
//! Shape (schema 1):
//! ```json
//! { "schema": 1, "generated_at": "…",
//!   "tools": { "pixi-unpack": { "version": "0.7.11",
//!     "url_template": "https://…/v{version}/pixi-unpack-{target}",
//!     "platforms": { "linux-64": { "target": "x86_64-unknown-linux-musl",
//!                                  "sha256": "8191f5…", "linkage": "static" } } } } }
//! ```

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Pin-file schema this build understands.
pub const LOCK_SCHEMA: u32 = 1;

/// Human-readable label used whenever the compiled catalogue is selected.
pub const EMBEDDED_SOURCE: &str = "embedded tools lock";

/// Filename used when a helper is placed in a platform-specific transport directory.
///
/// Tool identity remains extension-free in manifests (`pixi`, `pixi-unpack`), while Windows
/// transport paths preserve `.exe` so PowerShell/cmd users and `PATH` resolution can execute
/// them naturally.
pub fn executable_filename(tool: &str, platform: &str) -> String {
    if platform.starts_with("win-") && !tool.to_ascii_lowercase().ends_with(".exe") {
        format!("{tool}.exe")
    } else {
        tool.to_string()
    }
}

/// The one canonical catalogue ships in the binary. Keep this as data rather than Rust structs:
/// pin updates remain small, readable review diffs and external overrides use the same schema.
const EMBEDDED_JSON: &str = include_str!("../assets/tools.lock.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsLock {
    pub schema: u32,
    /// When `tools update` last refreshed it (informational).
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

impl ToolsLock {
    /// Load the reviewed catalogue compiled into this binary.
    pub fn embedded() -> Result<Self> {
        Self::parse(EMBEDDED_SOURCE, EMBEDDED_JSON.as_bytes())
    }

    /// Load a deliberate external override.
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
        Self::parse(&path.display().to_string(), &bytes)
    }

    fn parse(source: &str, bytes: &[u8]) -> Result<Self> {
        let lock: ToolsLock =
            serde_json::from_slice(bytes).map_err(|e| Error::InvalidManifest {
                path: source.to_string(),
                reason: e.to_string(),
            })?;
        if lock.schema != LOCK_SCHEMA {
            return Err(Error::Invalid(format!(
                "{source}: schema {} is not supported (expected {LOCK_SCHEMA})",
                lock.schema
            )));
        }
        Ok(lock)
    }

    /// The pin for a tool on a platform, if this catalogue has one.
    pub fn pin(&self, tool: &str, platform: &str) -> Option<&PlatformPin> {
        self.tools.get(tool)?.platforms.get(platform)
    }

    /// Fully substituted download URL for a tool on a platform.
    ///
    /// Returns `None` when either the tool or the platform is not pinned — callers must treat
    /// that as "cannot proceed", never as "fall back to whatever is on PATH".
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

    /// Tool names in a stable order, for `tools list`.
    pub fn names(&self) -> Vec<&str> {
        self.tools.keys().map(String::as_str).collect()
    }

    /// Platforms pinned for a tool, in a stable order.
    pub fn platforms_of(&self, tool: &str) -> Vec<&str> {
        self.tools
            .get(tool)
            .map(|tool| tool.platforms.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::executable_filename;

    #[test]
    fn transport_tool_names_keep_a_windows_executable_suffix() {
        assert_eq!(executable_filename("pixi", "win-64"), "pixi.exe");
        assert_eq!(
            executable_filename("pixi-unpack.exe", "win-64"),
            "pixi-unpack.exe"
        );
        assert_eq!(executable_filename("pixi", "linux-64"), "pixi");
    }
}
