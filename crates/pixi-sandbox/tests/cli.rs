//! CLI-level smoke tests.
//!
//! Everything that needs a payload uses the checked-in fixture transport, and everything that
//! writes copies it into a tempdir first: the fixtures are read-only, and no test ever points
//! at this repository (see `tests/fixtures/README.md`).

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
#[cfg(unix)]
use serde_json::json;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

fn bin() -> Command {
    Command::cargo_bin("pixi-sandbox").expect("binary builds")
}

/// A synthetic, already-packed payload committed for the test suite.
fn fixture_transport() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/transport")
}

/// Copy the fixture out of the repository: tests never mutate a fixture in place.
fn transport_copy() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    copy_tree(&fixture_transport(), dir.path());
    dir
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap().flatten() {
        let path = entry.path();
        let target = to.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            fs::copy(&path, &target).unwrap();
        }
    }
}

#[cfg(unix)]
fn write_executable(path: &Path, script: &str) {
    use std::os::unix::fs::PermissionsExt;

    fs::write(path, script).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

/// Minimal connected-side tools. They make the command contract testable without asking the
/// fixture project to install pixi or turning this suite into a network test.
#[cfg(unix)]
fn fake_tools(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    write_executable(
        &dir.join("pixi-pack"),
        r#"#!/bin/sh
set -eu
out=''
env=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o) out="$2"; shift 2 ;;
    -e) env="$2"; shift 2 ;;
    *) shift ;;
  esac
done
mkdir -p "$out/channel/noarch"
printf '{"version":"fake"}\n' > "$out/pixi-pack.json"
printf '# fake package for %s\n' "$env" > "$out/channel/noarch/$env-0.1.0-0.conda"
dd if=/dev/zero bs=1 count=1024 2>/dev/null >> "$out/channel/noarch/$env-0.1.0-0.conda"
printf 'unpacked 2 KiB\n'
"#,
    );
    write_executable(
        &dir.join("pixi"),
        r#"#!/bin/sh
if [ "${1:-}" = '--version' ]; then echo 'pixi 0.0.0'; else echo 'pixi fake'; fi
"#,
    );
    write_executable(
        &dir.join("pixi-unpack"),
        r#"#!/bin/sh
set -eu
if [ "${1:-}" = '--version' ]; then echo 'pixi-unpack 0.0.0'; exit 0; fi
pack="${1:-}"
test -f "$pack/pixi-pack.json" || { echo "expected pixi-pack.json at pack root" >&2; exit 42; }
out=''
env=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o) out="$2"; shift 2 ;;
    -e) env="$2"; shift 2 ;;
    *) shift ;;
  esac
done
mkdir -p "$out/$env/conda-meta"
printf 'unpacked %s\n' "$env" > "$out/$env/conda-meta/fake-package.json"
"#,
    );
}

#[cfg(unix)]
fn path_with_fake_tools(dir: &Path) -> std::ffi::OsString {
    let mut paths = vec![dir.to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    std::env::join_paths(paths).unwrap()
}

fn bare_remote(dir: &Path) -> String {
    let path = dir.join("remote.git");
    let out = StdCommand::new("git")
        .args(["init", "-q", "--bare"])
        .arg(&path)
        .output()
        .expect("git runs");
    assert!(out.status.success());
    path.to_string_lossy().into_owned()
}

/// Recursive `(path, size)` listing — how "nothing changed" is asserted on a plain directory.
fn tree_snapshot(dir: &Path) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, u64)>) {
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            let meta = fs::symlink_metadata(&path).unwrap();
            if meta.is_dir() {
                walk(root, &path, out);
            } else {
                out.push((
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    meta.len(),
                ));
            }
        }
    }
    walk(dir, dir, &mut out);
    out.sort();
    out
}

