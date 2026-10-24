//! The per-push airlock filter contract (TASK-135), driven through the public CLI against
//! throwaway repositories. The gate is structural text/YAML policy: what Actions actually does
//! with the skip is proven on real runners, so these tests pin the shape that makes that
//! behaviour possible and fail on every weakening measured in the task.
use std::{fs, process::Command};

/// A minimal airlock workflow carrying every property `check-airlock-filter` requires. The
/// glob list is byte-identical to the real workflow's trigger paths; everything else is
/// trimmed to what the gate reads.
const AIRLOCK: &str = r#"name: airlock

on:
  workflow_dispatch:
  schedule:
    - cron: "17 5 * * 1"
  pull_request:
    types: [opened, synchronize, reopened, ready_for_review]
    paths:
      - "crates/**"
      - "pixi.toml"
      - "scripts/restore.sh"
      - ".pixi-sandbox.toml"
      - ".github/workflows/airlock.yml"

permissions:
  contents: read

concurrency:
  group: airlock-${{ github.ref }}
  cancel-in-progress: true

jobs:
  changes:
    name: changes
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: read
    outputs:
      run-airlock: ${{ github.event_name != 'pull_request' || steps.filter.outputs.airlock != 'false' }}
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
      - uses: dorny/paths-filter@ceb8a2b8f2d89434be7ff52d3de7ec3738c5cc9d # v4.0.3
        id: filter
        if: github.event_name == 'pull_request'
        with:
          predicate-quantifier: some
          base: ${{ github.event.action == 'synchronize' && github.event.before || github.event.pull_request.base.sha }}
          ref: ${{ github.event.action == 'synchronize' && github.event.after || github.event.pull_request.head.sha }}
          filters: |
            airlock:
              - 'crates/**'
              - 'pixi.toml'
              - 'scripts/restore.sh'
              - '.pixi-sandbox.toml'
              - '.github/workflows/airlock.yml'

  plan:
    name: validate airlock plan
    needs: changes
    if: ${{ needs.changes.outputs.run-airlock != 'false' && (github.event_name != 'pull_request' || github.event.pull_request.draft == false) }}
    runs-on: ubuntu-latest
    outputs:
      matrix: ${{ steps.plan.outputs.matrix }}
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1

  proof:
    name: airlock ${{ matrix.platform }}
    needs: [changes, plan]
    if: ${{ needs.changes.outputs.run-airlock != 'false' && (github.event_name != 'pull_request' || github.event.pull_request.draft == false) }}
    runs-on: ${{ matrix.runner }}
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
"#;

fn invoke(workflow: &str) -> std::process::Output {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join(".github/workflows");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("airlock.yml"), workflow).unwrap();
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("--root")
        .arg(root.path())
        .arg("check-airlock-filter")
        .env("HOME", root.path())
        .env("USERPROFILE", root.path())
        .env("SHELL", "/bin/sh")
        .output()
        .unwrap()
}

#[test]
fn the_reference_shape_passes() {
    let output = invoke(AIRLOCK);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn every_weakening_is_refused() {
    let cases: [(&str, &str, &str); 10] = [
        // The quantifier is the whole mechanism: `every` skips the proof on a mixed push
        // because not every changed file matches a watched glob (PR #207's opening run).
        (
            "          predicate-quantifier: some",
            "          predicate-quantifier: every",
            "every-semantics skips the proof when code and docs change together",
        ),
        // The per-push globs must not drift from the trigger paths the ADR-29 guard reads.
        (
            "              - 'scripts/restore.sh'\n              - '.pixi-sandbox.toml'",
            "              - '.pixi-sandbox.toml'",
            "dropping a watched glob from the per-push filter",
        ),
        (
            "              - '.github/workflows/airlock.yml'",
            "              - '.github/workflows/airlock.yml'\n              - 'README.md'",
            "adding an unwatched glob to the per-push filter",
        ),
        // synchronize must judge the push range, not the whole PR diff.
        (
            "base: ${{ github.event.action == 'synchronize' && github.event.before || github.event.pull_request.base.sha }}",
            "base: ${{ github.event.pull_request.base.sha }}",
            "whole-diff base reintroduces the markdown-push cost",
        ),
        (
            "ref: ${{ github.event.action == 'synchronize' && github.event.after || github.event.pull_request.head.sha }}",
            "ref: ${{ github.event.pull_request.head.sha }}",
            "whole-diff ref reintroduces the markdown-push cost",
        ),
        // The output must default to "run" outside pull requests.
        (
            "run-airlock: ${{ github.event_name != 'pull_request' || steps.filter.outputs.airlock != 'false' }}",
            "run-airlock: ${{ steps.filter.outputs.airlock != 'false' }}",
            "output without the non-PR default would skip schedule and dispatch proofs",
        ),
        // Both expensive jobs must declare the skip and the draft guard.
        (
            "    needs: changes\n    if: ${{ needs.changes.outputs.run-airlock != 'false' && (github.event_name != 'pull_request' || github.event.pull_request.draft == false) }}\n    runs-on: ubuntu-latest\n    outputs:",
            "    if: ${{ github.event_name != 'pull_request' || github.event.pull_request.draft == false }}\n    runs-on: ubuntu-latest\n    outputs:",
            "plan job without the changes gate",
        ),
        (
            "    needs: [changes, plan]\n    if: ${{ needs.changes.outputs.run-airlock != 'false' && (github.event_name != 'pull_request' || github.event.pull_request.draft == false) }}",
            "    needs: plan\n    if: ${{ github.event_name != 'pull_request' || github.event.pull_request.draft == false }}",
            "proof job without the changes gate",
        ),
        (
            "needs.changes.outputs.run-airlock != 'false' && (github.event_name != 'pull_request' || github.event.pull_request.draft == false)",
            "needs.changes.outputs.run-airlock != 'false' && (github.event_name != 'pull_request')",
            "dropping the draft condition revives the TASK-134 cost",
        ),
        // The filter step stays PR-only and pinned under its known id.
        (
            "        id: filter\n        if: github.event_name == 'pull_request'",
            "        id: filter",
            "filter step running on schedule/dispatch events",
        ),
    ];
    for (original, weakened, why) in cases {
        assert!(AIRLOCK.contains(original), "test anchor missing: {why}");
        let output = invoke(&AIRLOCK.replace(original, weakened));
        assert!(
            !output.status.success(),
            "accepted a weakened airlock filter: {why}\n{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

#[test]
fn a_workflow_without_the_changes_job_is_refused() {
    let stripped: String = AIRLOCK
        .lines()
        .take_while(|line| !line.starts_with("  changes:"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!stripped.contains("changes:"));
    assert!(!invoke(&stripped).status.success());
}

#[test]
fn a_missing_workflow_fails_named() {
    let root = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("--root")
        .arg(root.path())
        .arg("check-airlock-filter")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("airlock.yml"), "unexpected error: {stderr}");
}
