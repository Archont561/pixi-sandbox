//! BDD feature: transport integrity (task-86).
//!
//! Scenarios live in `tests/features/transport_integrity.feature`; each one is bound to the
//! suite as a plain test below. Steps drive `doctor --verify` black-box through the
//! `tests/support` fixture builders, against tempdir copies of the fixture transport (D10).

mod support;

use rstest_bdd::{StepContext, StepError, StepExecution, StepKeyword, step};
use std::path::PathBuf;
use std::process::Output;
use support::bdd::{WORLD, run_scenario};
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

fn world<'a>(ctx: &'a StepContext<'_>) -> impl std::ops::DerefMut<Target = TransportWorld> + 'a {
    ctx.try_borrow_mut::<TransportWorld>(WORLD)
        .expect("the runner inserts the world")
}

// --- Given -----------------------------------------------------------------

fn a_packed_transport(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    world(ctx).staged();
    Ok(StepExecution::from_value(None))
}

fn a_packed_transport_carrying_one_tampered_byte(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let mut transport_world = world(ctx);
    transport_world.staged();
    // The same blob the doctor tests in cli.rs tamper: one package inside the packed channel.
    let tampered = transport_world
        .transport_path()
        .join(".pixi-sandbox/envs/demo/pack/channel/noarch/demo-pure-0.1.0-0.conda");
    std::fs::write(&tampered, b"tampered").expect("the blob tampers");
    Ok(StepExecution::from_value(None))
}

fn a_packed_transport_carrying_a_tampered_shard_part(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let mut transport_world = world(ctx);
    transport_world.staged();
    let tampered = transport_world
        .transport_path()
        .join(".pixi-sandbox/envs/demo/pack/channel/noarch/demo-big-0.1.0-0.conda.part001");
    std::fs::write(&tampered, b"tampered-part").expect("the shard part tampers");
    Ok(StepExecution::from_value(None))
}

// --- When ------------------------------------------------------------------

fn the_operator_verifies_the_transport(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let mut transport_world = world(ctx);
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
    Ok(StepExecution::from_value(None))
}

// --- Then ------------------------------------------------------------------

fn every_declared_byte_is_reported_to_match(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let transport_world = world(ctx);
    let output = transport_world.output();
    assert!(output.status.success(), "doctor --verify must pass");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("OK — every declared byte matches"),
        "the healthy verdict must reach the operator: {stdout}"
    );
    Ok(StepExecution::from_value(None))
}

fn the_verdict_is_honest_about_the_embedded_bootstrap_probe(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let transport_world = world(ctx);
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
    Ok(StepExecution::from_value(None))
}

fn the_verification_is_refused(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let transport_world = world(ctx);
    assert!(
        !transport_world.output().status.success(),
        "a tampered transport must fail verification"
    );
    Ok(StepExecution::from_value(None))
}

fn no_healthy_verdict_is_reported(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let transport_world = world(ctx);
    let stdout = String::from_utf8_lossy(&transport_world.output().stdout);
    assert!(
        !stdout.contains("OK"),
        "a failed verification must not print the healthy verdict: {stdout}"
    );
    Ok(StepExecution::from_value(None))
}

fn the_refusal_names_the_tampered_blob(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let transport_world = world(ctx);
    let stdout = String::from_utf8_lossy(&transport_world.output().stdout);
    assert!(
        stdout.contains("demo-pure-0.1.0-0.conda"),
        "the failure list must name the blob that failed: {stdout}"
    );
    Ok(StepExecution::from_value(None))
}

fn the_refusal_names_the_tampered_part(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let transport_world = world(ctx);
    let stdout = String::from_utf8_lossy(&transport_world.output().stdout);
    assert!(
        stdout.contains(".part001"),
        "the failure list must name the shard part that failed: {stdout}"
    );
    Ok(StepExecution::from_value(None))
}

// --- Registry --------------------------------------------------------------

step!(
    StepKeyword::Given,
    "a packed transport",
    a_packed_transport,
    &[WORLD]
);
step!(
    StepKeyword::Given,
    "a packed transport carrying one tampered byte",
    a_packed_transport_carrying_one_tampered_byte,
    &[WORLD]
);
step!(
    StepKeyword::Given,
    "a packed transport carrying a tampered shard part",
    a_packed_transport_carrying_a_tampered_shard_part,
    &[WORLD]
);
step!(
    StepKeyword::When,
    "the operator verifies the transport",
    the_operator_verifies_the_transport,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "every declared byte is reported to match",
    every_declared_byte_is_reported_to_match,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "the verdict is honest about the embedded bootstrap probe",
    the_verdict_is_honest_about_the_embedded_bootstrap_probe,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "the verification is refused",
    the_verification_is_refused,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "no healthy verdict is reported",
    no_healthy_verdict_is_reported,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "the refusal names the tampered blob",
    the_refusal_names_the_tampered_blob,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "the refusal names the tampered part",
    the_refusal_names_the_tampered_part,
    &[WORLD]
);

// --- Scenarios -------------------------------------------------------------

#[test]
fn scenario_a_packed_transport_round_trips_through_verification() {
    run_scenario::<TransportWorld>(
        "tests/features/transport_integrity.feature",
        "A packed transport round-trips through verification",
    );
}

#[test]
fn scenario_a_tampered_byte_loses_the_healthy_verdict() {
    run_scenario::<TransportWorld>(
        "tests/features/transport_integrity.feature",
        "A tampered byte loses the healthy verdict",
    );
}

#[test]
fn scenario_a_tampered_shard_part_is_named_in_the_refusal() {
    run_scenario::<TransportWorld>(
        "tests/features/transport_integrity.feature",
        "A tampered shard part is named in the refusal",
    );
}

#[test]
fn the_step_registry_has_no_duplicate_definitions() {
    support::bdd::assert_no_duplicate_steps();
}
