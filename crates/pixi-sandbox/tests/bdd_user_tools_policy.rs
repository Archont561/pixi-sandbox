//! BDD feature: user-tools policy (task-86, bound with the rstest-bdd attribute macros by
//! task-87).
//!
//! Scenarios live in `tests/features/user_tools_policy.feature`, byte-identical to the
//! sentences this file was written against; each one is bound below by `#[scenario]`, which
//! emits the rstest test itself, so the names nextest reports did not move. Steps drive the
//! binary black-box through the `tests/support` fixture builders against an isolated home
//! (task-33's contract, D10). The pre-policy scenario drives `scripts/restore.sh` against a
//! `transport_repo` staged with the pre-0.3.7 shim, so its binding and its steps carry the
//! same `#[cfg]` the descriptive suite uses (unix, linux-64 transport) — on a foreign host the
//! scenario is absent from the binary rather than failing in it.

mod support;

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use std::path::PathBuf;
use std::process::Output;
use support::bdd::assert_every_scenario_is_bound;
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

fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|meta| meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// The world every step in this binary borrows: zero-argument, per the `tests/support`
/// convention. rstest builds one instance per scenario, the steps share it, and it drops —
/// tempdir and all — when the scenario ends.
#[fixture]
fn policy_world() -> PolicyWorld {
    PolicyWorld::default()
}

// --- Given -----------------------------------------------------------------

#[given("a packed transport and an isolated home")]
fn a_packed_transport_and_an_isolated_home(policy_world: &mut PolicyWorld) {
    policy_world.staged();
}

#[given("the environment selects the skip policy")]
fn the_environment_selects_the_skip_policy(policy_world: &mut PolicyWorld) {
    policy_world.env_policy = Some("skip");
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[given("a bootstrap packed before user-tools")]
fn a_bootstrap_packed_before_user_tools(policy_world: &mut PolicyWorld) {
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

#[when("the operator restores the transport without stating a policy")]
fn the_operator_restores_the_transport_without_stating_a_policy(policy_world: &mut PolicyWorld) {
    let output = restore_binary_command(policy_world, &[]);
    policy_world.result = Some(output);
}

#[when("the operator restores the transport asking to skip registration")]
fn the_operator_restores_the_transport_asking_to_skip_registration(policy_world: &mut PolicyWorld) {
    let output = restore_binary_command(policy_world, &["--user-tools", "skip"]);
    policy_world.result = Some(output);
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[when("the operator runs the restore script asking to register")]
fn the_operator_runs_the_restore_script_asking_to_register(policy_world: &mut PolicyWorld) {
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
}

// --- Then ------------------------------------------------------------------

#[then("the restore registers pixi and pixi-sandbox in the home")]
fn the_restore_registers_pixi_and_pixi_sandbox_in_the_home(policy_world: &PolicyWorld) {
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
}

#[then("the launchers exec the manifest-verified tool copies")]
fn the_launchers_exec_the_manifest_verified_tool_copies(policy_world: &PolicyWorld) {
    let tools = policy_world.project_path().join(".pixi/tools/linux-64");
    for name in ["pixi", "pixi-sandbox"] {
        let body = std::fs::read_to_string(policy_world.home_path().join(".local/bin").join(name))
            .expect("launcher reads");
        assert!(
            body.contains(&format!("exec \"{}\"", tools.join(name).display())),
            "the {name} launcher must exec the manifest-verified copy: {body}"
        );
    }
}

#[then("the shell profile gains exactly one managed PATH block")]
fn the_shell_profile_gains_exactly_one_managed_path_block(policy_world: &PolicyWorld) {
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
}

#[then("the report says an already-running shell cannot be changed")]
fn the_report_says_an_already_running_shell_cannot_be_changed(policy_world: &PolicyWorld) {
    let stdout = String::from_utf8_lossy(&policy_world.output().stdout);
    assert!(
        stdout.contains("an already-running shell cannot be changed"),
        "the operator must be told what registration cannot do: {stdout}"
    );
}

#[then("the report says nothing was registered")]
fn the_report_says_nothing_was_registered(policy_world: &PolicyWorld) {
    assert!(
        policy_world.output().status.success(),
        "restore must succeed"
    );
    let stdout = String::from_utf8_lossy(&policy_world.output().stdout);
    assert!(
        stdout.contains("none registered (--user-tools skip"),
        "the opt-out report must reach the operator: {stdout}"
    );
}

#[then("nothing is registered in the home")]
fn nothing_is_registered_in_the_home(policy_world: &PolicyWorld) {
    assert!(
        !policy_world.home_path().join(".local").exists(),
        "no launcher directory may appear under the home"
    );
}

#[then("the project still receives its environment")]
fn the_project_still_receives_its_environment(policy_world: &PolicyWorld) {
    assert!(
        policy_world
            .project_path()
            .join(".pixi/envs/demo/conda-meta/pixi_env_prefix")
            .is_file(),
        "the opt-out is about the home, never the project"
    );
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[then("the report names the bundled version that ignored the policy")]
fn the_report_names_the_bundled_version_that_ignored_the_policy(policy_world: &PolicyWorld) {
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
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[then("the report does not claim a registration")]
fn the_report_does_not_claim_a_registration(policy_world: &PolicyWorld) {
    let log = policy_world.combined_log();
    assert!(
        !log.contains("user tools: registered pixi"),
        "a registration that did not happen must not be announced: {log}"
    );
}

// --- Scenarios -------------------------------------------------------------

#[scenario(
    path = "tests/features/user_tools_policy.feature",
    name = "A verified restore registers the tools by default"
)]
fn scenario_a_verified_restore_registers_the_tools_by_default(policy_world: PolicyWorld) {}

#[scenario(
    path = "tests/features/user_tools_policy.feature",
    name = "The skip policy touches nothing outside the project"
)]
fn scenario_the_skip_policy_touches_nothing_outside_the_project(policy_world: PolicyWorld) {}

#[scenario(
    path = "tests/features/user_tools_policy.feature",
    name = "The policy can travel as an environment variable"
)]
fn scenario_the_policy_can_travel_as_an_environment_variable(policy_world: PolicyWorld) {}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[scenario(
    path = "tests/features/user_tools_policy.feature",
    name = "A pre-policy bootstrap reports honestly instead of announcing"
)]
fn scenario_a_pre_policy_bootstrap_reports_honestly_instead_of_announcing(
    policy_world: PolicyWorld,
) {
}

/// The guard that replaces the retired duplicate-registry check: with `#[scenario]` the binding
/// *is* the test, so this is what notices a scenario the suite silently stopped running. It reads
/// the source rather than the compiled test list precisely because of the platform-gated binding
/// above — on a foreign host that scenario is not in the binary, but it is still bound.
#[test]
fn every_scenario_in_the_feature_file_is_bound() {
    assert_every_scenario_is_bound(
        "tests/features/user_tools_policy.feature",
        "tests/bdd_user_tools_policy.rs",
    );
}
