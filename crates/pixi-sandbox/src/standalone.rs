//! The standalone-execution probe: does this binary run with nothing but itself?
//!
//! Issue #81 is the failure this exists for: a consumer's workflow packed
//! `$(command -v pixi-sandbox)` — the launcher a `pixi global install` puts on PATH — as
//! `--self-bin`. Every declared check passed: the copy is byte-faithful (so hashes could not
//! catch it), the trampoline is statically linked (so the linkage guard could not), and a
//! self-bin's bytes are not pinned into the manifest at all. The published branch then died
//! at restore, looking for `trampoline_configuration/pixi-sandbox.json` next to the
//! executable — a file that only ever exists inside a pixi global prefix.
//!
//! The oracle here is behavioural, not structural: stage nothing, believe nothing — run
//! `<candidate> --version` with an *empty* environment anchored at a scratch directory, so
//! the machine the probe simulates has no pixi global prefix, no conda activation, and no
//! shell profile, which is exactly the machine a restore runs on. A structural pre-check
//! ([`pack_refusal_for_ownership`]) still runs first where the shape is already known (the
//! task-47 ownership ladder names trampolines and managed launchers outright) because its
//! remedy is more precise; everything else is judged by execution alone.
//!
//! The trust order is the caller's contract: **never probe bytes that failed verification**
//! — `pack` probes what it just embedded, `doctor` probes only after the hash report is
//! green. The probe itself runs the candidate read-only (`stdin` null, scratch HOME, capped
//! output, killed on a timeout) so a hostile or hung candidate costs one refusal, nothing
//! more.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How long `--version` may take before the candidate is judged broken. A real static
/// binary answers in milliseconds; the budget exists for a candidate that hangs on a
/// missing resource rather than exiting.
const PROBE_TIMEOUT: Duration = Duration::from_secs(30);

/// Cap on each captured stream. A candidate that spews cannot turn the probe into a memory
/// problem; the verdict only needs enough output to explain the refusal.
const OUTPUT_CAP: usize = 16 * 1024;

/// How often the probe polls the child while waiting.
const POLL_INTERVAL: Duration = Duration::from_millis(25);

/// What one probe execution observed. The runner reports; [`probe`] judges — keeping the
/// two apart is what lets tests script every outcome through a fake runner without ever
/// spawning a process (the `ReleaseSource` precedent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeVerdict {
    /// The candidate started and exited. `output` is combined, capped stdout/stderr.
    Exited { success: bool, output: String },
    /// The candidate could not be started at all (exec format, missing interpreter).
    SpawnFailed(String),
    /// The candidate did not exit within the probe timeout and was killed.
    TimedOut,
}

/// Runs the probe's single execution. Injection point (D9's shape): the real runner is
/// [`CommandRunner`]; tests drive [`probe`] with a scripted fake.
pub trait ProbeRunner {
    /// Run `<exe> --version` with an empty environment anchored at `anchor` (its HOME,
    /// TEMP and working directory), observing rather than judging.
    fn run_version(&self, exe: &Path, anchor: &Path) -> ProbeVerdict;
}

/// The real runner: an emptied environment, scratch HOME/TMP/CWD, null stdin, capped pipes,
/// and a kill on timeout.
pub struct CommandRunner {
    timeout: Duration,
}

impl CommandRunner {
    #[must_use]
    pub fn new() -> Self {
        Self {
            timeout: PROBE_TIMEOUT,
        }
    }

