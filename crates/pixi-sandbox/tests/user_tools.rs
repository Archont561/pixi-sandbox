//! task-33 integration tests: persisting the restored Pixi tools on the user PATH.
//!
//! Every test isolates HOME, USERPROFILE, SHELL and the user-bin directory in its own
//! tempdir — a test that touches the developer's real home is a broken test (D10's rule,
//! applied at the user level). The transport is the checked-in fixture; its `pixi` stub
//! resolves `pixi sandbox` through PATH exactly like the real pixi, so the discovery chain
//! is exercised end to end: launcher → verified tool → PATH lookup → `pixi-sandbox`.

#![cfg(unix)]

mod support;

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use support::{bin, copy_tree, fixture_transport, make_executable};

/// A restore whose user-tool side effects land in `home`, with a deterministic shell.
fn restore(home: &Path, project: &Path) -> Command {
    let mut command = bin();
    command
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("SHELL", "/usr/bin/bash")
        .args([
            "restore",
            "--branch-location",
            fixture_transport().to_str().unwrap(),
            "--output-path",
            project.to_str().unwrap(),
        ]);
    command
}

fn project_under(temp: &Path, name: &str) -> PathBuf {
    let project = temp.join(name);
    fs::create_dir_all(&project).unwrap();
    project
}

/// task-33 AC#1 + AC#6: a first successful restore registers both tools, only after the
/// restored-tree verification passed.
#[test]
fn first_restore_registers_pixi_and_pixi_sandbox_in_the_user_bin() {
    let temp = tempfile::tempdir().unwrap();
    let project = project_under(temp.path(), "project");

    restore(temp.path(), &project)
        .assert()
        .success()
        .stdout(predicate::str::contains("verify restored tree"))
        .stdout(predicate::str::contains(
            "OK — the restored tree matches the manifest",
        ))
        .stdout(predicate::str::contains("register user tools"))
        .stdout(predicate::str::contains("pixi -> "))
        .stdout(predicate::str::contains("pixi-sandbox -> "))
        .stdout(predicate::str::contains("PATH: added"))
        .stdout(predicate::str::contains(
            "an already-running shell cannot be changed",
        ));

    let bin = temp.path().join(".local/bin");
    for name in ["pixi", "pixi-sandbox"] {
        let launcher = bin.join(name);
        let body = fs::read_to_string(&launcher)
            .unwrap_or_else(|_| panic!("launcher {name} must exist at {}", launcher.display()));
        assert!(body.contains("managed by pixi-sandbox"), "{body}");
        assert!(
            body.contains(&format!(
                "exec \"{}\"",
                project.join(".pixi/tools/linux-64").join(name).display()
            )),
            "launcher must exec the manifest-verified tool copy: {body}"
        );
        assert!(is_executable(&launcher));
    }
    // The profile carries exactly one managed PATH block naming the bin directory.
    let profile = fs::read_to_string(temp.path().join(".profile")).unwrap();
    assert!(profile.contains(bin.to_str().unwrap()));
    assert_eq!(
        profile.matches("# >>> pixi-sandbox user tools").count(),
        1,
        "exactly one managed block: {profile}"
    );
    // Nothing was written outside the bin dir and the profile.
    assert!(
        fs::read_dir(temp.path())
            .unwrap()
            .flatten()
            .all(|entry| matches!(
                entry.file_name().to_string_lossy().as_ref(),
                ".local" | ".profile" | "project"
            ))
    );
}

/// task-33 AC#6: `pixi sandbox` must resolve through PATH after registration — pixi finds
/// `pixi-<command>` binaries on PATH, and the fixture's pixi stub does exactly that.
#[test]
fn registered_tools_resolve_pixi_sandbox_through_path() {
    let temp = tempfile::tempdir().unwrap();
    let project = project_under(temp.path(), "project");

    restore(temp.path(), &project).assert().success();

    // A shell that knows nothing but the user bin (and the system dirs the stubs' `sh`
    // needs): `pixi sandbox --version` must reach the *registered* pixi-sandbox launcher,
    // which execs the project's verified copy.
    let output = StdCommand::new("sh")
        .arg("-c")
        .arg("pixi sandbox --version")
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", temp.path().join(".local/bin").display()),
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "`pixi sandbox` did not resolve on a PATH with only the user bin: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "pixi-sandbox 0.1.0"
    );

    // And plain `pixi` is the registered launcher too, not whatever the host happens to have.
    let output = StdCommand::new("sh")
        .arg("-c")
        .arg("pixi --version")
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", temp.path().join(".local/bin").display()),
        )
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "pixi 0.81.0"
    );
}

/// task-33 AC#6: re-restoring the same project changes nothing (idempotent repeat).
#[test]
fn repeat_restore_is_idempotent() {
    let temp = tempfile::tempdir().unwrap();
    let project = project_under(temp.path(), "project");

    restore(temp.path(), &project).assert().success();
    let launcher_before = fs::read_to_string(temp.path().join(".local/bin/pixi")).unwrap();
    let profile_before = fs::read_to_string(temp.path().join(".profile")).unwrap();

    restore(temp.path(), &project)
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("already current"))
        .stdout(predicate::str::contains("already on PATH"));

    assert_eq!(
        fs::read_to_string(temp.path().join(".local/bin/pixi")).unwrap(),
        launcher_before
    );
    assert_eq!(
        fs::read_to_string(temp.path().join(".profile")).unwrap(),
        profile_before,
        "a repeat must not stack PATH blocks"
    );
}

