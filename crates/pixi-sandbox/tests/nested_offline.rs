//! TASK-129 AC#7: the two offline observables for a bundled nested Pixi workspace.
//!
//! The ordinary nested-workspace tests drive a hand-built v7 lock that is enough for `pack`
//! but that pixi itself refuses to load (`missing field 'subdir'`), so they can prove transport
//! identity and destination layout but not the runtime claim. This deliberately ignored proof
//! closes that gap the way the Stage A spike did: pixi itself solves against a local `file://`
//! channel and writes the locks, then the product packs the nested pair and restores them into
//! a clean consumer, and every pixi call afterwards runs inside a fresh Linux network
//! namespace with no egress:
//!
//! 1. `pixi install --frozen --offline` is a no-op: exit 0 and the restored payload is
//!    byte-identical afterwards (conda-meta bookkeeping mtimes excluded — pixi always rewrites
//!    those, see the Stage A spike notes in task-129-nested-workspaces.md).
//! 2. `pixi run --frozen --offline [--manifest-path …] <task>` succeeds and executes the
//!    package's payload, for both the root and the nested project.
//!
//! A cold negative control — environment removed, isolated cache, egress denied — must fail,
//! proving the no-op reads the restored environment rather than a cache. Executed by CI's
//! `test-nested-offline` gate; the network namespace and the pixi binary are Linux CI assets.

#![cfg(target_os = "linux")]

mod support;

use assert_cmd::cargo::cargo_bin;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::tempdir;

const PACKAGE_NAME: &str = "native-fixture";
const PACKAGE_SPEC: &str = "==1.0";
const PAYLOAD_LINE: &str = "native fixture";

