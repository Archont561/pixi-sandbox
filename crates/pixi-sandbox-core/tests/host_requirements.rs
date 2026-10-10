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

// ---------------------------------------------------------------------------------------------
// Evaluation: the classification the probes feed (TASK-75 AC#3/#5)
// ---------------------------------------------------------------------------------------------
//
// Every case below drives `evaluate` through a scripted probe. Nothing here reads the machine
// it runs on, so a verdict of "satisfied" or "missing" is a statement about this file, not
// about the host — which is the only way a suite can assert on probes without lying.

mod evaluation {
    use pixi_sandbox_core::host_requirements::{
        Distro, HostCapability, HostProbe, HostRequirementSet, HostRequirements, HostStatus,
        LibcFloor, LibcObservation, Observation, PackageManager, RequirementKind, evaluate,
    };
    use pixi_sandbox_core::platform::HostFamily;
    use std::cell::RefCell;

    /// A probe that answers from tables and counts what it was asked.
    struct FakeProbe {
        family: Option<HostFamily>,
        distro: Distro,
        libc: LibcObservation,
        packages: Vec<(String, Observation)>,
        services: Vec<(String, Observation)>,
        display: Observation,
        gpu: Observation,
        programs: Vec<(String, Observation)>,
        calls: RefCell<Vec<String>>,
    }

    impl FakeProbe {
        /// The everything-is-there host, on apt, with glibc 2.39.
        fn satisfied() -> Self {
            FakeProbe {
                family: Some(HostFamily::Linux),
                distro: Distro {
                    id: "ubuntu".to_string(),
                    manager: PackageManager::Apt,
                },
                libc: LibcObservation {
                    version: Some(LibcFloor::parse("2.39").unwrap()),
                    detail: "ldd reports 2.39".to_string(),
                },
                packages: Vec::new(),
                services: Vec::new(),
                display: Observation::satisfied("DISPLAY=:99"),
                gpu: Observation::satisfied("/dev/dri/renderD128"),
                programs: Vec::new(),
                calls: RefCell::new(Vec::new()),
            }
        }

        fn with_libc(mut self, version: Option<&str>, detail: &str) -> Self {
            self.libc = LibcObservation {
                version: version.map(|v| LibcFloor::parse(v).unwrap()),
                detail: detail.to_string(),
            };
            self
        }

        fn with_package(mut self, name: &str, observation: Observation) -> Self {
            self.packages.push((name.to_string(), observation));
            self
        }

        fn with_service(mut self, name: &str, observation: Observation) -> Self {
            self.services.push((name.to_string(), observation));
            self
        }

        fn with_program(mut self, name: &str, observation: Observation) -> Self {
            self.programs.push((name.to_string(), observation));
            self
        }

        fn with_display(mut self, observation: Observation) -> Self {
            self.display = observation;
            self
        }

        fn manager(mut self, manager: PackageManager) -> Self {
            self.distro = Distro {
                id: "some-linux".to_string(),
                manager,
            };
            self
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    fn answer(table: &[(String, Observation)], name: &str) -> Observation {
        table.iter().find(|(key, _)| key == name).map_or_else(
            || Observation::missing(format!("{name} is not present on this host")),
            |(_, observation)| observation.clone(),
        )
    }

    impl HostProbe for FakeProbe {
        fn host_family(&self) -> Option<HostFamily> {
            self.family
        }

        fn distro(&self) -> Distro {
            self.distro.clone()
        }

        fn libc(&self) -> LibcObservation {
            self.calls.borrow_mut().push("libc".to_string());
            self.libc.clone()
        }

        fn package(&self, name: &str) -> Observation {
            self.calls.borrow_mut().push(format!("package:{name}"));
            answer(&self.packages, name)
        }

        fn service(&self, name: &str) -> Observation {
            self.calls.borrow_mut().push(format!("service:{name}"));
            answer(&self.services, name)
        }

        fn display(&self) -> Observation {
            self.calls.borrow_mut().push("display".to_string());
            self.display.clone()
        }

        fn gpu(&self) -> Observation {
            self.calls.borrow_mut().push("gpu".to_string());
            self.gpu.clone()
        }

        fn program(&self, name: &str) -> Observation {
            self.calls.borrow_mut().push(format!("program:{name}"));
            answer(&self.programs, name)
        }
    }

    /// The issue #109 declaration: a GUI workload with fonts, a virtual display and D-Bus.
    fn gui_set() -> HostRequirementSet {
        let declared: HostRequirements = toml::from_str(
            r#"
libc = ">=2.34"
services = ["dbus"]
capabilities = ["display"]

[linux]
packages = ["fontconfig", "fonts-dejavu", "xvfb"]
headless = ["xvfb-run"]
"#,
        )
        .unwrap();
        declared.resolved_for(HostFamily::Linux)
    }

    fn status_of<'a>(
        report: &'a pixi_sandbox_core::host_requirements::HostReport,
        kind: RequirementKind,
        name: &str,
    ) -> &'a pixi_sandbox_core::host_requirements::HostFinding {
        report
            .findings
            .iter()
            .find(|finding| finding.kind == kind && finding.name == name)
            .unwrap_or_else(|| panic!("no {kind:?} finding for {name}: {:#?}", report.findings))
    }

