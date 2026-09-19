use crate::error::{Result, SandboxError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Minimal config per spec/configuration.md §6
/// Every key optional, kebab-case TOML ↔ snake_case Rust, deny_unknown_fields
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    #[serde(default)]
    pub meta: Meta,
    #[serde(default)]
    pub pack: Pack,
    #[serde(default)]
    pub vendor: Vendor,
    #[serde(default)]
    pub kit: Kit,
    #[serde(default)]
    pub transport: Transport,
    #[serde(default)]
    pub discovery: Discovery,
    #[serde(default)]
    pub platforms: Platforms,
    #[serde(default)]
    pub selfhost: Selfhost,
    #[serde(default)]
    pub reconstruct: Reconstruct,
    #[serde(default)]
    pub docs: Docs,
    #[serde(default)]
    pub doctor: Doctor,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
pub struct Meta {
    pub name: Option<String>,
    pub manifest: Option<String>,
    pub artifacts_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Pack {
    #[serde(default = "default_envs")]
    pub environments: String,
    #[serde(default = "default_platforms")]
    pub target_platforms: String,
    #[serde(default)]
    pub create_executable: bool,
    #[serde(default = "default_use_cache")]
    pub use_cache: String,
    #[serde(default = "default_true")]
    pub ignore_pypi_non_wheel: bool,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default)]
    pub inject: Vec<String>,
    #[serde(default)]
    pub runner: PackRunner,
}

