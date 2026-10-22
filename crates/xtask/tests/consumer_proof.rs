//! `xtask::consumer_proof` (TASK-89) — the release lane's consumer proof as tested Rust over
//! tempdir fixtures, ported out of the `multiline-run-allowed` blocks of `release.yml`.
//!
//! Every test here drives the module through its public API against a tempdir (ADR-10 forbids
//! pointing at this repository), and each was confirmed to fail with the behaviour it checks
//! neutralised — the mutation evidence is recorded per test in the task file.

use std::fs;
use std::path::{Path, PathBuf};
use xtask::consumer_proof;

fn read(dir: &Path, rel: &str) -> String {
    fs::read_to_string(dir.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

#[test]
fn the_consumer_fixture_is_a_solvable_pixi_project_carrying_the_proof_label() {
    let dir = tempfile::tempdir().expect("tempdir");
    let consumer = dir.path().join("consumer");
    let evidence = dir.path().join("evidence");

    consumer_proof::fixture(&consumer, &evidence, "v9.9.9").expect("fixture");

    // The manifest must parse as the Pixi project the proof then locks and packs.
    let manifest = read(&consumer, "pixi.toml");
    let parsed: toml::Table = toml::from_str(&manifest).expect("pixi.toml parses");
    let workspace = parsed.get("workspace").and_then(toml::Value::as_table);
    let deps = parsed.get("dependencies").and_then(toml::Value::as_table);

    assert_eq!(
        workspace
            .and_then(|w| w.get("name"))
            .and_then(toml::Value::as_str),
        Some("consumer-proof")
    );
    assert_eq!(
        workspace
            .and_then(|w| w.get("channels"))
            .and_then(toml::Value::as_array)
            .and_then(|c| c.first())
            .and_then(toml::Value::as_str),
        Some("https://prefix.dev/conda-forge")
    );
    // A dependency-free environment solves to no platform-specific lock entry at all, and
    // pixi-pack then fails with "platform not found in lockfile: linux-64". One small package
    // is what makes the environment real; the recipe's own comment has to survive the port.
    assert!(
        deps.is_some_and(|d| !d.is_empty()),
        "the environment must carry a dependency to lock: {manifest}"
    );
    assert!(
        deps.is_some_and(|d| d.contains_key("ripgrep")),
        "{manifest}"
    );
    assert!(
        manifest.contains("platform not found in lockfile"),
        "the reason for the dependency is part of the recipe: {manifest}"
    );

    assert_eq!(
        read(&consumer, "README.md"),
        "consumer fixture: v9.9.9\n",
        "the evidence records which release the fixture was built for"
    );
    for created in [
        consumer.join(".github/workflows"),
        consumer.join(".pixi"),
        evidence.clone(),
    ] {
        assert!(created.is_dir(), "{} is a directory", created.display());
    }
}

#[test]
fn a_stale_fixture_does_not_survive_into_the_next_proof() {
    let dir = tempfile::tempdir().expect("tempdir");
    let consumer = dir.path().join("consumer");
    let evidence = dir.path().join("evidence");

    // The shell began with `rm -rf` on both directories. That is not tidiness: a leftover
    // `publish-sandbox.yml` from an earlier run would make the byte-identical regeneration
    // check compare the previous release against itself.
    fs::create_dir_all(consumer.join(".github/workflows")).expect("mkdir");
    fs::create_dir_all(&evidence).expect("mkdir");
    fs::write(
        consumer.join(".github/workflows/publish-sandbox.yml"),
        "stale: true\n",
    )
    .expect("write");
    fs::write(evidence.join("manifest.json"), "{}").expect("write");

    consumer_proof::fixture(&consumer, &evidence, "v9.9.9").expect("fixture");

    assert!(
        !consumer
            .join(".github/workflows/publish-sandbox.yml")
            .exists(),
        "the previous run's publisher is gone"
    );
    assert!(
        !evidence.join("manifest.json").exists(),
        "the previous run's evidence is gone"
    );
    assert_eq!(
        read(&consumer, "README.md"),
        "consumer fixture: v9.9.9\n",
        "and the fresh fixture is in place"
    );
}

#[test]
fn a_publisher_missing_an_asserted_property_is_reported_by_name() {
    // The proof used to `grep -F` these two literals out of the generated publisher. The
    // assertions move into Rust unchanged: same two strings, same verdict, and the finding
    // names the literal so a red run says which property the generator dropped.
    let complete = "pixi-sandbox sha256sum --check selected.sha256 \"$PIXI_SANDBOX_BIN\"\n# against SHA256SUMS\n";
    assert_eq!(
        consumer_proof::assert_publisher(complete),
        Vec::<String>::new(),
        "a publisher carrying both properties is clean"
    );

    let findings = consumer_proof::assert_publisher("nothing to see here\n");
    assert_eq!(findings.len(), 2, "{findings:?}");
    for expected in consumer_proof::PUBLISHER_ASSERTIONS {
        assert!(
            findings.iter().any(|f| f.contains(expected)),
            "the finding names {expected:?}: {findings:?}"
        );
    }

    let partial = consumer_proof::assert_publisher("only sha256sum --check selected.sha256 here\n");
    assert_eq!(partial.len(), 1, "{partial:?}");
    assert!(
        partial[0].contains("SHA256SUMS"),
        "only the missing literal is reported: {partial:?}"
    );
}

/// Write an executable that only exists on the PATH this test builds.
fn only_on_path(dir: &Path, name: &str) -> PathBuf {
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).expect("mkdir");
    let program = bin.join(name);
    fs::write(&program, "#!/bin/sh\necho found\n").expect("write");
    set_executable(&program);
    program
}

#[cfg(unix)]
fn set_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod");
}

