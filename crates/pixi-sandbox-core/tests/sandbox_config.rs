use pixi_sandbox_core::sandbox_config::{CONFIG_SCHEMA, SandboxConfig};
use std::fs;

fn config(text: &str) -> SandboxConfig {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".pixi-sandbox.toml");
    fs::write(&path, text).unwrap();
    SandboxConfig::load(&path).unwrap()
}

#[test]
fn plans_one_native_publish_job_per_bundle_and_platform() {
    let config = config(
        r#"
schema = 1
branch_prefix = "sandbox"
cargo_vendor = true

[[bundle]]
name = "developer"
environments = ["dev", "docs"]
platforms = ["win-64", "linux-64", "osx-arm64"]

[[bundle]]
name = "minimal"
environments = ["default"]
platforms = ["linux-64"]
cargo_vendor = false
"#,
    );

    let plan = config.plan().unwrap();
    assert_eq!(plan.schema, CONFIG_SCHEMA);
    assert_eq!(plan.include.len(), 4);
    assert_eq!(
        plan.include
            .iter()
            .map(|target| target.branch.as_str())
            .collect::<Vec<_>>(),
        vec![
            "sandbox/developer-linux-64",
            "sandbox/developer-osx-arm64",
            "sandbox/developer-win-64",
            "sandbox/minimal-linux-64",
        ]
    );
    let mac = plan
        .include
        .iter()
        .find(|target| target.platform == "osx-arm64")
        .unwrap();
    assert_eq!(mac.runner, "macos-14");
    assert_eq!(mac.environments, "dev,docs");
    let minimal = plan
        .include
        .iter()
        .find(|target| target.bundle == "minimal")
        .unwrap();
    assert!(!minimal.cargo_vendor);
}

#[test]
fn requires_an_explicit_runner_for_nonstandard_platforms() {
    let err = config_error(
        r#"
schema = 1
[[bundle]]
name = "arm"
environments = ["dev"]
platforms = ["linux-aarch64"]
"#,
    );
    assert!(err.contains("runners."), "{err}");
}

#[test]
fn accepts_a_reviewed_runner_override() {
    let config = config(
        r#"
schema = 1
[runners]
linux-aarch64 = "ubuntu-24.04-arm"

[[bundle]]
name = "arm"
environments = ["dev"]
platforms = ["linux-aarch64"]
"#,
    );
    assert_eq!(config.plan().unwrap().include[0].runner, "ubuntu-24.04-arm");
}

#[test]
fn rejects_unused_or_malformed_runner_overrides() {
    let err = config_error(
        r#"
schema = 1
[runners]
osx-arm64 = "macos-14"

[[bundle]]
name = "linux"
environments = ["dev"]
platforms = ["linux-64"]
"#,
    );
    assert!(err.contains("is unused"), "{err}");

    let err = config_error(
        r#"
schema = 1
[runners]
linux-64 = " ubuntu-latest"

[[bundle]]
name = "linux"
environments = ["dev"]
platforms = ["linux-64"]
"#,
    );
    assert!(err.contains("leading/trailing whitespace"), "{err}");
}

#[test]
fn rejects_a_runner_backed_platform_without_embedded_tool_pins() {
    let config = config(
        r#"
schema = 1
[runners]
linux-ppc64le = "self-hosted-ppc64le"

[[bundle]]
name = "unsupported"
environments = ["dev"]
platforms = ["linux-ppc64le"]
"#,
    );
    let err = config.plan().unwrap_err().to_string();
    assert!(err.contains("embedded helper-tool pins"), "{err}");
    assert!(err.contains("pixi-pack"), "{err}");
}

#[test]
fn rejects_unsafe_branch_prefix_and_duplicate_targets() {
    let err = config_error(
        r#"
schema = 1
branch_prefix = "sandbox/../escape"
[[bundle]]
name = "developer"
environments = ["dev"]
platforms = ["linux-64", "linux-64"]
"#,
    );
    assert!(err.contains("safe git-ref"), "{err}");

    let err = config_error(
        r#"
schema = 1
[[bundle]]
name = "developer"
environments = ["dev"]
platforms = ["linux-64", "linux-64"]
"#,
    );
    assert!(err.contains("more than once"), "{err}");
}

fn config_error(text: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".pixi-sandbox.toml");
    fs::write(&path, text).unwrap();
    SandboxConfig::load(&path).unwrap_err().to_string()
}
