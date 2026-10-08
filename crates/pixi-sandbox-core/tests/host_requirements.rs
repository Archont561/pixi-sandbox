//! `[host_requirements]` schema, resolution and refusals (issue #109, TASK-75).
//!
//! Every case writes its config into a tempdir and loads it through the real
//! [`SandboxConfig::load`] path: the loader, not this test, decides what a config may say, and
//! nothing here reads the host or this checkout (D10).

use pixi_sandbox_core::host_requirements::{
    HostCapability, HostRequirementSet, HostRequirements, LibcFloor,
};
use pixi_sandbox_core::platform::HostFamily;
use pixi_sandbox_core::sandbox_config::SandboxConfig;
use proptest::prelude::*;
use rstest::rstest;
use std::fs;

/// The smallest valid config: one bundle, no host requirements.
const BASE: &str = r#"
schema = 1

[[bundle]]
name = "app"
environments = ["default"]
platforms = ["linux-64"]
"#;

fn load(text: &str) -> Result<SandboxConfig, String> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".pixi-sandbox.toml");
    fs::write(&path, text).unwrap();
    SandboxConfig::load(&path).map_err(|error| error.to_string())
}

fn config(text: &str) -> SandboxConfig {
    load(text).unwrap()
}

#[test]
fn a_config_without_the_table_loads_unchanged() {
    let config = config(BASE);
    assert!(config.host_requirements.is_none());
    assert_eq!(config.host_requirements_for("linux-64").unwrap(), None);
}

