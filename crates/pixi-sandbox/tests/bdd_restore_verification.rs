//! BDD feature: restore verification (task-86, bound with the rstest-bdd attribute macros by
//! task-87).
//!
//! Scenarios live in `tests/features/restore_verification.feature`, byte-identical to the
//! sentences this file was written against; each one is bound below by `#[scenario]`, which
//! emits the rstest test itself, so the names nextest reports did not move. Steps drive the
//! binary black-box through the `tests/support` fixture builders and never point at this
//! repository or a real HOME (D10).

mod support;

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::path::PathBuf;
use std::process::Output;
use support::bdd::assert_every_scenario_is_bound;
use support::{copy_tree, fixture_transport, isolated_bin};
use tempfile::TempDir;

/// Everything one scenario owns: a tempdir root, the copied transport, the airlock project,
/// the isolated home, and the captured outcome of the command under test.
#[derive(Default)]
struct RestoreWorld {
    root: Option<TempDir>,
    transport: Option<PathBuf>,
    project: Option<PathBuf>,
    home: Option<PathBuf>,
    branch_location_override: Option<PathBuf>,
    result: Option<Output>,
}

impl RestoreWorld {
    fn staged(&mut self) {
        let root = tempfile::tempdir().expect("scenario tempdir");
        let transport = root.path().join("transport");
        copy_tree(&fixture_transport(), &transport);
        let project = root.path().join("airlock-project");
        std::fs::create_dir_all(&project).expect("project directory");
        let home = root.path().join("home");
        std::fs::create_dir_all(&home).expect("home directory");
        self.root = Some(root);
        self.transport = Some(transport);
        self.project = Some(project);
        self.home = Some(home);
    }

    fn restore_command(&self, extra_args: &[&str]) -> Output {
        let location = self
            .branch_location_override
            .clone()
            .or_else(|| self.transport.clone())
            .expect("the scenario staged a branch location");
        let project = self
            .project
            .as_ref()
            .expect("the scenario staged a project");
        let home = self.home.as_ref().expect("the scenario staged a home");
        let mut command = isolated_bin(home);
        command
            .arg("restore")
            .arg("--branch-location")
            .arg(&location);
        command.arg("--output-path").arg(project);
        command.args(extra_args);
        command.output().expect("restore runs")
    }

    fn project_path(&self) -> &std::path::Path {
        self.project
            .as_ref()
            .expect("the scenario staged a project")
    }

    fn output(&self) -> &Output {
        self.result.as_ref().expect("the scenario ran a command")
    }
}

/// The world every step in this binary borrows: zero-argument, per the `tests/support`
/// convention. rstest builds one instance per scenario, the steps share it, and it drops —
/// tempdir and all — when the scenario ends.
#[fixture]
fn restore_world() -> RestoreWorld {
    RestoreWorld::default()
}

// --- Given -----------------------------------------------------------------

#[given("a packed transport and an empty project")]
fn a_packed_transport_and_an_empty_project(restore_world: &mut RestoreWorld) {
    restore_world.staged();
}

#[given("a packed transport with one tampered byte")]
fn a_packed_transport_with_one_tampered_byte(restore_world: &mut RestoreWorld) {
    restore_world.staged();
    // The same blob the doctor tests in cli.rs tamper: one package inside the packed channel.
    let tampered = restore_world
        .transport
        .as_ref()
        .expect("staged transport")
        .join(".pixi-sandbox/envs/demo/pack/channel/noarch/demo-pure-0.1.0-0.conda");
    std::fs::write(&tampered, b"tampered").expect("the blob tampers");
}

#[given("a branch location that does not exist")]
fn a_branch_location_that_does_not_exist(restore_world: &mut RestoreWorld) {
    restore_world.staged();
    let missing = restore_world
        .root
        .as_ref()
        .expect("staged root")
        .path()
        .join("no-such-branch");
    restore_world.branch_location_override = Some(missing);
}

// --- When ------------------------------------------------------------------

#[when("the operator restores the transport into the project")]
fn the_operator_restores_the_transport_into_the_project(restore_world: &mut RestoreWorld) {
    let output = restore_world.restore_command(&[]);
    restore_world.result = Some(output);
}

#[when("the operator asks restore to verify only")]
fn the_operator_asks_restore_to_verify_only(restore_world: &mut RestoreWorld) {
    let output = restore_world.restore_command(&["--verify-only"]);
    restore_world.result = Some(output);
}