impl Default for Pack {
    fn default() -> Self {
        Self {
            environments: default_envs(),
            target_platforms: default_platforms(),
            create_executable: false,
            use_cache: default_use_cache(),
            ignore_pypi_non_wheel: true,
            format: default_format(),
            inject: Vec::new(),
            runner: PackRunner::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
pub struct PackRunner {
    pub program: Option<String>,
    pub pin: Option<String>,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Vendor {
    #[serde(default = "default_vendor_dir")]
    pub dir: String,
    #[serde(default = "default_vendor_strategy")]
    pub strategy: String,
    #[serde(default = "default_vendor_branch")]
    pub branch: String,
    #[serde(default = "default_vendor_format")]
    pub format: String,
    #[serde(default = "default_vendor_wiring")]
    pub wiring: String,
    #[serde(default = "default_cargo_config")]
    pub cargo_config: String,
    #[serde(default = "default_true")]
    pub write_config: bool,
    #[serde(default)]
    pub all_features: bool,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub respect_source_config: bool,
    #[serde(default)]
    pub sync_manifests: Vec<String>,
    #[serde(default)]
    pub no_delete: bool,
    #[serde(default)]
    pub prune_unverified: bool,
}

impl Default for Vendor {
    fn default() -> Self {
        Self {
            dir: default_vendor_dir(),
            strategy: default_vendor_strategy(),
            branch: default_vendor_branch(),
            format: default_vendor_format(),
            wiring: default_vendor_wiring(),
            cargo_config: default_cargo_config(),
            write_config: true,
            all_features: false,
            features: Vec::new(),
            respect_source_config: false,
            sync_manifests: Vec::new(),
            no_delete: false,
            prune_unverified: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Kit {
    #[serde(default = "default_kit_output")]
    pub output_dir: String,
    #[serde(default = "default_components")]
    pub components: String,
    #[serde(default = "default_true")]
    pub checksums: bool,
    #[serde(default = "default_true")]
    pub bootstrap: bool,
    #[serde(default)]
    pub tarball: bool,
    #[serde(default = "default_cache_source")]
    pub cache_source: String,
    #[serde(default = "default_true")]
    pub ship_pixi: bool,
    #[serde(default = "default_auto")]
    pub targets_compile: String,
    #[serde(default = "default_true")]
    pub only_changed: bool,
    #[serde(default)]
    pub allowed_git_origins: Vec<String>,
    #[serde(default = "default_true")]
    pub emit_pixi_config: bool,
}

impl Default for Kit {
    fn default() -> Self {
        Self {
            output_dir: default_kit_output(),
            components: default_components(),
            checksums: true,
            bootstrap: true,
            tarball: false,
            cache_source: default_cache_source(),
            ship_pixi: true,
            targets_compile: default_auto(),
            only_changed: true,
            allowed_git_origins: Vec::new(),
            emit_pixi_config: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Transport {
    #[serde(default = "default_branch")]
    pub branch: String,
    #[serde(default = "default_remote")]
    pub remote: String,
    #[serde(default = "default_true")]
    pub bundle: bool,
    #[serde(default)]
    pub lfs: bool,
    #[serde(default = "default_keep_last")]
    pub keep_last: u32,
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            branch: default_branch(),
            remote: default_remote(),
            bundle: true,
            lfs: false,
            keep_last: default_keep_last(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Discovery {
    #[serde(default = "default_auto")]
    pub max_tier: String,
    #[serde(default = "default_true")]
    pub require_lock_for_selection: bool,
    #[serde(default = "default_ignore")]
    pub ignore: Vec<String>,
}

impl Default for Discovery {
    fn default() -> Self {
        Self {
            max_tier: default_auto(),
            require_lock_for_selection: true,
            ignore: default_ignore(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
pub struct Platforms {
    pub default: Vec<String>,
    pub on_missing: Option<String>,
    pub per_env: Option<std::collections::HashMap<String, Vec<String>>>,
    pub refresh_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Selfhost {
    #[serde(default = "default_true")]
    pub include_self: bool,
    #[serde(default = "default_true")]
    pub include_pixi: bool,
    #[serde(default = "default_include_pack_tools")]
    pub include_pack_tools: Vec<String>,
    #[serde(default = "default_pixi_version")]
    pub pixi_version: String,
    #[serde(default = "default_selfhost_branch")]
    pub branch: String,
    #[serde(default = "default_true")]
    pub emit_conda_file: bool,
    #[serde(default)]
    pub notice: bool,
}

impl Default for Selfhost {
    fn default() -> Self {
        Self {
            include_self: true,
            include_pixi: true,
            include_pack_tools: default_include_pack_tools(),
            pixi_version: default_pixi_version(),
            branch: default_selfhost_branch(),
            emit_conda_file: true,
            notice: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Reconstruct {
    #[serde(default = "default_auto")]
    pub mode: String,
    #[serde(default = "default_guaranteed_rung")]
    pub guaranteed_rung: String,
    #[serde(default = "default_off")]
    pub remote_mirrors: String,
    #[serde(default = "default_ship_activation")]
    pub ship_activation_cache: String,
    #[serde(default = "default_cache_subset")]
    pub cache_subset: String,
    #[serde(default = "default_pixi_install_args")]
    pub pixi_install_args: Vec<String>,
    #[serde(default = "default_true")]
    pub keep_env_names: bool,
    #[serde(default = "default_verify")]
    pub verify: Vec<String>,
    #[serde(default)]
    pub allow_partial: bool,
}

impl Default for Reconstruct {
    fn default() -> Self {
        Self {
            mode: default_auto(),
            guaranteed_rung: default_guaranteed_rung(),
            remote_mirrors: default_off(),
            ship_activation_cache: default_ship_activation(),
            cache_subset: default_cache_subset(),
            pixi_install_args: default_pixi_install_args(),
            keep_env_names: true,
            verify: default_verify(),
            allow_partial: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
pub struct Docs {
    pub enabled: bool,
    pub source: Vec<String>,
    pub skip: Vec<String>,
    pub site_dir: Option<String>,
    pub package_manager: Option<String>,
    pub site: Option<String>,
    pub base: Option<String>,
    pub publish_branch: Option<String>,
    pub codec: Option<String>,
    pub check_links: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
pub struct Doctor {
    pub require: Vec<String>,
    pub probe: Vec<String>,
    pub probe_timeout_secs: Option<u32>,
    pub expect_unreachable: Vec<String>,
    pub egress_fixture: Option<String>,
}

fn default_envs() -> String {
    "auto".to_string()
}
fn default_platforms() -> String {
    "auto".to_string()
}
fn default_use_cache() -> String {
    "~/.cache/rattler".to_string()
}
fn default_format() -> String {
    "tar".to_string()
}
fn default_vendor_dir() -> String {
    "vendor".to_string()
}
fn default_vendor_strategy() -> String {
    "branch".to_string()
}
fn default_vendor_branch() -> String {
    "pixi-sandbox-vendor".to_string()
}
fn default_vendor_format() -> String {
    "directory".to_string()
}
fn default_vendor_wiring() -> String {
    "config-file".to_string()
}
fn default_cargo_config() -> String {
    ".cargo/config.toml".to_string()
}
fn default_kit_output() -> String {
    "kit".to_string()
}
fn default_components() -> String {
    "auto".to_string()
}
fn default_cache_source() -> String {
    "use-cache".to_string()
}
fn default_branch() -> String {
    "pixi-sandbox-dist".to_string()
}
fn default_remote() -> String {
    "origin".to_string()
}
fn default_auto() -> String {
    "auto".to_string()
}
fn default_true() -> bool {
    true
}
fn default_off() -> String {
    "off".to_string()
}
fn default_guaranteed_rung() -> String {
    "R3".to_string()
}
fn default_ship_activation() -> String {
    "if-lock-matches".to_string()
}
fn default_cache_subset() -> String {
    "pack".to_string()
}
fn default_keep_last() -> u32 {
    3
}
fn default_ignore() -> Vec<String> {
    vec![
        "sandbox/**".to_string(),
        "target/**".to_string(),
        ".pixi/**".to_string(),
        "**/node_modules/**".to_string(),
    ]
}
fn default_include_pack_tools() -> Vec<String> {
    vec!["pixi-pack".to_string(), "pixi-unpack".to_string()]
}
fn default_pixi_version() -> String {
    "0.81.0".to_string()
}
fn default_selfhost_branch() -> String {
    "pixi-sandbox-mirror".to_string()
}
fn default_pixi_install_args() -> Vec<String> {
    vec!["install".to_string(), "--frozen".to_string()]
}
fn default_verify() -> Vec<String> {
    vec![
        "pixi run __selftest__".to_string(),
        "pixi list".to_string(),
        "pixi shell -e {env}".to_string(),
    ]
}

pub fn find_config_path(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        let candidate = dir.join("pixi-sandbox.toml");
        if candidate.exists() {
            return Some(candidate);
        }
        current = dir.parent();
    }
    None
}

pub fn load_config(
    cwd: Option<&Path>,
    explicit: Option<&Path>,
) -> Result<(Config, Option<PathBuf>)> {
    let config_path = if let Some(p) = explicit {
        Some(p.to_path_buf())
    } else if let Some(c) = cwd {
        find_config_path(c)
    } else {
        find_config_path(&std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    };

    if let Some(ref path) = config_path {
        let content = std::fs::read_to_string(path).map_err(|e| {
            SandboxError::Other(anyhow::anyhow!(
                "failed to read config {}: {e}",
                path.display()
            ))
        })?;
        let expanded = content
            .lines()
            .map(|line| {
                if line.contains('~') {
                    let mut result = line.to_string();
                    if result.contains("~/") {
                        result = result.replace(
                            "~/",
                            &format!(
                                "{}/",
                                std::env::var("HOME").unwrap_or_else(|_| "~".to_string())
                            ),
                        );
                    }
                    result
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let cfg: Config = toml::from_str(&expanded).map_err(|e| {
            SandboxError::Other(anyhow::anyhow!(
                "failed to parse config {}: {e}",
                path.display()
            ))
        })?;
        Ok((cfg, Some(path.clone())))
    } else {
        Ok((Config::default(), None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.pack.environments, "auto");
        assert_eq!(cfg.transport.branch, "pixi-sandbox-dist");
        assert!(cfg.kit.checksums);
    }

    #[test]
    fn test_parse_minimal() {
        let toml_str = r#"
[pack]
environments = "dev docs"
target-platforms = "linux-64"

[kit]
output-dir = "my-kit"
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.pack.environments, "dev docs");
        assert_eq!(cfg.kit.output_dir, "my-kit");
    }

    #[test]
    fn test_deny_unknown() {
        let toml_str = r#"
[pack]
unknown-key = true
"#;
        let result = toml::from_str::<Config>(toml_str);
        assert!(result.is_err(), "should fail on unknown field");
    }

    #[test]
    fn test_kebab_case() {
        let toml_str = r#"
[pack]
target-platforms = "linux-64"
create-executable = true
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.pack.target_platforms, "linux-64");
        assert!(cfg.pack.create_executable);
    }
}
