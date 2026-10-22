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