    #[doc(hidden)] // test boundary: the suite needs a short fuse, not a 30 s sleep.
    #[must_use]
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl Default for CommandRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl ProbeRunner for CommandRunner {
    fn run_version(&self, exe: &Path, anchor: &Path) -> ProbeVerdict {
        // Resolve both paths before the child gets a new working directory: a relative
        // path would otherwise be re-interpreted against the anchor and fail with a
        // baffling ENOENT that reports the wrong problem.
        let (exe, anchor) =
            match absolutize(exe).and_then(|exe| absolutize(anchor).map(|anchor| (exe, anchor))) {
                Ok(paths) => paths,
                Err(error) => {
                    return ProbeVerdict::SpawnFailed(format!(
                        "cannot resolve the probe paths: {error}"
                    ));
                }
            };
        let scratch_tmp = anchor.join("tmp");
        let mut command = Command::new(exe);
        command
            .arg("--version")
            .current_dir(&anchor)
            // An airlock has no environment to inherit: no PATH of pixi shims, no conda
            // activation variables, no config next to any HOME the packer happened to have.
            .env_clear()
            .env("HOME", &anchor)
            .env("USERPROFILE", &anchor)
            .env("TMPDIR", &scratch_tmp)
            .env("TMP", &scratch_tmp)
            .env("TEMP", &scratch_tmp)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            // A cleared environment on Windows still needs the system root for process
            // creation and DLL resolution; the PATH is the system directory and nothing else.
            if let Some(root) = std::env::var_os("SystemRoot") {
                command.env("SystemRoot", &root);
                command.env("PATH", Path::new(&root).join("System32"));
            }
        }
        #[cfg(not(windows))]
        {
            command.env("PATH", "/usr/bin:/bin");
        }

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                return ProbeVerdict::SpawnFailed(format!("could not be started: {error}"));
            }
        };
        let mut stdout = child.stdout.take();
        let mut stderr = child.stderr.take();

        // Drain both pipes on their own threads: polling `try_wait` with unread pipes
        // deadlocks a child that fills the buffer, and joining before waiting hangs on a
        // stuck child. Free threads, not a scope — see the timeout arm for why.
        let out = std::thread::spawn(move || read_capped(stdout.as_mut()));
        let err = std::thread::spawn(move || read_capped(stderr.as_mut()));
        let started = Instant::now();
        let verdict = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status.success()),
                Ok(None) if started.elapsed() > self.timeout => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err(ProbeVerdict::TimedOut);
                }
                Ok(None) => std::thread::sleep(POLL_INTERVAL),
                Err(error) => {
                    break Err(ProbeVerdict::SpawnFailed(format!(
                        "the process started but could not be waited on: {error}"
                    )));
                }
            }
        };
        match verdict {
            Ok(success) => {
                let stdout_text = out.join().unwrap_or_default();
                let stderr_text = err.join().unwrap_or_default();
                ProbeVerdict::Exited {
                    success,
                    output: append_stream(stdout_text, &stderr_text),
                }
            }
            Err(done) => {
                // A killed candidate can leave a *grandchild* alive (kill(2) reaches one
                // pid, not the tree) that inherited the pipes; joining the readers here
                // would wait out the orphan's whole lifetime. The verdict is already
                // decided, so the readers are detached: they die with the process, and the
                // refusal is prompt either way.
                drop(out);
                drop(err);
                done
            }
        }
    }
}

/// Absolute spelling of `path` without requiring it to exist (`std::fs::absolute` is the
/// same idea, newer than this crate's rust-version).
fn absolutize(path: &Path) -> std::io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn read_capped(reader: Option<&mut impl Read>) -> String {
    let Some(reader) = reader else {
        return String::new();
    };
    let mut collected = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let room = OUTPUT_CAP.saturating_sub(collected.len());
                if room > 0 {
                    collected.extend_from_slice(&chunk[..n.min(room)]);
                }
            }
        }
    }
    String::from_utf8_lossy(&collected).into_owned()
}

fn append_stream(mut stdout: String, stderr: &str) -> String {
    if !stderr.trim().is_empty() {
        if !stdout.is_empty() && !stdout.ends_with('\n') {
            stdout.push('\n');
        }
        stdout.push_str(stderr);
    }
    stdout
}

/// Why a candidate is not standalone, with the evidence attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    verdict: ProbeVerdict,
}

impl Refusal {
    #[must_use]
    pub fn verdict(&self) -> &ProbeVerdict {
        &self.verdict
    }

