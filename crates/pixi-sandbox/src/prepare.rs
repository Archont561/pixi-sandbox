//! `prepare` — run the config-declared command that fetches a payload, then gate on its digest
//! (TASK-78, ADR-28).
//!
//! pixi-sandbox never downloads a payload itself. The committed `pixi-sandbox-payload.toml`
//! names a `prepare` command — a `curl` pipeline, an install script, anything the consumer
//! reviews — and this module runs it, checks the fetched artifact's SHA256 against the declared
//! `source.sha256` (AC#13), and reads the resulting file or directory into the entries the pure
//! rules take. Splitting the network out this way is what keeps the airlock's "nothing is
//! downloaded at restore time" true: the download happened on the connected host, once, and its
//! digest is pinned in a reviewed file.
//!
//! The process seam is a trait, exactly as `pipeline::PhaseRunner` is: orchestration is tested
//! against a fake that records what it was asked to run, while the real runner is the one
//! platform-specific corner (`sh -c` / `cmd /C`).

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

use pixi_sandbox_core::payload::{PayloadEntry, PayloadError, read_payload_tree, verify_artifact};
use pixi_sandbox_core::payload_config::PayloadConfig;

/// Runs the config-declared `prepare` command in a working directory.
pub trait PrepareRunner: std::fmt::Debug {
    /// Run `command` in `working_dir`; a non-zero exit or a failure to start is an error whose
    /// message is shown to the operator.
    ///
    /// # Errors
    ///
    /// Returns an error when the command cannot be started or exits non-zero.
    fn run(&self, command: &str, working_dir: &Path) -> Result<()>;
}

/// Runs `prepare` for real through the host's shell: `sh -c` on Unix, `cmd /C` on Windows.
#[derive(Debug, Default)]
pub struct ShellRunner;

impl PrepareRunner for ShellRunner {
    fn run(&self, command: &str, working_dir: &Path) -> Result<()> {
        let (program, flag) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };
        let status = StdCommand::new(program)
            .arg(flag)
            .arg(command)
            .current_dir(working_dir)
            .status()
            .with_context(|| format!("starting `{program} {flag} {command}`"))?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("`{command}` exited with {status}")
        }
    }
}

/// What a successful `prepare` produced: the payload tree, the artifact that was digest-checked,
/// and where both live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPayload {
    /// The payload entries read from `payload_root`, ready for the pure rules.
    pub entries: Vec<PayloadEntry>,
    /// The digest-checked artifact's absolute path.
    pub artifact: PathBuf,
    /// The payload file or directory's absolute path.
    pub payload_root: PathBuf,
    /// The directory the command ran in (absolute).
    pub working_dir: PathBuf,
}

/// Run the config's `prepare` command, verify the fetched artifact's digest, and read the payload
/// tree.
///
/// `config_dir` is the directory holding the config file; `config.working_dir` is resolved
/// relative to it. The working directory is created if absent so a `prepare` command that writes
/// into a fresh subdirectory works, and the command runs there so relative paths in it mean what
/// the reviewed file says.
///
/// # Errors
///
/// [`PayloadError::PrepareFailed`] when the command fails, [`PayloadError::Io`] when the working
/// directory cannot be created or the artifact is missing, [`PayloadError::ArtifactMismatch`]
/// when the fetched bytes differ from the declared digest (AC#13), or a tree-read/identity error
/// from [`read_payload_tree`].
pub fn prepare(
    config: &PayloadConfig,
    config_dir: &Path,
    runner: &dyn PrepareRunner,
) -> Result<PreparedPayload> {
    let working_dir = config_dir.join(&config.working_dir);
    std::fs::create_dir_all(&working_dir).map_err(|error| PayloadError::Io {
        path: working_dir.display().to_string(),
        message: error.to_string(),
    })?;

    runner
        .run(&config.prepare, &working_dir)
        .map_err(|error| PayloadError::PrepareFailed {
            command: config.prepare.clone(),
            detail: format!("{error:#}"),
        })?;

    read_prepared(config, config_dir)
}

/// Verify and read a payload that is already on disk, without running the `prepare` command.
///
/// This is the `--skip-prepare` path: an operator who already fetched the artifact (or a rerun
/// after a downstream failure) reuses the bytes, but the digest gate and the tree read are the
/// same as a fresh run — a stale or tampered artifact is still refused.
///
/// # Errors
///
/// [`PayloadError::Io`] when the artifact is missing, [`PayloadError::ArtifactMismatch`] when it
/// differs from the declared digest, or a tree-read/identity error.
pub fn read_prepared(config: &PayloadConfig, config_dir: &Path) -> Result<PreparedPayload> {
    let working_dir = config_dir.join(&config.working_dir);
    let artifact = working_dir.join(&config.artifact);
    verify_artifact(&artifact, &config.source.sha256)?;

    let payload_root = working_dir.join(&config.payload);
    let entries = read_payload_tree(&payload_root)?;

    Ok(PreparedPayload {
        entries,
        artifact,
        payload_root,
        working_dir,
    })
}