#[test]
fn the_issue_example_resolves_for_a_linux_platform() {
    let config = config(&format!(
        "{BASE}
[host_requirements]
libc = \">=2.34\"

[host_requirements.linux]
packages = [\"fontconfig\", \"fonts-dejavu\", \"xvfb\"]
services = [\"dbus\"]
capabilities = [\"display\"]
headless = [\"xvfb-run\"]
"
    ));

    let resolved = config.host_requirements_for("linux-64").unwrap().unwrap();
    assert_eq!(resolved.libc.as_deref(), Some(">=2.34"));
    assert_eq!(
        resolved.packages,
        ["fontconfig", "fonts-dejavu", "xvfb"].map(String::from)
    );
    assert_eq!(resolved.services, ["dbus"].map(String::from));
    assert_eq!(resolved.capabilities, [HostCapability::Display]);
    assert_eq!(resolved.headless, ["xvfb-run"].map(String::from));

    // Architecture does not change the answer: both Linux platforms share one family section.
    assert_eq!(
        config
            .host_requirements_for("linux-aarch64")
            .unwrap()
            .unwrap(),
        resolved
    );
}

#[rstest]
#[case("osx-arm64", HostFamily::Osx)]
#[case("win-64", HostFamily::Windows)]
fn a_family_without_a_section_gets_the_shared_set(
    #[case] platform: &str,
    #[case] family: HostFamily,
) {
    let config = config(&format!(
        "{BASE}
[host_requirements]
packages = [\"fontconfig\"]

[host_requirements.linux]
packages = [\"xvfb\"]
"
    ));

    let shared = config
        .host_requirements
        .as_ref()
        .unwrap()
        .resolved_for(family);
    assert_eq!(shared.packages, ["fontconfig"].map(String::from));
    assert_eq!(
        config.host_requirements_for(platform).unwrap().unwrap(),
        shared
    );
}

#[test]
fn requirements_accumulate_and_the_family_libc_wins() {
    let config = config(&format!(
        "{BASE}
[host_requirements]
libc = \"2.31\"
packages = [\"fontconfig\"]
capabilities = [\"display\"]

[host_requirements.linux]
libc = \">=2.34\"
packages = [\"xvfb\", \"fontconfig\"]
"
    ));

    let resolved = config.host_requirements_for("linux-64").unwrap().unwrap();
    // Shared entries first, then the section's new ones, duplicates dropped.
    assert_eq!(resolved.packages, ["fontconfig", "xvfb"].map(String::from));
    assert_eq!(resolved.capabilities, [HostCapability::Display]);
    // A family floor replaces the shared one; the shared set keeps its own.
    assert_eq!(resolved.libc.as_deref(), Some(">=2.34"));
    assert_eq!(
        resolved.libc_floor().unwrap(),
        Some(LibcFloor::parse("2.34").unwrap())
    );
    let shared = config.host_requirements.as_ref().unwrap().shared();
    assert_eq!(shared.libc.as_deref(), Some("2.31"));
}

#[test]
fn an_empty_table_is_refused_as_a_typo() {
    let error = load(&format!("{BASE}\n[host_requirements]\n")).unwrap_err();
    assert!(error.contains("declares no requirements"), "got {error}");
}

#[test]
fn an_empty_family_section_is_refused() {
    let error = load(&format!(
        "{BASE}
[host_requirements]
packages = [\"fontconfig\"]

[host_requirements.linux]
"
    ))
    .unwrap_err();
    assert!(
        error.contains("[host_requirements.linux] declares nothing"),
        "got {error}"
    );
}

#[test]
fn an_unknown_family_section_names_the_known_families() {
    let error = load(&format!(
        "{BASE}\n[host_requirements.freebsd]\npackages = [\"x\"]\n"
    ))
    .unwrap_err();
    assert!(error.contains("freebsd"), "got {error}");
    assert!(error.contains("linux"), "got {error}");
}

#[test]
fn an_unknown_capability_is_refused_rather_than_skipped() {
    let error = load(&format!(
        "{BASE}
[host_requirements]
capabilities = [\"dipslay\"]
"
    ))
    .unwrap_err();
    assert!(error.contains("dipslay"), "got {error}");
    assert!(error.contains("display"), "got {error}");
    assert!(error.contains("gpu"), "got {error}");
}

#[test]
fn a_libc_floor_in_a_non_linux_section_is_refused() {
    let error = load(&format!(
        "{BASE}
[host_requirements.osx]
libc = \">=2.34\"
"
    ))
    .unwrap_err();
    assert!(
        error.contains("libc floor only applies to Linux"),
        "got {error}"
    );
}

#[test]
fn duplicate_and_malformed_entries_are_refused() {
    let duplicate = load(&format!(
        "{BASE}
[host_requirements]
packages = [\"fontconfig\", \"fontconfig\"]
"
    ))
    .unwrap_err();
    assert!(duplicate.contains("twice"), "got {duplicate}");

    let spaced = load(&format!(
        "{BASE}
[host_requirements]
services = [\"dbus session\"]
"
    ))
    .unwrap_err();
    assert!(spaced.contains("dbus session"), "got {spaced}");
}

#[rstest]
#[case("2.34", ">=2.34")]
#[case(">=2.34", ">=2.34")]
#[case(">=2.34.1", ">=2.34.1")]
fn accepted_libc_specs_are_canonicalised(#[case] spec: &str, #[case] canonical: &str) {
    assert_eq!(LibcFloor::parse(spec).unwrap().canonical(), canonical);
}

#[rstest]
#[case("2")]
#[case("2.34 ")]
#[case(">2.34")]
#[case("<2.34")]
#[case("2.x")]
#[case("2.34.1.0")]
#[case("")]
fn refused_libc_specs_name_the_accepted_forms(#[case] spec: &str) {
    let error = LibcFloor::parse(spec).unwrap_err().to_string();
    assert!(error.contains("major.minor"), "got {error}");
}

#[test]
fn floors_compare_by_version_and_a_bare_minor_equals_a_zero_patch() {
    let floor = LibcFloor::parse("2.39").unwrap();
    assert_eq!(floor, LibcFloor::parse("2.39.0").unwrap());
    assert!(floor.is_met_by(LibcFloor::parse("2.39").unwrap()));
    assert!(floor.is_met_by(LibcFloor::parse("2.40").unwrap()));
    assert!(!floor.is_met_by(LibcFloor::parse("2.38").unwrap()));
    assert!(!LibcFloor::parse("2.40").unwrap().is_met_by(floor));
}

#[test]
fn a_resolved_set_keeps_declaration_order_without_duplicates() {
    let set: HostRequirementSet = toml::from_str(
        r#"
packages = ["b", "a"]
services = ["dbus"]
capabilities = ["gpu", "display"]
headless = ["xvfb-run"]
libc = ">=2.34"
"#,
    )
    .unwrap();
    set.validate("host_requirements.linux").unwrap();
    assert_eq!(set.packages, ["b", "a"].map(String::from));
    assert_eq!(
        set.capabilities,
        [HostCapability::Gpu, HostCapability::Display]
    );
}

#[test]
fn an_unknown_field_inside_the_table_is_refused() {
    let error = load(&format!(
        "{BASE}
[host_requirements]
package = [\"fontconfig\"]
"
    ))
    .unwrap_err();
    assert!(error.contains("package"), "got {error}");
}

#[test]
fn host_requirements_for_refuses_an_unknown_platform() {
    let config = config(&format!(
        "{BASE}
[host_requirements]
packages = [\"fontconfig\"]
"
    ));
    let error = config
        .host_requirements_for("linux-riscv64")
        .unwrap_err()
        .to_string();
    assert!(error.contains("linux-riscv64"), "got {error}");
}

proptest! {
    /// `parse` is total: any string either yields a canonical floor or a refusal that quotes
    /// the input (in debug form, so control characters are escaped the same way on both sides).
    #[test]
    fn libc_parsing_never_panics_and_names_its_input(spec in ".{0,24}") {
        match LibcFloor::parse(&spec) {
            Ok(floor) => prop_assert!(floor.canonical().starts_with(">=")),
            Err(error) => {
                let quoted = format!("{spec:?}");
                prop_assert!(error.to_string().contains(&quoted));
            }
        }
    }

    /// Resolution is a superset of both levels: nothing declared is ever lost, and the output
    /// carries no duplicates.
    #[test]
    fn resolution_keeps_every_declared_entry(
        shared in proptest::collection::btree_set("[a-z][a-z0-9-]{0,8}", 1..4),
        extra in proptest::collection::btree_set("[a-z][a-z0-9-]{0,8}", 1..4),
    ) {
        let array = |names: &std::collections::BTreeSet<String>| {
            format!(
                "[{}]",
                names
                    .iter()
                    .map(|name| format!("{name:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let declared: HostRequirements = toml::from_str(&format!(
            "packages = {}\n\n[linux]\npackages = {}\n",
            array(&shared),
            array(&extra),
        ))
        .unwrap();
        declared.validate().unwrap();

        let resolved = declared.resolved_for(HostFamily::Linux).packages;
        let mut unique = resolved.clone();
        unique.sort();
        unique.dedup();
        prop_assert_eq!(unique.len(), resolved.len(), "resolution introduced a duplicate");

        for entry in shared.iter().chain(extra.iter()) {
            prop_assert!(resolved.contains(entry), "{} was lost", entry);
        }
    }
}