/// task-33 AC#4: re-restoring *another* project retargets only the managed launchers, and the
/// most recent restore becomes the user-level tool source.
#[test]
fn re_restoring_another_project_retargets_the_managed_launchers() {
    let temp = tempfile::tempdir().unwrap();
    let first = project_under(temp.path(), "first");
    let second = project_under(temp.path(), "second");

    restore(temp.path(), &first).assert().success();
    restore(temp.path(), &second)
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("retargeted from"))
        .stdout(predicate::str::contains(
            "the most recently registered restore is now the user-level source",
        ));

    let launcher = fs::read_to_string(temp.path().join(".local/bin/pixi")).unwrap();
    assert!(launcher.contains(second.to_str().unwrap()));
    assert!(!launcher.contains(first.to_str().unwrap()));
    // One PATH block, still the same bin directory.
    let profile = fs::read_to_string(temp.path().join(".profile")).unwrap();
    assert_eq!(profile.matches("# >>> pixi-sandbox user tools").count(), 1);
}

/// task-33 AC#4: an unrelated existing `pixi` or `pixi-sandbox` command is refused unless
/// explicitly forced — and the refusal does not undo the completed restore.
#[test]
fn an_unmanaged_collision_is_refused_unless_forced() {
    let temp = tempfile::tempdir().unwrap();
    let project = project_under(temp.path(), "project");
    let bin = temp.path().join(".local/bin");
    fs::create_dir_all(&bin).unwrap();
    // What a `pixi global install` leaves behind: real, user-owned, no marker.
    fs::write(bin.join("pixi"), "#!/bin/sh\nexec /opt/pixi/pixi \"$@\"\n").unwrap();

    restore(temp.path(), &project)
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not managed by pixi-sandbox"))
        .stderr(predicate::str::contains("--user-tools skip"))
        .stderr(predicate::str::contains(
            "the restored project itself is complete",
        ));

    // The project itself is intact and the user's file is untouched.
    assert!(
        project
            .join(".pixi/envs/demo/conda-meta/pixi_env_prefix")
            .is_file()
    );
    assert_eq!(
        fs::read_to_string(bin.join("pixi")).unwrap(),
        "#!/bin/sh\nexec /opt/pixi/pixi \"$@\"\n"
    );
    assert!(!temp.path().join(".profile").exists());

    // Force replaces the unmanaged entry with a managed launcher.
    restore(temp.path(), &project)
        .arg("--force")
        .assert()
        .success();
    assert!(
        fs::read_to_string(bin.join("pixi"))
            .unwrap()
            .contains("managed by pixi-sandbox")
    );
}

/// task-33 AC#5: the opt-out disables every home/profile change without changing the
/// restore or its verification.
#[test]
fn the_opt_out_touches_nothing_outside_the_project() {
    let temp = tempfile::tempdir().unwrap();
    let project = project_under(temp.path(), "project");

    restore(temp.path(), &project)
        .arg("--user-tools")
        .arg("skip")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "none registered (--user-tools skip",
        ))
        .stdout(predicate::str::contains(
            "OK — the restored tree matches the manifest",
        ));

    // The project is complete; the home directory never gained a thing.
    assert!(
        project
            .join(".pixi/envs/demo/conda-meta/pixi_env_prefix")
            .is_file()
    );
    assert!(!temp.path().join(".local").exists());
    assert!(!temp.path().join(".profile").exists());
}

/// task-33 AC#5, the other half of the contract: `skip` is also what the environment
/// variable selects, which is how the generated launcher forwards an operator's choice.
#[test]
fn the_policy_can_come_from_the_environment() {
    let temp = tempfile::tempdir().unwrap();
    let project = project_under(temp.path(), "project");

    let mut command = restore(temp.path(), &project);
    command
        .env("PIXI_SANDBOX_USER_TOOLS", "skip")
        .assert()
        .success()
        .stdout(predicate::str::contains("none registered"));
    assert!(!temp.path().join(".local").exists());

    // And an explicit --user-bin relocates the launchers and the PATH block with them.
    let custom = temp.path().join("airlock-bin");
    restore(temp.path(), &project)
        .arg("--force")
        .arg("--user-bin")
        .arg(custom.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains(custom.to_str().unwrap()));
    assert!(custom.join("pixi").is_file());
    let profile = fs::read_to_string(temp.path().join(".profile")).unwrap();
    assert!(profile.contains(custom.to_str().unwrap()));
    assert!(
        !profile.contains(".local/bin"),
        "old default must not linger: {profile}"
    );
}

