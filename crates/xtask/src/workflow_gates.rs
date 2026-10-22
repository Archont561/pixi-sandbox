//! Fail-closed scheduling contracts for the repository's supporting workflows (TASK-127).
//! These are structural checks, not a replacement for actual native-runner evidence.
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path};

fn load(root: &Path, name: &str) -> Result<Value> {
    let path = root.join(".github/workflows").join(name);
    serde_yaml::from_str(
        &fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?,
    )
    .with_context(|| format!("parsing {}", path.display()))
}

fn names(value: &Value) -> BTreeSet<&str> {
    if let Some(name) = value.as_str() {
        return [name].into_iter().collect();
    }
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn steps(job: &Value) -> impl Iterator<Item = &Value> {
    job["steps"].as_array().into_iter().flatten()
}

/// Enforce early Linux proof plus complete, success-dependent final publication.
///
/// # Errors
/// Refuses a missing/invalid workflow, weakened dependencies or an unexpected Linux artifact.
pub fn check_release_scheduling(root: &Path) -> Result<()> {
    let release = load(root, "release.yml")?;
    let jobs = &release["jobs"];
    for name in ["linux-binary", "build", "linux-package", "consumer-proof"] {
        ensure!(
            jobs[name]["continue-on-error"].is_null() && jobs[name]["if"].is_null(),
            "{name} must remain mandatory and success-gated"
        );
        ensure!(
            steps(&jobs[name]).all(|s| s["continue-on-error"].is_null()),
            "{name} must not ignore a failed build, artifact or proof step"
        );
    }
    ensure!(
        names(&jobs["consumer-proof"]["needs"]) == ["linux-binary"].into_iter().collect(),
        "consumer proof must depend only on linux-binary"
    );
    ensure!(
        jobs["consumer-proof"]["if"].is_null(),
        "consumer proof must retain default success gating"
    );
    ensure!(
        names(&jobs["release"]["needs"])
            == ["build", "linux-binary", "linux-package", "consumer-proof"]
                .into_iter()
                .collect(),
        "publication must require all binaries, packages and consumer proof"
    );
    ensure!(
        jobs["release"]["if"].as_str()
            == Some("${{ startsWith(github.ref, 'refs/tags/') || inputs.version != '' }}"),
        "publication must retain its tag/dispatch condition and implicit success gate"
    );
    let targets: BTreeSet<_> = jobs["build"]["strategy"]["matrix"]["include"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row["target"].as_str())
        .collect();
    ensure!(
        targets
            == [
                "aarch64-unknown-linux-musl",
                "x86_64-apple-darwin",
                "aarch64-apple-darwin",
                "x86_64-pc-windows-msvc"
            ]
            .into_iter()
            .collect(),
        "the remaining four native release targets must be retained"
    );
    ensure!(
        steps(&jobs["linux-binary"]).any(|s| s["run"]
            == "pixi run -e package build-release-binary -- --target x86_64-unknown-linux-musl"),
        "Linux bootstrap must remain a musl release build"
    );
    ensure!(
        names(&jobs["linux-package"]["needs"]) == ["linux-binary"].into_iter().collect(),
        "Linux packaging must follow its binary build"
    );
    ensure!(
        steps(&jobs["linux-package"])
            .any(|s| s["run"] == "pixi run -e package xtask smoke-conda-package"),
        "Linux package smoke test must remain"
    );
    ensure!(
        steps(&jobs["consumer-proof"]).any(|s| s["uses"]
            .as_str()
            .is_some_and(|u| u.starts_with("actions/download-artifact@"))
            && s["with"]["name"] == "binary-x86_64-unknown-linux-musl"
            && s["with"]["run-id"].is_null()
            && s["with"]["repository"].is_null()),
        "consumer proof must use the matching Linux artifact from this run"
    );
    for command in [
        "pixi run -e package xtask check-conda-platforms dist/conda",
        "pixi run -e package xtask check-release-binaries dist",
    ] {
        ensure!(
            steps(&jobs["release"]).any(|s| s["run"] == command),
            "missing final completeness gate: {command}"
        );
    }
    println!(
        "release scheduling: Linux proof is early; publication remains success-gated on every platform and proof"
    );
    Ok(())
}

const DOCS_TRUST: &str = "${{ github.event_name == 'workflow_run' && github.event.workflow_run.conclusion == 'success' && github.event.workflow_run.event == 'push' && github.event.workflow_run.head_branch == 'main' && github.event.workflow_run.head_repository.full_name == github.repository && github.event.workflow_run.head_sha == github.sha }}";
const DOCS_DEPLOY: &str = "${{ always() && !cancelled() && ((needs.reuse.result == 'success' && needs.build.result == 'skipped') || (needs.build.result == 'success' && needs.reuse.result == 'skipped')) }}";

/// Enforce trusted same-revision Pages reuse, with an independently checked manual fallback.
///
/// # Errors
/// Refuses missing quality gates, relaxed trust conditions or cross-run artifact ambiguity.
pub fn check_docs_artifacts(root: &Path) -> Result<()> {
    let docs = load(root, "docs.yml")?;
    let ci = load(root, "ci.yml")?;
    let jobs = &docs["jobs"];
    ensure!(
        names(&docs["on"]["workflow_run"]["workflows"]) == ["ci"].into_iter().collect(),
        "docs must observe only ci"
    );
    ensure!(
        names(&docs["on"]["workflow_run"]["branches"]) == ["main"].into_iter().collect(),
        "docs must observe only main"
    );
    ensure!(
        names(&docs["on"]["workflow_run"]["types"]) == ["completed"].into_iter().collect(),
        "docs must observe completed runs"
    );
    ensure!(
        jobs["reuse"]["if"] == DOCS_TRUST,
        "docs reuse must reject failed, fork/PR, wrong-branch and mismatched-SHA producers"
    );
    ensure!(
        jobs["build"]["if"]
            == "github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main'",
        "only manual docs builds may use the fallback"
    );
    ensure!(
        names(&jobs["deploy"]["needs"]) == ["reuse", "build"].into_iter().collect()
            && jobs["deploy"]["if"] == DOCS_DEPLOY,
        "deploy requires exactly one successful producer; missing/failed artifacts must not deploy"
    );
    for step in steps(&jobs["reuse"]) {
        let action = step["uses"]
            .as_str()
            .context("artifact reuse must not run code")?;
        ensure!(
            step["run"].is_null()
                && step["continue-on-error"].is_null()
                && step["if"].is_null()
                && (action.starts_with("actions/download-artifact@")
                    || action.starts_with("actions/upload-pages-artifact@")),
            "artifact reuse may only download and upload static files"
        );
    }
    ensure!(
        steps(&jobs["reuse"]).any(|s| s["uses"]
            .as_str()
            .is_some_and(|u| u.starts_with("actions/download-artifact@"))
            && s["with"]["name"] == "docs-site-${{ github.event.workflow_run.head_sha }}"
            && s["with"]["run-id"] == "${{ github.event.workflow_run.id }}"
            && s["with"]["repository"] == "${{ github.repository }}"
            && s["with"]["path"] == "docs/dist"),
        "docs artifact must bind repository, producer run and source SHA"
    );
    for job in [&ci["jobs"]["ci"], &jobs["build"]] {
        for command in [
            "pixi run docs build",
            "pixi run docs-frontmatter-check",
            "pixi run docs-link-check",
            "pixi run docs-a11y-check",
            "pixi run docs-snippet-check",
        ] {
            ensure!(
                steps(job).any(|s| s["run"] == command
                    && s["continue-on-error"].is_null()
                    && s["if"].is_null()),
                "missing unconditional docs quality gate: {command}"
            );
        }
    }
    let ci_steps: Vec<_> = steps(&ci["jobs"]["ci"]).collect();
    let upload = ci_steps
        .iter()
        .position(|s| {
            s["uses"]
                .as_str()
                .is_some_and(|u| u.starts_with("actions/upload-artifact@"))
                && s["with"]["name"] == "docs-site-${{ github.sha }}"
        })
        .context("missing SHA-bound CI docs artifact")?;
    ensure!(
        ci_steps[upload]["if"] == "github.event_name == 'push' && github.ref == 'refs/heads/main'"
            && ci_steps[upload]["with"]["if-no-files-found"] == "error"
            && ci_steps[upload]["with"]["path"] == "docs/dist",
        "CI artifact upload must be main-push-only and fail for missing output"
    );
    for gate in [
        "pixi run docs-frontmatter-check",
        "pixi run docs-link-check",
        "pixi run docs-a11y-check",
        "pixi run docs-snippet-check",
    ] {
        ensure!(
            ci_steps[..upload].iter().any(|s| s["run"] == gate),
            "artifact upload must follow {gate}"
        );
    }
    println!(
        "docs artifact trust: only successful main-push same-revision CI artifacts or quality-checked manual builds can deploy"
    );
    Ok(())
}
