//! `pipeline` — the connected-side publish pipeline as one tested boundary (TASK-76).
//!
//! The generated publisher used to carry this orchestration as a 62-line embedded Bash block
//! and a 45-line PowerShell twin: per-environment `pixi install`, then `pack`, `doctor
//! --verify`, `publish`, each phase's outcome recorded into a pipeline log, with a step
//! summary and an annotation on failure. That is the workflow's "how", so it lives here —
//! one cross-platform implementation driven by the same released binary the workflow
//! bootstrapped — and the generated workflow's step is a single `pixi-sandbox pipeline …`
//! invocation. The Bash/PowerShell twins collapse into it.
//!
//! The shape mirrors the shell it replaces, including its honest edges: the pipeline log is
//! reset at the start of every run (a rerun never appends to a previous run's evidence), a
//! phase's exit code becomes the pipeline's exit code, and a failed phase writes the same
//! `::error::` annotation and step-summary markdown the shell wrote — the summary goes to
//! `$GITHUB_STEP_SUMMARY` when the runner provides it and to stdout otherwise, so the same
//! invocation works locally.

use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::io::{Read, Write, stdout};
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Stdio};
use std::sync::Mutex;

use crate::pack::support::remove_path;

/// The pipeline log's file name inside the log directory, and the per-phase diagnostic logs
/// beside it. The names are the shell's: anything reading a failed run's artifact finds the
/// same layout.
pub const PIPELINE_LOG: &str = "pipeline.log";

/// One phase of the publish pipeline, as data: what to run, and under which name its outcome
/// is recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseCommand {
    /// The phase's name in the pipeline log: `install`, `pack`, `doctor`, or `publish`.
    pub phase: &'static str,
    /// The program to run.
    pub program: String,
    /// The arguments, in order.
    pub args: Vec<String>,
    /// Extra environment for the child — the publish phase's git auth header, and nothing
    /// else (credentials never reach the install/pack/doctor phases' logs).
    pub env: Vec<(String, String)>,
}

/// Everything `pipeline` needs, with every ambient input resolved by the caller — the same
/// rule `self_update::Request` follows (D10): tests pass tempdirs and explicit programs, so
/// no code path can reach a developer's real binary, PATH or HOME.
#[derive(Debug, Clone)]
pub struct Spec {
    /// Comma-separated environment list, passed to `pack --envs` verbatim (the plan
    /// matrix's value). The install phase splits it and strips whitespace from each name.
    pub envs: String,
    /// Target platform for the packed environments.
    pub platform: String,
    /// Orphan branch to publish the transport to.
    pub branch: String,
    /// Git remote (URL) to publish to.
    pub remote: String,
    /// Reviewed sandbox config: `pack --config` records host requirements from it, and
    /// `doctor --budget-config` enforces its budgets before publish.
    pub config: PathBuf,
    /// The standalone binary `pack` bundles as the transport bootstrap. The workflow runs
    /// the pipeline as that very binary, so the default is the running executable.
    pub self_bin: PathBuf,
    /// The executable that runs the pack/doctor/publish phases: the running binary.
    pub exe: PathBuf,
    /// The `pixi` executable the install phase runs (resolved from PATH by default).
    pub pixi: String,
    /// Whether to vendor cargo dependencies (`--cargo-vendor`).
    pub cargo_vendor: bool,
    /// Where `pack` writes the transport. Reset before the pack phase: `pack` refuses a
    /// stale output directory, and a rerun must not inherit one.
    pub transport_dir: PathBuf,
    /// Where the pipeline log and the per-phase diagnostic logs live. Reset at the start of
    /// every run.
    pub log_dir: PathBuf,
    /// The token the publish phase presents to the remote (the workflow's `github.token`),
    /// as a git `http.extraheader`. Absent locally: the operator's own git credentials apply.
    pub push_token: Option<String>,
    /// `$GITHUB_STEP_SUMMARY`, when the runner provides it. The failure summary is appended
    /// to it; when unset, the caller prints the summary to stdout instead.
    pub step_summary: Option<PathBuf>,
}

