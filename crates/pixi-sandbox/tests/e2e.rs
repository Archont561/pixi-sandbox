//! Full fixture-backed integration tests for the offline lifecycle.
//!
//! These tests are the cold proof: they use the committed transport fixture, real CLI commands,
//! local Git only, and (where the runner permits it) a severed network namespace.

#![cfg(unix)]

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

fn bin() -> Command {
    Command::cargo_bin("pixi-sandbox").expect("binary builds")
}

fn fixture_transport() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/transport")
}

/// The complete pixi project definition tests pack (never this repository, see
/// `tests/fixtures/README.md`).
fn demo_project() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/demo-project")
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap().flatten() {
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path).unwrap();
        if metadata.is_dir() {
            copy_tree(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).unwrap();
            fs::set_permissions(&destination_path, metadata.permissions()).unwrap();
        }
    }
}

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

    bin()
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
    // restore scratch or at an unsubstituted pack placeholder.
    let mut text = Vec::new();
    text_files_under(&prefix, &mut text);
    assert!(text.len() > 8, "the fixture must stage a realistic prefix");
    for path in &text {
        let body = fs::read_to_string(path).unwrap();
        for forbidden in [".restore-work/stage-demo", "@PREFIX@"] {
            assert!(
                !body.contains(forbidden),
                "{} still points at {forbidden}",
                path.display()
            );
        }
    }

    // Restore scratch is not a deliverable: a completed airlock has no `.restore-work` left.
    assert!(!airlock.join(".pixi/.restore-work").exists());
    assert!(
        airlock
            .join(".pixi-sandbox/vendor/demo-dep-1.0.0/Cargo.toml")
            .is_file()
    );
    assert!(airlock.join(".cargo/config.toml").is_file());
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

    bin()
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

/// The gate script is part of the deliverable, not fixture data: this test runs the real
/// `scripts/airlock-gate.sh` from this repository (the one sanctioned reach outside the crate,
/// recorded in `tests/fixtures.rs`), because testing a copy would prove nothing about the
/// script CI runs.
fn gate_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/airlock-gate.sh")
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

    bin()
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
/// (via --transport) and a stub prefix carrying a fabricated conda-meta record is rejected —
/// by a test, not by a comment in the script.
#[test]
fn the_airlock_gate_rejects_a_forged_conda_meta_record() {
    let temp = tempfile::tempdir().unwrap();
    let transport = temp.path().join("transport");
    let airlock = temp.path().join("airlock-project");
    copy_tree(&fixture_transport(), &transport);
    fs::create_dir_all(&airlock).unwrap();

    bin()
        .args([
            "restore",
            "--branch-location",
            transport.to_str().unwrap(),
            "--output-path",
            airlock.to_str().unwrap(),
        ])
        .assert()
        .success();

    // The gate command, shared by every run below.
    let run_gate = || {
        let mut command = StdCommand::new("bash");
        command
            .arg(gate_script())
            .arg(airlock.canonicalize().unwrap())
            .arg("demo")
            .arg("--skip-cargo")
            .arg("--transport")
            .arg(transport.canonicalize().unwrap());
        command
    };

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

    let degraded = run_gate().output().unwrap();
    let degraded_log = String::from_utf8_lossy(&degraded.stdout);
    assert!(
        degraded.status.success(),
        "the gate must pass while the bundled binary predates the oracle:\n{degraded_log}"
    );
    assert!(
        degraded_log.contains("predates the per-file oracle"),
        "the degradation must be said out loud, not passed silently:\n{degraded_log}"
    );
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
    let status = run_gate().status().unwrap();
    assert!(
        status.success(),
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
    let output = run_gate().output().unwrap();
    assert!(
        !output.status.success(),
        "the gate must reject a prefix with a fabricated conda-meta record"
    );
    let log = String::from_utf8_lossy(&output.stdout);
    assert!(
        log.contains("forged-9.9.9-0.json")
            && log.contains("present in the restored prefix but not in the manifest's file list"),
        "the gate must name the forged record, got:\n{log}"
    );

    // And the failure was the record, not flakiness: remove it and the gate passes again.
    fs::remove_file(records.join("forged-9.9.9-0.json")).unwrap();
    let status = run_gate().status().unwrap();
    assert!(
        status.success(),
        "the gate must pass again once the record is gone"
    );
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}
