//! Declarative publish planning for `.pixi-sandbox.toml`.
//!
//! A project must opt into the environments it publishes. Inferring "all environments" from
//! `pixi.toml` is unsafe: projects commonly keep experimental, CI-only, or platform-specific
//! environments there. This small schema turns a reviewed bundle list into a stable GitHub
//! Actions matrix without coupling the transport format to a CI provider.

use crate::error::{Error, Result};
use crate::tools_lock::ToolsLock;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Schema understood by this build.
pub const CONFIG_SCHEMA: u32 = 1;

/// Default project-level declaration consumed by `pixi-sandbox plan`.
pub const DEFAULT_FILE: &str = ".pixi-sandbox.toml";

/// A checked-in description of sandbox bundles and their native runners.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SandboxConfig {
    pub schema: u32,
    #[serde(default = "default_branch_prefix")]
    pub branch_prefix: String,
    #[serde(default = "default_cargo_vendor")]
    pub cargo_vendor: bool,
    /// Optional platform → single GitHub runner label overrides. The defaults cover the three
    /// common targets; nonstandard/arm runners must be named explicitly.
    #[serde(default)]
    pub runners: BTreeMap<String, String>,
    #[serde(default, rename = "bundle")]
    pub bundles: Vec<Bundle>,
}

/// One environment set to publish for one or more native platforms.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    /// Stable branch-name component, e.g. `developer`.
    pub name: String,
    /// Explicit Pixi environments. Their order is retained in the pack invocation and branch
    /// identity, so review changes carefully.
    pub environments: Vec<String>,
    /// Pixi platform names, e.g. `linux-64`, `osx-arm64`, `win-64`.
    pub platforms: Vec<String>,
    /// Override the top-level cargo-vendor policy for this bundle.
    #[serde(default)]
    pub cargo_vendor: Option<bool>,
}

/// JSON shape emitted by `pixi-sandbox plan --json`, directly consumable as an Actions matrix.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PublishPlan {
    pub schema: u32,
    pub include: Vec<PublishTarget>,
}

/// One native job: it creates exactly one orphan branch.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PublishTarget {
    pub bundle: String,
    /// Comma-separated specifically so the output can flow straight into `--envs`.
    pub environments: String,
    pub platform: String,
    pub runner: String,
    pub branch: String,
    pub cargo_vendor: bool,
}

impl SandboxConfig {
    /// Read and validate a project declaration.
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
        let config: SandboxConfig =
            toml::from_str(&text).map_err(|error| Error::InvalidManifest {
                path: path.display().to_string(),
                reason: error.to_string(),
            })?;
        config.validate()?;
        Ok(config)
    }

    /// Produce a stable one-entry-per-(bundle × platform) publish plan.
    pub fn plan(&self) -> Result<PublishPlan> {
        self.validate()?;
        // The release-driven publisher has no project-local tool lock input: it deliberately
        // fetches only the reviewed catalogue compiled into its verified binary. Fail before a
        // matrix job starts if a configured platform would later fail to fetch pixi, pack, or
        // unpack helpers.
        let embedded_tools = ToolsLock::embedded()?;
        let mut include = Vec::new();
        for bundle in &self.bundles {
            let environments = bundle.environments.join(",");
            for platform in &bundle.platforms {
                ensure_embedded_tools_cover(&embedded_tools, platform)?;
                let runner = self.runner_for(platform)?;
                include.push(PublishTarget {
                    bundle: bundle.name.clone(),
                    environments: environments.clone(),
                    platform: platform.clone(),
                    runner,
                    branch: format!("{}/{}-{}", self.branch_prefix, bundle.name, platform),
                    cargo_vendor: bundle.cargo_vendor.unwrap_or(self.cargo_vendor),
                });
            }
        }
        include.sort_by(|left, right| left.branch.cmp(&right.branch));
        Ok(PublishPlan {
            schema: CONFIG_SCHEMA,
            include,
        })
    }

    fn validate(&self) -> Result<()> {
        if self.schema != CONFIG_SCHEMA {
            return Err(Error::Invalid(format!(
                "sandbox config schema {} is not supported (expected {CONFIG_SCHEMA})",
                self.schema
            )));
        }
        validate_branch_prefix(&self.branch_prefix)?;
        for (platform, runner) in &self.runners {
            validate_platform(platform)?;
            validate_runner_label(platform, runner)?;
        }
        if self.bundles.is_empty() {
            return Err(Error::Invalid(
                "sandbox config needs at least one [[bundle]]".to_string(),
            ));
        }

        let mut names = BTreeSet::new();
        let mut branches = BTreeSet::new();
        let mut requested_platforms = BTreeSet::<&str>::new();
        for bundle in &self.bundles {
            validate_slug("bundle name", &bundle.name)?;
            if !names.insert(&bundle.name) {
                return Err(Error::Invalid(format!(
                    "bundle {:?} is declared more than once",
                    bundle.name
                )));
            }
            if bundle.environments.is_empty() {
                return Err(Error::Invalid(format!(
                    "bundle {:?} has no environments",
                    bundle.name
                )));
            }
            if bundle.platforms.is_empty() {
                return Err(Error::Invalid(format!(
                    "bundle {:?} has no platforms",
                    bundle.name
                )));
            }

            let mut environments = BTreeSet::new();
            for environment in &bundle.environments {
                validate_slug("environment", environment)?;
                if !environments.insert(environment) {
                    return Err(Error::Invalid(format!(
                        "bundle {:?} requests environment {:?} more than once",
                        bundle.name, environment
                    )));
                }
            }

            let mut platforms = BTreeSet::new();
            for platform in &bundle.platforms {
                validate_platform(platform)?;
                if !platforms.insert(platform) {
                    return Err(Error::Invalid(format!(
                        "bundle {:?} requests platform {:?} more than once",
                        bundle.name, platform
                    )));
                }
                requested_platforms.insert(platform.as_str());
                let branch = format!("{}/{}-{platform}", self.branch_prefix, bundle.name);
                if !branches.insert(branch.clone()) {
                    return Err(Error::Invalid(format!(
                        "multiple bundles would publish the same branch {branch:?}"
                    )));
                }
                let _ = self.runner_for(platform)?;
            }
        }
        for platform in self.runners.keys() {
            if !requested_platforms.contains(platform.as_str()) {
                return Err(Error::Invalid(format!(
                    "runner override for {platform:?} is unused; remove it or declare that platform in a bundle"
                )));
            }
        }
        Ok(())
    }

    fn runner_for(&self, platform: &str) -> Result<String> {
        if let Some(runner) = self.runners.get(platform) {
            if runner.trim().is_empty() {
                return Err(Error::Invalid(format!(
                    "runner override for {platform:?} is empty"
                )));
            }
            return Ok(runner.clone());
        }
        let default = match platform {
            "linux-64" => Some("ubuntu-latest"),
            // `macos-14` is Apple Silicon on GitHub-hosted runners. Override it when using a
            // self-hosted runner or a different hosted label.
            "osx-arm64" => Some("macos-14"),
            "osx-64" => Some("macos-13"),
            "win-64" => Some("windows-latest"),
            _ => None,
        };
        default.map(str::to_string).ok_or_else(|| {
            Error::Invalid(format!(
                "platform {platform:?} needs runners.{platform:?}; no safe default runner is known"
            ))
        })
    }
}

