//! BDD feature: restore verification (task-86).
//!
//! Scenarios live in `tests/features/restore_verification.feature`; each one is bound to the
//! suite as a plain test below. Steps drive the binary black-box through the `tests/support`
//! fixture builders and never point at this repository or a real HOME (D10).

mod support;

use rstest_bdd::{StepContext, StepError, StepExecution, StepKeyword, step};
use std::path::PathBuf;
use std::process::Output;
use support::bdd::{WORLD, run_scenario};
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

fn world<'a>(ctx: &'a StepContext<'_>) -> impl std::ops::DerefMut<Target = RestoreWorld> + 'a {
    ctx.try_borrow_mut::<RestoreWorld>(WORLD)
        .expect("the runner inserts the world")
}

// --- Given -----------------------------------------------------------------

fn a_packed_transport_and_an_empty_project(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    world(ctx).staged();
    Ok(StepExecution::from_value(None))
}

fn a_packed_transport_with_one_tampered_byte(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let mut restore_world = world(ctx);
    restore_world.staged();
    // The same blob the doctor tests in cli.rs tamper: one package inside the packed channel.
    let tampered = restore_world
        .transport
        .as_ref()
        .expect("staged transport")
        .join(".pixi-sandbox/envs/demo/pack/channel/noarch/demo-pure-0.1.0-0.conda");
    std::fs::write(&tampered, b"tampered").expect("the blob tampers");
    Ok(StepExecution::from_value(None))
}

fn a_branch_location_that_does_not_exist(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let mut restore_world = world(ctx);
    restore_world.staged();
    let missing = restore_world
        .root
        .as_ref()
        .expect("staged root")
        .path()
        .join("no-such-branch");
    restore_world.branch_location_override = Some(missing);
    Ok(StepExecution::from_value(None))
}

// --- When ------------------------------------------------------------------

fn the_operator_restores_the_transport_into_the_project(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let mut restore_world = world(ctx);
    let output = restore_world.restore_command(&[]);
    restore_world.result = Some(output);
    Ok(StepExecution::from_value(None))
}

fn the_operator_asks_restore_to_verify_only(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let mut restore_world = world(ctx);
    let output = restore_world.restore_command(&["--verify-only"]);
    restore_world.result = Some(output);
    Ok(StepExecution::from_value(None))
}

// --- Then ------------------------------------------------------------------

fn the_restore_reports_completion(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let restore_world = world(ctx);
    let output = restore_world.output();
    assert!(output.status.success(), "restore must succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("restore complete"),
        "the completion report must reach the operator: {stdout}"
    );
    Ok(StepExecution::from_value(None))
}

fn the_demo_environment_is_materialised_in_the_project(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let restore_world = world(ctx);
    let prefix_marker = restore_world
        .project_path()
        .join(".pixi/envs/demo/conda-meta/pixi_env_prefix");
    assert!(
        prefix_marker.is_file(),
        "the restored demo environment must carry its prefix marker"
    );
    Ok(StepExecution::from_value(None))
}

fn no_restore_scratch_is_left_behind(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let restore_world = world(ctx);
    assert!(
        !restore_world
            .project_path()
            .join(".pixi/.restore-work")
            .exists(),
        "restore scratch is not a deliverable"
    );
    Ok(StepExecution::from_value(None))
}

