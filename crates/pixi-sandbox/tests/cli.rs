//! CLI-level smoke tests.
//!
//! Everything that needs a payload uses the checked-in fixture transport, and everything that
//! writes copies it into a tempdir first: the fixtures are read-only, and no test ever points
//! at this repository (see `tests/fixtures/README.md`).

mod support;

use assert_cmd::Command;
use predicates::prelude::*;
use rstest::{fixture, rstest};
use serde_json::Value;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use support::{
    bin, copy_tree, duplicate_source_project, fixture_transport, host_platform, run_git,
};
#[cfg(unix)]
use support::{fake_tools, path_with_fake_tools, write_executable};

/// Copy the fixture out of the repository: tests never mutate a fixture in place.
fn transport_copy() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    copy_tree(&fixture_transport(), dir.path());
    dir
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

#[rstest]
fn prints_version(mut bin: Command) {
    bin.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[rstest]
#[case("pack")]
#[case("publish")]
#[case("restore")]
#[case("unpack")]
#[case("doctor")]
#[case("init")]
#[case("plan")]
#[case("tools")]
fn documents_every_verb(mut bin: Command, #[case] verb: &str) {
    let out = bin.arg("--help").assert().success();
    let text = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(text.contains(verb), "`{verb}` missing from --help");
    // The branch-facing restore command must remain discoverable from the top-level help.
    assert!(text.contains("Verify and unpack a sandbox branch"));
}

#[rstest]
#[case("pack")]
#[case("doctor")]
#[case("publish")]
fn pipeline_commands_document_durable_diagnostics(mut bin: Command, #[case] verb: &str) {
    let output = bin.args([verb, "--help"]).output().expect("CLI help");
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).expect("UTF-8 help");
    assert!(
        help.contains("-v"),
        "{verb} help has no verbosity flag\n{help}"
    );
    assert!(
        help.contains("--log-file"),
        "{verb} help has no durable log destination\n{help}"
    );

    support::bin()
        .args([verb, "--diagnostic-level", "debug"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}

#[test]
fn doctor_writes_phase_boundaries_to_a_durable_log_without_changing_normal_stdout() {
    let scratch = tempfile::tempdir().expect("diagnostic scratch");
    let log = scratch.path().join("doctor.log");
    let home = scratch.path().join("home");
    fs::create_dir(&home).expect("isolated HOME");
    let branch = fixture_transport();

    let normal = bin()
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .args(["doctor", "--branch-location"])
        .arg(&branch)
        .output()
        .expect("ordinary doctor");
    let diagnosed = bin()
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .args(["doctor", "--branch-location"])
        .arg(&branch)
        .args(["--log-file"])
        .arg(&log)
        .output()
        .expect("diagnosed doctor");

    assert!(normal.status.success());
    assert!(diagnosed.status.success());
    assert_eq!(
        diagnosed.stdout, normal.stdout,
        "default stdout must not change"
    );
    assert!(diagnosed.stderr.is_empty(), "log-only mode stays quiet");

    let contents = fs::read_to_string(log).expect("durable log");
    assert!(
        contents.contains("command=doctor event=start"),
        "{contents}"
    );
    assert!(contents.contains("phase=load-manifest"), "{contents}");
    assert!(contents.contains("phase=standalone-probe"), "{contents}");
    assert!(contents.contains("result=success"), "{contents}");
}

#[test]
fn diagnostic_log_keeps_the_complete_failure_chain_and_verbose_mode_surfaces_phases() {
    let scratch = tempfile::tempdir().expect("diagnostic scratch");
    let missing = scratch.path().join("missing-transport");
    let log = scratch.path().join("doctor.log");
    let home = scratch.path().join("home");
    fs::create_dir(&home).expect("isolated HOME");
    let output = bin()
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .args(["doctor", "--branch-location"])
        .arg(&missing)
        .args(["-vv", "--log-file"])
        .arg(&log)
        .output()
        .expect("failing doctor");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("load-manifest"), "{stderr}");
    let contents = fs::read_to_string(log).expect("failure log");
    assert!(contents.contains("result=failure"), "{contents}");
    assert!(contents.contains("loading"), "{contents}");
    assert!(contents.contains("manifest.json"), "{contents}");
    assert!(contents.contains("Caused by:"), "{contents}");
}

#[test]
fn diagnostics_refuse_to_overwrite_the_only_copy_and_never_record_remote_credentials() {
    let scratch = tempfile::tempdir().expect("diagnostic scratch");
    let existing = scratch.path().join("existing.log");
    let home = scratch.path().join("home");
    fs::create_dir(&home).expect("isolated HOME");
    fs::write(&existing, "operator evidence\n").expect("existing evidence");
    bin()
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .args(["doctor", "--branch-location"])
        .arg(fixture_transport())
        .args(["--log-file"])
        .arg(&existing)
        .assert()
        .failure()
        .stderr(predicate::str::contains("refusing to overwrite"));
    assert_eq!(
        fs::read_to_string(&existing).expect("preserved evidence"),
        "operator evidence\n"
    );

    let log = scratch.path().join("publish.log");
    bin()
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .args(["publish", "--input-dir"])
        .arg(scratch.path().join("missing"))
        .args([
            "--branch-name",
            "sandbox/test-linux-64",
            "--remote",
            "https://operator:top-secret@example.invalid/repo.git?token=also-secret",
            "--log-file",
        ])
        .arg(&log)
        .assert()
        .failure();
    let contents = fs::read_to_string(log).expect("redacted failure log");
    assert!(!contents.contains("top-secret"), "{contents}");
    assert!(!contents.contains("also-secret"), "{contents}");
    assert!(contents.contains("remote=<configured>"), "{contents}");
}

#[test]
fn diagnostic_failure_chain_redacts_credentials_returned_by_git() {
    let scratch = tempfile::tempdir().expect("diagnostic scratch");
    let home = scratch.path().join("home");
    fs::create_dir(&home).expect("isolated HOME");
    let transport = transport_copy();
    let log = scratch.path().join("publish.log");
    let output = bin()
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(["publish", "--input-dir"])
        .arg(transport.path())
        .args([
            "--branch-name",
            "sandbox/test-linux-64",
            "--remote",
            "https://operator:top-secret@127.0.0.1:9/repo.git?token=also-secret",
            "--log-file",
        ])
        .arg(&log)
        .output()
        .expect("failing publish");
    assert!(!output.status.success());

    let contents = fs::read_to_string(log).expect("redacted Git failure log");
    assert!(!contents.contains("top-secret"), "{contents}");
    assert!(!contents.contains("also-secret"), "{contents}");
    assert!(contents.contains("<redacted>@127.0.0.1"), "{contents}");
    assert!(contents.contains("token=<redacted>"), "{contents}");
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
        ["include", "push_paths", "schema"],
        "unexpected plan --json top level"
    );

    for key in &keys {
        let is_matrix_key = matches!(*key, "include" | "exclude");
        let is_known_scalar = *key == "schema";
        // `push_paths` (task-53 AC#3) is a known extra array, not a matrix dimension: the
        // generated workflow's `matrix: { include: ${{ fromJSON(...).include }} }` only ever
        // reads `.include`, so a sibling array here is inert the same way `schema` already is.
        let is_known_extra_array = *key == "push_paths";
        assert!(
            is_matrix_key || is_known_scalar || is_known_extra_array,
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
    // 12 blobs: the env (one split blob plus the real prefix archive) and its files.json oracle,
    // the three tool stubs, the vendored crate
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
        .stdout(predicate::str::contains("\"files\": 12"))
        // schema 2: the env carries a per-file oracle now (task-10)
        .stdout(predicate::str::contains("\"file_entries\": 14"));
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

#[test]
fn doctor_enforces_reviewed_transport_budgets_before_publish() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("pixi-sandbox.toml");
    fs::write(
        &config,
        r#"
schema = 1
[budgets]
max_blob_mib = 95
max_transport_mib = 0.001
max_repository_push_mib = 0.001
max_restore_required_mib = 0.001

[[bundle]]
name = "developer"
environments = ["demo"]
platforms = ["linux-64"]
"#,
    )
    .unwrap();

    bin()
        .args([
            "doctor",
            "--branch-location",
            fixture_transport().to_str().unwrap(),
            "--budget-config",
            config.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("budget    FAILED"))
        .stdout(predicate::str::contains("budgets.max_transport_mib"))
        .stdout(predicate::str::contains("[[bundle]]"))
        .stderr(predicate::str::contains("transport budget exceeded"));
}

#[test]
fn doctor_reports_budget_measurements_in_json() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("pixi-sandbox.toml");
    fs::write(
        &config,
        r#"
schema = 1
[budgets]
max_blob_mib = 95
max_transport_mib = 2048
max_repository_push_mib = 2048
max_restore_required_mib = 8192

[[bundle]]
name = "developer"
environments = ["demo"]
platforms = ["linux-64"]
"#,
    )
    .unwrap();

    let output = bin()
        .args([
            "doctor",
            "--branch-location",
            fixture_transport().to_str().unwrap(),
            "--budget-config",
            config.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let report: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(report["budget"]["violations"].as_array().unwrap().len(), 0);
    assert!(
        report["budget"]["measurements"]["transport_bytes"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(
        report["budget"]["measurements"]["repository_push_bytes"]
            .as_u64()
            .unwrap()
            > 0
    );
}

// ------------------------------------------------------------- publish, end to end

/// The payload the first publish places on the branch, published once and shared read-only by
/// every listing assertion below: publishing per case would trade a cheap string check for a
/// git round-trip per line, and the assertions never mutate the remote.
struct PublishedOrphan {
    remote: String,
    listing: String,
    // Owns the tempdirs so the bare remote and the transport body outlive the fixture call —
    // rstest hands this value out across tests, and dropping early would delete the remote the
    // commit-count test reads.
    _keep_alive: (tempfile::TempDir, tempfile::TempDir),
}

#[fixture]
#[once]
fn published_orphan() -> PublishedOrphan {
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

    let listing = run_git(
        &["ls-tree", "-r", "--name-only", "sandbox/demo-linux-64"],
        Path::new(&remote),
    );
    PublishedOrphan {
        remote,
        listing,
        _keep_alive: (tmp, transport),
    }
}

#[rstest]
#[case(".pixi-sandbox/manifest.json")]
#[case(".pixi-sandbox/envs/demo/pack/channel/noarch/demo-big-0.1.0-0.conda.part000")]
#[case(".pixi-sandbox/tools/linux-64/pixi-sandbox")]
#[case("AGENTS.md")]
#[case("README.md")]
fn the_orphan_commit_carries_every_expected_path(
    published_orphan: &PublishedOrphan,
    #[case] expected: &str,
) {
    assert!(
        published_orphan.listing.contains(expected),
        "{expected} missing from the branch:\n{}",
        published_orphan.listing
    );
}

#[rstest]
#[case("pixi-sandbox")]
#[case("restore.sh")]
#[case("restore.ps1")]
fn the_orphan_commit_omits_the_legacy_root_files(
    published_orphan: &PublishedOrphan,
    #[case] forbidden: &str,
) {
    assert!(
        !published_orphan
            .listing
            .lines()
            .any(|path| path == forbidden),
        "legacy root file {forbidden:?} must not be published:\n{}",
        published_orphan.listing
    );
}

#[rstest]
fn publish_pushes_the_transport_as_a_single_orphan_commit(published_orphan: &PublishedOrphan) {
    // one commit, no history to merge — the orphan-branch contract
    assert_eq!(
        run_git(
            &["rev-list", "--count", "sandbox/demo-linux-64"],
            Path::new(&published_orphan.remote),
        ),
        "1"
    );
}

#[test]
fn republishing_replaces_the_tip_instead_of_appending() {
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
        .stdout(predicate::str::contains("published"));

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
        run_git(
            &["rev-list", "--count", "sandbox/demo-linux-64"],
            Path::new(&remote),
        ),
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

/// Task-3: `--keep N` is the retention flag, and it has to be visible in the output — an
/// operator who asked for rotation must be able to read what the branch now carries.
#[test]
fn publish_keep_rotates_the_branch_instead_of_replacing_it() {
    let tmp = tempfile::tempdir().unwrap();
    let transport = transport_copy();
    let remote = bare_remote(tmp.path());
    let branch = "sandbox/demo-linux-64";

    for _ in 0..3 {
        bin()
            .args([
                "publish",
                "--input-dir",
                transport.path().to_str().unwrap(),
                "--branch-name",
                branch,
                "--remote",
                &remote,
                "--keep",
                "2",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("published"))
            .stdout(predicate::str::contains("keeping at most 2 snapshot"));
    }

    assert_eq!(
        run_git(&["rev-list", "--count", branch], Path::new(&remote)),
        "2",
        "three publishes with --keep 2 must leave two snapshots on the branch"
    );
}

/// Task-68 AC#4: A failed or rejected publish leaves any previously published healthy transport
/// branch completely byte-identical and fetchable.
#[test]
fn publish_failure_preserves_the_existing_healthy_transport_branch() {
    let tmp = tempfile::tempdir().unwrap();
    let transport = transport_copy();
    let remote = bare_remote(tmp.path());
    let branch = "sandbox/demo-linux-64";

    // 1. Initial healthy publish succeeds
    bin()
        .args([
            "publish",
            "--input-dir",
            transport.path().to_str().unwrap(),
            "--branch-name",
            branch,
            "--remote",
            &remote,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("published"));

    let initial_commit = run_git(&["rev-parse", branch], Path::new(&remote));
    assert!(!initial_commit.is_empty());

    // 2. An invalid publish attempt fails (e.g. invalid directory missing manifest)
    let bad_dir = tmp.path().join("invalid-transport");
    fs::create_dir(&bad_dir).unwrap();

    bin()
        .args([
            "publish",
            "--input-dir",
            bad_dir.to_str().unwrap(),
            "--branch-name",
            branch,
            "--remote",
            &remote,
        ])
        .assert()
        .failure();

    // 3. The remote branch head is untouched and identical
    let after_commit = run_git(&["rev-parse", branch], Path::new(&remote));
    assert_eq!(
        initial_commit, after_commit,
        "failed publish must not alter the healthy branch commit"
    );

    // 4. Offline doctor verification against the branch remains valid
    let worktree = tmp.path().join("worktree");
    run_git(
        &["worktree", "add", worktree.to_str().unwrap(), branch],
        Path::new(&remote),
    );

    bin()
        .args([
            "doctor",
            "--branch-location",
            worktree.to_str().unwrap(),
            "--verify",
        ])
        .assert()
        .success();
}

/// Task-6: an operator with no network must never be sent to a reference that no longer exists.
/// It used to send them to a reference implementation that 0.2.0 deleted from the repository,
/// which is exactly the kind of dead end an airlock cannot resolve. `tools update` was the last
/// verb that could emit such a pointer, so the rule is now asserted over every operator-visible
/// surface: no verb may claim to be unimplemented, and no help text may name a deleted reference.
#[test]
fn tools_update_never_defers_to_a_reference() {
    let deferred = bin()
        .args([
            "tools",
            "update",
            "--check",
            "--tools-lock",
            "/nonexistent/does-not-exist.json",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&deferred.get_output().stderr);
    assert!(
        !stderr.contains("not implemented yet"),
        "`tools update` is implemented; it must not still defer:\n{stderr}"
    );
}

// The stale words stay on one line: `#[case]` attributes would each need an opt-out marker.
// stale-ref-allowed: this list is the rule `lint-repo-consistency` enforces elsewhere.
const STALE_DELETED_REFERENCES: [&str; 3] = ["python", "prototype", ".knowledge/research"];

#[rstest]
#[case("--help", 0)]
#[case("--help", 1)]
#[case("--help", 2)]
#[case("tools --help", 0)]
#[case("tools --help", 1)]
#[case("tools --help", 2)]
#[case("pack --help", 0)]
#[case("pack --help", 1)]
#[case("pack --help", 2)]
#[case("restore --help", 0)]
#[case("restore --help", 1)]
#[case("restore --help", 2)]
fn no_help_surface_points_at_a_deleted_reference(#[case] surface: &str, #[case] stale: usize) {
    let stale = STALE_DELETED_REFERENCES[stale];
    let verb: Vec<&str> = surface.split_whitespace().collect();
    let help = bin().args(&verb).assert().success();
    let text = String::from_utf8_lossy(&help.get_output().stdout).to_ascii_lowercase();
    assert!(
        !text.contains(stale),
        "`{surface}` points at the deleted {stale} reference"
    );
}

// The flags only exist because the verb is implemented, so their presence is the positive
// half of "not a stub" — the failure above only rules out the old wording coming back.
#[rstest]
#[case("--tools-lock")]
#[case("--check")]
#[case("--tool")]
fn tools_update_help_documents_its_flags(#[case] flag: &str) {
    let update = bin().args(["tools", "update", "--help"]).assert().success();
    let text = String::from_utf8_lossy(&update.get_output().stdout);
    assert!(
        text.contains(flag),
        "`tools update --help` must document {flag}:\n{text}"
    );
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

/// Restores no longer generate an activation hook. The supported contract is the registered
/// pixi launcher plus `pixi run ...`; environment binaries stay inside their prefixes instead
/// of being exposed by a sourced `.pixi/sandbox-env.sh`.
#[test]
fn restore_does_not_generate_sandbox_env_and_points_to_pixi_run() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    bin()
        // task-33: restore registers user tools by default, so every restoring test points
        // HOME at a tempdir — a test that touches the developer's real home is a broken test.
        .env("HOME", temp.path())
        .env("USERPROFILE", temp.path())
        .env("SHELL", "/usr/bin/bash")
        .env("TMPDIR", temp.path())
        .args([
            "restore",
            "--branch-location",
            fixture_transport().to_str().unwrap(),
            "--output-path",
            project.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("restore complete"))
        .stdout(predicate::str::contains(
            "no .pixi/sandbox-env.sh is generated",
        ))
        .stdout(predicate::str::contains(
            "pixi run --frozen -- cargo build --offline",
        ));

    assert!(
        !project.join(".pixi/sandbox-env.sh").exists(),
        "restore must not leave the retired activation hook behind"
    );
    assert!(
        project
            .join(".pixi/envs/demo/bin/freetype-config")
            .is_file(),
        "the fixture still restores the environment binary; callers reach it through pixi run"
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
        // task-33: registration runs by default; keep it inside the tempdir.
        .env("HOME", temp.path())
        .env("USERPROFILE", temp.path())
        .env("SHELL", "/usr/bin/bash")
        .env("TMPDIR", temp.path())
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
    assert!(!restored_project.join(".pixi/sandbox-env.sh").exists());
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

#[cfg(unix)]
#[test]
fn pack_refuses_a_pixi_global_trampoline_self_bin_before_embedding() {
    // Issue #81's exact shape: the CI workaround packed `$(command -v pixi-sandbox)` after
    // `pixi global install` — a trampoline whose sibling configuration lives in the global
    // prefix, which is precisely the context a transport can never ship. The refusal is
    // structural (no execution needed) and must fire before the embed step copies anything.
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("project-source");
    let tools = temp.path().join("fake-tools");
    let transport = temp.path().join("transport");
    let global_bin = temp.path().join("global/bin");
    fs::create_dir_all(&repo).unwrap();
    fs::write(repo.join("pixi.lock"), "version: 7\n").unwrap();
    fs::create_dir_all(&global_bin).unwrap();
    fake_tools(&tools);
    let self_bin = global_bin.join("pixi-sandbox");
    write_executable(&self_bin, "#!/bin/sh\necho trampoline\n");
    fs::create_dir_all(global_bin.join("trampoline_configuration")).unwrap();
    fs::write(
        global_bin.join("trampoline_configuration/pixi-sandbox.json"),
        "{}",
    )
    .unwrap();

    let assert = bin()
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
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    assert!(
        stderr.contains("`pixi global install` trampoline"),
        "the refusal names the shape: {stderr}"
    );
    assert!(
        stderr.contains("standalone release asset `pixi-sandbox-<target>`")
            && stderr.contains("SHA256SUMS"),
        "the refusal names the remedy that packs a runnable binary: {stderr}"
    );
    assert!(
        !transport
            .join(".pixi-sandbox/tools/linux-64/pixi-sandbox")
            .exists(),
        "refused before the embed step copied anything"
    );
}

#[cfg(unix)]
#[test]
fn pack_refuses_a_self_bin_that_cannot_leave_its_prefix() {
    // The behavioural half: a candidate whose brokenness only execution can see — a
    // trampoline-shaped script with no configuration anywhere, exactly the state the
    // relocation to `tools/<platform>/` creates. Hashes are blind to it (a self-bin's
    // content is never pinned), the linkage check is blind to it (scripts and static
    // binaries pass); `--version` under the probe's empty environment is not.
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("project-source");
    let tools = temp.path().join("fake-tools");
    let transport = temp.path().join("transport");
    let self_bin = temp.path().join("self-bin");
    fs::create_dir_all(&repo).unwrap();
    fs::write(repo.join("pixi.lock"), "version: 7\n").unwrap();
    fake_tools(&tools);
    write_executable(
        &self_bin,
        r#"#!/bin/sh
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
config="$dir/trampoline_configuration/$(basename "$0").json"
if [ ! -f "$config" ]; then
  printf "Couldn't open \"%s\"\n" "$config" >&2
  exit 1
fi
echo 'pixi-sandbox 0.5.0'
"#,
    );

    let assert = bin()
        .env("PATH", path_with_fake_tools(&tools))
        .args([
            "pack",
            "--repo-root",
            repo.to_str().unwrap(),
            "--envs",
            "demo",
            "--output-dir",
            transport.to_str().unwrap(),
            "--platform",
            host_platform(),
            "--self-bin",
            self_bin.to_str().unwrap(),
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    assert!(stderr.contains("does not run standalone"), "{stderr}");
    assert!(
        stderr.contains("Couldn't open") && stderr.contains("trampoline_configuration"),
        "the child's own failure text is the evidence: {stderr}"
    );
    assert!(
        stderr.contains("standalone release asset `pixi-sandbox-<target>`"),
        "the remedy is attached: {stderr}"
    );
    assert!(
        !transport.join(".pixi-sandbox-standalone-probe").exists(),
        "the probe scratch is removed even when the pack is refused"
    );
}

#[cfg(all(unix, target_os = "linux"))]
#[test]
fn doctor_flags_a_hash_green_transport_whose_tool_cannot_run_standalone() {
    // Recreated issue #81: the fixture transport's self tool is replaced by a
    // trampoline-shaped script and the manifest is updated to stay faithful to the *new*
    // bytes — a faithful copy of a broken tool, which is the whole failure mode. The
    // fixture is linux-64, so the probe runs only on a linux host.
    let transport = transport_copy();
    let tool = transport
        .path()
        .join(".pixi-sandbox/tools/linux-64/pixi-sandbox");
    write_executable(
        &tool,
        r#"#!/bin/sh
printf "Couldn't open \"%s/trampoline_configuration/pixi-sandbox.json\"\n" "$(dirname "$0")" >&2
exit 1
"#,
    );
    let manifest_path = transport.path().join(".pixi-sandbox/manifest.json");
    let mut manifest: Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["tools"]["pixi-sandbox"]["size_bytes"] =
        Value::from(fs::metadata(&tool).unwrap().len());
    fs::write(
        &manifest_path,
        format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap()),
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
        .stdout(predicate::str::contains(
            "OK — every declared byte matches the manifest",
        ))
        .stdout(predicate::str::contains(
            "FAILED — the embedded tool does not run standalone",
        ))
        .stdout(predicate::str::contains(
            "trampoline_configuration/pixi-sandbox.json",
        ))
        .stdout(predicate::str::contains("standalone release asset"));
}

/// TASK-80 AC#4: a branch whose sharded blob declares a part path of `.` is refused by the
/// manifest validator, so both surfaces that read it exit non-zero with a diagnostic instead
/// of panicking inside `shard.rs`. The fixture's `demo-big` blob really is split into three
/// parts, so this is a shape an untrusted branch can actually carry, and the refusal has to
/// happen before a single byte of the payload is touched.
#[test]
fn a_part_path_with_no_file_name_fails_doctor_and_restore_without_panicking() {
    let transport = transport_copy();
    let manifest_path = transport.path().join(".pixi-sandbox/manifest.json");
    let mut manifest: Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    let blob = manifest["envs"]["demo"]["blobs"]
        .as_array_mut()
        .expect("fixture blobs")
        .iter_mut()
        .find(|blob| {
            blob["parts"]
                .as_array()
                .is_some_and(|parts| !parts.is_empty())
        })
        .expect("the fixture carries a sharded blob");
    blob["parts"][0]["path"] = Value::from(".");
    fs::write(
        &manifest_path,
        format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap()),
    )
    .unwrap();

    // `doctor --verify` is the read-only surface: it must name the shape it refused, and it
    // must refuse before the hash report — this is not a corrupted transport but an
    // unreadable one, so no byte-level verdict may be printed at all.
    let doctor = bin()
        .args([
            "doctor",
            "--branch-location",
            transport.path().to_str().unwrap(),
            "--verify",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&doctor.get_output().stderr);
    assert!(
        stderr.contains("part path has no file name"),
        "doctor must name the refused path, got: {stderr}"
    );
    assert!(
        !stderr.contains("panicked"),
        "doctor must not panic on a hostile part path: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&doctor.get_output().stdout);
    assert!(
        !stdout.contains("declared byte"),
        "the manifest must be refused before any hash verdict: {stdout}"
    );

    // `restore` writes, so the same refusal must arrive before anything lands on disk.
    let project_root = tempfile::tempdir().unwrap();
    let project = project_root.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let restore = bin()
        .env("HOME", project_root.path())
        .env("USERPROFILE", project_root.path())
        .env("TMPDIR", project_root.path())
        .args([
            "restore",
            "--branch-location",
            transport.path().to_str().unwrap(),
            "--output-path",
            project.to_str().unwrap(),
            "--user-tools",
            "skip",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&restore.get_output().stderr);
    assert!(
        stderr.contains("part path has no file name"),
        "restore must name the refused path, got: {stderr}"
    );
    assert!(
        !stderr.contains("panicked"),
        "restore must not panic on a hostile part path: {stderr}"
    );
    assert!(
        !project.join(".pixi/envs/demo").exists(),
        "a refused manifest must not materialise an environment"
    );
}

#[cfg(all(unix, target_os = "linux"))]
#[test]
fn doctor_never_executes_bytes_the_hash_report_rejected() {
    // The trust order made testable: tamper *without* updating the manifest and the probe
    // must refuse to run at all — executing first and hashing later would be the bug.
    let transport = transport_copy();
    let tool = transport
        .path()
        .join(".pixi-sandbox/tools/linux-64/pixi-sandbox");
    write_executable(&tool, "#!/bin/sh\necho tampered\n");

    bin()
        .args([
            "doctor",
            "--branch-location",
            transport.path().to_str().unwrap(),
            "--verify",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "FAILED — do not restore from this branch",
        ))
        .stdout(predicate::str::contains(
            "skipped — the hash report is not green",
        ));
}

fn init_command(project: &Path) -> Command {
    let manifest = project.join("pixi.toml");
    if !manifest.exists() {
        fs::write(&manifest, "[workspace]\nname = \"fixture\"\nchannels = [\"conda-forge\"]\nplatforms = [\"linux-64\"]\n").unwrap();
    }
    let mut command = bin();
    command.args(["init", "--project-root", project.to_str().unwrap()]);
    command
}

#[test]
fn init_is_provider_neutral_and_generates_only_this_platforms_launcher() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    init_command(&project)
        .args(["--branch", "sandbox/developer-linux-64"])
        .assert()
        .success()
        .stdout(predicate::str::contains("generated offline launcher"));

    #[cfg(unix)]
    {
        let shell = fs::read_to_string(project.join("restore.sh")).unwrap();
        assert!(shell.contains("git -C \"$ROOT\" archive"));
        assert!(shell.contains("sandbox/developer-linux-64"));
        assert!(shell.contains(".pixi-sandbox/tools/$PLATFORM/pixi-sandbox"));
        assert!(shell.contains("CONFIG=$ROOT/pixi-sandbox.toml"));
        assert!(!shell.contains("curl"));
        assert!(!project.join("restore.ps1").exists());
    }
    #[cfg(windows)]
    {
        let powershell = fs::read_to_string(project.join("restore.ps1")).unwrap();
        assert!(powershell.contains("git -C $Root archive"));
        assert!(powershell.contains(".pixi-sandbox/tools/win-64/pixi-sandbox.exe"));
        assert!(powershell.contains("Join-Path $Root 'pixi-sandbox.toml'"));
        assert!(!powershell.contains("Invoke-WebRequest"));
        assert!(!project.join("restore.sh").exists());
    }

    let workflow =
        fs::read_to_string(project.join(".github/workflows/publish-sandbox.yml")).unwrap();
    assert!(!workflow.contains("uses: Archont561/pixi-sandbox"));
    assert!(workflow.contains("https://prefix.dev/archont561/archont561"));
    assert!(workflow.contains(&format!(
        "PIXI_SANDBOX_VERSION: {}",
        env!("CARGO_PKG_VERSION")
    )));
    assert!(workflow.contains("--config pixi-sandbox.toml"));
    assert!(workflow.contains("--budget-config pixi-sandbox.toml"));
    assert!(workflow.contains("\"$SELF_BIN\" doctor --branch-location \"$TRANSPORT\" --verify"));
    assert!(project.join("pixi-sandbox.toml").is_file());
    assert!(!project.join(".pixi-sandbox.toml").exists());
}

#[test]
fn init_leaves_the_consumer_pixi_manifest_untouched() {
    // Issue #71 / D17: init used to append the publisher namespace root to
    // [workspace].channels — a URL that serves no repodata — so every consumer with at
    // least one dependency failed `pixi lock` immediately after init. An empty project
    // locked fine because an empty solve never fetches repodata, which is exactly the
    // smoke test that hid the bug: every manifest here carries a dependency.
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let manifests = [
        // The issue's reproduction shape: a dependency forces a repodata fetch on lock.
        concat!(
            "[workspace]\n",
            "name = \"probe\"\n",
            "channels = [\"conda-forge\"]\n",
            "platforms = [\"linux-64\"]\n",
            "\n",
            "[dependencies]\n",
            "bun = \"=1.3.11\"\n",
        ),
        // A hand-commented manifest (task-41's corpus) survives byte-identically too —
        // the comment-preservation machinery has nothing to preserve because nothing is
        // edited.
        concat!(
            "# This manifest is maintained by hand.\n",
            "[workspace] # Pixi project metadata\n",
            "name = \"demo\" # Shown in release reports\n",
            "channels = [ # Priority matters; do not reorder these.\n",
            "    \"internal\", # Private packages first\n",
            "    # Public fallback for everything else.\n",
            "    \"conda-forge\", # Broadest catalogue\n",
            "] # Channel list ends here\n",
            "platforms = [\"linux-64\"] # Current deployment target\n",
            "\n",
            "[dependencies] # Keep this table after workspace.\n",
            "demo = \"1\" # Pinned for the fixture\n",
        ),
        // No channels array at all: init must not invent one.
        concat!(
            "# Keep this heading and its blank line.\n",
            "[workspace]\n",
            "name = \"demo\" # Unrelated inline comment\n",
            "platforms = [\"linux-64\"]\n",
            "\n",
            "[dependencies]\n",
            "demo = \"1\" # Keep this trailing newline too.\n",
        ),
        // A pre-existing namespace spelling stays exactly as its author wrote it.
        concat!(
            "# A pre-existing spelling is valid.\n",
            "[workspace]\n",
            "name = \"demo\"\n",
            "channels = [\"HTTPS://PREFIX.DEV/Archont561/\"] # Keep this spelling.\n",
        ),
        // Malformed TOML is no longer init's business (D17): the manifest is not parsed,
        // so init succeeds and pixi's own lock reports the syntax error where it belongs.
        "not valid TOML [[[",
    ];
    for manifest in manifests {
        fs::write(project.join("pixi.toml"), manifest).unwrap();
        init_command(&project).assert().success();
        assert_eq!(
            fs::read_to_string(project.join("pixi.toml")).unwrap(),
            manifest
        );
        // Regeneration is equally inert.
        init_command(&project).assert().success();
        assert_eq!(
            fs::read_to_string(project.join("pixi.toml")).unwrap(),
            manifest
        );
    }
}

#[test]
fn init_still_refuses_a_directory_without_a_pixi_manifest() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    // The scaffolding assumes a Pixi project (the generated config defaults to the
    // `default` environment), so a missing manifest is named, not generated around.
    // `init_command` would paper over this by writing a manifest, so the command is
    // built directly.
    let mut command = bin();
    command.args(["init", "--project-root", project.to_str().unwrap()]);
    command
        .assert()
        .failure()
        .stderr(predicate::str::contains("run init from a Pixi project"));
}

#[test]
fn init_honours_path_overrides_and_safely_regenerates_owned_files() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let extra = [
        "--github-workflow-path",
        "ci/generated.yml",
        "--script-path",
        "bin/airlock launcher",
        "--config",
        "config/sandbox plan.toml",
    ];

    init_command(&project).args(extra).assert().success();
    let workflow_path = project.join("ci/generated.yml");
    let script_path = project.join("bin/airlock launcher");
    let config_path = project.join("config/sandbox plan.toml");
    let workflow = fs::read_to_string(&workflow_path).unwrap();
    assert!(workflow.contains("--config 'config/sandbox plan.toml'"));
    let script = fs::read_to_string(&script_path).unwrap();
    #[cfg(unix)]
    assert!(script.contains("CONFIG=$ROOT/'config/sandbox plan.toml'"));
    #[cfg(windows)]
    assert!(script.contains("Join-Path $Root 'config/sandbox plan.toml'"));
    assert!(config_path.is_file());
    assert!(
        !project
            .join(".github/workflows/publish-sandbox.yml")
            .exists()
    );

    // Generated markers make an unchanged invocation safely regenerable without --force. The
    // config is user-owned after creation and must never be reset by regeneration.
    fs::write(&config_path, "user-edited config\n").unwrap();
    init_command(&project).args(extra).assert().success();
    assert_eq!(
        fs::read_to_string(&config_path).unwrap(),
        "user-edited config\n"
    );

    // An unmarked file is protected unless the user explicitly selects --force.
    fs::write(&workflow_path, "user-owned workflow\n").unwrap();
    init_command(&project)
        .args(extra)
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not owned by pixi-sandbox"));
    assert_eq!(
        fs::read_to_string(&workflow_path).unwrap(),
        "user-owned workflow\n"
    );
    init_command(&project)
        .args(extra)
        .arg("--force")
        .assert()
        .success();
    assert!(
        fs::read_to_string(&workflow_path)
            .unwrap()
            .contains("Generated by pixi-sandbox init")
    );
    assert_eq!(
        fs::read_to_string(&config_path).unwrap(),
        "user-edited config\n"
    );
}

/// task-53 AC#5: with policy carried in `[workflow]`, `init` on an existing project reproduces
/// the consumer's workflow byte-identically — no hand edits, no `--force` ritual. `init --check`
/// after a first `init` must also report clean, the same "nothing to re-apply" guarantee the
/// unconfigured case already holds (task-47 AC#4).
#[test]
fn init_with_workflow_policy_reproduces_byte_identically_with_no_hand_edits() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join(".pixi-sandbox.toml"),
        r#"
schema = 1
branch_prefix = "sandbox"

[[bundle]]
name = "developer"
environments = ["default"]
platforms = ["linux-64"]

[workflow]
push_paths = ["pixi-sandbox.toml", "pixi.toml", "pixi.lock"]
permissions = true
pixi_version = "0.81.0"
setup_pixi_cache = true

[workflow.concurrency]
group = "publish-sandbox"
cancel_in_progress = false

[workflow.timeouts]
plan = 15
publish = 60
"#,
    )
    .unwrap();

    init_command(&project).assert().success();
    let workflow_path = project.join(".github/workflows/publish-sandbox.yml");
    let first = fs::read_to_string(&workflow_path).unwrap();
    assert!(first.contains("    paths:\n"));
    assert!(first.contains("permissions:\n  contents: read\n"));
    assert!(first.contains("concurrency:\n  group: \"publish-sandbox\"\n"));
    assert!(first.contains("timeout-minutes: 15"));
    assert!(first.contains("timeout-minutes: 60"));
    assert!(first.contains("pixi-version: \"0.81.0\""));
    assert!(first.contains("cache: true"));

    // A clean tree, no hand edits applied: `init --check` reports no drift at all.
    init_command(&project).arg("--check").assert().success();

    // A second ordinary `init` reproduces the exact same bytes — the config carries the
    // policy, so there is nothing left for a consumer to re-apply "by ritual" (issue #79).
    init_command(&project).assert().success();
    let second = fs::read_to_string(&workflow_path).unwrap();
    assert_eq!(first, second, "init must regenerate byte-identically");
}

#[test]
fn init_prefers_the_new_config_name_and_falls_back_to_the_legacy_name() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join(".pixi-sandbox.toml"), "legacy config\n").unwrap();

    init_command(&project).assert().success();
    let workflow_path = project.join(".github/workflows/publish-sandbox.yml");
    assert!(
        fs::read_to_string(&workflow_path)
            .unwrap()
            .contains("--config .pixi-sandbox.toml")
    );
    assert!(!project.join("pixi-sandbox.toml").exists());

    fs::write(project.join("pixi-sandbox.toml"), "preferred config\n").unwrap();
    init_command(&project).assert().success();
    assert!(
        fs::read_to_string(&workflow_path)
            .unwrap()
            .contains("--config pixi-sandbox.toml")
    );
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
fn restore_materialises_the_fixture_vendor_tree_and_wires_sandbox_cargo_home() {
    // The fixture transport is used exactly as checked in — including its real unpacker stub.
    // task-33 made restore verify the tree it produced against the manifest's per-file oracle
    // before registering user tools, and a hand-rolled fake unpacker (this test used to swap
    // one in) cannot reproduce the oracle by construction; the genuine fixture can.
    let transport = transport_copy();

    let project_root = tempfile::tempdir().unwrap();
    let project = project_root.path().join("project");
    fs::create_dir_all(project.join(".cargo")).unwrap();
    let project_config = "[env]\nTS_RS_EXPORT_DIR = \"generated\"\n";
    fs::write(project.join(".cargo/config.toml"), project_config).unwrap();
    bin()
        // task-33: registration runs by default; keep it inside the tempdir.
        .env("HOME", project_root.path())
        .env("USERPROFILE", project_root.path())
        .env("SHELL", "/usr/bin/bash")
        .env("TMPDIR", project_root.path())
        .args([
            "restore",
            "--branch-location",
            transport.path().to_str().unwrap(),
            "--output-path",
            project.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "OK — the restored tree matches the manifest",
        ));

    assert!(
        project
            .join(".pixi-sandbox/vendor/demo-dep-1.0.0/src/lib.rs")
            .is_file(),
        "the vendor tree must be materialised after full transport verification"
    );
    let config = fs::read_to_string(project.join(".cargo/config.toml")).unwrap();
    assert_eq!(
        config, project_config,
        "project-owned Cargo config survives"
    );
    let cargo_home_config = fs::read_to_string(
        project
            .join(".pixi-sandbox")
            .join("cargo-home")
            .join("config.toml"),
    )
    .unwrap();
    assert!(
        cargo_home_config.contains(&format!(
            "directory = \"{}\"",
            project.join(".pixi-sandbox/vendor").display()
        )),
        "{cargo_home_config}"
    );
    let hook = fs::read_to_string(
        project.join(".pixi/envs/demo/etc/conda/activate.d/pixi-sandbox-cargo-home.sh"),
    )
    .unwrap();
    assert!(hook.contains("export CARGO_HOME="), "{hook}");
    assert!(hook.contains("$CARGO_HOME/bin:$PATH"), "{hook}");
}

#[cfg(unix)]
#[rstest]
#[case("print")]
#[case("none")]
fn restore_cargo_config_print_and_none_do_not_claim_cargo_is_wired(#[case] mode: &str) {
    let transport = transport_copy();
    let project_root = tempfile::tempdir().unwrap();
    let project = project_root.path().join(format!("project-{mode}"));
    fs::create_dir_all(&project).unwrap();

    let output = bin()
        .args([
            "restore",
            "--branch-location",
            transport.path().to_str().unwrap(),
            "--output-path",
            project.to_str().unwrap(),
            "--user-tools",
            "skip",
            "--cargo-config",
            mode,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).unwrap();

    assert!(
        output.contains(&format!("cargo vendor: NOT wired (--cargo-config {mode})")),
        "{output}"
    );
    assert!(
        !output.contains("pixi run --frozen -- cargo build --offline"),
        "{mode} must not print an offline build recipe it did not wire: {output}"
    );
    assert!(!project.join(".cargo/config.toml").exists());
    assert!(
        !project
            .join(".pixi-sandbox/cargo-home/config.toml")
            .exists()
    );
    assert!(
        !project
            .join(".pixi/envs/demo/etc/conda/activate.d/pixi-sandbox-cargo-home.sh")
            .exists()
    );
    if mode == "print" {
        assert!(
            output.contains("# Cargo source replacement for .pixi-sandbox/cargo-home/config.toml"),
            "{output}"
        );
    }
}

#[cfg(unix)]
#[test]
fn restore_cargo_config_write_keeps_the_destructive_project_config_mode() {
    let transport = transport_copy();
    let project_root = tempfile::tempdir().unwrap();
    let project = project_root.path().join("project");
    fs::create_dir_all(project.join(".cargo")).unwrap();
    fs::write(project.join(".cargo/config.toml"), "[env]\nKEEP = \"me\"\n").unwrap();

    bin()
        .args([
            "restore",
            "--branch-location",
            transport.path().to_str().unwrap(),
            "--output-path",
            project.to_str().unwrap(),
            "--user-tools",
            "skip",
            "--cargo-config",
            "write",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("cargo vendor: wired via"))
        .stdout(predicate::str::contains(
            "pixi run --frozen -- cargo build --offline",
        ));

    let config = fs::read_to_string(project.join(".cargo/config.toml")).unwrap();
    assert!(config.contains("[source.crates-io]"), "{config}");
    assert!(!config.contains("KEEP"), "write is explicitly destructive");
    assert!(
        !project
            .join(".pixi-sandbox/cargo-home/config.toml")
            .exists()
    );
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

#[cfg(unix)]
#[test]
fn generated_restore_archives_a_local_sandbox_branch_and_runs_its_nested_binary() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    init_command(&project)
        .args(["--branch", "sandbox/developer-linux-64"])
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
/// launcher reads the selected `pixi-sandbox.toml` at run time. Here the branch baked in at init time does
/// not exist, so only a launcher that consults the config can find the transport.
#[cfg(unix)]
#[test]
fn generated_restore_prefers_the_branch_declared_in_the_config() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    init_command(&project)
        .args(["--branch", "sandbox/never-published-linux-64"])
        .assert()
        .success();

    let platform = host_platform();
    fs::write(
        project.join("pixi-sandbox.toml"),
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
        "the launcher must resolve envs/tools-{platform} from pixi-sandbox.toml"
    );
    assert!(project.join("restored-by-config").is_file());
}

#[cfg(unix)]
#[test]
fn pack_refuses_a_lockfile_cargo_cannot_vendor() {
    // Without this check, `cargo vendor` aborts with "found duplicate version of package"
    // and no remedy, and the operator has to work out from cargo's message that the fix is
    // in their own dependency graph. It also happens *after* the graph is resolved and
    // downloaded, so the failure is slow to arrive and says nothing about what to do.
    let temp = tempfile::tempdir().unwrap();
    let repo = duplicate_source_project();
    let tools = temp.path().join("fake-tools");
    let transport = temp.path().join("transport");
    let self_bin = temp.path().join("self-bin");
    fake_tools(&tools);
    write_executable(&self_bin, "#!/bin/sh\necho pixi-sandbox self-binary\n");

    let output = bin()
        .env("PATH", path_with_fake_tools(&tools))
        .env("CARGO_NET_OFFLINE", "true")
        .args([
            "pack",
            "--repo-root",
            repo.to_str().unwrap(),
            "--envs",
            "default",
            "--output-dir",
            transport.to_str().unwrap(),
            "--self-bin",
            self_bin.to_str().unwrap(),
            "--cargo-vendor",
        ])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8_lossy(&output);

    // The crate and both sources, so the operator knows which collision to resolve...
    assert!(
        stderr.contains("itoa 1.0.15"),
        "the error must name the colliding crate: {stderr}"
    );
    assert!(
        stderr.contains("registry+https://github.com/rust-lang/crates.io-index"),
        "the error must name the registry source: {stderr}"
    );
    assert!(
        stderr.contains("git+file:///tmp/duplicate-source-gitdep"),
        "the error must name the git source: {stderr}"
    );
    // ...and the remedy, which is the whole point (AC#1). Cargo states the condition and
    // stops; a maintainer cannot act on it.
    assert!(
        stderr.contains("make the versions differ") && stderr.contains("drop one of the"),
        "the error must offer both remedies: {stderr}"
    );
    // Why it matters, so the message is not just an assertion.
    assert!(
        stderr.contains("<name>-<version>"),
        "the error should explain the collision: {stderr}"
    );

    // "before anything is published" (AC#1): the failure is pack-time, and a rejected lockfile
    // must not leave a half-built transport behind for the next command to trip over.
    assert!(
        !transport.exists(),
        "a refused pack must not leave a transport directory behind"
    );
}

#[cfg(unix)]
#[test]
fn pack_vendors_a_lockfile_whose_crates_come_from_one_source_each() {
    // The negative case, and the one that keeps the check honest. `demo-project` has several
    // crates, repeated version strings and a path member of its own, so a check that merely
    // looked for "more than one entry with the same name" would reject it. The duplicate
    // fixture also carries a `source`-less package on purpose: a path member is never
    // vendored, so it is never a duplicate source.
    let temp = tempfile::tempdir().unwrap();
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/demo-project");
    let tools = temp.path().join("fake-tools");
    let transport = temp.path().join("transport");
    let self_bin = temp.path().join("self-bin");
    fake_tools(&tools);
    write_executable(&self_bin, "#!/bin/sh\necho pixi-sandbox self-binary\n");

    // `cargo vendor` itself is not stubbed, so a pass here means the real dependency graph was
    // accepted. It needs the crates in the local cargo cache: `cargo test` must stay offline
    // (tests/fixtures/README.md), and on a machine without them the only thing this asserts
    // is that the *duplicate* error is absent, which is still the property under test.
    let output = bin()
        .env("PATH", path_with_fake_tools(&tools))
        .env("CARGO_NET_OFFLINE", "true")
        .args([
            "pack",
            "--repo-root",
            repo.to_str().unwrap(),
            "--envs",
            "default",
            "--output-dir",
            transport.to_str().unwrap(),
            "--self-bin",
            self_bin.to_str().unwrap(),
            "--cargo-vendor",
        ])
        .output()
        .expect("pack runs");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("more than one source"),
            "a single-source lockfile must not be rejected as a duplicate: {stderr}"
        );
    }
}

// ---------------------------------------------------------------------------
// self-update: the argument surface, exercised black-box.
//
// `cli.rs` and `commands/` are private to the binary target, so `tests/` cannot import them
// (only `lib.rs` is reachable — see AGENTS.md, Test conventions). Driving the real binary is
// therefore the way these are covered, and it is also the stronger test: it is the surface a
// user actually meets.
// ---------------------------------------------------------------------------

/// clap's structural audit (duplicate argument names, bad defaults, conflicting flags) runs as
/// `debug_assert!`s when the command tree is *built*, so any successful invocation of a debug
/// binary proves the tree is sound. This is the test that catches a regression of the
/// propagated `--version` colliding with `self-update --version`.
#[test]
fn the_self_update_command_tree_is_structurally_valid() {
    bin()
        .args(["self-update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--version <X.Y.Z>"))
        .stdout(predicate::str::contains("--check"))
        .stdout(predicate::str::contains("--dest <PATH>"));
}

/// The default repository is part of the contract: a consumer must not have to know it.
#[test]
fn self_update_defaults_to_the_canonical_release_repository() {
    bin()
        .args(["self-update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Archont561/pixi-sandbox"));
}

/// `--version` must take an exact release, and a non-version is refused before any network
/// access — the resolver's strictness, seen from outside.
#[test]
fn self_update_refuses_a_version_that_is_not_an_exact_release() {
    bin()
        .args([
            "self-update",
            "--version",
            "latest",
            "--dest",
            "/nonexistent/x",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("exact version"));
}

/// A Pixi-managed destination is refused with its remedy, and nothing is downloaded or
/// written. Pinning the version keeps the whole test offline.
#[test]
fn self_update_refuses_a_pixi_managed_destination_with_a_remedy() {
    let temp = tempfile::tempdir().expect("tempdir");
    let prefix = temp.path().join(".pixi/envs/default");
    std::fs::create_dir_all(prefix.join("conda-meta")).expect("mkdir");
    std::fs::create_dir_all(prefix.join("bin")).expect("mkdir");
    let managed = prefix.join("bin/pixi-sandbox");
    std::fs::write(&managed, b"pixi-managed bytes").expect("write");

    bin()
        .args([
            "self-update",
            "--version",
            "0.4.3",
            "--dest",
            managed.to_str().expect("utf-8"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("pixi update pixi-sandbox"));

    assert_eq!(
        std::fs::read(&managed).expect("read"),
        b"pixi-managed bytes",
        "a refused destination must be left byte-for-byte alone"
    );
}
