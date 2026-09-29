use pixi_sandbox_core::sandbox_config::{CONFIG_SCHEMA, SandboxConfig, plan_override};
use pixi_sandbox_core::tools_lock::ToolsLock;
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

/// An ad-hoc dispatch must resolve its runner through the same table a reviewed bundle uses.
/// The workflow this replaces carried a second `case` statement listing the runners, so the two
/// could disagree; these assertions are the reason they cannot any more.
#[test]
fn an_override_plan_uses_the_same_runner_table_as_a_reviewed_bundle() {
    let envs = vec!["default".to_string()];
    let expected = [
        ("linux-64", "ubuntu-latest"),
        ("osx-arm64", "macos-14"),
        ("osx-64", "macos-13"),
        ("win-64", "windows-latest"),
    ];
    for (platform, runner) in expected {
        let plan = plan_override("custom", &envs, platform, "sandbox", true).unwrap();
        assert_eq!(plan.schema, CONFIG_SCHEMA);
        assert_eq!(plan.include.len(), 1);
        let target = &plan.include[0];
        assert_eq!(target.runner, runner, "{platform}");
        assert_eq!(target.platform, platform);
        assert_eq!(target.bundle, "custom");
        assert_eq!(target.environments, "default");
        assert_eq!(target.branch, format!("sandbox/custom-{platform}"));
        assert!(target.cargo_vendor);
    }
}

/// `linux-aarch64` has no safe hosted default on purpose (README documents the override), so an
/// ad-hoc dispatch must not invent one. The bash table this replaced mapped it to
/// `ubuntu-24.04-arm`, which would have dispatched a job the project never agreed to.
#[test]
fn an_override_plan_refuses_a_platform_with_no_hosted_default() {
    let envs = vec!["default".to_string()];
    let err = plan_override("custom", &envs, "linux-aarch64", "sandbox", true)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("linux-aarch64") && err.contains("runners."),
        "{err}"
    );
}

/// The override path is a synthetic config, so it inherits the platform slug and git-ref rules
/// rather than being a looser back door into the matrix.
#[test]
fn an_override_plan_is_validated_like_a_config() {
    let envs = vec!["default".to_string()];
    for (platform, prefix, expected) in [
        ("BAD/Name", "sandbox", "must use only lowercase"),
        ("linux-64", "../evil", "safe git-ref"),
    ] {
        let err = plan_override("custom", &envs, platform, prefix, true)
            .unwrap_err()
            .to_string();
        assert!(err.contains(expected), "{platform} / {prefix}: {err}");
    }

    let err = plan_override("custom", &[], "linux-64", "sandbox", true)
        .unwrap_err()
        .to_string();
    assert!(err.contains("no environments"), "{err}");
}

/// The runner table and the embedded helper catalogue are two independent lists, and a platform
/// that has a runner but no pins would fail on a native runner minutes after the matrix started.
/// Declaring every platform the table knows must therefore plan cleanly — which is also the
/// precondition for adding a macOS bundle: the pins for it already exist.
#[test]
fn every_platform_with_a_hosted_runner_has_complete_embedded_helper_pins() {
    let config = config(
        r#"
schema = 1
branch_prefix = "sandbox"
cargo_vendor = true

[[bundle]]
name = "developer"
environments = ["default"]
platforms = ["linux-64", "osx-arm64", "osx-64", "win-64"]
"#,
    );
    let plan = config.plan().unwrap();
    assert_eq!(plan.include.len(), 4);
    for target in &plan.include {
        let lock = ToolsLock::embedded().unwrap();
        for tool in ["pixi", "pixi-pack", "pixi-unpack"] {
            assert!(
                lock.pin(tool, &target.platform).is_some(),
                "{tool} has no pin for {}",
                target.platform
            );
        }
    }
}
