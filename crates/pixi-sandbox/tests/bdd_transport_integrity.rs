//! BDD feature: transport integrity (task-86, bound with the rstest-bdd attribute macros by
//! task-87).
//!
//! Scenarios live in `tests/features/transport_integrity.feature`, byte-identical to the
//! sentences this file was written against; each one is bound below by `#[scenario]`, which
//! emits the rstest test itself, so the names nextest reports did not move. Steps drive
//! `doctor --verify` black-box through the `tests/support` fixture builders, against tempdir
//! copies of the fixture transport (D10).

mod support;

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::path::PathBuf;
use std::process::Output;
use support::bdd::assert_every_scenario_is_bound;
use support::{bin, copy_tree, fixture_transport, host_platform};
use tempfile::TempDir;

/// One scenario's transport copy and the captured doctor verdict.
#[derive(Default)]
struct TransportWorld {
    root: Option<TempDir>,
    transport: Option<PathBuf>,
    result: Option<Output>,
}

impl TransportWorld {
    fn staged(&mut self) {
        let root = tempfile::tempdir().expect("scenario tempdir");
        let transport = root.path().join("transport");
        copy_tree(&fixture_transport(), &transport);
        self.root = Some(root);
        self.transport = Some(transport);
    }

    fn transport_path(&self) -> &std::path::Path {
        self.transport
            .as_ref()
            .expect("the scenario staged a transport")
    }

    fn output(&self) -> &Output {
        self.result.as_ref().expect("the scenario ran a command")
    }
}

/// The world every step in this binary borrows: zero-argument, per the `tests/support`
/// convention. rstest builds one instance per scenario, the steps share it, and it drops —
/// tempdir and all — when the scenario ends.
#[fixture]
fn transport_world() -> TransportWorld {
    TransportWorld::default()
}

// --- Given -----------------------------------------------------------------

#[given("a packed transport")]
fn a_packed_transport(transport_world: &mut TransportWorld) {
    transport_world.staged();
}

#[given("a packed transport carrying one tampered byte")]
fn a_packed_transport_carrying_one_tampered_byte(transport_world: &mut TransportWorld) {
    transport_world.staged();
    // The same blob the doctor tests in cli.rs tamper: one package inside the packed channel.
    let tampered = transport_world
        .transport_path()
        .join(".pixi-sandbox/envs/demo/pack/channel/noarch/demo-pure-0.1.0-0.conda");
    std::fs::write(&tampered, b"tampered").expect("the blob tampers");
}

#[given("a packed transport carrying a tampered shard part")]
fn a_packed_transport_carrying_a_tampered_shard_part(transport_world: &mut TransportWorld) {
    transport_world.staged();
    let tampered = transport_world
        .transport_path()
        .join(".pixi-sandbox/envs/demo/pack/channel/noarch/demo-big-0.1.0-0.conda.part001");
    std::fs::write(&tampered, b"tampered-part").expect("the shard part tampers");
}

// --- When ------------------------------------------------------------------

#[when("the operator verifies the transport")]
fn the_operator_verifies_the_transport(transport_world: &mut TransportWorld) {
    let transport = transport_world.transport_path().to_path_buf();
    let output = bin()
        .args([
            "doctor",
            "--branch-location",
            transport.to_str().expect("utf-8 path"),
            "--verify",
        ])
        .output()
        .expect("doctor runs");
    transport_world.result = Some(output);
}

// --- Then ------------------------------------------------------------------

#[then("every declared byte is reported to match")]
fn every_declared_byte_is_reported_to_match(transport_world: &TransportWorld) {
    let output = transport_world.output();
    assert!(output.status.success(), "doctor --verify must pass");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("OK — every declared byte matches"),
        "the healthy verdict must reach the operator: {stdout}"
    );
}

#[then("the verdict is honest about the embedded bootstrap probe")]
fn the_verdict_is_honest_about_the_embedded_bootstrap_probe(transport_world: &TransportWorld) {
    let stdout = String::from_utf8_lossy(&transport_world.output().stdout);
    let probe = "tool pixi-sandbox v0.1.0: runs standalone";
    if host_platform() == "linux-64" {
        // On the fixture's own platform, hashes bless the bytes but only execution blesses
        // the behaviour (issue #81), so the probe line must be present.
        assert!(
            stdout.contains(probe),
            "the fixture platform must prove the embedded bootstrap runs: {stdout}"
        );
    } else {
        assert!(
            !stdout.contains(probe),
            "a foreign platform must not claim the probe ran: {stdout}"
        );
    }
}

#[then("the verification is refused")]
fn the_verification_is_refused(transport_world: &TransportWorld) {
    assert!(
        !transport_world.output().status.success(),
        "a tampered transport must fail verification"
    );
}

#[then("no healthy verdict is reported")]
fn no_healthy_verdict_is_reported(transport_world: &TransportWorld) {
    let stdout = String::from_utf8_lossy(&transport_world.output().stdout);
    assert!(
        !stdout.contains("OK"),
        "a failed verification must not print the healthy verdict: {stdout}"
    );
}

#[then("the refusal names the tampered blob")]
fn the_refusal_names_the_tampered_blob(transport_world: &TransportWorld) {
    let stdout = String::from_utf8_lossy(&transport_world.output().stdout);
    assert!(
        stdout.contains("demo-pure-0.1.0-0.conda"),
        "the failure list must name the blob that failed: {stdout}"
    );
}

#[then("the refusal names the tampered part")]
fn the_refusal_names_the_tampered_part(transport_world: &TransportWorld) {
    let stdout = String::from_utf8_lossy(&transport_world.output().stdout);
    assert!(
        stdout.contains(".part001"),
        "the failure list must name the shard part that failed: {stdout}"
    );
}

// --- Scenarios -------------------------------------------------------------

#[scenario(
    path = "tests/features/transport_integrity.feature",
    name = "A packed transport round-trips through verification"
)]
fn scenario_a_packed_transport_round_trips_through_verification(transport_world: TransportWorld) {}

#[scenario(
    path = "tests/features/transport_integrity.feature",
    name = "A tampered byte loses the healthy verdict"
)]
fn scenario_a_tampered_byte_loses_the_healthy_verdict(transport_world: TransportWorld) {}

#[scenario(
    path = "tests/features/transport_integrity.feature",
    name = "A tampered shard part is named in the refusal"
)]
fn scenario_a_tampered_shard_part_is_named_in_the_refusal(transport_world: TransportWorld) {}

/// The guard that replaces the retired duplicate-registry check: with `#[scenario]` the binding
/// *is* the test, so this is what notices a scenario the suite silently stopped running. The
/// duplicate half of the old guard is the macros' job now, at compile time.
#[test]
fn every_scenario_in_the_feature_file_is_bound() {
    assert_every_scenario_is_bound(
        "tests/features/transport_integrity.feature",
        "tests/bdd_transport_integrity.rs",
    );
}