/// Run `sh -c <script>` under `env` (an empty environment plus exactly those variables).
fn run_isolated(script: &str, env: &[(&str, String)], path: &str) -> (bool, String) {
    let out = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(script)
        .env_clear()
        .env("PATH", path)
        .envs(env.iter().map(|(k, v)| (*k, v.as_str())))
        .output()
        .expect("sh runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    )
}

#[test]
fn the_lifecycle_environment_keeps_the_runners_path_and_the_restore_environment_removes_it() {
    // The shell recipe used two different isolation levels and the difference is the proof.
    // `pixi lock` and `pack` run with an isolated HOME but the runner's PATH, because they need
    // the runner's pixi; `restore` runs under `env -i` with `PATH=/usr/bin:/bin`, because with
    // nothing else reachable the restore cannot have fetched anything.
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path().join("consumer-home");
    only_on_path(dir.path(), "only-here");
    let runner_path = format!("{}:/usr/bin:/bin", dir.path().join("bin").display());

    let isolated = consumer_proof::isolated_env(&home);
    assert_eq!(
        isolated,
        vec![("HOME", home.to_string_lossy().into_owned())],
        "an isolated HOME overrides nothing else, so PATH is inherited"
    );
    let empty = consumer_proof::empty_env(&home);
    assert_eq!(
        empty,
        vec![
            ("HOME", home.to_string_lossy().into_owned()),
            ("PATH", consumer_proof::OFFLINE_PATH.to_string()),
        ],
        "an empty environment is HOME and a base-system PATH, nothing more"
    );
    assert_eq!(
        consumer_proof::OFFLINE_PATH,
        "/usr/bin:/bin",
        "the offline PATH is the one the shell recipe used"
    );

    // And the consequence, observed rather than asserted: under the isolated environment a
    // program the runner installed is reachable; under the empty one it is not.
    assert!(
        run_isolated("command -v only-here", &isolated, &runner_path).0,
        "the lifecycle steps can still reach the runner's tools"
    );
    assert!(
        !run_isolated("command -v only-here", &empty, &runner_path).0,
        "the restore steps cannot reach anything the runner installed"
    );
    assert!(
        run_isolated("command -v sh", &empty, &runner_path).0,
        "the base system is still reachable, so the restore is running, not merely failing"
    );
}

#[test]
fn adopting_the_built_binary_keeps_its_bytes_and_makes_it_executable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let artifact = dir.path().join("pixi-sandbox-x86_64-unknown-linux-musl");
    fs::write(&artifact, b"not really a binary").expect("write");
    let consumer = dir.path().join("consumer");
    fs::create_dir_all(&consumer).expect("mkdir");
    let adopted = consumer.join("pixi-sandbox");

    consumer_proof::adopt(&artifact, &adopted).expect("adopt");

    assert_eq!(
        fs::read(&adopted).expect("read"),
        b"not really a binary",
        "the consumer runs the exact bytes the build produced"
    );
    assert!(is_executable(&adopted), "the adopted CLI can be executed");
}