/// A planned release job always uses compiled pins: allowing a target whose helper releases are
/// absent would turn a reviewed plan into a slow, avoidable failure on a native runner.
fn ensure_embedded_tools_cover(lock: &ToolsLock, platform: &str) -> Result<()> {
    let missing = ["pixi", "pixi-pack", "pixi-unpack"]
        .into_iter()
        .filter(|tool| lock.pin(tool, platform).is_none())
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }
    Err(Error::Invalid(format!(
        "platform {platform:?} has no complete embedded helper-tool pins (missing {}); \
         add reviewed pins and release a new pixi-sandbox binary before publishing it",
        missing.join(", ")
    )))
}

fn default_branch_prefix() -> String {
    "sandbox".to_string()
}

const fn default_cargo_vendor() -> bool {
    true
}

fn validate_slug(kind: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(Error::Invalid(format!(
            "{kind} {value:?} must use only lowercase ASCII letters, digits, and hyphens"
        )));
    }
    Ok(())
}

fn validate_platform(value: &str) -> Result<()> {
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || value.is_empty()
    {
        return Err(Error::Invalid(format!(
            "platform {value:?} must use only lowercase ASCII letters, digits, and hyphens"
        )));
    }
    Ok(())
}

fn validate_runner_label(platform: &str, runner: &str) -> Result<()> {
    if runner.trim().is_empty() || runner != runner.trim() || runner.chars().any(char::is_control) {
        return Err(Error::Invalid(format!(
            "runner override for {platform:?} must be a non-empty label without leading/trailing whitespace or control characters"
        )));
    }
    Ok(())
}

fn validate_branch_prefix(value: &str) -> Result<()> {
    if value.is_empty()
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains("//")
        || value.contains("..")
        || value.contains("@{")
        || value.bytes().any(|byte| {
            byte.is_ascii_control()
                || byte.is_ascii_whitespace()
                || matches!(byte, b'~' | b'^' | b':' | b'?' | b'*' | b'[' | b'\\')
        })
    {
        return Err(Error::Invalid(format!(
            "branch_prefix {value:?} is not a safe git-ref prefix"
        )));
    }
    for component in value.split('/') {
        if component.is_empty()
            || component == "."
            || component == ".."
            || component.starts_with('.')
            || component.ends_with('.')
            || component.ends_with(".lock")
        {
            return Err(Error::Invalid(format!(
                "branch_prefix {value:?} contains an unsafe git-ref component {component:?}"
            )));
        }
    }
    Ok(())
}