    #[test]
    fn a_fully_satisfied_host_reports_satisfied_without_remedies() {
        let probe = FakeProbe::satisfied()
            .with_package("fontconfig", Observation::satisfied("dpkg: installed"))
            .with_package("fonts-dejavu", Observation::satisfied("dpkg: installed"))
            .with_package("xvfb", Observation::satisfied("dpkg: installed"))
            .with_service("dbus", Observation::satisfied("systemctl: active"))
            .with_program("xvfb-run", Observation::satisfied("on PATH"));

        let report = evaluate(&gui_set(), Some(HostFamily::Linux), &probe);
        assert!(report.applicable());
        assert!(report.ok());
        assert_eq!(report.missing(), 0);
        assert_eq!(report.unknown(), 0);
        assert_eq!(report.satisfied(), report.findings.len());
        assert!(
            report
                .findings
                .iter()
                .all(|finding| finding.remedy.is_none()),
            "a satisfied requirement needs no remedy: {:#?}",
            report.findings
        );
    }

    #[test]
    fn missing_packages_and_services_carry_per_distro_remedies() {
        let probe = FakeProbe::satisfied()
            .with_package("fontconfig", Observation::missing("dpkg: not installed"))
            .with_program("xvfb-run", Observation::missing("not on PATH"))
            .with_service("dbus", Observation::missing("systemctl: inactive"))
            .with_display(Observation::missing(
                "DISPLAY and WAYLAND_DISPLAY are unset",
            ));

        let report = evaluate(&gui_set(), Some(HostFamily::Linux), &probe);
        assert_eq!(report.missing(), 6, "{:#?}", report.findings);
        assert!(!report.ok());

        let fontconfig = status_of(&report, RequirementKind::Package, "fontconfig");
        assert_eq!(fontconfig.status, HostStatus::Missing);
        assert_eq!(
            fontconfig.remedy.as_deref(),
            Some("install it: sudo apt install fontconfig")
        );

        let fonts = status_of(&report, RequirementKind::Package, "fonts-dejavu");
        assert_eq!(
            fonts.remedy.as_deref(),
            Some("install it: sudo apt install fonts-dejavu-core"),
            "curated names must resolve to this distribution's package"
        );

        let xvfb = status_of(&report, RequirementKind::Package, "xvfb");
        assert_eq!(
            xvfb.remedy.as_deref(),
            Some("install it: sudo apt install xvfb")
        );

        let dbus = status_of(&report, RequirementKind::Service, "dbus");
        let remedy = dbus.remedy.as_deref().unwrap();
        assert!(remedy.contains("systemctl enable --now dbus"), "{remedy}");
        assert!(
            remedy.contains("dbus-run-session"),
            "a GUI workload needs the session-bus hint: {remedy}"
        );

        let display = status_of(&report, RequirementKind::Capability, "display");
        let remedy = display.remedy.as_deref().unwrap();
        assert!(remedy.contains("sudo apt install xvfb"), "{remedy}");
        assert!(remedy.contains("xvfb-run"), "{remedy}");
    }