/// How a run ended: the phase that failed, its exit code, and the operator-facing texts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The phase that failed.
    pub phase: String,
    /// The phase's exit code — the pipeline exits with it, exactly as the shell's
    /// `exit "$exit_code"` did.
    pub exit_code: i32,
    /// The `::error::` workflow annotation for the failure.
    pub annotation: String,
    /// The step-summary markdown: appended to `$GITHUB_STEP_SUMMARY` when the runner
    /// provides it, printed to stdout otherwise.
    pub summary: String,
}

/// A child-process boundary: run one phase, forwarding its output live to this process's
/// stdout/stderr while appending the same bytes to the pipeline log. Faking this is how the
/// orchestration is tested without a runner, a network, or a real pixi (D10).
pub trait PhaseRunner: std::fmt::Debug {
    /// Run `command`; return the child's exit code. An `Err` means the child could not be
    /// started at all — the shell's answer to that was exit 127, so the caller maps it there.
    fn run(&mut self, command: &PhaseCommand, log: &mut (dyn Write + Send)) -> Result<i32>;
}

/// Runs phases for real: spawns the child and tees its stdout and stderr to this process's
/// streams and to the pipeline log, so a long pack is visible in the step log as it runs.
#[derive(Debug, Default)]
pub struct ProcessRunner;

impl PhaseRunner for ProcessRunner {
    fn run(&mut self, command: &PhaseCommand, log: &mut (dyn Write + Send)) -> Result<i32> {
        let mut child = StdCommand::new(&command.program)
            .args(&command.args)
            .envs(command.env.iter().map(|(key, value)| (key, value)))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("starting {} {}", command.program, command.args.join(" ")))?;
        let mut child_stdout = child.stdout.take().expect("piped stdout");
        let mut child_stderr = child.stderr.take().expect("piped stderr");

        // The log is shared by both streams: a mutex keeps the interleaving honest. Declared
        // outside the scope so the reader threads can borrow it for the scope's lifetime.
        let log = Mutex::new(log);
        std::thread::scope(|scope| {
            let out_handle = scope.spawn(|| {
                tee(&mut child_stdout, stdout(), &log);
            });
            let err_handle = scope.spawn(|| {
                tee(&mut child_stderr, std::io::stderr(), &log);
            });
            let status = child
                .wait()
                .with_context(|| format!("waiting for {}", command.program))?;
            let _ = out_handle.join();
            let _ = err_handle.join();
            Ok(status.code().unwrap_or(-1))
        })
    }
}

/// Copy `stream` to `sink` (this process's stdout or stderr) and to the shared log.
fn tee(stream: &mut dyn Read, mut sink: impl Write, log: &Mutex<&mut (dyn Write + Send)>) {
    let mut buffer = [0u8; 8192];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                let chunk = &buffer[..count];
                let _ = sink.write_all(chunk);
                let _ = sink.flush();
                if let Ok(mut log) = log.lock() {
                    let _ = log.write_all(chunk);
                }
            }
            Err(_) => break,
        }
    }
}

