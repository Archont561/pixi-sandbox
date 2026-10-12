//! TASK-78 / ADR-28 Slice 1: the `prepare` orchestration and its digest gate.
//!
//! Orchestration is exercised through a fake runner that records what it was asked to run and
//! writes into a tempdir; one `#[cfg(unix)]` case drives the real `ShellRunner` through `sh -c`.
//! Nothing reaches the network or this checkout.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use pixi_sandbox_core::payload::PayloadError;
use pixi_sandbox_core::payload_config::{PayloadConfig, SourceDecl};
use pixi_sandbox_core::shard::sha256_bytes;

use pixi_sandbox::prepare::{PrepareRunner, PreparedPayload, ShellRunner, prepare};

/// A runner that records the calls it received and optionally writes an artifact through a
/// closure, so each test controls exactly what the "prepare" step produced.
struct FakeRunner<F: Fn(&Path)> {
    calls: RefCell<Vec<(String, PathBuf)>>,
    effect: F,
}

impl<F: Fn(&Path)> std::fmt::Debug for FakeRunner<F> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FakeRunner")
            .field("calls", &self.calls.borrow())
            .finish_non_exhaustive()
    }
}

impl<F: Fn(&Path)> PrepareRunner for FakeRunner<F> {
    fn run(&self, command: &str, working_dir: &Path) -> anyhow::Result<()> {
        self.calls
            .borrow_mut()
            .push((command.to_string(), working_dir.to_path_buf()));
        (self.effect)(working_dir);
        Ok(())
    }
}

fn config(sha256: &str) -> PayloadConfig {
    PayloadConfig {
        schema: 1,
        name: "demo".to_string(),
        version: "1.0.0".to_string(),
        platform: "linux-64".to_string(),
        noarch: false,
        entry_point: None,
        prepare: "fetch the payload".to_string(),
        working_dir: ".".to_string(),
        artifact: "artifact.bin".to_string(),
        payload: "payload".to_string(),
        source: SourceDecl {
            url: "https://example.invalid/demo-1.0.0.bin".to_string(),
            sha256: sha256.to_string(),
        },
    }
}

#[test]
fn prepare_runs_the_declared_command_then_reads_the_gate_checked_tree() {
    let dir = tempfile::tempdir().expect("tempdir");
    let digest = sha256_bytes(b"the fetched bytes");
    let runner = FakeRunner {
        calls: RefCell::new(Vec::new()),
        effect: |working_dir: &Path| {
            std::fs::write(working_dir.join("artifact.bin"), b"the fetched bytes")
                .expect("artifact");
            std::fs::create_dir_all(working_dir.join("payload/bin")).expect("payload dir");
            std::fs::write(working_dir.join("payload/bin/tool"), b"\x7fELFtool").expect("tool");
        },
    };

    let prepared: PreparedPayload =
        prepare(&config(&digest), dir.path(), &runner).expect("a matching artifact passes");

    assert_eq!(
        *runner.calls.borrow(),
        [("fetch the payload".to_string(), dir.path().to_path_buf())],
        "the command ran verbatim in the config directory"
    );
    assert_eq!(prepared.artifact, dir.path().join("artifact.bin"));
    assert_eq!(prepared.payload_root, dir.path().join("payload"));
    assert_eq!(prepared.working_dir, dir.path());
    assert!(
        prepared
            .entries
            .iter()
            .any(|entry| matches!(entry, pixi_sandbox_core::payload::PayloadEntry::File { path, .. } if path == "bin/tool")),
        "the tree under `payload` is read"
    );
}

#[test]
fn prepare_resolves_a_relative_working_dir_under_the_config_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let digest = sha256_bytes(b"bytes");
    let runner = FakeRunner {
        calls: RefCell::new(Vec::new()),
        effect: |working_dir: &Path| {
            std::fs::write(working_dir.join("artifact.bin"), b"bytes").expect("artifact");
            std::fs::write(working_dir.join("payload"), b"bytes").expect("payload");
        },
    };
    let mut config = config(&digest);
    config.working_dir = "work".to_string();
    config.payload = "payload".to_string();

    let prepared = prepare(&config, dir.path(), &runner).expect("a relative working dir works");

    assert_eq!(prepared.working_dir, dir.path().join("work"));
    assert!(
        prepared.working_dir.is_dir(),
        "the working directory is created"
    );
    assert_eq!(
        *runner.calls.borrow(),
        [("fetch the payload".to_string(), dir.path().join("work"))]
    );
}

#[test]
fn prepare_refuses_when_the_fetched_artifact_does_not_match_the_declared_digest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let declared = sha256_bytes(b"what the config pinned");
    let runner = FakeRunner {
        calls: RefCell::new(Vec::new()),
        effect: |working_dir: &Path| {
            std::fs::write(working_dir.join("artifact.bin"), b"the upstream changed")
                .expect("artifact");
        },
    };

    let error = prepare(&config(&declared), dir.path(), &runner)
        .expect_err("a changed upstream must fail the run");

    let payload_error = error
        .downcast_ref::<PayloadError>()
        .expect("a typed payload error");
    assert!(
        matches!(payload_error, PayloadError::ArtifactMismatch { expected, actual, .. } if expected == &declared && actual != &declared)
    );
}

#[test]
fn prepare_refuses_a_command_that_did_not_leave_an_artifact() {
    let dir = tempfile::tempdir().expect("tempdir");
    let digest = sha256_bytes(b"anything");
    let runner = FakeRunner {
        calls: RefCell::new(Vec::new()),
        effect: |_working_dir: &Path| {},
    };

    let error = prepare(&config(&digest), dir.path(), &runner)
        .expect_err("a command that wrote nothing is caught by the gate");

    let payload_error = error
        .downcast_ref::<PayloadError>()
        .expect("a typed payload error");
    assert!(
        matches!(payload_error, PayloadError::Io { path, .. } if path.contains("artifact.bin"))
    );
}

#[test]
fn a_failing_command_is_reported_as_a_prepare_failure_naming_the_command() {
    #[derive(Debug)]
    struct FailingRunner;

    impl PrepareRunner for FailingRunner {
        fn run(&self, _command: &str, _working_dir: &Path) -> anyhow::Result<()> {
            anyhow::bail!("exit status: 7")
        }
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let error = prepare(&config(&sha256_bytes(b"x")), dir.path(), &FailingRunner)
        .expect_err("a failing prepare fails the run");

    let payload_error = error
        .downcast_ref::<PayloadError>()
        .expect("a typed payload error");
    assert!(
        matches!(payload_error, PayloadError::PrepareFailed { command, detail } if command == "fetch the payload" && detail.contains('7')),
        "the operator is told which command failed and why: {payload_error}"
    );
}

#[cfg(unix)]
#[test]
fn the_real_shell_runner_fetches_a_payload_the_way_a_config_would() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bytes = b"a real fetched payload\n";
    let digest = sha256_bytes(bytes);
    let mut config = config(&digest);
    config.prepare =
        "printf 'a real fetched payload\\n' > artifact.bin && mkdir payload && cp artifact.bin payload/".to_string();

    let prepared =
        prepare(&config, dir.path(), &ShellRunner).expect("sh -c runs the command for real");

    assert_eq!(std::fs::read(&prepared.artifact).expect("artifact"), bytes);
    assert!(
        prepared
            .entries
            .iter()
            .any(|entry| matches!(entry, pixi_sandbox_core::payload::PayloadEntry::File { path, .. } if path == "artifact.bin")),
        "the payload tree is read after the real command"
    );
}