    #[test]
    fn fedora_style_hosts_get_their_own_package_names() {
        let probe = FakeProbe::satisfied()
            .manager(PackageManager::Dnf)
            .with_package("xvfb", Observation::missing("rpm: not installed"));

        let report = evaluate(
            &HostRequirementSet {
                packages: vec!["xvfb".to_string()],
                ..HostRequirementSet::default()
            },
            Some(HostFamily::Linux),
            &probe,
        );

        assert_eq!(
            status_of(&report, RequirementKind::Package, "xvfb")
                .remedy
                .as_deref(),
            Some("install it: sudo dnf install xorg-x11-server-Xvfb")
        );
    }

    #[test]
    fn an_unsupported_manager_degrades_to_manual_guidance() {
        let probe = FakeProbe::satisfied()
            .manager(PackageManager::Unknown)
            .with_package("fontconfig", Observation::missing("no manager"));

        let report = evaluate(
            &HostRequirementSet {
                packages: vec!["fontconfig".to_string()],
                ..HostRequirementSet::default()
            },
            Some(HostFamily::Linux),
            &probe,
        );

        let remedy = status_of(&report, RequirementKind::Package, "fontconfig")
            .remedy
            .as_deref()
            .unwrap();
        assert!(
            remedy.contains("with this host's package manager"),
            "{remedy}"
        );
    }

    #[test]
    fn an_uncurated_package_keeps_its_declared_name_and_says_so() {
        let probe = FakeProbe::satisfied()
            .with_package("libfoo", Observation::missing("dpkg: not installed"));

        let report = evaluate(
            &HostRequirementSet {
                packages: vec!["libfoo".to_string()],
                ..HostRequirementSet::default()
            },
            Some(HostFamily::Linux),
            &probe,
        );

        let remedy = status_of(&report, RequirementKind::Package, "libfoo")
            .remedy
            .as_deref()
            .unwrap();
        assert!(remedy.contains("sudo apt install libfoo"), "{remedy}");
        assert!(
            remedy.contains("names differ between distributions"),
            "an uncurated name must carry the caveat: {remedy}"
        );
    }

    #[test]
    fn a_host_without_the_queries_reports_unknown_and_never_fails() {
        // Every requirement unanswered, the way a minimal container answers them: the probe
        // never guesses, and the report must turn that into `unknown` for each one.
        let probe = FakeProbe::satisfied()
            .manager(PackageManager::Unknown)
            .with_libc(None, "ldd and getconf are both unavailable")
            .with_package(
                "fontconfig",
                Observation::unknown("no package query available"),
            )
            .with_package(
                "fonts-dejavu",
                Observation::unknown("no package query available"),
            )
            .with_package("xvfb", Observation::unknown("no package query available"))
            .with_service("dbus", Observation::unknown("no systemctl on this host"))
            .with_program(
                "xvfb-run",
                Observation::unknown("no PATH inspection available"),
            )
            .with_display(Observation::unknown(
                "DISPLAY is unset and no display server answered",
            ));

        let report = evaluate(&gui_set(), Some(HostFamily::Linux), &probe);
        assert_eq!(report.missing(), 0, "{:#?}", report.findings);
        assert_eq!(report.unknown(), report.findings.len());
        assert!(
            report.ok(),
            "an unknown must never fail the gate: {:#?}",
            report.findings
        );
        assert_eq!(
            status_of(&report, RequirementKind::Libc, ">=2.34")
                .remedy
                .as_deref(),
            Some("check the host C runtime directly with `ldd --version`")
        );
    }

    #[test]
    fn an_older_host_libc_is_missing_and_says_why() {
        let probe = FakeProbe::satisfied().with_libc(Some("2.31"), "ldd reports 2.31");

        let report = evaluate(
            &HostRequirementSet {
                libc: Some(">=2.34".to_string()),
                ..HostRequirementSet::default()
            },
            Some(HostFamily::Linux),
            &probe,
        );

        let finding = status_of(&report, RequirementKind::Libc, ">=2.34");
        assert_eq!(finding.status, HostStatus::Missing);
        assert!(finding.detail.contains("2.31"), "{}", finding.detail);
        assert!(
            finding
                .remedy
                .as_deref()
                .unwrap()
                .contains("cannot be installed per project"),
            "a libc floor needs an honest remedy: {:?}",
            finding.remedy
        );
    }

