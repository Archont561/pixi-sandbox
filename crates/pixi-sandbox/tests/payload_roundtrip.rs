//! TASK-78 AC#3/#5: the connected build and the egress-denied consumer round trip.
//!
//! This deliberately ignored proof fetches the reviewed `rattler-build` binary, builds a real
//! `.conda`, publishes a filesystem channel, then uses Pixi against that local channel inside a
//! fresh Linux network namespace with no egress. The fixture and both Pixi projects live only in
//! a test-owned temporary directory (ADR-10); no payload bytes enter this checkout.

#![cfg(target_os = "linux")]

use assert_cmd::cargo::cargo_bin;
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::tempdir;

const PACKAGE_NAME: &str = "demo-payload";
const PACKAGE_VERSION: &str = "1.0.0";

#[test]
#[ignore = "downloads pinned rattler-build; executed by CI's payload round-trip task"]
#[allow(clippy::too_many_lines)]
fn real_package_preserves_payload_and_installs_under_egress_denial() {
    let scratch = tempdir().expect("test tempdir");
    let publisher = scratch.path().join("publisher");
    let consumer = scratch.path().join("consumer");
    let home = scratch.path().join("home");
    let temporary_dir = scratch.path().join("tmp");
    let rattler_cache = scratch.path().join("tools-cache");
    fs::create_dir_all(&publisher).expect("publisher project");
    fs::create_dir_all(&consumer).expect("consumer project");
    fs::create_dir_all(&home).expect("isolated home");
    fs::create_dir_all(&temporary_dir).expect("isolated tmp");

    // The default working directory, stage directory and channel all stay in ignored `.pixi/`.
    let work = publisher.join(".pixi/payload-work");
    let payload = work.join("payload");
    fs::create_dir_all(payload.join("bin")).expect("payload bin");
    fs::create_dir_all(payload.join("share")).expect("payload share");
    fs::create_dir_all(payload.join("empty/nested")).expect("empty nested directory");

    let artifact = b"fixture source artifact; the payload itself is already prepared\n";
    fs::write(work.join("artifact.tar.gz"), artifact).expect("fixture artifact");
    let message = b"relative-payload-data\n";
    fs::write(payload.join("share/message.txt"), message).expect("payload message");
    fs::write(payload.join("share/binary.dat"), b"\x00\xff\x01\x7f\x00B")
        .expect("payload binary data");
    let payload_tool = payload.join("bin/tool");
    fs::write(
        &payload_tool,
        b"#!/bin/sh\nset -eu\ncat \"$(dirname \"$0\")/../share/message.txt\"\n",
    )
    .expect("payload entry point");
    fs::set_permissions(&payload_tool, fs::Permissions::from_mode(0o755)).expect("executable mode");
    symlink("bin/tool", payload.join("tool-link")).expect("relative symlink");
    symlink("missing/target", payload.join("dangling-link")).expect("dangling symlink");

    let digest = pixi_sandbox_core::shard::sha256_bytes(artifact);
    let config = format!(
        "schema = 1\n\
         name = \"{PACKAGE_NAME}\"\n\
         version = \"{PACKAGE_VERSION}\"\n\
         platform = \"linux-64\"\n\
         prepare = \"true\"\n\
         artifact = \"artifact.tar.gz\"\n\
         payload = \"payload\"\n\
         entry_point = \"bin/tool\"\n\n\
         [source]\n\
         url = \"https://vendor.example/{PACKAGE_NAME}-{PACKAGE_VERSION}.tar.gz\"\n\
         sha256 = \"{digest}\"\n"
    );
    let config_path = publisher.join("pixi-sandbox-payload.toml");
    fs::write(&config_path, config).expect("payload config");

    let mut build = Command::new(cargo_bin("pixi-sandbox"));
    build
        .current_dir(&publisher)
        .args(["wrap-payload", "--config"])
        .arg(config_path)
        .args(["--skip-prepare", "--build", "--tools-cache"])
        .arg(rattler_cache)
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("TMPDIR", &temporary_dir);
    let build_output = build.output().expect("run real payload build");
    assert_success(&build_output, "wrap-payload --build");

    let stage = publisher.join(".pixi/payload-build");
    let channel = publisher.join(".pixi/payload-channel");
    assert!(
        stage.join("recipe.yaml").is_file(),
        "the real recipe was staged"
    );
    assert!(
        channel.join("linux-64/repodata.json").is_file(),
        "rattler-build published a real indexed channel"
    );
    let package_count = fs::read_dir(channel.join("linux-64"))
        .expect("channel subdir")
        .map(|entry| entry.expect("channel entry"))
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "conda")
        })
        .count();
    assert_eq!(package_count, 1, "the indexed channel contains one package");

    let manifest = format!(
        "[workspace]\nname = \"payload-roundtrip-consumer\"\n\
         channels = [\"file://{}\"]\nplatforms = [\"linux-64\"]\n",
        channel.display()
    );
    fs::write(consumer.join("pixi.toml"), manifest).expect("consumer manifest");

    // Prove the namespace has no external route before attempting either consumer operation.
    let curl = Path::new("/usr/bin/curl");
    assert!(curl.is_file(), "the Linux CI runner provides system curl");
    let probe = run_egress_denied(
        curl,
        &[
            "--silent".to_string(),
            "--show-error".to_string(),
            "--insecure".to_string(),
            "--connect-timeout".to_string(),
            "2".to_string(),
            "--max-time".to_string(),
            "5".to_string(),
            "https://example.com/".to_string(),
        ],
        &consumer,
        &home,
        &temporary_dir,
    );
    assert!(
        !probe.status.success(),
        "the egress-denial probe unexpectedly reached the internet: {}",
        String::from_utf8_lossy(&probe.stdout)
    );

    let pixi = executable_on_path("pixi").expect("CI installs Pixi on PATH");
    let add = run_egress_denied(
        &pixi,
        &[
            "add".to_string(),
            "--no-install".to_string(),
            format!("{PACKAGE_NAME}=={PACKAGE_VERSION}"),
        ],
        &consumer,
        &home,
        &temporary_dir,
    );
    assert_success(&add, "pixi add from the local channel with egress denied");
    assert!(
        consumer.join("pixi.lock").is_file(),
        "pixi add resolved the local package into a lockfile"
    );
    assert!(
        fs::read_to_string(consumer.join("pixi.lock"))
            .expect("consumer lockfile")
            .contains(PACKAGE_NAME),
        "the lockfile pins the wrapped payload"
    );

    let install = run_egress_denied(
        &pixi,
        &[
            "install".to_string(),
            "--frozen".to_string(),
            "--offline".to_string(),
        ],
        &consumer,
        &home,
        &temporary_dir,
    );
    assert_success(
        &install,
        "pixi install --frozen --offline with egress denied",
    );

    let prefix = consumer.join(".pixi/envs/default");
    let installed_payload = prefix.join(format!("libexec/{PACKAGE_NAME}"));
    assert_payload_tree_matches(&payload, &installed_payload);
    assert!(
        prefix.join("bin/tool").is_file(),
        "the package exposes the requested entry point on PATH"
    );
    assert_eq!(
        fs::read_link(installed_payload.join("tool-link")).expect("installed symlink"),
        Path::new("bin/tool"),
        "the relative symlink target survived the package round trip"
    );
    assert_eq!(
        fs::read_link(installed_payload.join("dangling-link")).expect("installed dangling link"),
        Path::new("missing/target"),
        "the dangling symlink target survived the package round trip"
    );
    let installed_mode = fs::metadata(installed_payload.join("bin/tool"))
        .expect("installed tool")
        .permissions()
        .mode();
    assert_eq!(installed_mode & 0o111, 0o111, "the executable bit survived");

    // Execute the installed launcher in the same egress-denied namespace. The script reads a
    // sibling payload file relative to its real path, checking launch and in-place semantics.
    let launcher = prefix.join("bin/tool");
    let launcher_output = run_egress_denied(&launcher, &[], &consumer, &home, &temporary_dir);
    assert_success(
        &launcher_output,
        "installed payload launcher with egress denied",
    );
    assert_eq!(
        String::from_utf8(launcher_output.stdout).expect("launcher output is UTF-8"),
        String::from_utf8(message.to_vec()).expect("fixture message is UTF-8"),
        "the launcher executed the original payload in place"
    );
}