fn the_restore_is_refused(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let restore_world = world(ctx);
    let output = restore_world.output();
    assert!(
        !output.status.success(),
        "a restore that cannot verify must fail; stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(StepExecution::from_value(None))
}

fn the_refusal_reports_that_nothing_was_written(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let restore_world = world(ctx);
    let stderr = String::from_utf8_lossy(&restore_world.output().stderr);
    assert!(
        stderr.contains("nothing was written"),
        "the refusal must say the project was left alone: {stderr}"
    );
    Ok(StepExecution::from_value(None))
}

fn the_refusal_names_the_tampered_blob(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let restore_world = world(ctx);
    let stderr = String::from_utf8_lossy(&restore_world.output().stderr);
    assert!(
        stderr.contains("demo-pure-0.1.0-0.conda"),
        "the failure list must name the blob that failed verification: {stderr}"
    );
    Ok(StepExecution::from_value(None))
}

fn the_refusal_names_the_branch_location(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let restore_world = world(ctx);
    let stderr = String::from_utf8_lossy(&restore_world.output().stderr);
    assert!(
        stderr.contains("--branch-location") && stderr.contains("no-such-branch"),
        "the refusal must name the flag and the missing location: {stderr}"
    );
    Ok(StepExecution::from_value(None))
}

fn the_project_receives_no_environment(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let restore_world = world(ctx);
    assert!(
        !restore_world
            .project_path()
            .join(".pixi/envs/demo")
            .exists(),
        "no environment may be materialised by a refused restore"
    );
    Ok(StepExecution::from_value(None))
}

fn the_verdict_reports_that_no_project_files_were_written(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let restore_world = world(ctx);
    let output = restore_world.output();
    assert!(output.status.success(), "verify-only must succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--verify-only: no project files were written"),
        "the verdict must say nothing was written: {stdout}"
    );
    Ok(StepExecution::from_value(None))
}

// --- Registry --------------------------------------------------------------

step!(
    StepKeyword::Given,
    "a packed transport and an empty project",
    a_packed_transport_and_an_empty_project,
    &[WORLD]
);
step!(
    StepKeyword::Given,
    "a packed transport with one tampered byte",
    a_packed_transport_with_one_tampered_byte,
    &[WORLD]
);
step!(
    StepKeyword::Given,
    "a branch location that does not exist",
    a_branch_location_that_does_not_exist,
    &[WORLD]
);
step!(
    StepKeyword::When,
    "the operator restores the transport into the project",
    the_operator_restores_the_transport_into_the_project,
    &[WORLD]
);
step!(
    StepKeyword::When,
    "the operator asks restore to verify only",
    the_operator_asks_restore_to_verify_only,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "the restore reports completion",
    the_restore_reports_completion,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "the demo environment is materialised in the project",
    the_demo_environment_is_materialised_in_the_project,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "no restore scratch is left behind",
    no_restore_scratch_is_left_behind,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "the restore is refused",
    the_restore_is_refused,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "the refusal reports that nothing was written",
    the_refusal_reports_that_nothing_was_written,
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
    "the refusal names the branch location",
    the_refusal_names_the_branch_location,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "the project receives no environment",
    the_project_receives_no_environment,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "the verdict reports that no project files were written",
    the_verdict_reports_that_no_project_files_were_written,
    &[WORLD]
);

// --- Scenarios -------------------------------------------------------------

#[test]
fn scenario_a_verified_restore_writes_the_project_and_cleans_its_scratch() {
    run_scenario::<RestoreWorld>(
        "tests/features/restore_verification.feature",
        "A verified restore writes the project and cleans its scratch",
    );
}

#[test]
fn scenario_a_tampered_transport_is_refused_with_nothing_written() {
    run_scenario::<RestoreWorld>(
        "tests/features/restore_verification.feature",
        "A tampered transport is refused with nothing written",
    );
}

#[test]
fn scenario_a_missing_transport_is_refused_before_writing() {
    run_scenario::<RestoreWorld>(
        "tests/features/restore_verification.feature",
        "A missing transport is refused before writing",
    );
}

#[test]
fn scenario_a_verify_only_run_reports_the_verdict_and_writes_nothing() {
    run_scenario::<RestoreWorld>(
        "tests/features/restore_verification.feature",
        "A verify-only run reports the verdict and writes nothing",
    );
}

#[test]
fn the_step_registry_has_no_duplicate_definitions() {
    support::bdd::assert_no_duplicate_steps();
}