#[test]
#[ignore = "needs pixi plus a sudo unshare network namespace; executed by CI's nested offline gate"]
#[allow(clippy::too_many_lines)]
fn nested_workspace_survives_pack_restore_and_offline_pixi_operations() {
    if env::var_os("GITHUB_ACTIONS").is_some() {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic| {
            default_hook(panic);
            let message = panic
                .payload()
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| panic.payload().downcast_ref::<&str>().copied())
                .unwrap_or("non-string panic");
            let message = message
                .replace('%', "%25")
                .replace('\r', "%0D")
                .replace('\n', "%0A");
            eprintln!("::error title=nested-offline::{message}");
        }));
    }

    let scratch = tempdir().expect("test tempdir");
    let home = scratch.path().join("home");
    let temporary_dir = scratch.path().join("tmp");
    let channel = scratch.path().join("channel");
    let repo = scratch.path().join("repo");
    let transport = scratch.path().join("transport");
    let consumer = scratch.path().join("consumer");
    fs::create_dir_all(&home).expect("isolated home");
    fs::create_dir_all(&temporary_dir).expect("isolated tmp");
    fs::create_dir_all(repo.join("subproject")).expect("nested project");
    fs::create_dir_all(&consumer).expect("consumer");

    // The committed local channel, copied into test-owned scratch (ADR-10): one noarch package
    // whose payload is share/hello.txt.
    support::copy_tree(
        &support::crate_dir().join("tests/fixtures/native-pack/channel"),
        &channel,
    );
    let channel_url = url::Url::from_file_path(&channel)
        .expect("channel path is a file URL")
        .to_string();

    // Prove the namespace has no external route before any pixi call depends on the claim.
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
        scratch.path(),
        &home,
        &temporary_dir,
        &home.join(".cache"),
    );
    assert!(
        !probe.status.success(),
        "the egress-denial probe unexpectedly reached the internet"
    );

    write_pixi_project(&repo, "offline-root", &channel_url);
    write_pixi_project(&repo.join("subproject"), "offline-subproject", &channel_url);

    let pixi = executable_on_path("pixi").expect("CI installs Pixi on PATH");

    // First solve inside the denied namespace: a local file:// channel needs no route, and the
    // locks pixi writes here are the genuine v7 records the hand-built fixture could not produce.
    assert_success(
        &run_egress_denied(
            &pixi,
            &["install".to_string()],
            &repo,
            &home,
            &temporary_dir,
            &home.join(".cache"),
        ),
        "root pixi install from the local channel with egress denied",
    );
    assert_success(
        &run_egress_denied(
            &pixi,
            ["install".to_string()].as_slice(),
            &repo.join("subproject"),
            &home,
            &temporary_dir,
            &home.join(".cache"),
        ),
        "nested pixi install from the local channel with egress denied",
    );
    for lock in [repo.join("pixi.lock"), repo.join("subproject/pixi.lock")] {
        let text = fs::read_to_string(&lock).expect("pixi wrote a lock");
        assert!(
            text.contains("version: 7") && text.contains("subdir: noarch"),
            "{} must be a pixi-written v7 lock carrying subdir, not the hand-built fixture shape",
            lock.display()
        );
    }

    fs::write(
        repo.join(".pixi-sandbox.toml"),
        "schema = 2\n\
         branch_prefix = \"sandbox\"\n\
         cargo_vendor = false\n\n\
         [[bundle]]\n\
         name = \"developer\"\n\
         environments = [\"default\"]\n\
         platforms = [\"linux-64\"]\n\n\
         [[bundle.workspaces]]\n\
         manifest = \"subproject/pixi.toml\"\n",
    )
    .expect("sandbox config");

    // Product half, outside the namespace: one transport carries both environments.
    let mut pack = Command::new(cargo_bin("pixi-sandbox"));
    pack.env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("TMPDIR", &temporary_dir)
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .current_dir(&repo)
        .args(["pack", "--repo-root"])
        .arg(&repo)
        .args(["--output-dir"])
        .arg(&transport)
        .args([
            "--envs",
            "default,subproject/default",
            "--platform",
            "linux-64",
        ])
        .args(["--config"])
        .arg(repo.join(".pixi-sandbox.toml"));
    assert_success(&pack.output().expect("run pack"), "nested pack");

    // The consumer starts with manifests and locks only, the way an airlocked checkout does.
    for relative in ["pixi.toml", "pixi.lock"] {
        fs::copy(repo.join(relative), consumer.join(relative)).expect("root project file");
        fs::create_dir_all(consumer.join("subproject")).expect("nested consumer dir");
        fs::copy(
            repo.join("subproject").join(relative),
            consumer.join("subproject").join(relative),
        )
        .expect("nested project file");
    }
    let mut restore = Command::new(cargo_bin("pixi-sandbox"));
    restore
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("TMPDIR", &temporary_dir)
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .args(["restore", "--branch-location"])
        .arg(&transport)
        .args(["--output-path"])
        .arg(&consumer);
    let restore_output = restore.output().expect("run restore");
    assert_success(&restore_output, "nested restore");
    assert!(
        String::from_utf8_lossy(&restore_output.stdout).contains("restore complete"),
        "restore reports completion"
    );

    let root_env = consumer.join(".pixi/envs/default");
    let nested_env = consumer.join("subproject/.pixi/envs/default");
    assert!(
        root_env.is_dir() && nested_env.is_dir(),
        "both destinations materialise"
    );

    // Every post-restore pixi call gets a cache directory that has never held the package, so
    // the observables prove the restored environment alone is sufficient (Stage A spike: the
    // warm default cache can seed an empty XDG cache, which would make the proof vacuous).
    let runtime_cache = home.join("runtime-cache");
    fs::create_dir_all(&runtime_cache).expect("fresh runtime cache");

    // Observable 1: frozen/offline install is a no-op. Hash the payload excluding conda-meta,
    // whose history/pixi bookkeeping pixi rewrites on every invocation.
    let root_before = payload_hashes(&root_env);
    let nested_before = payload_hashes(&nested_env);
    assert_success(
        &run_egress_denied(
            &pixi,
            &[
                "install".to_string(),
                "--frozen".to_string(),
                "--offline".to_string(),
            ],
            &consumer,
            &home,
            &temporary_dir,
            &runtime_cache,
        ),
        "root frozen/offline install",
    );
    assert_success(
        &run_egress_denied(
            &pixi,
            &[
                "install".to_string(),
                "--frozen".to_string(),
                "--offline".to_string(),
                "--manifest-path".to_string(),
                "subproject/pixi.toml".to_string(),
            ],
            &consumer,
            &home,
            &temporary_dir,
            &runtime_cache,
        ),
        "nested frozen/offline install",
    );
    assert_eq!(
        root_before,
        payload_hashes(&root_env),
        "the root frozen/offline install changed no payload byte"
    );
    assert_eq!(
        nested_before,
        payload_hashes(&nested_env),
        "the nested frozen/offline install changed no payload byte"
    );

    // Observable 2: delegated tasks run the installed package with egress denied, both ways in.
    let root_run = run_egress_denied(
        &pixi,
        &[
            "run".to_string(),
            "--frozen".to_string(),
            "--offline".to_string(),
            "hello".to_string(),
        ],
        &consumer,
        &home,
        &temporary_dir,
        &runtime_cache,
    );
    assert_success(&root_run, "root pixi run with egress denied");
    assert!(
        String::from_utf8_lossy(&root_run.stdout).contains(PAYLOAD_LINE),
        "the root task executed the packaged payload"
    );
    let nested_run = run_egress_denied(
        &pixi,
        &[
            "run".to_string(),
            "--frozen".to_string(),
            "--offline".to_string(),
            "--manifest-path".to_string(),
            "subproject/pixi.toml".to_string(),
            "hello".to_string(),
        ],
        &consumer,
        &home,
        &temporary_dir,
        &runtime_cache,
    );
    assert_success(&nested_run, "delegated nested pixi run with egress denied");
    assert!(
        String::from_utf8_lossy(&nested_run.stdout).contains(PAYLOAD_LINE),
        "the delegated task executed the nested packaged payload"
    );

    // Negative control, last: environment removed, a never-used cache, and the local channel
    // cut (a network namespace cannot deny a file:// channel, so the directory itself goes).
    // Frozen/offline must then fail — otherwise the no-op above was proving nothing.
    fs::remove_dir_all(consumer.join("subproject/.pixi/envs")).expect("remove nested env");
    let cold_cache = home.join("cold-cache");
    fs::create_dir_all(&cold_cache).expect("fresh cold cache");
    let denied_channel = scratch.path().join("channel-denied");
    fs::rename(&channel, &denied_channel).expect("cut the local channel");
    let cold = run_egress_denied(
        &pixi,
        &[
            "install".to_string(),
            "--frozen".to_string(),
            "--offline".to_string(),
            "--manifest-path".to_string(),
            "subproject/pixi.toml".to_string(),
        ],
        &consumer,
        &home,
        &temporary_dir,
        &cold_cache,
    );
    assert!(
        !cold.status.success(),
        "a missing environment with the channel and every cache denied must not silently succeed"
    );
}

