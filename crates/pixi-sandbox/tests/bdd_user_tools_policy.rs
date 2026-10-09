//! BDD feature: user-tools policy (task-86).
//!
//! Scenarios live in `tests/features/user_tools_policy.feature`; each one is bound to the
//! suite as a plain test below. Steps drive the binary black-box through the `tests/support`
//! fixture builders against an isolated home (task-33's contract, D10). The pre-policy
//! scenario drives `scripts/restore.sh` against a `transport_repo` staged with the
//! pre-0.3.7 shim, so it binds only where that fixture exists (unix, linux-64 transport).

mod support;

use rstest_bdd::{StepContext, StepError, StepExecution, StepKeyword, step};
use std::path::PathBuf;
use std::process::Output;
use support::bdd::{WORLD, run_scenario};
use support::{copy_tree, fixture_transport, isolated_bin};
use tempfile::TempDir;

/// One scenario's isolated world: everything under a tempdir root, the captured command
/// outcome, and the optional environment-carried policy.
#[derive(Default)]
struct PolicyWorld {
    root: Option<TempDir>,
    transport: Option<PathBuf>,
    project: Option<PathBuf>,
    home: Option<PathBuf>,
    env_policy: Option<&'static str>,
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    script_repo: Option<PathBuf>,
    result: Option<Output>,
}

impl PolicyWorld {
    fn staged(&mut self) {
        let root = tempfile::tempdir().expect("scenario tempdir");
        let transport = root.path().join("transport");
        copy_tree(&fixture_transport(), &transport);
        let project = root.path().join("project");
        std::fs::create_dir_all(&project).expect("project directory");
        let home = root.path().join("home");
        std::fs::create_dir_all(home.join("tmp")).expect("home directory");
        self.root = Some(root);
        self.transport = Some(transport);
        self.project = Some(project);
        self.home = Some(home);
    }

    fn home_path(&self) -> &std::path::Path {
        self.home.as_ref().expect("the scenario staged a home")
    }

    fn project_path(&self) -> &std::path::Path {
        self.project
            .as_ref()
            .expect("the scenario staged a project")
    }

    fn output(&self) -> &Output {
        self.result.as_ref().expect("the scenario ran a command")
    }

