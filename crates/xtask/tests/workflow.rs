//! `workflow.rs` — the generated-workflow surfaces (moved from the module's inline
//! `#[cfg(test)]`, TASK-83: names kept, bodies verbatim).

use std::fs;
use std::path::Path;
use xtask::workflow::{WORKFLOW_PATH, relock_render, run_actionlint};

#[test]
fn the_repository_relock_stamp_comes_from_the_workspace_being_rendered() {
    let project = tempfile::tempdir().expect("temp project");
    fs::write(
        project.path().join("Cargo.toml"),
        "[workspace]\n\n[workspace.package]\nversion = \"9.8.7\"\n",
    )
    .expect("workspace manifest");

    let rendered = relock_render(project.path()).expect("render relock");
    assert!(
        rendered.contains("# pixi-sandbox-version: 9.8.7"),
        "the just-stamped workspace version must win over the xtask binary's compile-time version:\n{rendered}"
    );
    assert!(
        !rendered.contains(concat!(
            "# pixi-sandbox-version: ",
            env!("CARGO_PKG_VERSION")
        )),
        "the fixture deliberately differs from the compiled xtask version"
    );
}

#[test]
fn a_missing_generated_file_names_the_expected_artifact() {
    let project = tempfile::tempdir().expect("temp project");
    let error = run_actionlint(
        Path::new("actionlint-does-not-run"),
        project.path(),
        Path::new(WORKFLOW_PATH),
    )
    .expect_err("the absent workflow must fail before actionlint starts");
    let message = format!("{error:#}");
    assert!(
        message.contains("generated workflow is missing"),
        "{message}"
    );
    assert!(message.contains(WORKFLOW_PATH), "{message}");
}

/// task-66 follow-up: actionlint only checks the Bash inside `run:` blocks when a
/// `shellcheck` executable is on PATH, and says nothing when it is absent. That silence is
/// what let malformed generated quoting pass `pixi run lint` locally and fail on GitHub.
/// This test fails if the development environment ever loses ShellCheck again.
#[test]
fn actionlint_runs_shellcheck_on_workflow_bash() {
    let project = tempfile::tempdir().expect("temp project");
    let workflow = project.path().join(WORKFLOW_PATH);
    fs::create_dir_all(workflow.parent().expect("workflow parent")).expect("workflow directory");
    fs::write(
        &workflow,
        concat!(
            "name: shellcheck-probe\n",
            "on: push\n",
            "jobs:\n",
            "  probe:\n",
            "    runs-on: ubuntu-latest\n",
            "    steps:\n",
            "      - run: |\n",
            "          if [ \"unterminated ]; then\n",
            "            echo broken\n",
            "          fi\n",
        ),
    )
    .expect("probe workflow");

    let error = run_actionlint(
        Path::new("actionlint"),
        project.path(),
        Path::new(WORKFLOW_PATH),
    )
    .expect_err("actionlint with shellcheck must reject malformed run: Bash");
    let message = format!("{error:#}");
    assert!(
        message.contains("shellcheck"),
        "actionlint accepted broken Bash, so shellcheck is missing from the environment: {message}"
    );
}

#[test]
fn missing_actionlint_names_the_required_tool_and_environment() {
    let project = tempfile::tempdir().expect("temp project");
    let workflow = project.path().join(WORKFLOW_PATH);
    fs::create_dir_all(workflow.parent().expect("workflow parent")).expect("workflow directory");
    fs::write(&workflow, "name: fixture\n").expect("fixture workflow");

    let error = run_actionlint(
        Path::new("actionlint-does-not-exist-for-this-test"),
        project.path(),
        Path::new(WORKFLOW_PATH),
    )
    .expect_err("the deliberately absent actionlint must fail");
    let message = format!("{error:#}");
    assert!(message.contains("actionlint is required"), "{message}");
    assert!(message.contains("Pixi environment"), "{message}");
}