#[test]
fn adopting_a_binary_that_was_not_built_is_an_error_naming_the_artifact() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("no-such-artifact");

    let error = consumer_proof::adopt(&missing, &dir.path().join("pixi-sandbox"))
        .expect_err("a missing artifact cannot be adopted");

    // A release whose upload step silently produced nothing must say so at this step, with the
    // path in the message: this is the first step that touches the build's output.
    assert!(error.to_string().contains("no-such-artifact"), "{error}");
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
}

/// A stand-in CLI that reports the environment it was invoked in, so a test can observe which
/// isolation level it actually ran under.
fn fake_cli(dir: &Path) -> PathBuf {
    let binary = dir.join("pixi-sandbox");
    fs::write(
        &binary,
        r#"#!/bin/sh
case "$1" in
  --version) echo "pixi-sandbox 9.9.9 HOME=$HOME PATH=$PATH" ;;
  --help) echo "usage: pixi-sandbox [OPTIONS] PATH=$PATH" ;;
esac
"#,
    )
    .expect("write");
    set_executable(&binary);
    binary
}

#[test]
fn the_version_evidence_is_recorded_under_the_empty_environment_the_recipe_used() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path().join("consumer-home");
    let evidence = dir.path().join("evidence");
    fs::create_dir_all(&evidence).expect("mkdir");
    let binary = fake_cli(dir.path());

    consumer_proof::record_version(&binary, &home, &evidence).expect("record");

    let version = read(&evidence, "version.txt");
    assert!(
        version.starts_with("pixi-sandbox 9.9.9"),
        "the version line is the binary's own: {version}"
    );
    // The `--version` half ran under `env -i`: the consumer's HOME, and a PATH with nothing
    // the runner installed on it. That is the step which proves the binary stands alone.
    assert!(
        version.contains(&format!("HOME={}", home.display())),
        "{version}"
    );
    assert!(
        version.contains(&format!("PATH={}", consumer_proof::OFFLINE_PATH)),
        "the version check runs offline: {version}"
    );
    // The `--help` half deliberately did not: it runs with the ambient environment, exactly as
    // the shell recipe had it. Preserving the asymmetry is the port's contract — this task
    // changes the spelling of the proof, never what it proves.
    let help = read(&evidence, "help.txt");
    assert!(help.contains("usage: pixi-sandbox"), "{help}");
    assert!(
        !help.contains(&format!("PATH={}", consumer_proof::OFFLINE_PATH)),
        "the help check keeps the ambient PATH: {help}"
    );
}

#[test]
fn a_binary_that_cannot_report_its_version_fails_the_proof_naming_the_step() {
    let dir = tempfile::tempdir().expect("tempdir");
    let binary = dir.path().join("pixi-sandbox");
    fs::write(&binary, "#!/bin/sh\nexit 3\n").expect("write");
    set_executable(&binary);
    let evidence = dir.path().join("evidence");
    fs::create_dir_all(&evidence).expect("mkdir");

    let error =
        consumer_proof::record_version(&binary, &dir.path().join("consumer-home"), &evidence)
            .expect_err("a binary that exits non-zero cannot be evidence");

    // `main` reports `{error:#}`, which walks the whole chain, so this is the text a CI
    // annotation actually carries: the step, and the invocation underneath it.
    let annotation = format!("{error:#}");
    assert!(annotation.contains("--version"), "{annotation}");
    assert!(
        !evidence.join("version.txt").exists(),
        "a failed step leaves no evidence behind to be mistaken for a result"
    );
}

