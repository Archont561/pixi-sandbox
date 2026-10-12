//! TASK-78 / ADR-28: the `pixi-sandbox-payload.toml` declaration and its load-time refusals.
//!
//! Every case here is a tempdir or an in-memory string: nothing fetches, builds, or reads this
//! checkout.

use pixi_sandbox_core::payload::PayloadError;
use pixi_sandbox_core::payload_config::{DEFAULT_FILE, PAYLOAD_CONFIG_SCHEMA, PayloadConfig};
use pixi_sandbox_core::platform::Platform;
use std::path::Path;

const SHA256: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn valid_toml() -> String {
    format!(
        r#"schema = 1
name = "firefox"
version = "1.0.0"
platform = "linux-64"
entry_point = "firefox/firefox"

prepare = "curl -fsSL -o artifact.tar.gz https://example.invalid/firefox.tar.gz && tar -xzf artifact.tar.gz"
artifact = "artifact.tar.gz"
payload = "firefox"

[source]
url = "https://example.invalid/firefox-1.0.0.tar.gz"
sha256 = "{SHA256}"
"#
    )
}

fn write_config(dir: &Path, text: &str) -> std::path::PathBuf {
    let path = dir.join(DEFAULT_FILE);
    std::fs::write(&path, text).expect("the config is writable");
    path
}

#[test]
fn a_valid_declaration_loads_and_yields_the_spec_the_pure_rules_take() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_config(dir.path(), &valid_toml());

    let config = PayloadConfig::load(&path).expect("a well-formed declaration loads");
    let spec = config.spec().expect("the identity is valid");

    assert_eq!(spec.name, "firefox");
    assert_eq!(spec.version, "1.0.0");
    assert_eq!(spec.platform, Platform::Linux64);
    assert!(!spec.noarch);
    assert_eq!(spec.entry_point.as_deref(), Some("firefox/firefox"));
    assert_eq!(
        config.working_dir, ".pixi/payload-work",
        "fetched inputs default to Pixi's ignored project storage"
    );
    assert_eq!(config.artifact, "artifact.tar.gz");
    assert_eq!(config.payload, "firefox");
    assert_eq!(PAYLOAD_CONFIG_SCHEMA, 1);
}

#[test]
fn an_unknown_key_is_refused_rather_than_ignored() {
    let dir = tempfile::tempdir().expect("tempdir");
    let text = format!("{}\nmystery = true\n", valid_toml());
    let path = write_config(dir.path(), &text);

    let error = PayloadConfig::load(&path).expect_err("a typo must not be silently dropped");

    assert!(matches!(error, PayloadError::InvalidConfig(message) if message.contains("mystery")));
}

#[test]
fn a_newer_schema_is_refused_with_the_understood_range() {
    let dir = tempfile::tempdir().expect("tempdir");
    let text = valid_toml().replace("schema = 1", "schema = 2");
    let path = write_config(dir.path(), &text);

    let error = PayloadConfig::load(&path).expect_err("a future schema is refused");

    assert!(
        matches!(error, PayloadError::InvalidConfig(message) if message.contains("schema 2") && message.contains("1..=1"))
    );
}

#[test]
fn an_empty_prepare_command_is_refused_because_nothing_would_fetch_the_payload() {
    let dir = tempfile::tempdir().expect("tempdir");
    let text = valid_toml().replace(
        "prepare = \"curl -fsSL -o artifact.tar.gz https://example.invalid/firefox.tar.gz && tar -xzf artifact.tar.gz\"",
        "prepare = \"   \"",
    );
    let path = write_config(dir.path(), &text);

    let error = PayloadConfig::load(&path).expect_err("an empty prepare fetches nothing");

    assert!(matches!(error, PayloadError::InvalidConfig(message) if message.contains("prepare")));
}

#[test]
fn a_missing_prepare_key_is_itself_a_parse_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let text = valid_toml()
        .lines()
        .filter(|line| !line.starts_with("prepare ="))
        .collect::<Vec<_>>()
        .join("\n");
    let path = write_config(dir.path(), &text);

    let error = PayloadConfig::load(&path).expect_err("prepare is required");

    assert!(matches!(error, PayloadError::InvalidConfig(message) if message.contains("prepare")));
}

#[test]
fn an_invalid_package_name_is_refused_at_load_time() {
    let dir = tempfile::tempdir().expect("tempdir");
    let text = valid_toml().replace("name = \"firefox\"", "name = \"Bad Name\"");
    let path = write_config(dir.path(), &text);

    let error =
        PayloadConfig::load(&path).expect_err("an invalid name is refused before any fetch");

    assert_eq!(error, PayloadError::InvalidName("Bad Name".to_string()));
}

#[test]
fn an_unknown_platform_is_refused_naming_the_value() {
    let dir = tempfile::tempdir().expect("tempdir");
    let text = valid_toml().replace("platform = \"linux-64\"", "platform = \"plan9\"");
    let path = write_config(dir.path(), &text);

    let error = PayloadConfig::load(&path).expect_err("an unknown platform is refused");

    assert_eq!(error, PayloadError::InvalidPlatform("plan9".to_string()));
}

#[test]
fn a_source_that_is_not_https_is_refused_at_load_time() {
    let dir = tempfile::tempdir().expect("tempdir");
    let text = valid_toml().replace(
        "url = \"https://example.invalid/firefox-1.0.0.tar.gz\"",
        "url = \"http://example.invalid/firefox-1.0.0.tar.gz\"",
    );
    let path = write_config(dir.path(), &text);

    let error = PayloadConfig::load(&path).expect_err("only https sources are recorded");

    assert_eq!(
        error,
        PayloadError::InvalidSourceUrl("http://example.invalid/firefox-1.0.0.tar.gz".to_string())
    );
}

#[test]
fn a_digest_that_is_not_64_lowercase_hex_is_refused_at_load_time() {
    let dir = tempfile::tempdir().expect("tempdir");
    let text = valid_toml().replace(SHA256, &SHA256.to_uppercase());
    let path = write_config(dir.path(), &text);

    let error =
        PayloadConfig::load(&path).expect_err("a digest an auditor cannot compare is refused");

    assert_eq!(error, PayloadError::InvalidSha256(SHA256.to_uppercase()));
}

#[rstest::rstest]
#[case("artifact", "/etc/artifact.tar.gz")]
#[case("payload", "../outside")]
#[case("working_dir", "a/../../b")]
fn an_output_path_that_is_absolute_or_escapes_is_refused(#[case] field: &str, #[case] value: &str) {
    let dir = tempfile::tempdir().expect("tempdir");
    let needle = match field {
        "artifact" => "artifact = \"artifact.tar.gz\"",
        "payload" => "payload = \"firefox\"",
        _ => "working_dir",
    };
    let text = if field == "working_dir" {
        format!("{}\nworking_dir = {value:?}\n", valid_toml())
    } else {
        valid_toml().replace(needle, &format!("{field} = {value:?}"))
    };
    let path = write_config(dir.path(), &text);

    let error = PayloadConfig::load(&path)
        .expect_err("a committed config may not carry an absolute or escaping path");

    assert!(matches!(error, PayloadError::InvalidConfig(message) if message.contains(field)));
}