/// task-33 AC#6: a machine with no HOME gets a clear remedy, not a silent skip — and the
/// restore itself has already completed by then.
#[test]
fn a_missing_home_is_an_error_that_names_the_remedy() {
    let temp = tempfile::tempdir().unwrap();
    let project = project_under(temp.path(), "project");

    let mut command = restore(temp.path(), &project);
    command
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "neither HOME nor USERPROFILE is set",
        ))
        .stderr(predicate::str::contains("--user-tools skip"));

    // The project is complete: the failure is about registration, not the restore.
    assert!(
        project
            .join(".pixi/envs/demo/conda-meta/pixi_env_prefix")
            .is_file()
    );
    assert!(!project.join(".pixi/sandbox-env.sh").exists());
}

/// The generated launchers pass `--user-tools` explicitly *and* forward their own arguments
/// (`exec … --user-tools "${PIXI_SANDBOX_USER_TOOLS:-register}" "$@"`). The CLI must let the
/// operator's later flag win — a "cannot be used multiple times" error there would make the
/// documented override path (`./restore.sh --user-tools skip`) unusable.
#[test]
fn a_later_user_tools_flag_wins_over_an_earlier_one() {
    let temp = tempfile::tempdir().unwrap();
    let project = project_under(temp.path(), "project");

    restore(temp.path(), &project)
        .arg("--user-tools")
        .arg("register")
        .arg("--user-tools")
        .arg("skip")
        .assert()
        .success()
        .stdout(predicate::str::contains("none registered"));
    assert!(!temp.path().join(".local").exists());
}

/// The full hand-off, with no stubs in the policy path: `init` generates `restore.sh`, a
/// branch carries the fixture transport with the *real* binary embedded, and the launcher —
/// which selects `--user-tools` explicitly — drives a real restore that registers the tools.
///
/// Gated to linux-64 like the severed-network proof: the fixture transport and its manifest
/// are linux-64, and the launcher resolves the branch for the host platform.
#[test]
#[cfg(all(unix, target_os = "linux", target_arch = "x86_64"))]
fn the_generated_launcher_selects_the_policy_and_registers_user_tools() {
    let temp = tempfile::tempdir().unwrap();
    let project = project_under(temp.path(), "project");
    fs::write(
        project.join("pixi.toml"),
        "[workspace]\nname = \"fixture\"\nchannels = [\"conda-forge\"]\nplatforms = [\"linux-64\"]\n",
    )
    .unwrap();

    bin()
        .args(["init", "--project-root", project.to_str().unwrap()])
        .assert()
        .success();

    let git = |args: &[&str]| {
        let status = StdCommand::new("git")
            .args(args)
            .current_dir(&project)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["add", "."]);
    git(&[
        "-c",
        "user.name=test",
        "-c",
        "user.email=test@example.invalid",
        "commit",
        "-qm",
        "main",
    ]);

    // The orphan branch carries the fixture transport, with the stub bootstrap replaced by a
    // script shim around the *real* binary this test was built with: the manifest's declared
    // size follows the shim, and the airlock linkage rule is honoured (a `#!` wrapper is a
    // launcher, not a dynamically linked tool — the same shape the user-bin registration
    // itself writes).
    git(&["checkout", "-q", "--orphan", "sandbox/developer-linux-64"]);
    git(&["rm", "-qrf", "."]);
    copy_tree(&fixture_transport(), &project);
    let nested = project.join(".pixi-sandbox/tools/linux-64/pixi-sandbox");
    fs::write(
        &nested,
        format!(
            "#!/bin/sh\nexec {} \"$@\"\n",
            assert_cmd::cargo::cargo_bin("pixi-sandbox").display()
        ),
    )
    .unwrap();
    make_executable(&nested);
    let manifest_path = project.join(".pixi-sandbox/manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["tools"]["pixi-sandbox"]["size_bytes"] =
        serde_json::json!(fs::metadata(&nested).unwrap().len());
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    git(&["add", "."]);
    git(&[
        "-c",
        "user.name=test",
        "-c",
        "user.email=test@example.invalid",
        "commit",
        "-qm",
        "sandbox",
    ]);
    git(&["checkout", "-q", "main"]);

    let output = StdCommand::new("sh")
        .arg("restore.sh")
        .current_dir(&project)
        .env("HOME", temp.path())
        .env("USERPROFILE", temp.path())
        .env("SHELL", "/usr/bin/bash")
        .env_remove("PIXI_SANDBOX_USER_TOOLS")
        .env_remove("PIXI_SANDBOX_USER_BIN")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "the generated launcher failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("register user tools"), "{stdout}");
    assert!(
        stdout.contains("OK — the restored tree matches the manifest"),
        "{stdout}"
    );

    // The registered launcher execs the manifest-verified copy this restore materialised.
    let launcher = fs::read_to_string(temp.path().join(".local/bin/pixi-sandbox")).unwrap();
    assert!(
        launcher.contains("exec \"") && launcher.contains(".pixi/tools/linux-64/pixi-sandbox\""),
        "launcher must exec the verified copy: {launcher}"
    );
    assert!(
        project
            .join(".pixi/envs/demo/conda-meta/pixi_env_prefix")
            .is_file()
    );
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|meta| meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}