    #[test]
    fn a_headless_provider_satisfies_the_display_capability() {
        let probe = FakeProbe::satisfied()
            .with_display(Observation::missing(
                "DISPLAY and WAYLAND_DISPLAY are unset",
            ))
            .with_program("xvfb-run", Observation::satisfied("on PATH"));

        let report = evaluate(
            &HostRequirementSet {
                capabilities: vec![HostCapability::Display],
                headless: vec!["xvfb-run".to_string()],
                ..HostRequirementSet::default()
            },
            Some(HostFamily::Linux),
            &probe,
        );

        let display = status_of(&report, RequirementKind::Capability, "display");
        assert_eq!(display.status, HostStatus::Satisfied);
        assert!(
            display.detail.contains("xvfb-run"),
            "the report must name the provider that satisfied it: {}",
            display.detail
        );
    }

    #[test]
    fn a_missing_display_with_no_declared_provider_says_what_to_declare() {
        let probe = FakeProbe::satisfied().with_display(Observation::missing(
            "DISPLAY and WAYLAND_DISPLAY are unset",
        ));

        let report = evaluate(
            &HostRequirementSet {
                capabilities: vec![HostCapability::Display],
                ..HostRequirementSet::default()
            },
            Some(HostFamily::Linux),
            &probe,
        );

        let finding = status_of(&report, RequirementKind::Capability, "display");
        assert_eq!(finding.status, HostStatus::Missing);
        assert!(
            finding.detail.contains("no headless provider is declared"),
            "{}",
            finding.detail
        );
    }

    #[test]
    fn the_gpu_capability_is_reported_never_granted() {
        let probe = FakeProbe::satisfied().with_package("fontconfig", Observation::satisfied("ok"));
        let declared = HostRequirementSet {
            capabilities: vec![HostCapability::Gpu],
            ..HostRequirementSet::default()
        };
        let mut probe = probe;
        probe.gpu = Observation::missing("/dev/dri has no device nodes");

        let report = evaluate(&declared, Some(HostFamily::Linux), &probe);
        let gpu = status_of(&report, RequirementKind::Capability, "gpu");
        assert_eq!(gpu.status, HostStatus::Missing);
        assert!(
            gpu.remedy
                .as_deref()
                .unwrap()
                .contains("cannot be provided from inside the environment"),
            "{:?}",
            gpu.remedy
        );
    }

    #[test]
    fn a_transport_for_another_family_is_not_applicable_and_probes_nothing() {
        let probe = FakeProbe {
            family: Some(HostFamily::Osx),
            ..FakeProbe::satisfied()
        };

        let report = evaluate(&gui_set(), Some(HostFamily::Linux), &probe);
        assert!(!report.applicable());
        assert!(report.ok(), "not-applicable must never fail");
        assert_eq!(report.findings.len(), 7);
        assert!(
            report
                .findings
                .iter()
                .all(|finding| finding.status == HostStatus::NotApplicable
                    && finding.detail.contains("declared for linux hosts")
                    && finding.remedy.is_none())
        );
        assert!(
            probe.calls().is_empty(),
            "nothing on the host may be queried for a transport of another family: {:?}",
            probe.calls()
        );
    }

    #[test]
    fn an_unrecognised_transport_platform_reports_unknown_rather_than_guessing() {
        let probe = FakeProbe::satisfied();
        let report = evaluate(&gui_set(), None, &probe);
        assert!(!report.applicable());
        assert!(report.ok());
        assert!(
            report
                .findings
                .iter()
                .all(|finding| finding.status == HostStatus::NotApplicable
                    && finding.detail.contains("does not recognise"))
        );
    }

    #[test]
    fn an_absent_declaration_produces_an_empty_report() {
        let probe = FakeProbe::satisfied();
        let report = evaluate(
            &HostRequirementSet::default(),
            Some(HostFamily::Linux),
            &probe,
        );
        assert!(report.findings.is_empty());
        assert!(report.ok());
        assert_eq!(report.missing(), 0);
    }