fn run_egress_denied(
    program: &Path,
    args: &[String],
    working_dir: &Path,
    home: &Path,
    temporary_dir: &Path,
) -> Output {
    let user = env::var("USER").expect("the Linux CI runner sets USER");
    let path = env::var_os("PATH").expect("Pixi task PATH");
    let mut command = Command::new("sudo");
    command
        .args(["-n", "unshare", "-n", "--", "sudo", "-n", "-u"])
        .arg(user)
        .args(["--", "/usr/bin/env"])
        .arg(format!("HOME={}", home.display()))
        .arg(format!("USERPROFILE={}", home.display()))
        .arg(format!("TMPDIR={}", temporary_dir.display()))
        .arg(format!("XDG_CACHE_HOME={}", home.join(".cache").display()))
        .arg("PIXI_NO_CONFIG=true")
        .arg(format!("PATH={}", path.to_string_lossy()))
        .arg(program)
        .args(args)
        .current_dir(working_dir)
        .output()
        .expect("run command in an egress-denied network namespace")
}

fn assert_payload_tree_matches(source: &Path, installed: &Path) {
    let mut source_entries = BTreeSet::new();
    let mut installed_entries = BTreeSet::new();
    collect_entries(source, source, &mut source_entries);
    collect_entries(installed, installed, &mut installed_entries);
    assert_eq!(
        source_entries, installed_entries,
        "the installed path set must exactly match the source payload"
    );

    for relative in source_entries {
        let source_path = source.join(&relative);
        let installed_path = installed.join(&relative);
        let source_metadata = fs::symlink_metadata(&source_path).expect("source metadata");
        let installed_metadata = fs::symlink_metadata(&installed_path).expect("installed metadata");
        let source_type = source_metadata.file_type();
        let installed_type = installed_metadata.file_type();
        assert_eq!(
            source_type.is_dir(),
            installed_type.is_dir(),
            "directory type for {}",
            relative.display()
        );
        assert_eq!(
            source_type.is_symlink(),
            installed_type.is_symlink(),
            "symlink type for {}",
            relative.display()
        );
        if source_type.is_symlink() {
            assert_eq!(
                fs::read_link(&source_path).expect("source link"),
                fs::read_link(&installed_path).expect("installed link"),
                "symlink target for {}",
                relative.display()
            );
        } else if source_type.is_file() {
            assert_eq!(
                fs::read(&source_path).expect("source bytes"),
                fs::read(&installed_path).expect("installed bytes"),
                "file bytes for {}",
                relative.display()
            );
            assert_eq!(
                source_metadata.permissions().mode() & 0o111,
                installed_metadata.permissions().mode() & 0o111,
                "executable bits for {}",
                relative.display()
            );
        }
    }
}

fn collect_entries(root: &Path, directory: &Path, entries: &mut BTreeSet<PathBuf>) {
    let directory_entries = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()));
    for entry in directory_entries {
        let entry = entry.expect("directory entry");
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .expect("entry is below payload root")
            .to_path_buf();
        entries.insert(relative);
        if fs::symlink_metadata(&path)
            .expect("entry metadata")
            .file_type()
            .is_dir()
        {
            collect_entries(root, &path, entries);
        }
    }
}

fn executable_on_path(name: &str) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?)
        .map(|directory| directory.join(name))
        .find(|path| path.is_file())
}

fn assert_success(output: &Output, operation: &str) {
    assert!(
        output.status.success(),
        "{operation} failed ({}):\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