fn write_pixi_project(project: &Path, name: &str, channel_url: &str) {
    let manifest = format!(
        "[workspace]\n\
         name = \"{name}\"\n\
         channels = [\"{channel_url}\"]\n\
         platforms = [\"linux-64\"]\n\n\
         [dependencies]\n\
         {PACKAGE_NAME} = \"{PACKAGE_SPEC}\"\n\n\
         [tasks]\n\
         hello = \"cat $CONDA_PREFIX/share/hello.txt\"\n"
    );
    fs::write(project.join("pixi.toml"), manifest).expect("pixi manifest");
}

/// Relative payload paths mapped to their SHA-256, excluding `conda-meta` bookkeeping.
fn payload_hashes(env_root: &Path) -> BTreeMap<PathBuf, String> {
    let mut hashes = BTreeMap::new();
    collect_payload_hashes(env_root, env_root, &mut hashes);
    hashes
}

fn collect_payload_hashes(root: &Path, current: &Path, hashes: &mut BTreeMap<PathBuf, String>) {
    for entry in fs::read_dir(current).expect("read env directory") {
        let entry = entry.expect("directory entry");
        let path = entry.path();
        if path.is_dir() {
            collect_payload_hashes(root, &path, hashes);
        } else if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .expect("below env root")
                .to_path_buf();
            if relative.starts_with("conda-meta") {
                continue;
            }
            let digest = pixi_sandbox_core::shard::sha256_file(&path).expect("hash payload file");
            hashes.insert(relative, digest);
        }
    }
}

/// Same namespace shape as the payload round-trip proof: a fresh `unshare -n` network
/// namespace, root dropped back to the calling user, an isolated HOME/TMPDIR and an explicit
/// cache directory the caller controls (a never-used one for the cold negative control).
fn run_egress_denied(
    program: &Path,
    args: &[String],
    working_dir: &Path,
    home: &Path,
    temporary_dir: &Path,
    cache_home: &Path,
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
        .arg(format!("XDG_CACHE_HOME={}", cache_home.display()))
        .arg("PIXI_NO_CONFIG=true")
        .arg(format!("PATH={}", path.to_string_lossy()))
        .arg(program)
        .args(args)
        .current_dir(working_dir)
        .output()
        .expect("run command in an egress-denied network namespace")
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