// --- Then ------------------------------------------------------------------

#[then("the restore reports completion")]
fn the_restore_reports_completion(restore_world: &RestoreWorld) {
    let output = restore_world.output();
    assert!(output.status.success(), "restore must succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("restore complete"),
        "the completion report must reach the operator: {stdout}"
    );
}

#[then("the demo environment is materialised in the project")]
fn the_demo_environment_is_materialised_in_the_project(restore_world: &RestoreWorld) {
    let prefix_marker = restore_world
        .project_path()
        .join(".pixi/envs/demo/conda-meta/pixi_env_prefix");
    assert!(
        prefix_marker.is_file(),
        "the restored demo environment must carry its prefix marker"
    );
}

#[then("no restore scratch is left behind")]
fn no_restore_scratch_is_left_behind(restore_world: &RestoreWorld) {
    assert!(
        !restore_world
            .project_path()
            .join(".pixi/.restore-work")
            .exists(),
        "restore scratch is not a deliverable"
    );
}

#[then("the restore is refused")]
fn the_restore_is_refused(restore_world: &RestoreWorld) {
    let output = restore_world.output();
    assert!(
        !output.status.success(),
        "a restore that cannot verify must fail; stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[then("the refusal reports that nothing was written")]
fn the_refusal_reports_that_nothing_was_written(restore_world: &RestoreWorld) {
    let stderr = String::from_utf8_lossy(&restore_world.output().stderr);
    assert!(
        stderr.contains("nothing was written"),
        "the refusal must say the project was left alone: {stderr}"
    );
}

#[then("the refusal names the tampered blob")]
fn the_refusal_names_the_tampered_blob(restore_world: &RestoreWorld) {
    let stderr = String::from_utf8_lossy(&restore_world.output().stderr);
    assert!(
        stderr.contains("demo-pure-0.1.0-0.conda"),
        "the failure list must name the blob that failed verification: {stderr}"
    );
}

#[then("the refusal names the branch location")]
fn the_refusal_names_the_branch_location(restore_world: &RestoreWorld) {
    let stderr = String::from_utf8_lossy(&restore_world.output().stderr);
    assert!(
        stderr.contains("--branch-location") && stderr.contains("no-such-branch"),
        "the refusal must name the flag and the missing location: {stderr}"
    );
}

#[then("the project receives no environment")]
fn the_project_receives_no_environment(restore_world: &RestoreWorld) {
    assert!(
        !restore_world
            .project_path()
            .join(".pixi/envs/demo")
            .exists(),
        "no environment may be materialised by a refused restore"
    );
}

#[then("the verdict reports that no project files were written")]
fn the_verdict_reports_that_no_project_files_were_written(restore_world: &RestoreWorld) {
    let output = restore_world.output();
    assert!(output.status.success(), "verify-only must succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--verify-only: no project files were written"),
        "the verdict must say nothing was written: {stdout}"
    );
}

// --- Scenarios -------------------------------------------------------------

#[scenario(
    path = "tests/features/restore_verification.feature",
    name = "A verified restore writes the project and cleans its scratch"
)]
fn scenario_a_verified_restore_writes_the_project_and_cleans_its_scratch(
    restore_world: RestoreWorld,
) {
}

#[scenario(
    path = "tests/features/restore_verification.feature",
    name = "A tampered transport is refused with nothing written"
)]
fn scenario_a_tampered_transport_is_refused_with_nothing_written(restore_world: RestoreWorld) {}

#[scenario(
    path = "tests/features/restore_verification.feature",
    name = "A missing transport is refused before writing"
)]
fn scenario_a_missing_transport_is_refused_before_writing(restore_world: RestoreWorld) {}

#[scenario(
    path = "tests/features/restore_verification.feature",
    name = "A verify-only run reports the verdict and writes nothing"
)]
fn scenario_a_verify_only_run_reports_the_verdict_and_writes_nothing(restore_world: RestoreWorld) {}

/// The guard that replaces the retired duplicate-registry check: with `#[scenario]` the binding
/// *is* the test, so this is what notices a scenario the suite silently stopped running. The
/// duplicate half of the old guard is the macros' job now, at compile time.
#[test]
fn every_scenario_in_the_feature_file_is_bound() {
    assert_every_scenario_is_bound(
        "tests/features/restore_verification.feature",
        "tests/bdd_restore_verification.rs",
    );
}