fn run_git(args: &[&str], cwd: &Path) -> String {
    let out = StdCommand::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn prints_version() {
    bin()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn documents_every_verb() {
    let out = bin().arg("--help").assert().success();
    let text = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    for verb in [
        "pack", "publish", "restore", "unpack", "doctor", "init", "plan", "tools",
    ] {
        assert!(text.contains(verb), "`{verb}` missing from --help");
    }
    // The branch-facing restore command must remain discoverable from the top-level help.
    assert!(text.contains("Verify and unpack a sandbox branch"));
}

#[test]
fn pack_requires_envs_and_output_dir() {
    // the transport contract must be explicit, not defaulted
    bin()
        .arg("pack")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--envs").or(predicate::str::contains("required")));
}

#[test]
fn restore_refuses_a_missing_branch_location_before_writing() {
    bin()
        .args([
            "restore",
            "--branch-location",
            "/nope",
            "--output-path",
            "/nope",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a directory"));
}

#[test]
fn unpack_requires_input_and_output() {
    bin()
        .arg("unpack")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--input-dir").or(predicate::str::contains("required")));

    bin()
        .args(["unpack", "--input-dir", "/nope", "--output-dir", "/nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a directory"));
}

#[test]
fn tools_list_reads_a_pin_file() {
    let dir = tempfile::tempdir().unwrap();
    let lock = dir.path().join("tools.lock.json");
    fs::write(
        &lock,
        r#"{"schema":1,"tools":{"pixi-unpack":{"version":"0.7.11",
        "url_template":"https://example.invalid/v{version}/{target}",
        "platforms":{"linux-64":{"target":"x86_64-unknown-linux-musl",
        "sha256":"8191f586b734e634e2f1644e553dcb8e07718d0bafbfefb65c5e6e22b4d484b7","linkage":"static"}}}}}"#,
    )
    .unwrap();

    bin()
        .args(["tools", "list", "--tools-lock", lock.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("pixi-unpack 0.7.11"))
        .stdout(predicate::str::contains("linux-64"))
        .stdout(predicate::str::contains("static"));
}

#[test]
fn tools_list_uses_embedded_pins_without_a_root_sidecar_file() {
    let dir = tempfile::tempdir().unwrap();
    bin()
        .current_dir(dir.path())
        .args(["tools", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("embedded tools lock"))
        .stdout(predicate::str::contains("pixi-unpack"));
}

#[test]
fn plan_emits_a_publish_matrix_from_a_reviewed_project_config() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join(".pixi-sandbox.toml");
    fs::write(
        &config,
        r#"
schema = 1
branch_prefix = "sandbox"

[[bundle]]
name = "developer"
environments = ["dev", "docs"]
platforms = ["linux-64", "win-64"]
"#,
    )
    .unwrap();

    let output = bin()
        .args(["plan", "--config", config.to_str().unwrap(), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let plan: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(plan["schema"], 1);
    assert_eq!(plan["include"].as_array().unwrap().len(), 2);
    assert_eq!(plan["include"][0]["branch"], "sandbox/developer-linux-64");
    assert_eq!(plan["include"][1]["branch"], "sandbox/developer-win-64");
    assert_eq!(plan["include"][0]["environments"], "dev,docs");
}

/// Locks the top-level shape of `plan --json`.
///
/// Actions treats every key other than `include`/`exclude` as a matrix
/// dimension and requires an array, so any *scalar* sibling of `include`
/// silently expands the matrix to zero jobs. `schema` is a deliberately
/// excluded scalar, and consumers must therefore build the matrix from
/// `.include` alone. Adding a further top-level field means revisiting that.
#[test]
fn plan_json_top_level_keys_stay_matrix_safe() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join(".pixi-sandbox.toml");
    fs::write(
        &config,
        r#"
schema = 1
branch_prefix = "sandbox"

[[bundle]]
name = "developer"
environments = ["default"]
platforms = ["linux-64"]
"#,
    )
    .unwrap();

    let output = bin()
        .args(["plan", "--config", config.to_str().unwrap(), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let plan: Value = serde_json::from_slice(&output).unwrap();
    let object = plan.as_object().unwrap();

    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["include", "schema"],
        "unexpected plan --json top level"
    );

    for key in &keys {
        let is_matrix_key = matches!(*key, "include" | "exclude");
        let is_known_scalar = *key == "schema";
        assert!(
            is_matrix_key || is_known_scalar,
            "{key} would be read as a matrix dimension and must be an array"
        );
    }
}

#[test]
fn plan_rejects_unreviewed_implicit_platform_runner_selection() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join(".pixi-sandbox.toml");
    fs::write(
        &config,
        r#"
schema = 1
[[bundle]]
name = "arm"
environments = ["dev"]
platforms = ["linux-aarch64"]
"#,
    )
    .unwrap();

    bin()
        .args(["plan", "--config", config.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("needs runners"));
}

#[test]
fn plan_rejects_a_custom_runner_without_embedded_tool_support() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join(".pixi-sandbox.toml");
    fs::write(
        &config,
        r#"
schema = 1
[runners]
linux-ppc64le = "self-hosted-ppc64le"

[[bundle]]
name = "unsupported"
environments = ["dev"]
platforms = ["linux-ppc64le"]
"#,
    )
    .unwrap();

    bin()
        .args(["plan", "--config", config.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("embedded helper-tool pins"));
}

// --------------------------------------------------------------- doctor, on the fixture

#[test]
fn doctor_summarises_the_fixture_transport() {
    bin()
        .args([
            "doctor",
            "--branch-location",
            fixture_transport().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("env demo"))
        .stdout(predicate::str::contains("payload"))
        .stdout(predicate::str::contains("hint"));
}

#[test]
fn doctor_verify_checks_every_blob_of_the_fixture() {
    // 11 blobs: the env (one split blob plus the real prefix archive), the three tool stubs, the
    // vendored crate
    bin()
        .args([
            "doctor",
            "--branch-location",
            fixture_transport().to_str().unwrap(),
            "--verify",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\": true"))
        .stdout(predicate::str::contains("\"files\": 11"));
}

#[test]
fn doctor_verify_fails_loudly_on_a_corrupted_branch() {
    let transport = transport_copy();
    fs::write(
        transport
            .path()
            .join(".pixi-sandbox/envs/demo/pack/channel/noarch/demo-pure-0.1.0-0.conda"),
        b"tampered",
    )
    .unwrap();
    bin()
        .args([
            "doctor",
            "--branch-location",
            transport.path().to_str().unwrap(),
            "--verify",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("OK").not());
}

#[test]
fn doctor_verify_catches_a_tampered_split_part() {
    let transport = transport_copy();
    fs::write(
        transport
            .path()
            .join(".pixi-sandbox/envs/demo/pack/channel/noarch/demo-big-0.1.0-0.conda.part001"),
        b"tampered-part",
    )
    .unwrap();
    bin()
        .args([
            "doctor",
            "--branch-location",
            transport.path().to_str().unwrap(),
            "--verify",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains(".part001"));
}

// ------------------------------------------------------------- publish, end to end

#[test]
fn publish_pushes_the_transport_as_a_single_orphan_commit() {
    let tmp = tempfile::tempdir().unwrap();
    let transport = transport_copy();
    let remote = bare_remote(tmp.path());

    bin()
        .args([
            "publish",
            "--input-dir",
            transport.path().to_str().unwrap(),
            "--branch-name",
            "sandbox/demo-linux-64",
            "--remote",
            &remote,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("published"))
        .stdout(predicate::str::contains("sandbox/demo-linux-64"));

    // one commit, no history to merge — the orphan-branch contract
    let bare = Path::new(&remote);
    assert_eq!(
        run_git(&["rev-list", "--count", "sandbox/demo-linux-64"], bare),
        "1"
    );
    let listed = run_git(
        &["ls-tree", "-r", "--name-only", "sandbox/demo-linux-64"],
        bare,
    );
    for expected in [
        ".pixi-sandbox/manifest.json",
        ".pixi-sandbox/envs/demo/pack/channel/noarch/demo-big-0.1.0-0.conda.part000",
        ".pixi-sandbox/tools/linux-64/pixi-sandbox",
        "AGENTS.md",
        "README.md",
    ] {
        assert!(
            listed.contains(expected),
            "{expected} missing from the branch:\n{listed}"
        );
    }
    for forbidden in ["pixi-sandbox", "restore.sh", "restore.ps1"] {
        assert!(
            !listed.lines().any(|path| path == forbidden),
            "legacy root file {forbidden:?} must not be published:\n{listed}"
        );
    }

    // publishing again replaces the tip instead of appending
    bin()
        .args([
            "publish",
            "--input-dir",
            transport.path().to_str().unwrap(),
            "--branch-name",
            "sandbox/demo-linux-64",
            "--remote",
            &remote,
        ])
        .assert()
        .success();
    assert_eq!(
        run_git(&["rev-list", "--count", "sandbox/demo-linux-64"], bare),
        "1"
    );
}

#[test]
fn publish_accepts_a_relative_transport_path() {
    let tmp = tempfile::tempdir().unwrap();
    let transport = tmp.path().join("transport");
    copy_tree(&fixture_transport(), &transport);
    let remote = bare_remote(tmp.path());

    // `ShellGit` changes cwd to the work tree while staging the index. A relative transport
    // path must therefore be made absolute before it is exported as GIT_DIR/GIT_INDEX_FILE.
    bin()
        .current_dir(tmp.path())
        .args([
            "publish",
            "--input-dir",
            "transport",
            "--branch-name",
            "sandbox/relative-linux-64",
            "--remote",
            "remote.git",
        ])
        .assert()
        .success();

    assert_eq!(
        run_git(
            &["rev-list", "--count", "sandbox/relative-linux-64"],
            Path::new(&remote)
        ),
        "1"
    );
}

#[test]
fn publish_dry_run_changes_nothing() {
    let transport = transport_copy();
    let before = tree_snapshot(transport.path());

    bin()
        .args([
            "publish",
            "--input-dir",
            transport.path().to_str().unwrap(),
            "--branch-name",
            "sandbox/demo-linux-64",
            "--remote",
            "origin",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("would publish"))
        .stdout(predicate::str::contains("push --force"))
        .stdout(predicate::str::contains("nothing was written"));

    assert_eq!(
        tree_snapshot(transport.path()),
        before,
        "a dry run must leave the transport exactly as it found it"
    );
    let mut entries: Vec<String> = fs::read_dir(transport.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        [".pixi-sandbox", "AGENTS.md", "README.md"],
        "a dry run must not alter the v0.3 transport root: {entries:?}"
    );
    assert!(
        !transport.path().join(".git").exists(),
        "publish must never create a repository inside the transport"
    );
}

#[test]
fn publish_refuses_a_directory_that_is_not_a_transport() {
    let dir = tempfile::tempdir().unwrap();
    bin()
        .args([
            "publish",
            "--input-dir",
            dir.path().to_str().unwrap(),
            "--branch-name",
            "sandbox/nope",
            "--remote",
            "origin",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("manifest.json"));
}

#[test]
fn publish_keep_is_not_silently_ignored() {
    let transport = transport_copy();
    bin()
        .args([
            "publish",
            "--input-dir",
            transport.path().to_str().unwrap(),
            "--branch-name",
            "sandbox/demo-linux-64",
            "--keep",
            "3",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("design.md"));
}

/// Task-6: a deferred verb is read by an operator who has no network to check what it names.
/// It used to send them to a reference implementation that 0.2.0 deleted from the repository,
/// which is exactly the kind of dead end an airlock cannot resolve, so the only thing it may
/// cite is the design section that specifies the verb.
#[test]
fn a_deferred_verb_cites_only_the_design_section() {
    let transport = transport_copy();
    let publish = bin()
        .args([
            "publish",
            "--input-dir",
            transport.path().to_str().unwrap(),
            "--branch-name",
            "sandbox/demo-linux-64",
            "--keep",
            "3",
        ])
        .assert()
        .failure();
    let tools = bin().args(["tools", "update"]).assert().failure();

    for (verb, output, section) in [
        ("publish --keep", publish.get_output(), "§2"),
        ("tools update", tools.get_output(), "§10"),
    ] {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(".knowledge/design.md") && stderr.contains(section),
            "`{verb}` must name the design section that specifies it, got:\n{stderr}"
        );
        // stale-ref-allowed: this list is the rule `lint-repo-consistency` enforces elsewhere.
        for stale in ["python", "prototype", ".knowledge/research"] {
            assert!(
                !stderr.to_ascii_lowercase().contains(stale),
                "`{verb}` still points at the deleted {stale} reference:\n{stderr}"
            );
        }
    }
}

// ---------------------------------------------------------- unpack / restore, fixture proof

#[test]
fn restore_verify_only_checks_the_fixture_without_writing_a_project() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    bin()
        .args([
            "restore",
            "--branch-location",
            fixture_transport().to_str().unwrap(),
            "--output-path",
            project.to_str().unwrap(),
            "--verify-only",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("every declared byte matches"))
        .stdout(predicate::str::contains("no project files were written"));

    assert!(
        !project.join(".pixi").exists(),
        "verify-only must not create a Pixi directory"
    );
}

/// Task-5: `restore.mdx` step 4 tells the operator to `source .pixi/sandbox-env.sh` and then
/// run restored tools directly. That contract only holds if sourcing the generated script alone
/// — no hand-added `envs/<name>/bin` export — puts the restored environment's binaries on PATH.
/// The fixture's `demo` environment ships a real `bin/freetype-config` executable (see
/// `fixture_doctor_publish_and_restore_is_the_complete_offline_proof` in `tests/e2e.rs`), so it
/// stands in for `cargo`/`bun`/`rustc` here. It is picked over the fixture's other executable,
/// `lzmainfo`, because a real `xz-utils` install can already put a system `lzmainfo` on PATH,
/// which would let this test pass for the wrong reason.
#[cfg(unix)]
#[test]
fn sourcing_the_generated_sandbox_env_resolves_restored_environment_binaries() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    bin()
        .args([
            "restore",
            "--branch-location",
            fixture_transport().to_str().unwrap(),
            "--output-path",
            project.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("restore complete"));

    let expected = project.join(".pixi/envs/demo/bin/freetype-config");
    assert!(expected.is_file(), "fixture must restore a real binary");

    let output = StdCommand::new("bash")
        .arg("-c")
        .arg("source .pixi/sandbox-env.sh && command -v freetype-config")
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "freetype-config did not resolve on PATH after sourcing sandbox-env.sh alone: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        expected.to_str().unwrap(),
    );
}

#[test]
fn unpack_verify_only_checks_the_fixture_without_creating_output() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("raw-prefix");

    bin()
        .args([
            "unpack",
            "--input-dir",
            fixture_transport().to_str().unwrap(),
            "--output-dir",
            output.to_str().unwrap(),
            "--verify-only",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("verified demo"))
        .stdout(predicate::str::contains("no output was written"));

    assert!(
        !output.exists(),
        "verify-only must not create its output prefix"
    );
}

/// This is deliberately Unix-only: the fake helper programs are POSIX scripts. The portable
/// command paths are still covered by verify-only tests above, while this test proves the full
/// pack → verify → standalone-unpack → restore flow without a network or a real conda payload.
#[cfg(unix)]
#[test]
fn pack_unpack_and_restore_a_verified_synthetic_environment() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("project-source");
    let tools = temp.path().join("fake-tools");
    let transport = temp.path().join("transport");
    let raw_prefix = temp.path().join("raw-prefix");
    let restored_project = temp.path().join("restored-project");
    fs::create_dir_all(&repo).unwrap();
    fs::create_dir_all(&restored_project).unwrap();
    fs::write(repo.join("pixi.lock"), "version: 7\n").unwrap();
    fake_tools(&tools);
    let path = path_with_fake_tools(&tools);

    bin()
        .env("PATH", &path)
        .args([
            "pack",
            "--repo-root",
            repo.to_str().unwrap(),
            "--envs",
            "demo",
            "--output-dir",
            transport.to_str().unwrap(),
            "--platform",
            "linux-64",
            // The generated fake package is just over one KiB, while the tools remain
            // smaller than this limit. That exercises record_file + join_parts end to end.
            "--shard-limit-mib",
            "0.0005",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("packed transport"));

    let manifest = fs::read_to_string(transport.join(".pixi-sandbox/manifest.json")).unwrap();
    assert!(
        manifest.contains(".part000"),
        "the synthetic package must be sharded"
    );

    bin()
        .env("PATH", &path)
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
        .env("PATH", &path)
        .args([
            "unpack",
            "--input-dir",
            transport.to_str().unwrap(),
            "--output-dir",
            raw_prefix.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(raw_prefix.join("conda-meta/fake-package.json").is_file());
    assert!(
        !raw_prefix.join("conda-meta/pixi_env_prefix").exists(),
        "unpack makes a raw prefix; restore owns Pixi's markers"
    );

    bin()
        .env("PATH", &path)
        .args([
            "restore",
            "--branch-location",
            transport.to_str().unwrap(),
            "--output-path",
            restored_project.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("restore complete"));
    let prefix = restored_project.join(".pixi/envs/demo");
    assert!(prefix.join("conda-meta/fake-package.json").is_file());
    assert!(prefix.join("conda-meta/pixi_env_prefix").is_file());
    assert!(
        restored_project
            .join(".pixi/tools/linux-64/pixi-unpack")
            .is_file()
    );
    assert!(restored_project.join(".pixi/sandbox-env.sh").is_file());
}

#[cfg(unix)]
#[test]
fn pack_keeps_the_self_binary_only_in_tools_and_the_branch_root_documentation_only() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("project-source");
    let tools = temp.path().join("fake-tools");
    let transport = temp.path().join("transport");
    let self_bin = temp.path().join("self-bin");
    fs::create_dir_all(&repo).unwrap();
    fs::write(repo.join("pixi.lock"), "version: 7\n").unwrap();
    fake_tools(&tools);
    write_executable(&self_bin, "#!/bin/sh\necho pixi-sandbox self-binary\n");

    bin()
        .env("PATH", path_with_fake_tools(&tools))
        .args([
            "pack",
            "--repo-root",
            repo.to_str().unwrap(),
            "--envs",
            "demo",
            "--output-dir",
            transport.to_str().unwrap(),
            "--self-bin",
            self_bin.to_str().unwrap(),
        ])
        .assert()
        .success();

    let nested = transport.join(".pixi-sandbox/tools/linux-64/pixi-sandbox");
    assert_eq!(fs::read(&self_bin).unwrap(), fs::read(&nested).unwrap());
    assert!(nested.metadata().unwrap().permissions().mode() & 0o111 != 0);
    assert!(!transport.join("pixi-sandbox").exists());
    assert!(!transport.join("restore.sh").exists());
    assert!(!transport.join("restore.ps1").exists());

    let mut root_files = fs::read_dir(&transport)
        .unwrap()
        .flatten()
        .filter(|entry| entry.file_type().unwrap().is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    root_files.sort();
    assert_eq!(root_files, ["AGENTS.md", "README.md"]);
}

#[test]
fn init_github_generates_minimal_project_launchers_and_workflow() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    let action_sha = "0123456789abcdef0123456789abcdef01234567";
    bin()
        .env("PIXI_SANDBOX_ACTION_SHA", action_sha)
        .args([
            "init",
            "github",
            "--project-root",
            project.to_str().unwrap(),
            "--branch",
            "sandbox/developer-linux-64",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "generated GitHub sandbox workflow",
        ));

    let shell = fs::read_to_string(project.join("restore.sh")).unwrap();
    assert!(shell.contains("git -C \"$ROOT\" archive"));
    assert!(shell.contains("sandbox/developer-linux-64"));
    assert!(shell.contains(".pixi-sandbox/tools/$PLATFORM/pixi-sandbox"));
    assert!(!shell.contains("curl"));
    // The launcher resolves the branch from the generated plan instead of trusting only the
    // branch baked in at init time.
    assert!(shell.contains("CONFIG=$ROOT/.pixi-sandbox.toml"));
    assert!(shell.contains("branch_prefix"));

    let powershell = fs::read_to_string(project.join("restore.ps1")).unwrap();
    assert!(powershell.contains("git -C $Root archive"));
    assert!(powershell.contains(".pixi-sandbox/tools/win-64/pixi-sandbox.exe"));
    assert!(!powershell.contains("Invoke-WebRequest"));
    assert!(powershell.contains(".pixi-sandbox.toml"));
    assert!(powershell.contains("branch_prefix"));

    let workflow =
        fs::read_to_string(project.join(".github/workflows/publish-sandbox.yml")).unwrap();
    assert!(workflow.contains(&format!("uses: Archont561/pixi-sandbox/setup@{action_sha}")));
    assert!(workflow.contains(&format!(
        "uses: Archont561/pixi-sandbox/publish@{action_sha}"
    )));
    assert!(workflow.contains(&format!("version: v{}", env!("CARGO_PKG_VERSION"))));
    assert!(!workflow.contains(".github/workflows/publish-sandbox.yml@"));
    assert!(project.join(".pixi-sandbox.toml").is_file());
}

#[cfg(unix)]
#[test]
fn a_bare_root_invocation_verifies_the_branch_and_only_prints_a_hint() {
    let output = bin()
        .current_dir(fixture_transport())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("every declared byte matches"));
    assert!(output.contains(".pixi-sandbox/tools/linux-64/pixi-sandbox"));
    assert!(output.contains(" restore --branch-location"));
    assert!(!output.contains("restore complete"));
}

#[cfg(unix)]
#[test]
fn restore_materialises_the_fixture_vendor_tree_and_writes_relative_cargo_config() {
    let transport = transport_copy();
    let unpacker = transport
        .path()
        .join(".pixi-sandbox/tools/linux-64/pixi-unpack");
    write_executable(
        &unpacker,
        r#"#!/bin/sh
set -eu
out=''
env=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o) out="$2"; shift 2 ;;
    -e) env="$2"; shift 2 ;;
    *) shift ;;
  esac
done
mkdir -p "$out/$env/conda-meta"
printf '{}\n' > "$out/$env/conda-meta/fake.json"
"#,
    );

    // This fixture intentionally leaves tools unpinned, so changing its shell stub only needs
    // the declared size updated; all environment/vendor blob hashes remain the checked-in ones.
    let manifest_path = transport.path().join(".pixi-sandbox/manifest.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["tools"]["pixi-unpack"]["size_bytes"] = json!(fs::metadata(&unpacker).unwrap().len());
    manifest["tools"]["pixi-unpack"]["pinned_sha256"] = Value::Null;
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let project_root = tempfile::tempdir().unwrap();
    let project = project_root.path().join("project");
    fs::create_dir_all(&project).unwrap();
    bin()
        .args([
            "restore",
            "--branch-location",
            transport.path().to_str().unwrap(),
            "--output-path",
            project.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        project
            .join(".pixi-sandbox/vendor/demo-dep-1.0.0/src/lib.rs")
            .is_file(),
        "the vendor tree must be materialised after full transport verification"
    );
    let config = fs::read_to_string(project.join(".cargo/config.toml")).unwrap();
    assert!(config.contains("directory = \".pixi-sandbox/vendor\""));
}

#[test]
fn restore_accepts_legacy_path_to_main_repo_code_alias() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    bin()
        .args([
            "restore",
            "--branch-location",
            fixture_transport().to_str().unwrap(),
            "--path-to-main-repo-code",
            project.to_str().unwrap(),
            "--verify-only",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("every declared byte matches"));
}

/// The Pixi platform name the generated launchers resolve to on this host. Tests that create a
/// sandbox branch must name it the same way, or they only pass on x86_64 Linux.
#[cfg(unix)]
fn host_platform() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-64",
        ("linux", "aarch64") => "linux-aarch64",
        ("macos", "aarch64") => "osx-arm64",
        ("macos", "x86_64") => "osx-64",
        other => panic!("unsupported test host {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn generated_restore_archives_a_local_sandbox_branch_and_runs_its_nested_binary() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    bin()
        .env(
            "PIXI_SANDBOX_ACTION_SHA",
            "0123456789abcdef0123456789abcdef01234567",
        )
        .args([
            "init",
            "github",
            "--project-root",
            project.to_str().unwrap(),
            "--branch",
            "sandbox/developer-linux-64",
        ])
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
    let branch = format!("sandbox/developer-{}", host_platform());
    git(&["checkout", "-q", "--orphan", &branch]);
    git(&["rm", "-qrf", "."]);
    let nested = project.join(format!(
        ".pixi-sandbox/tools/{}/pixi-sandbox",
        host_platform()
    ));
    fs::create_dir_all(nested.parent().unwrap()).unwrap();
    write_executable(
        &nested,
        "#!/bin/sh\nset -eu\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = --output-path ]; then shift; touch \"$1/restored-by-bootstrap\"; exit 0; fi\n  shift\ndone\nexit 3\n",
    );
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

    let status = StdCommand::new("sh")
        .arg("restore.sh")
        .current_dir(&project)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(project.join("restored-by-bootstrap").is_file());
}

/// A project can rename its bundle or branch prefix without regenerating `restore.sh`: the
/// launcher reads `.pixi-sandbox.toml` at run time. Here the branch baked in at init time does
/// not exist, so only a launcher that consults the config can find the transport.
#[cfg(unix)]
#[test]
fn generated_restore_prefers_the_branch_declared_in_the_config() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    bin()
        .env(
            "PIXI_SANDBOX_ACTION_SHA",
            "0123456789abcdef0123456789abcdef01234567",
        )
        .args([
            "init",
            "github",
            "--project-root",
            project.to_str().unwrap(),
            "--branch",
            "sandbox/never-published-linux-64",
        ])
        .assert()
        .success();

    let platform = host_platform();
    fs::write(
        project.join(".pixi-sandbox.toml"),
        format!(
            "schema = 1\nbranch_prefix = \"envs\"\ncargo_vendor = true\n\n\
             [[bundle]]\nname = \"tools\"\nenvironments = [\"default\"]\nplatforms = [\"{platform}\"]\n"
        ),
    )
    .unwrap();

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

    // The transport lives on the branch the config names, never on the init-time default.
    git(&[
        "checkout",
        "-q",
        "--orphan",
        &format!("envs/tools-{platform}"),
    ]);
    git(&["rm", "-qrf", "."]);
    let nested = project.join(format!(".pixi-sandbox/tools/{platform}/pixi-sandbox"));
    fs::create_dir_all(nested.parent().unwrap()).unwrap();
    write_executable(
        &nested,
        "#!/bin/sh\nset -eu\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = --output-path ]; then shift; touch \"$1/restored-by-config\"; exit 0; fi\n  shift\ndone\nexit 3\n",
    );
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

    let status = StdCommand::new("sh")
        .arg("restore.sh")
        .current_dir(&project)
        .status()
        .unwrap();
    assert!(
        status.success(),
        "the launcher must resolve envs/tools-{platform} from .pixi-sandbox.toml"
    );
    assert!(project.join("restored-by-config").is_file());
}