    fn combined_log(&self) -> String {
        let output = self.output();
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

fn world<'a>(ctx: &'a StepContext<'_>) -> impl std::ops::DerefMut<Target = PolicyWorld> + 'a {
    ctx.try_borrow_mut::<PolicyWorld>(WORLD)
        .expect("the runner inserts the world")
}

fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|meta| meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

// --- Given -----------------------------------------------------------------

fn a_packed_transport_and_an_isolated_home(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    world(ctx).staged();
    Ok(StepExecution::from_value(None))
}

fn the_environment_selects_the_skip_policy(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    world(ctx).env_policy = Some("skip");
    Ok(StepExecution::from_value(None))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn a_bootstrap_packed_before_user_tools(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let mut policy_world = world(ctx);
    policy_world.staged();
    let root = policy_world
        .root
        .as_ref()
        .expect("staged root")
        .path()
        .to_path_buf();
    let repo = support::transport_repo(
        &root.join("repo"),
        "sandbox/demo-linux-64",
        support::Bundled::PreUserTools,
    );
    policy_world.script_repo = Some(repo);
    Ok(StepExecution::from_value(None))
}

// --- When ------------------------------------------------------------------

fn restore_binary_command(policy_world: &PolicyWorld, extra_args: &[&str]) -> Output {
    let transport = policy_world
        .transport
        .as_ref()
        .expect("the scenario staged a transport");
    let mut command = isolated_bin(policy_world.home_path());
    command
        .arg("restore")
        .arg("--branch-location")
        .arg(transport);
    command
        .arg("--output-path")
        .arg(policy_world.project_path());
    if let Some(policy) = policy_world.env_policy {
        command.env("PIXI_SANDBOX_USER_TOOLS", policy);
    }
    command.args(extra_args);
    command.output().expect("restore runs")
}

fn the_operator_restores_the_transport_without_stating_a_policy(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let mut policy_world = world(ctx);
    let output = restore_binary_command(&policy_world, &[]);
    policy_world.result = Some(output);
    Ok(StepExecution::from_value(None))
}

fn the_operator_restores_the_transport_asking_to_skip_registration(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let mut policy_world = world(ctx);
    let output = restore_binary_command(&policy_world, &["--user-tools", "skip"]);
    policy_world.result = Some(output);
    Ok(StepExecution::from_value(None))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn the_operator_runs_the_restore_script_asking_to_register(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let mut policy_world = world(ctx);
    let repo = policy_world
        .script_repo
        .as_ref()
        .expect("the scenario staged a bootstrap repository")
        .clone();
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/restore.sh");
    let home = policy_world.home_path().to_path_buf();
    let project = policy_world.project_path().to_path_buf();
    let output = std::process::Command::new("bash")
        .arg(&script)
        .arg("sandbox/demo-linux-64")
        .arg(&project)
        .current_dir(&repo)
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("SHELL", "/usr/bin/bash")
        .env("TMPDIR", home.join("tmp"))
        .env("PIXI_SANDBOX_USER_TOOLS", "register")
        .env_remove("PIXI_SANDBOX_BRANCH")
        .env_remove("PIXI_SANDBOX_BUNDLE")
        .env_remove("PIXI_SANDBOX_CONFIG")
        .env_remove("PIXI_SANDBOX_USER_BIN")
        .output()
        .expect("the restore script runs");
    policy_world.result = Some(output);
    Ok(StepExecution::from_value(None))
}

// --- Then ------------------------------------------------------------------

fn the_restore_registers_pixi_and_pixi_sandbox_in_the_home(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let policy_world = world(ctx);
    assert!(
        policy_world.output().status.success(),
        "restore must succeed"
    );
    let bin = policy_world.home_path().join(".local/bin");
    for name in ["pixi", "pixi-sandbox"] {
        let launcher = bin.join(name);
        let body = std::fs::read_to_string(&launcher)
            .unwrap_or_else(|_| panic!("launcher {name} must exist at {}", launcher.display()));
        assert!(
            body.contains("managed by pixi-sandbox"),
            "the {name} launcher must carry the managed marker: {body}"
        );
        assert!(is_executable(&launcher), "the {name} launcher must run");
    }
    Ok(StepExecution::from_value(None))
}

fn the_launchers_exec_the_manifest_verified_tool_copies(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let policy_world = world(ctx);
    let tools = policy_world.project_path().join(".pixi/tools/linux-64");
    for name in ["pixi", "pixi-sandbox"] {
        let body = std::fs::read_to_string(policy_world.home_path().join(".local/bin").join(name))
            .expect("launcher reads");
        assert!(
            body.contains(&format!("exec \"{}\"", tools.join(name).display())),
            "the {name} launcher must exec the manifest-verified copy: {body}"
        );
    }
    Ok(StepExecution::from_value(None))
}

fn the_shell_profile_gains_exactly_one_managed_path_block(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let policy_world = world(ctx);
    let profile = std::fs::read_to_string(policy_world.home_path().join(".profile"))
        .expect("the profile exists");
    let bin = policy_world.home_path().join(".local/bin");
    assert!(
        profile.contains(bin.to_str().expect("utf-8 path")),
        "the managed block must name the bin directory: {profile}"
    );
    assert_eq!(
        profile.matches("# >>> pixi-sandbox user tools").count(),
        1,
        "exactly one managed block: {profile}"
    );
    Ok(StepExecution::from_value(None))
}

fn the_report_says_an_already_running_shell_cannot_be_changed(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let policy_world = world(ctx);
    let stdout = String::from_utf8_lossy(&policy_world.output().stdout);
    assert!(
        stdout.contains("an already-running shell cannot be changed"),
        "the operator must be told what registration cannot do: {stdout}"
    );
    Ok(StepExecution::from_value(None))
}

fn the_report_says_nothing_was_registered(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let policy_world = world(ctx);
    assert!(
        policy_world.output().status.success(),
        "restore must succeed"
    );
    let stdout = String::from_utf8_lossy(&policy_world.output().stdout);
    assert!(
        stdout.contains("none registered (--user-tools skip"),
        "the opt-out report must reach the operator: {stdout}"
    );
    Ok(StepExecution::from_value(None))
}

fn nothing_is_registered_in_the_home(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let policy_world = world(ctx);
    assert!(
        !policy_world.home_path().join(".local").exists(),
        "no launcher directory may appear under the home"
    );
    Ok(StepExecution::from_value(None))
}

fn the_project_still_receives_its_environment(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let policy_world = world(ctx);
    assert!(
        policy_world
            .project_path()
            .join(".pixi/envs/demo/conda-meta/pixi_env_prefix")
            .is_file(),
        "the opt-out is about the home, never the project"
    );
    Ok(StepExecution::from_value(None))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn the_report_names_the_bundled_version_that_ignored_the_policy(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let policy_world = world(ctx);
    let log = policy_world.combined_log();
    assert!(
        policy_world.output().status.success(),
        "the restore itself succeeds: {log}"
    );
    assert!(
        log.contains(
            "user tools: NOT registered — the bundled pixi-sandbox 0.3.6 predates --user-tools (0.3.7),"
        ),
        "the report must name the version that ignored the request: {log}"
    );
    assert!(
        log.contains("no longer sources or supports"),
        "the report must say what to do instead: {log}"
    );
    assert!(
        !log.contains("unknown version"),
        "the version must come from the restored copy: {log}"
    );
    Ok(StepExecution::from_value(None))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn the_report_does_not_claim_a_registration(
    ctx: &mut StepContext<'_>,
    _text: &str,
    _doc: Option<&str>,
    _table: Option<&[&[&str]]>,
) -> Result<StepExecution, StepError> {
    let policy_world = world(ctx);
    let log = policy_world.combined_log();
    assert!(
        !log.contains("user tools: registered pixi"),
        "a registration that did not happen must not be announced: {log}"
    );
    Ok(StepExecution::from_value(None))
}

// --- Registry --------------------------------------------------------------

step!(
    StepKeyword::Given,
    "a packed transport and an isolated home",
    a_packed_transport_and_an_isolated_home,
    &[WORLD]
);
step!(
    StepKeyword::Given,
    "the environment selects the skip policy",
    the_environment_selects_the_skip_policy,
    &[WORLD]
);
step!(
    StepKeyword::When,
    "the operator restores the transport without stating a policy",
    the_operator_restores_the_transport_without_stating_a_policy,
    &[WORLD]
);
step!(
    StepKeyword::When,
    "the operator restores the transport asking to skip registration",
    the_operator_restores_the_transport_asking_to_skip_registration,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "the restore registers pixi and pixi-sandbox in the home",
    the_restore_registers_pixi_and_pixi_sandbox_in_the_home,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "the launchers exec the manifest-verified tool copies",
    the_launchers_exec_the_manifest_verified_tool_copies,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "the shell profile gains exactly one managed PATH block",
    the_shell_profile_gains_exactly_one_managed_path_block,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "the report says an already-running shell cannot be changed",
    the_report_says_an_already_running_shell_cannot_be_changed,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "the report says nothing was registered",
    the_report_says_nothing_was_registered,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "nothing is registered in the home",
    nothing_is_registered_in_the_home,
    &[WORLD]
);
step!(
    StepKeyword::Then,
    "the project still receives its environment",
    the_project_still_receives_its_environment,
    &[WORLD]
);

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
step!(
    StepKeyword::Given,
    "a bootstrap packed before user-tools",
    a_bootstrap_packed_before_user_tools,
    &[WORLD]
);
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
step!(
    StepKeyword::When,
    "the operator runs the restore script asking to register",
    the_operator_runs_the_restore_script_asking_to_register,
    &[WORLD]
);
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
step!(
    StepKeyword::Then,
    "the report names the bundled version that ignored the policy",
    the_report_names_the_bundled_version_that_ignored_the_policy,
    &[WORLD]
);
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
step!(
    StepKeyword::Then,
    "the report does not claim a registration",
    the_report_does_not_claim_a_registration,
    &[WORLD]
);

// --- Scenarios -------------------------------------------------------------

#[test]
fn scenario_a_verified_restore_registers_the_tools_by_default() {
    run_scenario::<PolicyWorld>(
        "tests/features/user_tools_policy.feature",
        "A verified restore registers the tools by default",
    );
}

#[test]
fn scenario_the_skip_policy_touches_nothing_outside_the_project() {
    run_scenario::<PolicyWorld>(
        "tests/features/user_tools_policy.feature",
        "The skip policy touches nothing outside the project",
    );
}

#[test]
fn scenario_the_policy_can_travel_as_an_environment_variable() {
    run_scenario::<PolicyWorld>(
        "tests/features/user_tools_policy.feature",
        "The policy can travel as an environment variable",
    );
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn scenario_a_pre_policy_bootstrap_reports_honestly_instead_of_announcing() {
    run_scenario::<PolicyWorld>(
        "tests/features/user_tools_policy.feature",
        "A pre-policy bootstrap reports honestly instead of announcing",
    );
}

#[test]
fn the_step_registry_has_no_duplicate_definitions() {
    support::bdd::assert_no_duplicate_steps();
}