/// The phases a run executes, in order: install (once per environment), pack, doctor,
/// publish. The argument assembly is the policy — the exact flags the generated workflow's
/// shell used to spell out in two dialects.
#[must_use]
pub fn plan(spec: &Spec) -> Vec<PhaseCommand> {
    let mut phases = Vec::new();

    for env in spec.envs.split(',') {
        // The shell stripped every whitespace character from each name
        // (`${ENVIRONMENT//[[:space:]]/}`); keep that, so `default, web` and `default,web`
        // install the same environments.
        let name: String = env.chars().filter(|c| !c.is_whitespace()).collect();
        phases.push(PhaseCommand {
            phase: "install",
            program: spec.pixi.clone(),
            args: vec![
                "install".to_string(),
                "--frozen".to_string(),
                "-e".to_string(),
                name,
            ],
            env: Vec::new(),
        });
    }

    let mut pack_args = vec![
        "pack".to_string(),
        "--repo-root".to_string(),
        ".".to_string(),
        "--envs".to_string(),
        spec.envs.clone(),
        "--output-dir".to_string(),
        spec.transport_dir.display().to_string(),
        "--platform".to_string(),
        spec.platform.clone(),
        "--fetch-tools".to_string(),
        "--self-bin".to_string(),
        spec.self_bin.display().to_string(),
        "--config".to_string(),
        spec.config.display().to_string(),
        "--log-file".to_string(),
        spec.log_dir.join("pack.log").display().to_string(),
    ];
    if spec.cargo_vendor {
        pack_args.push("--cargo-vendor".to_string());
    }
    phases.push(PhaseCommand {
        phase: "pack",
        program: spec.exe.display().to_string(),
        args: pack_args,
        env: Vec::new(),
    });

    phases.push(PhaseCommand {
        phase: "doctor",
        program: spec.exe.display().to_string(),
        args: vec![
            "doctor".to_string(),
            "--branch-location".to_string(),
            spec.transport_dir.display().to_string(),
            "--verify".to_string(),
            "--budget-config".to_string(),
            spec.config.display().to_string(),
            "--log-file".to_string(),
            spec.log_dir.join("doctor.log").display().to_string(),
        ],
        env: Vec::new(),
    });

    // The publish phase is the only one that touches the remote, so it is the only one that
    // carries credentials: a git `http.extraheader` built from the push token, exactly the
    // `GIT_CONFIG_*` triple the shell exported.
    let env = match &spec.push_token {
        Some(token) => vec![
            ("GIT_CONFIG_COUNT".to_string(), "1".to_string()),
            (
                "GIT_CONFIG_KEY_0".to_string(),
                "http.extraheader".to_string(),
            ),
            (
                "GIT_CONFIG_VALUE_0".to_string(),
                format!(
                    "AUTHORIZATION: basic {}",
                    base64_standard(format!("x-access-token:{token}").as_bytes())
                ),
            ),
        ],
        None => Vec::new(),
    };
    phases.push(PhaseCommand {
        phase: "publish",
        program: spec.exe.display().to_string(),
        args: vec![
            "publish".to_string(),
            "--input-dir".to_string(),
            spec.transport_dir.display().to_string(),
            "--branch-name".to_string(),
            spec.branch.clone(),
            "--remote".to_string(),
            spec.remote.clone(),
            "--log-file".to_string(),
            spec.log_dir.join("publish.log").display().to_string(),
        ],
        env,
    });

    phases
}

/// Run the whole pipeline. `Ok(())` when every phase succeeded; `Err(Failure)` naming the
/// phase that failed, with the annotation and summary already produced (and the summary
/// appended to `$GITHUB_STEP_SUMMARY` when the runner provides it).
pub fn run(spec: &Spec, runner: &mut dyn PhaseRunner) -> Result<(), Failure> {
    // The log directory and the transport directory are reset first, exactly like the
    // shell's `rm -rf` + `mkdir -p`: a rerun never appends to a previous run's evidence,
    // and `pack` refuses a stale output directory.
    reset_log_dir(&spec.log_dir).map_err(|error| spawn_failure("pipeline", error))?;
    remove_path(&spec.transport_dir).map_err(|error| spawn_failure("pack", error))?;

    let log_path = spec.log_dir.join(PIPELINE_LOG);
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|error| spawn_failure("pipeline", error))?;

    for phase in plan(spec) {
        writeln!(log, "command=pipeline phase={} event=start", phase.phase)
            .map_err(|error| spawn_failure(phase.phase, error))?;
        let code = match runner.run(&phase, &mut log) {
            Ok(code) => code,
            // The shell's answer to a child that cannot start is exit 127; keep it.
            Err(error) => {
                let _ = writeln!(log, "{error:#}");
                127
            }
        };
        if code == 0 {
            writeln!(log, "command=pipeline phase={} result=success", phase.phase)
                .map_err(|error| spawn_failure(phase.phase, error))?;
            continue;
        }
        writeln!(
            log,
            "command=pipeline phase={} result=failure exit={code}",
            phase.phase
        )
        .map_err(|error| spawn_failure(phase.phase, error))?;
        let failure = failure(spec, phase.phase, code, &log_path);
        // The shell appended the summary with `|| true` — a summary that cannot be written
        // never turns the failure into a different failure.
        if let Some(path) = &spec.step_summary {
            // `>>` semantics: create the file when the runner has not yet written to it.
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
                let _ = file.write_all(failure.summary.as_bytes());
            }
        }
        return Err(failure);
    }
    Ok(())
}

