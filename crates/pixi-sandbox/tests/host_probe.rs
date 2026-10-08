//! Unit tests for the host probe's parse and classification helpers
//! (`src/host_probe.rs`, TASK-75 AC#3/#4).
//!
//! Every case here feeds captured real-world output to the pure helper that interprets it, so
//! the suite asserts what the classification *means* without touching the machine it runs on.
//! The process-spawning wrappers around these helpers stay thin enough to read.

use pixi_sandbox::host_probe::{
    classify_dbus, classify_display, classify_dpkg, classify_gpu, classify_rpm_like,
    classify_systemctl, parse_libc_version,
};
use pixi_sandbox_core::host_requirements::{LibcFloor, Outcome};

#[test]
fn ubuntu_ldd_banner_yields_the_glibc_version_not_the_package_revision() {
    let observation = parse_libc_version("ldd (Ubuntu GLIBC 2.39-0ubuntu8.3) 2.39\n");
    assert_eq!(observation.version, Some(LibcFloor::parse("2.39").unwrap()));
    assert!(observation.detail.contains("2.39-0ubuntu8.3"));
}

#[test]
fn a_plain_gnu_banner_and_the_getconf_fallback_parse() {
    assert_eq!(
        parse_libc_version("ldd (GNU libc) 2.34\n").version,
        Some(LibcFloor::parse("2.34").unwrap())
    );
    assert_eq!(
        parse_libc_version("glibc 2.39\n").version,
        Some(LibcFloor::parse("2.39").unwrap())
    );
}

#[test]
fn a_musl_host_reports_its_own_version_which_compares_as_older() {
    let observation = parse_libc_version("musl libc (x86_64)\nVersion 1.2.4\n");
    let version = observation.version.expect("musl reports a version");
    assert_eq!(version, LibcFloor::parse("1.2.4").unwrap());
    assert!(
        !LibcFloor::parse("2.34").unwrap().is_met_by(version),
        "a musl version must not satisfy a glibc floor"
    );
}

#[test]
fn an_unparsable_banner_is_no_version_rather_than_a_guess() {
    let observation = parse_libc_version("ldd: not found\n");
    assert_eq!(observation.version, None);
    assert!(observation.detail.contains("no parsable version"));
}

#[test]
fn dpkg_outputs_map_to_three_outcomes() {
    assert_eq!(
        classify_dpkg("install ok installed").outcome,
        Outcome::Satisfied
    );
    assert_eq!(classify_dpkg("").outcome, Outcome::Missing);
    assert_eq!(
        classify_dpkg("deinstall ok config-files").outcome,
        Outcome::Missing
    );
}

#[test]
fn rpm_and_pacman_outputs_map_to_two_outcomes() {
    assert_eq!(
        classify_rpm_like("rpm", "fontconfig-2.14.2-1.fc40.x86_64", "").outcome,
        Outcome::Satisfied
    );
    assert_eq!(
        classify_rpm_like("rpm", "", "package fontconfig is not installed").outcome,
        Outcome::Missing
    );
    assert_eq!(
        classify_rpm_like("pacman", "xvfb 21.1.13-1", "").outcome,
        Outcome::Satisfied
    );
}

#[test]
fn systemctl_states_map_to_satisfied_missing_and_unknown() {
    assert_eq!(classify_systemctl("active\n").outcome, Outcome::Satisfied);
    assert_eq!(classify_systemctl("inactive\n").outcome, Outcome::Missing);
    assert_eq!(classify_systemctl("failed\n").outcome, Outcome::Missing);
    assert_eq!(classify_systemctl("").outcome, Outcome::Unknown);
}

#[test]
fn dbus_is_satisfied_by_either_signal_and_missing_only_without_both() {
    assert_eq!(classify_dbus(true, false).outcome, Outcome::Satisfied);
    assert_eq!(classify_dbus(false, true).outcome, Outcome::Satisfied);
    let missing = classify_dbus(false, false);
    assert_eq!(missing.outcome, Outcome::Missing);
    assert!(missing.detail.contains("DBUS_SESSION_BUS_ADDRESS"));
}

#[test]
fn display_state_names_the_variable_it_saw() {
    assert_eq!(
        classify_display(Some(":0"), None).detail,
        "DISPLAY=:0".to_string()
    );
    assert_eq!(
        classify_display(None, Some("wayland-0")).detail,
        "WAYLAND_DISPLAY=wayland-0".to_string()
    );
    // An empty variable is "unset" in practice: exported-but-blank is how a container
    // hides a display it does not have.
    assert_eq!(classify_display(Some(""), None).outcome, Outcome::Missing);
}

#[test]
fn gpu_state_follows_the_device_directory() {
    let directory = tempfile::tempdir().unwrap();
    assert_eq!(
        classify_gpu(directory.path()).outcome,
        Outcome::Missing,
        "an empty /dev/dri is a missing GPU"
    );
    std::fs::write(directory.path().join("renderD128"), b"").unwrap();
    assert_eq!(classify_gpu(directory.path()).outcome, Outcome::Satisfied);
    assert_eq!(
        classify_gpu(&directory.path().join("absent")).outcome,
        Outcome::Missing
    );
}
