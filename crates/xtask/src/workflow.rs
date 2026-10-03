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

/// Where this repository keeps its own render. Unlike the publisher — which no consumer-shaped
/// rule lets us commit, because its template is multi-line shell by necessity — the relock
/// render is house-shaped, so the repository runs the very artifact `init` hands consumers.
pub const RELOCK_PATH: &str = ".github/workflows/relock.yml";

/// Render the relock workflow *for this repository*: the pixi pin from the embedded catalogue,
/// the cargo half decided by the reviewed publish plan, and this repository's own CI workflow
/// as the dispatch target.
pub fn relock_render(root: &Path) -> Result<String> {
    let pixi_version =
        embedded_pixi_pin().context("the embedded tools lock declares no pixi pin")?;
    Ok(render_relock_workflow(RelockWorkflowOptions {
        cli_version: env!("CARGO_PKG_VERSION"),
        pixi_version: &pixi_version,
        cargo: plan_vendors_cargo(&root.join(DEFAULT_FILE)),
        ci_workflow: "ci.yml",
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

    let workflow = render_github_workflow(GithubWorkflowOptions {
        version: env!("CARGO_PKG_VERSION"),
        config_path: "pixi-sandbox.toml",
    });
    fs::write(&workflow_path, workflow)
        .with_context(|| format!("writing generated workflow {}", workflow_path.display()))?;

    run_actionlint(actionlint, project.path(), Path::new(WORKFLOW_PATH))?;

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
            ci_workflow: "ci.yml",
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

    let output = Command::new(actionlint)
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
        bail!(
            "actionlint rejects the workflow generated by pixi-sandbox (status {}):\n{}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{WORKFLOW_PATH, run_actionlint};
    use std::fs;
    use std::path::Path;

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