    /// The full refusal text: what was tried, how it failed, why a hash-green transport can
    /// still be unrestorable, and the remedy that actually packs a runnable binary.
    #[must_use]
    pub fn render(&self, exe: &Path) -> String {
        let detail = match &self.verdict {
            ProbeVerdict::Exited { output, .. } => {
                let output = output.trim();
                if output.is_empty() {
                    "exited non-zero with no output".to_string()
                } else {
                    format!("exited non-zero:\n{}", indent_tail(output, 8))
                }
            }
            ProbeVerdict::SpawnFailed(error) => error.clone(),
            ProbeVerdict::TimedOut => format!(
                "did not exit within the {} s probe timeout and was killed",
                PROBE_TIMEOUT.as_secs()
            ),
        };
        format!(
            "{} does not run standalone\n\
             probed as it will be executed at restore: `<tool> --version` with an empty\n\
             environment (no pixi global prefix, no conda activation, no shell profile) — {detail}\n\
             a transport's hashes can be green for these bytes and the branch still unrestorable:\n\
             a self-bin's content is not pinned into the manifest, and a static trampoline passes\n\
             the linkage check too (issue #81)\n\
             remedy: pass the standalone release asset `pixi-sandbox-<target>` (verified against\n\
             the release's SHA256SUMS) — never `$(command -v pixi-sandbox)` from a `pixi global\n\
             install`, which is a launcher that needs its global prefix beside it",
            exe.display()
        )
    }
}

fn indent_tail(output: &str, lines: usize) -> String {
    let collected: Vec<&str> = output.lines().collect();
    let start = collected.len().saturating_sub(lines);
    collected[start..]
        .iter()
        .map(|line| format!("  | {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Judge one execution: exit 0 is the whole contract.
pub fn probe(exe: &Path, anchor: &Path, runner: &impl ProbeRunner) -> Result<(), Refusal> {
    match runner.run_version(exe, anchor) {
        ProbeVerdict::Exited { success: true, .. } => Ok(()),
        verdict => Err(Refusal { verdict }),
    }
}

/// The structural pre-check pack runs *before* embedding: the candidates whose brokenness
/// is already known from path provenance alone (the task-47 ownership ladder), so their
/// refusal can name the exact shape instead of just the probe output. Everything the ladder
/// calls runnable — standalone, a conda prefix binary, a restored transport tool — returns
/// `None` and is judged by execution instead.
#[must_use]
pub fn pack_refusal_for_ownership(path: &Path) -> Option<String> {
    match crate::self_update::ownership::classify(path) {
        crate::self_update::ownership::Ownership::GlobalTrampoline => Some(format!(
            "{} is a `pixi global install` trampoline, not the standalone pixi-sandbox binary\n\
             it only runs beside its global prefix (trampoline_configuration/<name>.json),\n\
             which a transport never ships — packed as --self-bin it is exactly the failure of\n\
             issue #81: the branch passes every hash check and dies at restore\n\
             remedy: download the standalone release asset `pixi-sandbox-<target>` from the\n\
             pixi-sandbox release and verify it against the release's SHA256SUMS",
            path.display()
        )),
        crate::self_update::ownership::Ownership::ManagedLauncher => Some(format!(
            "{} is a pixi-sandbox-managed launcher script, not the binary itself\n\
             it only execs the manifest-verified tool under .pixi/tools/<platform>/; embedding\n\
             the script ships the indirection without the binary\n\
             remedy: pass the tool copy itself (<project>/.pixi/tools/<platform>/pixi-sandbox),\n\
             or the standalone release asset",
            path.display()
        )),
        _ => None,
    }
}

/// The pixi platform of the machine running this binary, when it is one the release matrix
/// ships. The probe is only meaningful when the embedded bytes can execute here — a pack or
/// doctor host that cannot run the target platform reports a skip, never a verdict.
#[must_use]
pub fn host_platform() -> Option<&'static str> {
    platform(std::env::consts::OS, std::env::consts::ARCH)
}

/// Rust's host spelling (`std::env::consts::{OS, ARCH}`) to the pixi platform label. Pure,
/// so every supported row — and the unsupported one — is exercised without needing that
/// host; keep it in step with the matrix in `release.yml`.
///
/// Thin wrapper over [`pixi_sandbox_core::platform::Platform`], the single source of this
/// mapping (task-55); kept as its own function so the `&'static str` boundary this module's
/// callers and tests already depend on does not change.
#[doc(hidden)] // test boundary: named rows, not API.
#[must_use]
pub fn platform(os: &str, arch: &str) -> Option<&'static str> {
    pixi_sandbox_core::platform::Platform::from_os_arch(os, arch)
        .map(pixi_sandbox_core::platform::Platform::as_str)
}