    #[test]
    fn findings_come_back_in_declaration_order() {
        let probe = FakeProbe::satisfied()
            .with_package("fontconfig", Observation::satisfied("ok"))
            .with_package("xvfb", Observation::satisfied("ok"))
            .with_service("dbus", Observation::satisfied("ok"))
            .with_program("xvfb-run", Observation::satisfied("ok"));

        let report = evaluate(&gui_set(), Some(HostFamily::Linux), &probe);
        let kinds: Vec<&str> = report
            .findings
            .iter()
            .map(|finding| finding.kind.as_str())
            .collect();
        assert_eq!(
            kinds,
            [
                "libc",
                "package",
                "package",
                "package",
                "service",
                "capability",
                "headless"
            ]
        );
        let names: Vec<&str> = report
            .findings
            .iter()
            .map(|finding| finding.name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                ">=2.34",
                "fontconfig",
                "fonts-dejavu",
                "xvfb",
                "dbus",
                "display",
                "xvfb-run"
            ]
        );
    }

    #[test]
    fn the_gui_runtime_names_resolve_per_distribution() {
        // The two names a Tauri/WebKit workload declares: the remedy must name the package the
        // *host's* distribution actually ships, because that is the difference between a
        // copy-pasteable command and a puzzle.
        for (declared, manager, expected) in [
            (
                "libgtk-3",
                PackageManager::Apt,
                "install it: sudo apt install libgtk-3-0",
            ),
            (
                "gtk3",
                PackageManager::Dnf,
                "install it: sudo dnf install gtk3",
            ),
            (
                "webkit2gtk",
                PackageManager::Apt,
                "install it: sudo apt install libwebkit2gtk-4.1-0",
            ),
            (
                "libwebkit2gtk",
                PackageManager::Dnf,
                "install it: sudo dnf install webkit2gtk4.1",
            ),
        ] {
            let probe = FakeProbe::satisfied()
                .manager(manager)
                .with_package(declared, Observation::missing("not installed"));
            let report = evaluate(
                &HostRequirementSet {
                    packages: vec![declared.to_string()],
                    ..HostRequirementSet::default()
                },
                Some(HostFamily::Linux),
                &probe,
            );
            assert_eq!(
                status_of(&report, RequirementKind::Package, declared)
                    .remedy
                    .as_deref(),
                Some(expected),
                "for {declared} on {manager}"
            );
        }
    }

    #[test]
    fn os_release_parsing_covers_the_named_distributions_and_degrades_elsewhere() {
        let ubuntu = Distro::parse_os_release(
            "NAME=\"Ubuntu\"\nID=ubuntu\nID_LIKE=debian\nVERSION_ID=\"24.04\"\n",
        );
        assert_eq!(ubuntu.manager, PackageManager::Apt);
        assert_eq!(ubuntu.id, "ubuntu");

        // Mint advertises Debian only through ID_LIKE.
        let mint = Distro::parse_os_release("ID=linuxmint\nID_LIKE=\"ubuntu debian\"\n");
        assert_eq!(mint.manager, PackageManager::Apt);

        let fedora = Distro::parse_os_release("NAME=\"Fedora Linux\"\nID=fedora\n");
        assert_eq!(fedora.manager, PackageManager::Dnf);

        let rocky = Distro::parse_os_release("ID=\"rocky\"\nID_LIKE=\"rhel centos fedora\"\n");
        assert_eq!(rocky.manager, PackageManager::Dnf);

        let arch = Distro::parse_os_release("ID=arch\nID_LIKE=archlinux\n");
        assert_eq!(arch.manager, PackageManager::Pacman);

        // Alpine is musl-based and uses apk: no supported manager, so no install command —
        // which is exactly the "explicit manual guidance" the acceptance criteria require.
        let alpine = Distro::parse_os_release("ID=alpine\n");
        assert_eq!(alpine.manager, PackageManager::Unknown);
        assert_eq!(alpine.manager.install_command("xvfb"), None);

        let empty = Distro::parse_os_release("");
        assert_eq!(empty, Distro::unknown());
    }
}