/// A stand-in `pixi-sandbox` whose `init` writes a publisher carrying both asserted properties.
/// When `unstable` is set the render differs on every invocation, which is what the
/// regenerate-and-compare half of the proof exists to catch.
fn publisher_cli(dir: &Path, unstable: bool) -> PathBuf {
    let binary = dir.join("pixi-sandbox");
    // Guarded on `init`, exactly like the real CLI: a stand-in that wrote its publisher on
    // every invocation would also write it for `--version` and `--help`, which run with an
    // inherited working directory — and land the file outside the tempdir.
    let body = if unstable {
        r#"#!/bin/sh
case "$1" in
  init)
    mkdir -p .github/workflows
    n=$(cat .render-count 2>/dev/null || echo 0)
    n=$((n+1))
    echo "$n" > .render-count
    printf 'pixi-sandbox sha256sum --check selected.sha256 "$BIN"\n# SHA256SUMS\nrender %s\n' "$n" > .github/workflows/publish-sandbox.yml
    ;;
esac
"#
    } else {
        r#"#!/bin/sh
case "$1" in
  init)
    mkdir -p .github/workflows
    printf 'pixi-sandbox sha256sum --check selected.sha256 "$BIN"\n# SHA256SUMS\n' > .github/workflows/publish-sandbox.yml
    ;;
esac
"#
    };
    fs::write(&binary, body).expect("write");
    set_executable(&binary);
    binary
}

fn consumer_with_cli(dir: &tempfile::TempDir, unstable: bool) -> (PathBuf, PathBuf, PathBuf) {
    let consumer = dir.path().join("consumer");
    let evidence = dir.path().join("evidence");
    fs::create_dir_all(&evidence).expect("mkdir");
    consumer_proof::fixture(&consumer, &evidence, "v9.9.9").expect("fixture");
    let binary = publisher_cli(dir.path(), unstable);
    (binary, consumer, evidence)
}

#[test]
fn a_byte_identical_regeneration_leaves_both_renders_as_evidence() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (binary, consumer, evidence) = consumer_with_cli(&dir, false);

    consumer_proof::render_publisher(&binary, &consumer, &dir.path().join("home"), &evidence)
        .expect("render");

    let first = read(&evidence, "publish-sandbox.first.yml");
    let second = read(&evidence, "publish-sandbox.yml");
    assert_eq!(first, second, "the two renders agree");
    assert!(
        first.contains("sha256sum --check selected.sha256"),
        "{first}"
    );
    // The evidence is the render itself, not a copy of something else: this is the text the
    // assertions below were made against and the artifact a reviewer will open.
    assert_eq!(
        read(&consumer, ".github/workflows/publish-sandbox.yml"),
        second,
        "the evidence is the consumer's own publisher"
    );
}

#[test]
fn a_publisher_that_renders_differently_the_second_time_fails_the_proof() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (binary, consumer, evidence) = consumer_with_cli(&dir, true);

    let error =
        consumer_proof::render_publisher(&binary, &consumer, &dir.path().join("home"), &evidence)
            .expect_err("a non-deterministic publisher is exactly what this step catches");

    let annotation = format!("{error:#}");
    assert!(
        annotation.contains("publish-sandbox.yml"),
        "the failure names the file that moved: {annotation}"
    );
    // Both renders survive as evidence, so the operator can diff them without re-running.
    assert_ne!(
        read(&evidence, "publish-sandbox.first.yml"),
        read(&evidence, "publish-sandbox.yml"),
        "the evidence preserves the disagreement"
    );
}

#[test]
fn a_publisher_missing_an_asserted_property_fails_the_proof() {
    let dir = tempfile::tempdir().expect("tempdir");
    let consumer = dir.path().join("consumer");
    let evidence = dir.path().join("evidence");
    fs::create_dir_all(&evidence).expect("mkdir");
    consumer_proof::fixture(&consumer, &evidence, "v9.9.9").expect("fixture");
    let binary = dir.path().join("pixi-sandbox");
    fs::write(
        &binary,
        "#!/bin/sh\nmkdir -p .github/workflows\necho 'curl the binary' > .github/workflows/publish-sandbox.yml\n",
    )
    .expect("write");
    set_executable(&binary);

    let error =
        consumer_proof::render_publisher(&binary, &consumer, &dir.path().join("home"), &evidence)
            .expect_err("a publisher with neither property cannot pass");

    let annotation = format!("{error:#}");
    for expected in consumer_proof::PUBLISHER_ASSERTIONS {
        assert!(annotation.contains(expected), "{expected}: {annotation}");
    }
}

