use pixi_sandbox::generated::{RelockWorkflowOptions, render_relock_workflow};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn render(cargo: bool) -> String {
    render_relock_workflow(RelockWorkflowOptions {
        cli_version: "9.8.7",
        pixi_version: "0.81.0",
        cargo,
        ci_workflow: "ci.yml",
        publisher_workflow: "publish-sandbox.yml",
    })
}

fn directives(workflow: &str) -> String {
    workflow
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn vendoring_projects_guard_both_lockfiles_before_the_repair_job() {
    let workflow = render(true);
    let guard = workflow
        .split_once("  relock:")
        .expect("rendered workflow has a relock job")
        .0;

    assert!(guard.contains("- run: pixi lock --check"), "{guard}");
    assert!(guard.contains("- run: cargo fetch --locked"), "{guard}");
    assert_eq!(
        directives(&workflow)
            .matches("cargo fetch --locked")
            .count(),
        1,
        "only the guard is locked; the repair still refreshes Cargo.lock\n{workflow}"
    );
    assert!(
        workflow.contains("- run: cargo fetch\n"),
        "the repair job must still refresh Cargo.lock\n{workflow}"
    );
}

#[test]
fn conda_only_projects_have_no_cargo_guard_or_repair_step() {
    let workflow = directives(&render(false));
    assert!(!workflow.contains("cargo fetch"), "{workflow}");
    assert!(!workflow.contains("Cargo.lock"), "{workflow}");
}

#[test]
fn rendered_cargo_guard_accepts_a_clean_lock_and_rejects_a_stale_one_without_writing() {
    let scratch = tempdir().expect("temporary consumer");
    let project = scratch.path().join("consumer");
    let dependency = scratch.path().join("dependency");
    let home = scratch.path().join("home");
    let cargo_home = scratch.path().join("cargo-home");
    fs::create_dir_all(project.join("src")).expect("consumer src");
    fs::create_dir_all(dependency.join("src")).expect("dependency src");
    fs::create_dir_all(&home).expect("isolated HOME");
    fs::create_dir_all(&cargo_home).expect("isolated CARGO_HOME");
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("consumer manifest");
    fs::write(project.join("src/lib.rs"), "pub fn consumer() {}\n").expect("consumer source");
    fs::write(
        dependency.join("Cargo.toml"),
        "[package]\nname = \"dependency\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("dependency manifest");
    fs::write(dependency.join("src/lib.rs"), "pub fn dependency() {}\n")
        .expect("dependency source");

    let cargo = |args: &[&str]| {
        Command::new("cargo")
            .args(args)
            .current_dir(&project)
            .env("HOME", &home)
            .env("CARGO_HOME", &cargo_home)
            .env("CARGO_NET_OFFLINE", "true")
            .output()
            .expect("run Cargo from the restored Pixi environment")
    };
    let generated = cargo(&["generate-lockfile", "--offline"]);
    assert!(
        generated.status.success(),
        "generate lock: {}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let clean_lock = fs::read(project.join("Cargo.lock")).expect("clean lock");

    let guard = cargo(&["fetch", "--locked"]);
    assert!(
        guard.status.success(),
        "clean guard: {}",
        String::from_utf8_lossy(&guard.stderr)
    );
    assert_eq!(
        fs::read(project.join("Cargo.lock")).expect("lock after clean guard"),
        clean_lock,
        "the rendered guard must not rewrite a clean lock"
    );

    fs::write(
        project.join("Cargo.toml"),
        format!(
            "[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\ndependency = {{ path = {:?} }}\n",
            dependency
        ),
    )
    .expect("stale consumer manifest");
    let stale = cargo(&["fetch", "--locked"]);
    assert!(!stale.status.success(), "a stale Cargo.lock must fail");
    let stderr = String::from_utf8_lossy(&stale.stderr);
    assert!(
        stderr.contains("lock file") && stderr.contains("--locked"),
        "{stderr}"
    );
    assert_eq!(
        fs::read(project.join("Cargo.lock")).expect("lock after stale guard"),
        clean_lock,
        "--locked must leave the stale lock untouched"
    );
}
