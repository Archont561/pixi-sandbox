use pixi_sandbox_core::sandbox_config::{
    CONFIG_SCHEMA, SandboxConfig, is_safe_git_ref, peek_schema, plan_override, schema_supported,
};
use pixi_sandbox_core::tools_lock::ToolsLock;
use proptest::prelude::*;
use rstest::rstest;
use std::collections::BTreeSet;
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

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// A reviewed configuration always produces one unique, ref-safe target per declared
    /// bundle/platform pair. Two randomly named bundles exercise ordering, branch construction,
    /// runner selection, and the duplicate-target guard together instead of sampling one plan.
    #[test]
    fn a_config_plan_has_unique_valid_targets(
        first in "[a-z][a-z0-9-]{0,10}",
        second in "[a-z][a-z0-9-]{0,10}",
        environment in "[a-z][a-z0-9-]{0,10}",
        first_platform in prop_oneof![Just("linux-64"), Just("osx-arm64"), Just("osx-64"), Just("win-64")],
        second_platform in prop_oneof![Just("linux-64"), Just("osx-arm64"), Just("osx-64"), Just("win-64")],
    ) {
        prop_assume!(first != second);
        let plan = config(&format!(
            "schema = 1\nbranch_prefix = \"sandbox\"\n\
             [[bundle]]\nname = \"{first}\"\nenvironments = [\"{environment}\"]\nplatforms = [\"{first_platform}\"]\n\
             [[bundle]]\nname = \"{second}\"\nenvironments = [\"{environment}\"]\nplatforms = [\"{second_platform}\"]\n"
        )).plan().expect("generated config plans");
        let branches = plan.include.iter().map(|target| target.branch.as_str()).collect::<BTreeSet<_>>();
        prop_assert_eq!(branches.len(), plan.include.len());
        prop_assert!(plan.include.iter().all(|target| is_safe_git_ref(&target.branch)));
        prop_assert!(plan.include.iter().all(|target| target.environments == environment));
    }
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
#[rstest]
#[case("linux-64", "ubuntu-latest")]
#[case("osx-arm64", "macos-14")]
#[case("osx-64", "macos-13")]
#[case("win-64", "windows-latest")]
fn an_override_plan_uses_the_same_runner_table_as_a_reviewed_bundle(
    #[case] platform: &str,
    #[case] runner: &str,
) {
    let envs = vec!["default".to_string()];
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
#[rstest]
#[case("BAD/Name", "sandbox", "must use only lowercase")]
#[case("linux-64", "../evil", "safe git-ref")]
fn an_override_plan_is_validated_like_a_config(
    #[case] platform: &str,
    #[case] prefix: &str,
    #[case] expected: &str,
) {
    let envs = vec!["default".to_string()];
    let err = plan_override("custom", &envs, platform, prefix, true)
        .unwrap_err()
        .to_string();
    assert!(err.contains(expected), "{platform} / {prefix}: {err}");
}

#[test]
fn an_override_plan_refuses_no_environments() {
    let err = plan_override("custom", &[], "linux-64", "sandbox", true)
        .unwrap_err()
        .to_string();
    assert!(err.contains("no environments"), "{err}");
}

/// The runner table and the embedded helper catalogue are two independent lists, and a platform
/// that has a runner but no pins would fail on a native runner minutes after the matrix started.
/// Declaring every platform the table knows must therefore plan cleanly — which is also the
/// precondition for adding a macOS bundle: the pins for it already exist.
fn runner_table_plan() -> pixi_sandbox_core::sandbox_config::PublishPlan {
    config(
        r#"
schema = 1
branch_prefix = "sandbox"
cargo_vendor = true

[[bundle]]
name = "developer"
environments = ["default"]
platforms = ["linux-64", "osx-arm64", "osx-64", "win-64"]
"#,
    )
    .plan()
    .unwrap()
}

#[test]
fn the_runner_table_plans_exactly_the_platforms_the_cases_cover() {
    let plan = runner_table_plan();
    let platforms: Vec<&str> = plan
        .include
        .iter()
        .map(|target| target.platform.as_str())
        .collect();
    assert_eq!(platforms, ["linux-64", "osx-64", "osx-arm64", "win-64"]);
}

#[rstest]
#[case("linux-64", "pixi")]
#[case("linux-64", "pixi-pack")]
#[case("linux-64", "pixi-unpack")]
#[case("osx-arm64", "pixi")]
#[case("osx-arm64", "pixi-pack")]
#[case("osx-arm64", "pixi-unpack")]
#[case("osx-64", "pixi")]
#[case("osx-64", "pixi-pack")]
#[case("osx-64", "pixi-unpack")]
#[case("win-64", "pixi")]
#[case("win-64", "pixi-pack")]
#[case("win-64", "pixi-unpack")]
fn every_platform_with_a_hosted_runner_has_complete_embedded_helper_pins(
    #[case] platform: &str,
    #[case] tool: &str,
) {
    let plan = runner_table_plan();
    assert!(
        plan.include
            .iter()
            .any(|target| target.platform == platform),
        "the case list drifted from the plan: {platform} is no longer included"
    );
    let lock = ToolsLock::embedded().unwrap();
    assert!(
        lock.pin(tool, platform).is_some(),
        "{tool} has no pin for {platform}"
    );
}

/// Mirrors `manifest::schema_supported`'s policy exactly (task-47 AC#5 / decision D16): every
/// schema from 1 up to the current one is understood, zero is not a schema that ever existed,
/// and only a schema newer than this build knows about is refused. The range reads as `{1}`
/// today only because `CONFIG_SCHEMA` has never bumped — this property holds for the function
/// itself, independent of that one current value.
#[test]
fn schema_support_is_an_inclusive_range_from_one_to_current() {
    assert!(!schema_supported(0));
    assert!(schema_supported(1));
    assert!(schema_supported(CONFIG_SCHEMA));
    assert!(!schema_supported(CONFIG_SCHEMA + 1));
    assert!(!schema_supported(CONFIG_SCHEMA + 50));
}

/// `peek_schema` must read the schema out of a config this build cannot otherwise parse at all
/// (a newer schema, or `deny_unknown_fields`-violating content from a hypothetical different
/// shape) — that is the whole point of keeping it separate from `SandboxConfig::load`.
#[test]
fn peek_schema_reads_the_field_even_when_the_rest_of_the_file_is_unrecognisable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pixi-sandbox.toml");

    fs::write(&path, "schema = 1\nbranch_prefix = \"sandbox\"\n").unwrap();
    assert_eq!(peek_schema(&path).unwrap(), 1);

    // A schema this build has never shipped, carrying a field set `SandboxConfig::load` would
    // reject outright under `deny_unknown_fields` — peek still reports the schema number.
    fs::write(
        &path,
        "schema = 7\nsome_future_field = \"this build has no idea what this is\"\n",
    )
    .unwrap();
    assert_eq!(peek_schema(&path).unwrap(), 7);

    fs::write(&path, "branch_prefix = \"sandbox\"\n").unwrap();
    assert!(peek_schema(&path).is_err(), "no schema key at all is a real error");
}

/// `SandboxConfig::load` refuses a schema newer than this build understands — it could mean
/// anything — naming both the found and the understood schema so the remedy is obvious.
#[test]
fn load_refuses_a_schema_newer_than_this_build_understands() {
    let message = config_error(
        "schema = 2\nbranch_prefix = \"sandbox\"\n\n[[bundle]]\nname = \"developer\"\nenvironments = [\"default\"]\nplatforms = [\"linux-64\"]\n",
    );
    assert!(message.contains("schema 2 is not supported"), "{message}");
    assert!(message.contains("understands 1..=1"), "{message}");
}