#[test]
fn the_pack_and_restore_command_lines_are_the_ones_the_recipe_ran() {
    let transport = Path::new("/tmp/proof/transport");
    let checkout = Path::new("/tmp/proof/checkout");
    let restored = Path::new("/tmp/proof/restored");

    // `--pixi-bin` and `--self-bin` are what make the packed transport self-contained, and
    // `--user-tools skip` is what keeps the restore from writing into the runner's HOME. Each
    // is a property of the proof, so each is pinned here rather than retyped in a workflow.
    assert_eq!(
        consumer_proof::pack_argv(transport, "linux-64", Path::new("./pixi-sandbox")),
        vec![
            "pack",
            "--repo-root",
            ".",
            "--envs",
            "default",
            "--output-dir",
            "/tmp/proof/transport",
            "--platform",
            "linux-64",
            "--pixi-bin",
            "pixi",
            "--self-bin",
            "./pixi-sandbox",
        ]
    );
    assert_eq!(
        consumer_proof::doctor_argv(transport),
        vec![
            "doctor",
            "--branch-location",
            "/tmp/proof/transport",
            "--verify"
        ]
    );
    assert_eq!(
        consumer_proof::publish_argv(transport, Path::new("/tmp/proof/remote")),
        vec![
            "publish",
            "--input-dir",
            "/tmp/proof/transport",
            "--branch-name",
            consumer_proof::CONSUMER_BRANCH,
            "--remote",
            "/tmp/proof/remote",
        ]
    );
    assert_eq!(
        consumer_proof::restore_argv(checkout, restored),
        vec![
            "restore",
            "--branch-location",
            "/tmp/proof/checkout",
            "--output-path",
            "/tmp/proof/restored",
            "--user-tools",
            "skip",
        ]
    );
}

#[test]
fn a_lifecycle_started_without_its_earlier_steps_says_which_one_is_missing() {
    // The shell enforced this ordering by failing somewhere deep inside `pack`. Naming the
    // missing precondition does not change the verdict — the run still fails — but the failure
    // lands at the step that was skipped instead of three tools later.
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = consumer_proof::LifecyclePaths {
        consumer: dir.path().join("consumer"),
        transport: dir.path().join("transport"),
        remote: dir.path().join("consumer-proof.git"),
        checkout: dir.path().join("fresh-checkout"),
        restored: dir.path().join("restored"),
        home: dir.path().join("consumer-home"),
        evidence: dir.path().join("evidence"),
    };

    let refusals = consumer_proof::lifecycle_refusals(&paths);
    assert_eq!(refusals.len(), 3, "nothing was prepared: {refusals:?}");
    for expected in ["pixi.toml", "pixi-sandbox", "publish-sandbox.yml"] {
        assert!(
            refusals.iter().any(|r| r.contains(expected)),
            "{expected} is named: {refusals:?}"
        );
    }

    consumer_proof::fixture(&paths.consumer, &paths.evidence, "v9.9.9").expect("fixture");
    let refusals = consumer_proof::lifecycle_refusals(&paths);
    assert_eq!(refusals.len(), 2, "the fixture is in place: {refusals:?}");

    fs::write(paths.consumer.join("pixi-sandbox"), b"binary").expect("write");
    fs::create_dir_all(paths.consumer.join(".github/workflows")).expect("mkdir");
    fs::write(
        paths.consumer.join(consumer_proof::PUBLISHER_REL_PATH),
        "publisher\n",
    )
    .expect("write");
    assert_eq!(
        consumer_proof::lifecycle_refusals(&paths),
        Vec::<String>::new(),
        "every earlier step has run, so the lifecycle may proceed"
    );
}

#[test]
fn the_lifecycle_refuses_before_it_invokes_a_single_tool() {
    // Locally provable, and the reason the refusals are a separate function: with a missing
    // precondition the lifecycle bails without spawning `pixi`, `git` or the consumer CLI, so
    // the failure needs no network and no toolchain to reproduce.
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = consumer_proof::LifecyclePaths {
        consumer: dir.path().join("consumer"),
        transport: dir.path().join("transport"),
        remote: dir.path().join("consumer-proof.git"),
        checkout: dir.path().join("fresh-checkout"),
        restored: dir.path().join("restored"),
        home: dir.path().join("consumer-home"),
        evidence: dir.path().join("evidence"),
    };

    let error = consumer_proof::lifecycle(&paths, "linux-64")
        .expect_err("a lifecycle with no fixture cannot run");

    let annotation = format!("{error:#}");
    assert!(annotation.contains("pixi.toml"), "{annotation}");
    assert!(
        !paths.transport.exists() && !paths.remote.exists(),
        "nothing was created: the refusal came first"
    );
}