/// The failure record for `phase` exiting `code`: the annotation, and the step-summary
/// markdown in the shell's exact shape (failed phase, branch, artifact name, and the last
/// 50 lines of the pipeline log when it holds anything).
fn failure(spec: &Spec, phase: &str, code: i32, log_path: &Path) -> Failure {
    let annotation = format!("::error::publish pipeline failed during {phase} (exit {code})");

    let mut summary = format!(
        "### ❌ pixi-sandbox publish failed on {}\n\n",
        spec.platform
    );
    summary.push_str(&format!(
        "- **Failed phase**: `{phase}` (exit code {code})\n"
    ));
    summary.push_str(&format!("- **Branch**: `{}`\n", spec.branch));
    summary.push_str(&format!(
        "- **Diagnostic log**: uploaded as workflow artifact `publish-diagnostics-{}`\n\n",
        spec.platform
    ));
    let excerpt = last_lines(log_path, 50);
    if !excerpt.is_empty() {
        summary.push_str("<details><summary>Diagnostic log excerpt (last 50 lines)</summary>\n\n");
        summary.push_str("```text\n");
        summary.push_str(&excerpt);
        summary.push_str("```\n\n</details>\n");
    }

    Failure {
        phase: phase.to_string(),
        exit_code: code,
        annotation,
        summary,
    }
}

/// The last `count` lines of `path`, each keeping its newline — `tail -n` semantics.
fn last_lines(path: &Path, count: usize) -> String {
    let Ok(text) = std::fs::read_to_string(path) else {
        return String::new();
    };
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(count);
    if lines.is_empty() {
        return String::new();
    }
    let mut out = lines[start..].join("\n");
    out.push('\n');
    out
}

/// The failure record for a run that could not even start writing its log: reported as the
/// phase it belongs to, with the shell's 127.
fn spawn_failure(phase: &'static str, error: impl std::fmt::Display) -> Failure {
    let annotation = format!("::error::publish pipeline failed during {phase} (exit 127)");
    Failure {
        phase: phase.to_string(),
        exit_code: 127,
        annotation,
        summary: format!("{error}"),
    }
}

/// Recreate the log directory: a rerun never appends to a previous run's evidence.
fn reset_log_dir(log_dir: &Path) -> Result<()> {
    remove_path(log_dir)?;
    std::fs::create_dir_all(log_dir)
        .with_context(|| format!("creating pipeline log directory {}", log_dir.display()))
}

/// Standard base64 (RFC 4648, with padding) — the git `http.extraheader` auth value. Kept
/// local and tiny on purpose: the workspace has no direct base64 dependency, and one header
/// is not worth a lockfile change.
#[doc(hidden)] // test boundary: the RFC 4648 vectors in tests/pipeline.rs pin this encoder
#[must_use]
pub fn base64_standard(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = u32::from(*chunk.get(1).unwrap_or(&0));
        let b2 = u32::from(*chunk.get(2).unwrap_or(&0));
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(triple >> 18) as usize & 0x3f] as char);
        out.push(ALPHABET[(triple >> 12) as usize & 0x3f] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(triple >> 6) as usize & 0x3f] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[triple as usize & 0x3f] as char
        } else {
            '='
        });
    }
    out
}
