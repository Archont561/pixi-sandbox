//! Full fixture-backed integration tests for the offline lifecycle.
//!
//! These tests are the cold proof: they use the committed transport fixture, real CLI commands,
//! local Git only, and (where the runner permits it) a severed network namespace.

#![cfg(unix)]

mod support;

use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use support::{
    bin, copy_tree, demo_project, fixture_transport, host_platform, isolated_bin, make_executable,
};

/// The relocation rule `restore.rs` promises, applied here as an independent check: valid UTF-8
/// text without NUL bytes is rewritten, and everything else is left exactly as it was staged.
fn text_files_under(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap().flatten() {
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            text_files_under(&path, out);
        } else if let Ok(bytes) = fs::read(&path) {
            if !bytes.contains(&0) && std::str::from_utf8(&bytes).is_ok() {
                out.push(path);
            }
        }
    }
}

#[test]
fn fixture_doctor_publish_and_restore_is_the_complete_offline_proof() {
    let temp = tempfile::tempdir().unwrap();
    let transport = temp.path().join("transport");
    let remote = temp.path().join("remote.git");
    let airlock = temp.path().join("airlock-project");
    copy_tree(&fixture_transport(), &transport);
    fs::create_dir_all(&airlock).unwrap();

    let doctor = bin()
        .args([
            "doctor",
            "--branch-location",
            transport.to_str().unwrap(),
            "--verify",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("OK — every declared byte matches"));
    if host_platform() == "linux-64" {
        // On the fixture's own platform, doctor also proves the embedded bootstrap runs —
        // hashes bless the bytes, only execution blesses the behaviour (issue #81).
        doctor.stdout(predicate::str::contains(
            "tool pixi-sandbox v0.1.0: runs standalone",
        ));
    }

    // task-54 AC#4: the packed branch's embedded tool executes with nothing but its own
    // bytes — no pixi global prefix, no profile. The order is the trust boundary: only a
    // hash-green transport earns execution, so this probe follows doctor, not precedes it.
    let probe_home = temp.path().join("standalone-probe-home");
    fs::create_dir_all(&probe_home).unwrap();
    let version = StdCommand::new(transport.join(".pixi-sandbox/tools/linux-64/pixi-sandbox"))
        .arg("--version")
        .current_dir(&probe_home)
        .env_clear()
        .env("HOME", &probe_home)
        .env("TMPDIR", &probe_home)
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("the embedded tool starts");
    assert!(
        version.status.success(),
        "the embedded bootstrap must run standalone: {}",
        String::from_utf8_lossy(&version.stderr)
    );
    assert!(
        String::from_utf8_lossy(&version.stdout).contains("pixi-sandbox 0.1.0"),
        "the embedded bootstrap answers --version: {}",
        String::from_utf8_lossy(&version.stdout)
    );

    assert!(
        StdCommand::new("git")
            .args(["init", "-q", "--bare"])
            .arg(&remote)
            .status()
            .unwrap()
            .success()
    );
    bin()
        .args([
            "publish",
            "--input-dir",
            transport.to_str().unwrap(),
            "--branch-name",
            "sandbox/demo-linux-64",
            "--remote",
            remote.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("published"));

    isolated_bin(temp.path())
        .args([
            "restore",
            "--branch-location",
            transport.to_str().unwrap(),
            "--output-path",
            airlock.to_str().unwrap(),
        ])
        .assert()
        .success()
        // pkg-config, a CMake config, a gdb script, a `bin` script and a linker script: the five
        // kinds a real conda prefix ships with its install prefix baked in.
        .stdout(predicate::str::contains("relocated 8 text file(s)"))
        .stdout(predicate::str::contains("restore complete"));

    let prefix = airlock.join(".pixi/envs/demo");
    let final_prefix = prefix.to_str().unwrap();
    assert!(prefix.join("conda-meta/pixi_env_prefix").is_file());
    assert!(
        fs::read_to_string(prefix.join("conda-meta/pixi_env_prefix"))
            .unwrap()
            .contains(final_prefix)
    );
    // Real conda metadata, real activation script, and a real shared library, all from the
    // committed prefix archive rather than a hand-written payload.
    assert!(
        prefix
            .join("conda-meta/zlib-1.3.2-h25fd6f3_3.json")
            .is_file()
    );
    assert!(
        prefix
            .join("etc/conda/activate.d/activate-gxx_linux-64.sh")
            .is_file()
    );
    assert!(prefix.join("lib/libz.so.1.3.2").is_file());
    assert!(fs::read_link(prefix.join("lib/libz.so")).is_ok());

    let pkg_config = fs::read_to_string(prefix.join("lib/pkgconfig/zlib.pc")).unwrap();
    assert!(pkg_config.contains(final_prefix));

    // A binary that embeds its build prefix keeps that path: rewriting it would corrupt a
    // 26 KiB executable, and the length proves the staging prefix was never spliced in.
    let lzmainfo = prefix.join("bin/lzmainfo");
    let lzmainfo_bytes = fs::read(&lzmainfo).unwrap();
    assert_eq!(lzmainfo_bytes.len(), 26336);
    assert!(
        String::from_utf8_lossy(&lzmainfo_bytes)
            .contains("/opt/conda/envs/demo-build-fixture-00000"),
        "a NUL-containing file must not be relocated"
    );

    // The blanket invariant behind #18: no text file in the restored environment may point into
    // restore scratch or at an unsubstituted pack placeholder. The file set is restored data, so
    // it cannot be a static #[case] — collect every (file, word) offender and report them all.
    let mut text = Vec::new();
    text_files_under(&prefix, &mut text);
    assert!(text.len() > 8, "the fixture must stage a realistic prefix");
    let offenders: Vec<String> = text
        .iter()
        .flat_map(|path| {
            let body = fs::read_to_string(path).unwrap();
            [".restore-work/stage-demo", "@PREFIX@"]
                .into_iter()
                .filter(|forbidden| body.contains(forbidden))
                .map(move |forbidden| format!("{} still points at {forbidden}", path.display()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(offenders.is_empty(), "{}", offenders.join("\n"));

    // Restore scratch is not a deliverable: a completed airlock has no `.restore-work` left.
    assert!(!airlock.join(".pixi/.restore-work").exists());
    assert!(
        airlock
            .join(".pixi-sandbox/vendor/demo-dep-1.0.0/Cargo.toml")
            .is_file()
    );
    assert!(
        airlock
            .join(".pixi-sandbox/cargo-home/config.toml")
            .is_file()
    );
    assert!(!airlock.join(".cargo/config.toml").exists());
    assert!(
        airlock
            .join(".pixi/envs/demo/etc/conda/activate.d/pixi-sandbox-cargo-home.sh")
            .is_file()
    );
}

#[test]
#[cfg(target_os = "linux")]
fn fixture_restore_succeeds_in_a_severed_network_namespace() {
    if !StdCommand::new("unshare")
        .args(["-rn", "true"])
        .status()
        .is_ok_and(|status| status.success())
    {
        return;
    }

    let temp = tempfile::tempdir().unwrap();
    let transport = temp.path().join("transport");
    let airlock = temp.path().join("airlock-project");
    copy_tree(&fixture_transport(), &transport);
    fs::create_dir_all(&airlock).unwrap();

    let status = StdCommand::new("unshare")
        .args([
            "-rn",
            assert_cmd::cargo::cargo_bin("pixi-sandbox")
                .to_str()
                .unwrap(),
        ])
        // task-33: registration runs by default inside the namespace too; keep it in the tempdir.
        .env("HOME", temp.path())
        .env("USERPROFILE", temp.path())
        .env("SHELL", "/usr/bin/bash")
        .env("TMPDIR", temp.path())
        .args([
            "restore",
            "--branch-location",
            transport.to_str().unwrap(),
            "--output-path",
            airlock.to_str().unwrap(),
        ])
        .status()
        .expect("unshare runs");

    assert!(status.success(), "fixture restore must not need a network");
    assert!(
        airlock
            .join(".pixi/envs/demo/conda-meta/pixi_env_prefix")
            .is_file()
    );
}

/// The cold proof for #18 needs a *real* prefix: `pixi-pack` records conda's build-path
/// placeholders and the real `pixi-unpack` substitutes the staging prefix into them, so this is
/// the only test that sees what the tools actually produce. Ignored because it needs the network
/// for both the solver and the pinned helper tools.
#[test]
#[ignore = "needs the network: solves an environment and downloads the pinned helper tools"]
fn a_real_packed_environment_restores_with_nothing_pointing_into_restore_scratch() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("demo-project");
    let transport = temp.path().join("transport");
    let airlock = temp.path().join("airlock-project");
    copy_tree(&demo_project(), &project);
    fs::create_dir_all(&airlock).unwrap();

    bin()
        .args([
            "pack",
            "--repo-root",
            project.to_str().unwrap(),
            "--envs",
            "default",
            "--output-dir",
            transport.to_str().unwrap(),
            "--platform",
            "linux-64",
            "--fetch-tools",
        ])
        .assert()
        .success();

    bin()
        .args([
            "doctor",
            "--branch-location",
            transport.to_str().unwrap(),
            "--verify",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("OK — every declared byte matches"));

    isolated_bin(temp.path())
        .args([
            "restore",
            "--branch-location",
            transport.to_str().unwrap(),
            "--output-path",
            airlock.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("restore complete"));

    let prefix = airlock.join(".pixi/envs/default");
    let pkg_config = fs::read_to_string(prefix.join("lib/pkgconfig/zlib.pc")).unwrap();
    assert!(pkg_config.contains(prefix.to_str().unwrap()));

    let mut text = Vec::new();
    text_files_under(&prefix, &mut text);
    for path in &text {
        let body = fs::read_to_string(path).unwrap();
        assert!(
            !body.contains(".restore-work/stage-default"),
            "{} still points into restore scratch",
            path.display()
        );
    }
    assert!(!airlock.join(".pixi/.restore-work").exists());
}

/// Issue #109 / TASK-75 AC#2, the read half: a transport that declares host requirements says so
/// from the branch itself — `doctor` reads the section out of the manifest, not out of any
/// source checkout — and a transport without the section stays exactly as it was: it still
/// verifies, and its report carries no host section at all. Exercised against the committed
/// fixture transport, copied into a tempdir, with the section injected into `manifest.json` the
/// same way the packer now writes it.
#[test]
fn doctor_reports_the_host_requirements_the_transport_declares() {
    let temp = tempfile::tempdir().unwrap();
    let transport = temp.path().join("transport");
    copy_tree(&fixture_transport(), &transport);

    // The fixture carries no section, so the report has none — the backward-compatible half.
    let plain = bin()
        .args([
            "doctor",
            "--branch-location",
            transport.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();
    let plain_json = String::from_utf8_lossy(&plain.get_output().stdout).to_string();
    assert!(
        !plain_json.contains("host_requirements"),
        "a transport that declares nothing must not report a host section:\n{plain_json}"
    );

    // Inject the section the packer writes, then read it back.
    let manifest_path = transport.join(".pixi-sandbox/manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["host_requirements"] = serde_json::json!({
        "libc": ">=2.34",
        "packages": ["fontconfig", "fonts-dejavu", "xvfb"],
        "services": ["dbus"],
        "capabilities": ["display"],
        "headless": ["xvfb-run"],
    });
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let declared = bin()
        .args([
            "doctor",
            "--branch-location",
            transport.to_str().unwrap(),
            "--verify",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&declared.get_output().stdout).to_string();
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor emits one JSON value");
    assert_eq!(json["host_requirements"]["libc"], ">=2.34");
    assert_eq!(
        json["host_requirements"]["packages"],
        serde_json::json!(["fontconfig", "fonts-dejavu", "xvfb"])
    );
    assert_eq!(
        json["host_requirements"]["capabilities"],
        serde_json::json!(["display"])
    );
    // Adding the section changed no blob, so a section-carrying transport still verifies green.
    assert_eq!(json["verify"]["ok"], true, "stdout: {stdout}");

    // The human report names the entries and does not claim a verdict about this host.
    let human = bin()
        .args(["doctor", "--branch-location", transport.to_str().unwrap()])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&human.get_output().stdout).to_string();
    assert!(
        stdout.contains("declared requirements: libc >=2.34 · packages fontconfig, fonts-dejavu, xvfb · services dbus · capabilities display · headless xvfb-run"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("not probed by this build"),
        "the human report must not imply a host verdict: {stdout}"
    );
}

const NOOP_DRIFT_KIB: u64 = 64;

#[cfg(feature = "ci")]
#[derive(Clone, Debug)]
struct GateInputs {
    project: PathBuf,
    transport: PathBuf,
    envs: Vec<String>,
    skip_cargo: bool,
}

#[cfg(feature = "ci")]
impl GateInputs {
    fn from_env() -> Self {
        let project = required_gate_path("PIXI_SANDBOX_GATE_PROJECT");
        let transport = required_gate_path("PIXI_SANDBOX_GATE_TRANSPORT");
        let envs = required_gate_envs();
        let skip_cargo = std::env::var("PIXI_SANDBOX_GATE_SKIP_CARGO")
            .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "yes"));
        Self {
            project,
            transport,
            envs,
            skip_cargo,
        }
    }
}

#[cfg(feature = "ci")]
fn required_gate_path(name: &str) -> PathBuf {
    let value = std::env::var_os(name)
        .unwrap_or_else(|| panic!("{name} must name the real-transport airlock gate input"));
    let path = PathBuf::from(value);
    assert!(
        path.is_dir(),
        "{name}={} is not a directory",
        path.display()
    );
    path
}

#[cfg(feature = "ci")]
fn required_gate_envs() -> Vec<String> {
    let value = std::env::var("PIXI_SANDBOX_GATE_ENVS")
        .expect("PIXI_SANDBOX_GATE_ENVS must list the environments to prove");
    let envs: Vec<String> = value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect();
    assert!(
        !envs.is_empty(),
        "PIXI_SANDBOX_GATE_ENVS must name at least one environment"
    );
    envs
}

fn gate_tools_bin(project: &Path) -> PathBuf {
    let tools_root = project.join(".pixi/tools");
    let mut candidates: Vec<PathBuf> = fs::read_dir(&tools_root)
        .unwrap_or_else(|err| panic!("reading {} failed: {err}", tools_root.display()))
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| {
            gate_tool_candidate(&path.join("pixi")) || gate_tool_candidate(&path.join("pixi.exe"))
        })
        .collect();
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no manifest-owned pixi executable found under {}",
            tools_root.display()
        )
    })
}

fn gate_tool_candidate(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn gate_tool(tools_bin: &Path, name: &str) -> PathBuf {
    let unix = tools_bin.join(name);
    if gate_tool_candidate(&unix) {
        return unix;
    }
    let windows = tools_bin.join(format!("{name}.exe"));
    if gate_tool_candidate(&windows) {
        return windows;
    }
    panic!(
        "bundled {name} is missing or not executable under {}",
        tools_bin.display()
    );
}

fn run_gate_command(command: &mut StdCommand, description: &str) -> String {
    let output = command
        .output()
        .unwrap_or_else(|err| panic!("starting {description} failed: {err}"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "{description} failed with {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status
    );
    stdout.into_owned()
}

#[cfg(feature = "ci")]
fn gate_path_with_tools_first(tools_bin: &Path) -> std::ffi::OsString {
    let mut paths = vec![tools_bin.to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    std::env::join_paths(paths).expect("PATH entries are valid")
}

#[cfg(feature = "ci")]
fn assert_gate_tools_are_first_on_path_and_execute(project: &Path) {
    let tools_bin = gate_tools_bin(project);
    let path = gate_path_with_tools_first(&tools_bin);
    let first_path_entry = std::env::split_paths(&path)
        .next()
        .expect("the gate PATH is non-empty");
    assert_eq!(
        first_path_entry, tools_bin,
        "the manifest-owned tools directory must be first on PATH"
    );

    let pixi = gate_tool(&tools_bin, "pixi");
    let sandbox = gate_tool(&tools_bin, "pixi-sandbox");
    let unpack = gate_tool(&tools_bin, "pixi-unpack");
    assert!(unpack.is_file(), "pixi-unpack must be bundled for restore");

    let pixi_version = run_gate_command(
        StdCommand::new(&pixi).env("PATH", &path).arg("--version"),
        "bundled pixi --version",
    );
    assert!(
        pixi_version.contains("pixi"),
        "bundled pixi printed an unexpected version line: {pixi_version}"
    );
    let sandbox_version = run_gate_command(
        StdCommand::new(&sandbox)
            .env("PATH", &path)
            .arg("--version"),
        "bundled pixi-sandbox --version",
    );
    assert!(
        sandbox_version.contains("pixi-sandbox"),
        "bundled pixi-sandbox printed an unexpected version line: {sandbox_version}"
    );
}

fn assert_gate_prefix(project: &Path, env_name: &str, when: &str) {
    let prefix = project.join(".pixi/envs").join(env_name);
    assert!(
        prefix.is_dir(),
        "environment '{env_name}' has no prefix at {} ({when})",
        prefix.display()
    );
    let conda_meta = prefix.join("conda-meta");
    let records = fs::read_dir(&conda_meta)
        .unwrap_or_else(|err| panic!("reading {} failed: {err}", conda_meta.display()))
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .count();
    assert!(
        records > 0,
        "environment '{env_name}' has an empty conda-meta ({when}); the prefix was never populated"
    );
    let files = files_under(&prefix).len();
    assert!(
        files > records,
        "environment '{env_name}' holds only its {records} conda-meta records and no payload ({when})"
    );
}

fn files_under(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_files(root, &mut out);
    out.sort();
    out
}

fn collect_files(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root)
        .unwrap_or_else(|err| panic!("reading {} failed: {err}", root.display()))
        .flatten()
    {
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            collect_files(&path, out);
        } else if entry.file_type().unwrap().is_file() {
            out.push(path);
        }
    }
}

fn relative_file_set(root: &Path) -> Vec<String> {
    files_under(root)
        .into_iter()
        .map(|path| {
            path.strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

fn du_kib(path: &Path) -> u64 {
    let output = StdCommand::new("du")
        .arg("-sk")
        .arg(path)
        .output()
        .unwrap_or_else(|err| panic!("starting du -sk {} failed: {err}", path.display()));
    assert!(
        output.status.success(),
        "du -sk {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .expect("du prints a size")
        .parse()
        .expect("du size is numeric")
}

fn gate_restored_tree_result(
    project: &Path,
    transport: &Path,
    envs: &[String],
) -> Result<GateIntegrity, String> {
    let tools_bin = gate_tools_bin(project);
    let sandbox = gate_tool(&tools_bin, "pixi-sandbox");
    let help = StdCommand::new(&sandbox)
        .args(["doctor", "--help"])
        .output()
        .map_err(|err| format!("starting bundled pixi-sandbox doctor --help failed: {err}"))?;
    if !help.status.success() {
        return Err(format!(
            "bundled pixi-sandbox doctor --help failed with {}\nstdout:\n{}\nstderr:\n{}",
            help.status,
            String::from_utf8_lossy(&help.stdout),
            String::from_utf8_lossy(&help.stderr)
        ));
    }
    let help_text = String::from_utf8_lossy(&help.stdout);
    if !help_text.contains("--verify-restored") {
        return Ok(GateIntegrity::DegradedPreOracle);
    }

    let envs_arg = envs.join(",");
    let output = StdCommand::new(&sandbox)
        .arg("doctor")
        .arg("--branch-location")
        .arg(transport)
        .arg("--verify-restored")
        .arg(project)
        .arg("--envs")
        .arg(envs_arg)
        .output()
        .map_err(|err| format!("starting doctor --verify-restored failed: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "doctor --verify-restored failed with {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(GateIntegrity::Verified)
}

fn assert_gate_restored_tree(project: &Path, transport: &Path, envs: &[String]) -> GateIntegrity {
    gate_restored_tree_result(project, transport, envs)
        .unwrap_or_else(|message| panic!("{message}"))
}

#[derive(Debug, PartialEq, Eq)]
enum GateIntegrity {
    Verified,
    DegradedPreOracle,
}

fn assert_gate_pixi_install_is_noop(project: &Path, envs: &[String]) {
    let tools_bin = gate_tools_bin(project);
    let pixi = gate_tool(&tools_bin, "pixi");
    for env_name in envs {
        assert_gate_prefix(project, env_name, "after restore");

        run_gate_command(
            StdCommand::new(&pixi).current_dir(project).args([
                "install",
                "--frozen",
                "--offline",
                "-e",
                env_name,
            ]),
            "pixi install settle",
        );

        let pixi_root = project.join(".pixi");
        let list_before = relative_file_set(&pixi_root);
        let kib_before = du_kib(&pixi_root);

        run_gate_command(
            StdCommand::new(&pixi).current_dir(project).args([
                "install",
                "--frozen",
                "--offline",
                "-e",
                env_name,
            ]),
            "pixi install no-op check",
        );

        let list_after = relative_file_set(&pixi_root);
        let kib_after = du_kib(&pixi_root);
        assert_eq!(
            list_before, list_after,
            "pixi install was not a no-op for '{env_name}': the set of files under .pixi changed"
        );
        let drift = kib_after.abs_diff(kib_before);
        assert!(
            drift <= NOOP_DRIFT_KIB,
            "pixi install was not a no-op for '{env_name}': .pixi drifted by {drift} KiB ({kib_before} -> {kib_after}), over the {NOOP_DRIFT_KIB} KiB tolerance"
        );

        assert_gate_prefix(project, env_name, "after the offline install");
    }
}

#[cfg(feature = "ci")]
fn assert_gate_cargo_check_uses_pixi(project: &Path, skip_cargo: bool) {
    if skip_cargo || !project.join("Cargo.toml").is_file() {
        return;
    }
    let sandbox_config = project.join(".pixi-sandbox/cargo-home/config.toml");
    let project_config = project.join(".cargo/config.toml");
    let cargo_config = if sandbox_config.is_file() {
        let hooks = fs::read_dir(project.join(".pixi/envs"))
            .unwrap_or_else(|err| panic!("reading restored envs failed: {err}"))
            .filter_map(Result::ok)
            .map(|entry| {
                entry
                    .path()
                    .join("etc/conda/activate.d/pixi-sandbox-cargo-home.sh")
            })
            .filter(|path| path.is_file())
            .count();
        assert!(
            hooks > 0,
            "no Pixi activation hook wires CARGO_HOME for cargo check"
        );
        sandbox_config
    } else {
        // The airlock workflow intentionally restores with the latest released binary, not the
        // PR build. Until the issue-55 restore fix is itself released, that binary still wires
        // the project `.cargo/config.toml`; accept that legacy shape here so the workflow keeps
        // proving the released asset while source-level tests prove the new sandbox-owned path.
        assert!(
            project_config.is_file(),
            "neither sandbox-owned Cargo config nor legacy project Cargo config exists; the vendored tree was not wired in"
        );
        project_config
    };
    let cargo_config_text = fs::read_to_string(&cargo_config)
        .unwrap_or_else(|err| panic!("reading {} failed: {err}", cargo_config.display()));
    assert!(
        cargo_config_text.contains("source.crates-io")
            && cargo_config_text.contains(".pixi-sandbox/vendor"),
        "Cargo config {} does not redirect crates.io to the restored vendor tree",
        cargo_config.display()
    );

    let pixi = gate_tool(&gate_tools_bin(project), "pixi");
    run_gate_command(
        StdCommand::new(&pixi).current_dir(project).args([
            "run",
            "--frozen",
            "--",
            "cargo",
            "check",
            "--offline",
            "--locked",
            "--quiet",
        ]),
        "pixi run -- cargo check --offline",
    );
}

// CI-only airlock gate input contract:
//
// * PIXI_SANDBOX_GATE_PROJECT: restored project path.
// * PIXI_SANDBOX_GATE_TRANSPORT: extracted sandbox branch/transport path.
// * PIXI_SANDBOX_GATE_ENVS: comma-separated environments to prove.
// * PIXI_SANDBOX_GATE_SKIP_CARGO: optional true/1/yes to skip the cargo check when the
//   transport intentionally did not vendor crates.
//
// The default feature set must not compile this module; CI builds a nextest archive with
// `--features ci` while egress is still available, then replays the archive in both tiers.
#[cfg(feature = "ci")]
mod ci_gate {
    use super::*;

    #[test]
    fn airlock_gate_inputs_are_present() {
        let inputs = GateInputs::from_env();
        assert!(inputs.project.is_dir());
        assert!(inputs.transport.is_dir());
        assert!(!inputs.envs.is_empty());
    }

    #[test]
    fn airlock_gate_manifest_owned_tools_are_first_on_path_and_execute() {
        let inputs = GateInputs::from_env();
        assert_gate_tools_are_first_on_path_and_execute(&inputs.project);
    }

    #[test]
    fn airlock_gate_prefixes_are_real_installed_prefixes() {
        let inputs = GateInputs::from_env();
        for env_name in &inputs.envs {
            assert_gate_prefix(&inputs.project, env_name, "after restore");
        }
    }

    #[test]
    fn airlock_gate_manifest_restored_tree_matches_the_manifest() {
        let inputs = GateInputs::from_env();
        match assert_gate_restored_tree(&inputs.project, &inputs.transport, &inputs.envs) {
            GateIntegrity::Verified => {}
            GateIntegrity::DegradedPreOracle => eprintln!(
                "::notice::the bundled pixi-sandbox predates the per-file oracle (D13); \
                 restored-tree integrity cannot be checked for this transport"
            ),
        }
    }

    #[test]
    fn airlock_gate_pixi_install_is_a_noop() {
        let inputs = GateInputs::from_env();
        assert_gate_pixi_install_is_noop(&inputs.project, &inputs.envs);
    }

    #[test]
    fn airlock_gate_cargo_check_uses_the_pixi_entrypoint() {
        let inputs = GateInputs::from_env();
        assert_gate_cargo_check_uses_pixi(&inputs.project, inputs.skip_cargo);
    }

    #[test]
    fn airlock_gate_rejects_a_forged_conda_meta_record() {
        let inputs = GateInputs::from_env();
        let env_name = inputs
            .envs
            .first()
            .expect("gate inputs have at least one environment");
        let forged = inputs
            .project
            .join(".pixi/envs")
            .join(env_name)
            .join("conda-meta/forged-9.9.9-0.json");
        fs::write(
            &forged,
            r#"{"name":"forged","version":"9.9.9","build":"0","files":[]}"#,
        )
        .unwrap();
        let result = gate_restored_tree_result(
            &inputs.project,
            &inputs.transport,
            std::slice::from_ref(env_name),
        );
        let _ = fs::remove_file(&forged);
        match result {
            Ok(GateIntegrity::DegradedPreOracle) => eprintln!(
                "::notice::the bundled pixi-sandbox predates the per-file oracle (D13); \
                 forged-record rejection cannot be checked for this transport"
            ),
            Ok(GateIntegrity::Verified) => {
                panic!("the gate must reject a forged conda-meta record")
            }
            Err(message) => assert!(
                message.contains("forged-9.9.9-0.json")
                    && message.contains(
                        "present in the restored prefix but not in the manifest's file list"
                    ),
                "the forged-record failure must name the fabricated record, got:\n{message}"
            ),
        }
    }
}

/// task-10: `doctor --verify-restored` checks the tree restore produced, not just the branch
/// it came from. A faithful restore passes; a fabricated conda-meta record — the exact stub
/// prefix that fooled every shape check — is rejected, and so is tampered payload content.
#[test]
fn doctor_verify_restored_checks_the_tree_restore_produced() {
    let temp = tempfile::tempdir().unwrap();
    let transport = temp.path().join("transport");
    let airlock = temp.path().join("airlock-project");
    copy_tree(&fixture_transport(), &transport);
    fs::create_dir_all(&airlock).unwrap();

    isolated_bin(temp.path())
        .args([
            "restore",
            "--branch-location",
            transport.to_str().unwrap(),
            "--output-path",
            airlock.to_str().unwrap(),
        ])
        .assert()
        .success();

    bin()
        .args([
            "doctor",
            "--branch-location",
            transport.to_str().unwrap(),
            "--verify-restored",
            airlock.to_str().unwrap(),
            "--envs",
            "demo",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "checked against the manifest's file list — 14 entry(ies), 0 failure(s)",
        ))
        .stdout(predicate::str::contains(
            "OK — the restored tree matches the manifest",
        ));

    // The attack the transport-shape checks could not see: a record that was never packed.
    let prefix = airlock.join(".pixi/envs/demo");
    fs::write(
        prefix.join("conda-meta/forged-9.9.9-0.json"),
        r#"{"name":"forged","version":"9.9.9","build":"0","files":[]}"#,
    )
    .unwrap();
    bin()
        .args([
            "doctor",
            "--branch-location",
            transport.to_str().unwrap(),
            "--verify-restored",
            airlock.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("forged-9.9.9-0.json"))
        .stdout(predicate::str::contains(
            "present in the restored prefix but not in the manifest's file list",
        ));

    // And tampered payload content, not just extra files: the oracle lists per-file digests.
    fs::remove_file(prefix.join("conda-meta/forged-9.9.9-0.json")).unwrap();
    let pc = prefix.join("lib/pkgconfig/zlib.pc");
    let body = fs::read_to_string(&pc).unwrap();
    fs::write(&pc, body.replace("prefix=", "prefix=/tampered/")).unwrap();
    bin()
        .args([
            "doctor",
            "--branch-location",
            transport.to_str().unwrap(),
            "--verify-restored",
            airlock.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "content does not match the manifest's file list",
        ));
}

/// task-10's headline claim, proven end to end: the airlock gate calls doctor --verify-restored
/// and a stub prefix carrying a fabricated conda-meta record is rejected — by a test, not by a
/// comment in a script.
#[test]
fn the_airlock_gate_rejects_a_forged_conda_meta_record() {
    let temp = tempfile::tempdir().unwrap();
    let transport = temp.path().join("transport");
    let airlock = temp.path().join("airlock-project");
    copy_tree(&fixture_transport(), &transport);
    fs::create_dir_all(&airlock).unwrap();

    isolated_bin(temp.path())
        .args([
            "restore",
            "--branch-location",
            transport.to_str().unwrap(),
            "--output-path",
            airlock.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Degraded mode first, while the fixture stub still plays the bundled binary: a transport
    // packed by an older release embeds a pixi-sandbox without --verify-restored, and the gate
    // must pass with a loud notice — failing the release transition would teach operators to
    // ignore the gate. The stub's `doctor --help` echoes its arguments, never naming the flag.
    //
    // pixi is replaced with a recording stub first, for a separate reason: the gate must run
    // `pixi install` inside the restored project, not wherever the gate itself was invoked
    // from (CI invokes it from a checkout that is itself a pixi project — the wrong one). The
    // stub writes its working directory to a marker so the assertion below is a fact.
    let marker = airlock.join("gate-pixi-cwd.txt");
    let pixi_stub = airlock.join(".pixi/tools/linux-64/pixi");
    fs::write(
        &pixi_stub,
        format!(
            "#!/bin/sh\npwd > {}\nif [ \"$1\" = \"--version\" ]; then echo \"pixi 0.81.0\"; exit 0; fi\n",
            marker.display()
        ),
    )
    .unwrap();
    make_executable(&pixi_stub);

    assert_eq!(
        assert_gate_restored_tree(&airlock, &transport, &["demo".to_string()]),
        GateIntegrity::DegradedPreOracle,
        "the gate must explicitly degrade while the bundled binary predates the oracle"
    );
    assert_gate_pixi_install_is_noop(&airlock, &["demo".to_string()]);
    let recorded_cwd = fs::read_to_string(&marker).unwrap();
    assert_eq!(
        recorded_cwd.trim(),
        airlock.canonicalize().unwrap().to_str().unwrap(),
        "pixi install must run inside the restored project, not the gate's invocation cwd"
    );

    // The fixture's pixi-sandbox is a script stub that cannot run doctor. Restore verified it
    // as shipped; now swap in the real binary the test build produced, which is what the gate
    // executes. (Restore itself would refuse a dynamically linked tool — this happens after.)
    let tool = airlock.join(".pixi/tools/linux-64/pixi-sandbox");
    fs::copy(env!("CARGO_BIN_EXE_pixi-sandbox"), &tool).unwrap();
    make_executable(&tool);

    // A faithful restore passes the whole gate, integrity section included.
    assert_eq!(
        assert_gate_restored_tree(&airlock, &transport, &["demo".to_string()]),
        GateIntegrity::Verified,
        "the gate must pass for a faithful fixture restore"
    );

    // The stub prefix: everything the shape checks look at is present, plus one fabricated
    // record the transport never carried. The gate must fail and name the file.
    let records = airlock.join(".pixi/envs/demo/conda-meta");
    fs::write(
        records.join("forged-9.9.9-0.json"),
        r#"{"name":"forged","version":"9.9.9","build":"0","files":[]}"#,
    )
    .unwrap();
    let message = gate_restored_tree_result(&airlock, &transport, &["demo".to_string()])
        .expect_err("the gate must reject a forged conda-meta record");
    assert!(
        message.contains("forged-9.9.9-0.json")
            && message
                .contains("present in the restored prefix but not in the manifest's file list"),
        "the forged-record failure must name the fabricated record, got:\n{message}"
    );

    // And the failure was the record, not flakiness: remove it and the gate passes again.
    fs::remove_file(records.join("forged-9.9.9-0.json")).unwrap();
    assert_eq!(
        assert_gate_restored_tree(&airlock, &transport, &["demo".to_string()]),
        GateIntegrity::Verified,
        "the gate must pass again once the record is gone"
    );
}
