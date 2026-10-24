//! Publication scheduling and artifact trust at the public automation CLI boundary.
use std::{fs, process::Command};

const RELEASE: &str = r"
jobs:
  linux-binary:
    steps:
      - run: pixi run -e package build-release-binary -- --target x86_64-unknown-linux-musl
  build:
    strategy:
      matrix:
        include:
          - target: aarch64-unknown-linux-musl
          - target: x86_64-apple-darwin
          - target: aarch64-apple-darwin
          - target: x86_64-pc-windows-msvc
  linux-package:
    needs: linux-binary
    steps:
      - run: pixi run -e package xtask smoke-conda-package
  consumer-proof:
    needs: linux-binary
    steps:
      - uses: actions/download-artifact@pinned
        with:
          name: binary-x86_64-unknown-linux-musl
  release:
    needs: [build, linux-binary, linux-package, consumer-proof]
    if: ${{ startsWith(github.ref, 'refs/tags/') || inputs.version != '' }}
    steps:
      - run: pixi run -e package xtask check-conda-platforms dist/conda
      - run: pixi run -e package xtask check-release-binaries dist
";

fn invoke(release: &str) -> std::process::Output {
    let root = tempfile::tempdir().unwrap();
    let workflows = root.path().join(".github/workflows");
    fs::create_dir_all(&workflows).unwrap();
    fs::write(workflows.join("release.yml"), release).unwrap();
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("--root")
        .arg(root.path())
        .arg("check-release-scheduling")
        .env("HOME", root.path())
        .env("USERPROFILE", root.path())
        .env("SHELL", "/bin/sh")
        .output()
        .unwrap()
}

