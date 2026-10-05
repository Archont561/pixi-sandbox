use anyhow::{Context, Result, bail};
use pixi_sandbox::generated::{
    GithubWorkflowOptions, RelockWorkflowOptions, embedded_pixi_pin, plan_vendors_cargo,
    render_github_workflow, render_relock_workflow,
};
use pixi_sandbox_core::sandbox_config::DEFAULT_FILE;
use std::fs;
use std::path::Path;
use std::process::Command;

const WORKFLOW_PATH: &str = ".github/workflows/publish-sandbox.yml";

/// The same file by name alone, which is how a workflow is named in a `gh workflow run`
/// dispatch — the relock render's third dispatch target.
const PUBLISHER_WORKFLOW: &str = "publish-sandbox.yml";

/// Where this repository keeps its own render. Unlike the publisher — which no consumer-shaped
/// rule lets us commit, because its template is multi-line shell by necessity — the relock
/// render is house-shaped, so the repository runs the very artifact `init` hands consumers.
pub const RELOCK_PATH: &str = ".github/workflows/relock.yml";

/// Render the relock workflow *for this repository*: the pixi pin from the embedded catalogue,
/// the cargo half decided by the reviewed publish plan, and this repository's own CI workflow
/// as the dispatch target.
pub fn relock_render(root: &Path) -> Result<String> {
    let cli_version = crate::version::workspace_version(root)
        .context("reading the workspace version for the relock workflow")?;
    let pixi_version =
        embedded_pixi_pin().context("the embedded tools lock declares no pixi pin")?;
    Ok(render_relock_workflow(RelockWorkflowOptions {
        cli_version: &cli_version,
        pixi_version: &pixi_version,
        cargo: plan_vendors_cargo(&root.join(DEFAULT_FILE)),
        relock_workflow: "relock.yml",
        ci_workflow: "ci.yml",
        publisher_workflow: PUBLISHER_WORKFLOW,
    }))
}

/// Write this repository's render. The committed file is derived, so this is the one command
/// that may change it — `check-repository` fails when the two disagree.
pub fn render_relock(root: &Path) -> Result<()> {
    let path = root.join(RELOCK_PATH);
    let rendered = relock_render(root)?;
    let parent = path
        .parent()
        .context("relock workflow path has no parent")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating workflow directory {}", parent.display()))?;
    let unchanged = fs::read_to_string(&path).is_ok_and(|current| current == rendered);
    fs::write(&path, &rendered)
        .with_context(|| format!("writing generated workflow {}", path.display()))?;
    eprintln!(
        "{RELOCK_PATH}: {}",
        if unchanged {
            "already current"
        } else {
            "re-rendered"
        }
    );
    Ok(())
}