#[test]
fn the_version_check_runs_with_nothing_but_the_home_and_the_offline_path() {
    // `Isolation::Empty` and `Isolation::Ambient` both apply the same overrides, so a test that
    // only looks at HOME or PATH cannot tell them apart. The difference is everything else: an
    // empty environment carries no leaked runner configuration into the proof. This asks the
    // binary what its own environment is, and counts what came back.
    let dir = tempfile::tempdir().expect("tempdir");
    let binary = dir.path().join("pixi-sandbox");
    fs::write(
        &binary,
        "#!/bin/sh\ncase \"$1\" in --version) env | sort ;; --help) echo help ;; esac\n",
    )
    .expect("write");
    set_executable(&binary);
    let evidence = dir.path().join("evidence");
    fs::create_dir_all(&evidence).expect("mkdir");
    let home = dir.path().join("consumer-home");

    consumer_proof::record_version(&binary, &home, &evidence).expect("record");

    let dump = read(&evidence, "version.txt");
    let names: Vec<&str> = dump.lines().filter_map(|l| l.split('=').next()).collect();
    assert!(
        dump.contains(&format!("HOME={}\n", home.display()))
            || dump.ends_with(&format!("HOME={}", home.display())),
        "{dump}"
    );
    assert!(
        dump.contains(&format!("PATH={}\n", consumer_proof::OFFLINE_PATH))
            || dump.ends_with(&format!("PATH={}", consumer_proof::OFFLINE_PATH)),
        "{dump}"
    );
    // `PWD` is the shell's own: a POSIX shell sets it on startup, so it survives `env_clear`.
    // Everything else must be gone — under `Isolation::Ambient` this list would carry the whole
    // runner environment with it.
    let unexpected: Vec<&&str> = names
        .iter()
        .filter(|n| **n != "HOME" && **n != "PATH" && **n != "PWD")
        .collect();
    assert!(
        unexpected.is_empty(),
        "the version check inherited more than HOME and PATH: {dump}"
    );
}

#[test]
fn every_evidence_file_a_step_writes_is_one_the_manifest_declares() {
    // `EVIDENCE_FILES` is the contract with the workflow's upload step: the artifact is a glob
    // over the evidence directory, so a step that writes a name nobody declared still uploads —
    // and a manifest that lists a name no step writes uploads a file that is silently absent.
    // This is the recipe's own list, spelled independently, so the two cannot drift together.
    assert_eq!(
        consumer_proof::EVIDENCE_FILES,
        [
            "version.txt",
            "help.txt",
            "publish-sandbox.first.yml",
            "publish-sandbox.yml",
            "pack.log",
            "doctor.log",
            "publish.log",
            "restore.log",
            "restored-pixi-version.txt",
            "manifest.json",
        ]
    );

    // And the steps that run locally write only declared names, so a typo in one of them fails
    // here rather than on a runner.
    let dir = tempfile::tempdir().expect("tempdir");
    let (binary, consumer, evidence) = consumer_with_cli(&dir, false);
    let home = dir.path().join("home");
    consumer_proof::record_version(&binary, &home, &evidence).expect("version");
    consumer_proof::render_publisher(&binary, &consumer, &home, &evidence).expect("render");

    let written: Vec<String> = fs::read_dir(&evidence)
        .expect("evidence")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert!(!written.is_empty(), "the steps wrote evidence");
    for name in &written {
        assert!(
            consumer_proof::EVIDENCE_FILES.contains(&name.as_str()),
            "{name} is written but not declared"
        );
    }
    for expected in ["version.txt", "help.txt", "publish-sandbox.yml"] {
        assert!(
            written.iter().any(|n| n == expected),
            "{expected} is missing"
        );
    }
}