#[test]
fn linux_proof_can_start_without_unrelated_matrix_legs_but_publication_waits() {
    let result = invoke(RELEASE);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

const CI: &str = r"
jobs:
  ci:
    steps:
      - run: pixi run docs build
      - run: pixi run docs-frontmatter-check
      - run: pixi run docs-link-check
      - run: pixi run docs-a11y-check
      - run: pixi run docs-snippet-check
      - uses: actions/upload-artifact@pinned
        if: github.event_name == 'push' && github.ref == 'refs/heads/main'
        with:
          name: docs-site-${{ github.sha }}
          path: docs/dist
          if-no-files-found: error
";
const DOCS: &str = include_str!("fixtures/workflow-gates/docs.yml");

fn invoke_docs(docs: &str, ci: &str) -> std::process::Output {
    let root = tempfile::tempdir().unwrap();
    let workflows = root.path().join(".github/workflows");
    fs::create_dir_all(&workflows).unwrap();
    fs::write(workflows.join("docs.yml"), docs).unwrap();
    fs::write(workflows.join("ci.yml"), ci).unwrap();
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("--root")
        .arg(root.path())
        .arg("check-docs-artifacts")
        .env("HOME", root.path())
        .env("USERPROFILE", root.path())
        .env("SHELL", "/bin/sh")
        .output()
        .unwrap()
}

#[test]
fn pages_accepts_only_a_trusted_same_revision_ci_artifact_or_a_manual_build() {
    let output = invoke_docs(DOCS, CI);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn a_failed_matrix_leg_cannot_be_made_optional() {
    let weakened = RELEASE.replace("  build:\n", "  build:\n    continue-on-error: true\n");
    let result = invoke(&weakened);
    assert!(
        !result.status.success(),
        "a failed platform must block publication"
    );
}

#[test]
fn missing_proof_packages_or_platforms_cannot_reach_publication() {
    for (original, weakened) in [
        (
            "[build, linux-binary, linux-package, consumer-proof]",
            "[build, linux-binary, linux-package]",
        ),
        (
            "[build, linux-binary, linux-package, consumer-proof]",
            "[build, linux-binary, consumer-proof]",
        ),
        ("          - target: x86_64-apple-darwin\n", ""),
        ("needs: linux-binary", "needs: build"),
        (
            "${{ startsWith(github.ref, 'refs/tags/') || inputs.version != '' }}",
            "${{ always() }}",
        ),
        (
            "pixi run -e package xtask check-conda-platforms dist/conda",
            "echo skipped",
        ),
    ] {
        assert!(RELEASE.contains(original));
        let output = invoke(&RELEASE.replace(original, weakened));
        assert!(
            !output.status.success(),
            "accepted weakened release gate: {original}"
        );
    }
}

#[test]
fn missing_wrong_or_cross_run_linux_artifacts_are_refused() {
    for replacement in [
        "name: unrelated-binary",
        "name: binary-x86_64-unknown-linux-musl\n          run-id: 123",
        "name: binary-x86_64-unknown-linux-musl\n          repository: stranger/repo",
    ] {
        let output =
            invoke(&RELEASE.replace("name: binary-x86_64-unknown-linux-musl", replacement));
        assert!(!output.status.success(), "accepted {replacement}");
    }
}

#[test]
fn pages_refuses_relaxed_producer_identity_and_artifact_binding() {
    for (original, weakened) in [
        ("github.event.workflow_run.conclusion == 'success'", "true"),
        ("github.event.workflow_run.event == 'push'", "true"),
        ("github.event.workflow_run.head_branch == 'main'", "true"),
        (
            "github.event.workflow_run.head_repository.full_name == github.repository",
            "true",
        ),
        ("github.event.workflow_run.head_sha == github.sha", "true"),
        ("run-id: ${{ github.event.workflow_run.id }}", "run-id: 123"),
        (
            "repository: ${{ github.repository }}",
            "repository: stranger/repo",
        ),
        (
            "name: docs-site-${{ github.event.workflow_run.head_sha }}",
            "name: docs-site",
        ),
    ] {
        assert!(DOCS.contains(original));
        let output = invoke_docs(&DOCS.replace(original, weakened), CI);
        assert!(
            !output.status.success(),
            "accepted relaxed identity: {original}"
        );
    }
}

#[test]
fn pages_refuses_artifact_execution_missing_gates_and_failed_producer_bypass() {
    for (original, weakened) in [
        (
            "      # No checkout, shell, dependency install or binary execution in this job.",
            "      - run: ./downloaded-program",
        ),
        ("- run: pixi run docs-snippet-check", "- run: echo skipped"),
        (
            "needs.reuse.result == 'success'",
            "needs.reuse.result != 'cancelled'",
        ),
        ("github.event_name == 'workflow_dispatch'", "true"),
    ] {
        assert!(DOCS.contains(original));
        assert!(
            !invoke_docs(&DOCS.replace(original, weakened), CI)
                .status
                .success()
        );
    }
    for (original, weakened) in [
        ("- run: pixi run docs-a11y-check", "- run: echo skipped"),
        ("name: docs-site-${{ github.sha }}", "name: docs-site"),
        ("if-no-files-found: error", "if-no-files-found: ignore"),
        (
            "github.event_name == 'push' && github.ref == 'refs/heads/main'",
            "always()",
        ),
    ] {
        assert!(CI.contains(original));
        assert!(
            !invoke_docs(DOCS, &CI.replace(original, weakened))
                .status
                .success()
        );
    }
}

#[test]
fn a_manual_build_from_an_untrusted_branch_cannot_deploy() {
    let unsafe_manual = DOCS.replace(
        "github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main'",
        "github.event_name == 'workflow_dispatch'",
    );
    assert!(
        !invoke_docs(&unsafe_manual, CI).status.success(),
        "manual fallback must also be main-only"
    );
}

// --- doc-18 §5.2: the `milestone-v*` auto-release trigger and the v*.*.* release split.

const AUTO_RELEASE_TRIGGERS: &str = r"
on:
  workflow_dispatch:
    inputs:
      bump:
        default: auto
      version:
        default: ''
  workflow_call:
    inputs:
      bump:
        default: auto
      version:
        default: ''
  push:
    tags:
      - milestone-v*
jobs:
  cut:
    steps:
      - id: milestone
        if: github.event_name == 'push'
        run: pixi run xtask resolve-milestone-tag -- ${{ github.ref_name }}
      - id: prep
        run: pixi run xtask prepare-release ${{ github.event_name == 'push' && steps.milestone.outputs.version || (inputs.version != '' && inputs.version || inputs.bump) }}
";

const RELEASE_TRIGGERS: &str = r"
on:
  push:
    tags:
      - v*.*.*
jobs:
  publish:
    runs-on: ubuntu-latest
";

fn invoke_triggers(auto_release: &str, release: &str) -> std::process::Output {
    let root = tempfile::tempdir().unwrap();
    let workflows = root.path().join(".github/workflows");
    fs::create_dir_all(&workflows).unwrap();
    fs::write(workflows.join("auto-release.yml"), auto_release).unwrap();
    fs::write(workflows.join("release.yml"), release).unwrap();
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("--root")
        .arg(root.path())
        .arg("check-release-triggers")
        .env("HOME", root.path())
        .env("USERPROFILE", root.path())
        .env("SHELL", "/bin/sh")
        .output()
        .unwrap()
}

#[test]
fn the_milestone_and_semver_tag_globs_split_the_two_workflows() {
    let output = invoke_triggers(AUTO_RELEASE_TRIGGERS, RELEASE_TRIGGERS);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn auto_release_must_keep_all_three_trigger_lanes() {
    let missing_tag =
        AUTO_RELEASE_TRIGGERS.replace("  push:\n    tags:\n      - milestone-v*\n", "");
    let output = invoke_triggers(&missing_tag, RELEASE_TRIGGERS);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("milestone-v*"),
        "the failure names the required glob"
    );

    let missing_dispatch = AUTO_RELEASE_TRIGGERS.replace(
        "  workflow_dispatch:\n    inputs:\n      bump:\n        default: auto\n      version:\n        default: ''\n",
        "",
    );
    let output = invoke_triggers(&missing_dispatch, RELEASE_TRIGGERS);
    assert!(
        !output.status.success(),
        "manual dispatch must remain a lane"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("workflow_dispatch"));

    let missing_call = AUTO_RELEASE_TRIGGERS.replace(
        "  workflow_call:\n    inputs:\n      bump:\n        default: auto\n      version:\n        default: ''\n",
        "",
    );
    let output = invoke_triggers(&missing_call, RELEASE_TRIGGERS);
    assert!(
        !output.status.success(),
        "reusable calls must remain a lane"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("workflow_call"));
}

#[test]
fn the_milestone_glob_must_not_be_widened_or_duplicated() {
    let widened = AUTO_RELEASE_TRIGGERS.replace("- milestone-v*", "- milestone-*");
    let output = invoke_triggers(&widened, RELEASE_TRIGGERS);
    assert!(!output.status.success(), "a broader prefix glob must fail");
    assert!(String::from_utf8_lossy(&output.stderr).contains("milestone-v*"));

    let doubled = AUTO_RELEASE_TRIGGERS.replace(
        "      - milestone-v*\n",
        "      - milestone-v*\n      - v*.*.*\n",
    );
    let output = invoke_triggers(&doubled, RELEASE_TRIGGERS);
    assert!(
        !output.status.success(),
        "the release glob must not be claimed twice"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("exactly"));
}

#[test]
fn the_milestone_resolution_step_must_be_conditional_and_feed_prepare() {
    let no_guard = AUTO_RELEASE_TRIGGERS.replace("        if: github.event_name == 'push'\n", "");
    let output = invoke_triggers(&no_guard, RELEASE_TRIGGERS);
    assert!(
        !output.status.success(),
        "resolving on dispatch events would fail on empty refs"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("github.event_name == 'push'"));

    let no_step = AUTO_RELEASE_TRIGGERS.replace(
        "      - id: milestone\n        if: github.event_name == 'push'\n        run: pixi run xtask resolve-milestone-tag -- ${{ github.ref_name }}\n",
        "",
    );
    let output = invoke_triggers(&no_step, RELEASE_TRIGGERS);
    assert!(!output.status.success(), "the resolver step is required");
    assert!(String::from_utf8_lossy(&output.stderr).contains("resolve-milestone-tag"));

    let no_selector =
        AUTO_RELEASE_TRIGGERS.replace("steps.milestone.outputs.version", "inputs.version");
    let output = invoke_triggers(&no_selector, RELEASE_TRIGGERS);
    assert!(
        !output.status.success(),
        "prepare must consume the resolved selector on tag pushes"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("steps.milestone.outputs.version"));
}

#[test]
fn release_yml_must_keep_the_strict_semver_glob() {
    let widened = RELEASE_TRIGGERS.replace("- v*.*.*", "- v*");
    let output = invoke_triggers(AUTO_RELEASE_TRIGGERS, &widened);
    assert!(
        !output.status.success(),
        "a widened release glob could double-trigger milestone tags"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("v*.*.*"));

    let missing = RELEASE_TRIGGERS.replace("    tags:\n      - v*.*.*\n", "");
    let output = invoke_triggers(AUTO_RELEASE_TRIGGERS, &missing);
    assert!(
        !output.status.success(),
        "release.yml without its tag filter must fail"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("v*.*.*"));
}