/// Validate the exact workflow renderer used by `pixi-sandbox init` with GitHub's external
/// workflow linter. Structural contracts that actionlint cannot evaluate live in the product
/// crate's Rust tests; this adapter owns only temporary-project and process orchestration.
pub fn lint_generated_workflow(actionlint: &Path) -> Result<()> {
    let project = tempfile::tempdir().context("creating a temporary generated project")?;
    let workflow_path = project.path().join(WORKFLOW_PATH);
    let parent = workflow_path
        .parent()
        .context("generated workflow path has no parent")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating generated workflow directory {}", parent.display()))?;

    let base = GithubWorkflowOptions {
        version: env!("CARGO_PKG_VERSION"),
        config_path: "pixi-sandbox.toml",
        workflow_path: WORKFLOW_PATH,
        relock_workflow_path: RELOCK_PATH,
        relock_ci_workflow: "ci.yml",
        script_path: "restore.sh",
        branch: "sandbox/developer-linux-64",
        push_paths: &[],
        scoped_permissions: false,
        concurrency: None,
        plan_timeout_minutes: None,
        publish_timeout_minutes: None,
        pixi_version: None,
        setup_pixi_cache: None,
    };
    let workflow = render_github_workflow(base);
    fs::write(&workflow_path, workflow)
        .with_context(|| format!("writing generated workflow {}", workflow_path.display()))?;

    run_actionlint(actionlint, project.path(), Path::new(WORKFLOW_PATH))?;

    // task-53 AC#2: every `[workflow]`-table block, individually and combined, stays
    // actionlint-clean. The default (above) and every other row here share one base so the
    // only thing varying is the policy under test.
    let push_paths = ["pixi-sandbox.toml".to_string(), "pixi.toml".to_string()];
    let combinations: [(&str, GithubWorkflowOptions<'_>); 5] = [
        (
            "publish-sandbox.push-paths.yml",
            GithubWorkflowOptions {
                push_paths: &push_paths,
                ..base
            },
        ),
        (
            "publish-sandbox.permissions.yml",
            GithubWorkflowOptions {
                scoped_permissions: true,
                ..base
            },
        ),
        (
            "publish-sandbox.concurrency.yml",
            GithubWorkflowOptions {
                concurrency: Some(("publish-sandbox", true)),
                ..base
            },
        ),
        (
            "publish-sandbox.timeouts.yml",
            GithubWorkflowOptions {
                plan_timeout_minutes: Some(15),
                publish_timeout_minutes: Some(60),
                ..base
            },
        ),
        (
            "publish-sandbox.pixi-pin-and-cache.yml",
            GithubWorkflowOptions {
                pixi_version: Some("0.81.0"),
                setup_pixi_cache: Some(true),
                ..base
            },
        ),
    ];
    for (relative, options) in combinations {
        let path = project.path().join(relative);
        fs::write(&path, render_github_workflow(options))
            .with_context(|| format!("writing generated workflow {}", path.display()))?;
        run_actionlint(actionlint, project.path(), Path::new(relative))?;
    }
    // And every block configured at once — the combination most likely to interact badly.
    let everything_relative = "publish-sandbox.everything.yml";
    let everything_path = project.path().join(everything_relative);
    fs::write(
        &everything_path,
        render_github_workflow(GithubWorkflowOptions {
            push_paths: &push_paths,
            scoped_permissions: true,
            concurrency: Some(("publish-sandbox", true)),
            plan_timeout_minutes: Some(15),
            publish_timeout_minutes: Some(60),
            pixi_version: Some("0.81.0"),
            setup_pixi_cache: Some(true),
            ..base
        }),
    )
    .with_context(|| format!("writing generated workflow {}", everything_path.display()))?;
    run_actionlint(actionlint, project.path(), Path::new(everything_relative))?;

    // The second generated artifact, linted the same way. Its content for *this* repository is
    // additionally byte-checked by `check-repository`; here it is the consumer render that
    // matters, including the conda-only shape no consumer of this repository exercises.
    for (relative, cargo) in [
        (RELOCK_PATH, true),
        (".github/workflows/relock-nocargo.yml", false),
    ] {
        let path = project.path().join(relative);
        let rendered = render_relock_workflow(RelockWorkflowOptions {
            cli_version: env!("CARGO_PKG_VERSION"),
            pixi_version: &embedded_pixi_pin()
                .context("the embedded tools lock declares no pixi pin")?,
            cargo,
            relock_workflow: "relock.yml",
            ci_workflow: "ci.yml",
            publisher_workflow: PUBLISHER_WORKFLOW,
        });
        fs::write(&path, rendered)
            .with_context(|| format!("writing generated workflow {}", path.display()))?;
        run_actionlint(actionlint, project.path(), Path::new(relative))?;
    }

    eprintln!(
        "generated workflows: actionlint clean (publisher + relock; Rust tests cover matrix shape, plan keys and step shape)"
    );
    Ok(())
}

fn run_actionlint(actionlint: &Path, project: &Path, workflow: &Path) -> Result<()> {
    let workflow_path = project.join(workflow);
    if !workflow_path.is_file() {
        bail!(
            "generated workflow is missing at {} — the renderer did not create the artifact actionlint must check",
            workflow_path.display()
        );
    }

    // Verbose mode makes a failing CI trace name the exact generated artifact and error count.
    // The normal success path deliberately stays quiet because this output is captured below.
    let output = Command::new(actionlint)
        .arg("--verbose")
        .arg(workflow)
        .current_dir(project)
        .output()
        .with_context(|| {
            format!(
                "starting {} — actionlint is required; run this command inside the Pixi environment",
                actionlint.display()
            )
        })?;

    if !output.status.success() {
        // Keep diagnostics on the error's first line: GitHub's check annotation only retains
        // that line when an enclosing `cargo run` fails. Newlines are still visible in raw logs,
        // but a compact trace makes a remote failure actionable when log retrieval is unavailable.
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .replace(['\r', '\n'], " | ");
        bail!(
            "actionlint rejects generated workflow {} (status {}; diagnostics: {}):",
            workflow.display(),
            output.status,
            diagnostics
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{WORKFLOW_PATH, relock_render, run_actionlint};
    use std::fs;
    use std::path::Path;

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

    #[test]
    fn missing_actionlint_names_the_required_tool_and_environment() {
        let project = tempfile::tempdir().expect("temp project");
        let workflow = project.path().join(WORKFLOW_PATH);
        fs::create_dir_all(workflow.parent().expect("workflow parent"))
            .expect("workflow directory");
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
}
