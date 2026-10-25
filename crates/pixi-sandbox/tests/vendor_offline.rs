//! TASK-131 AC#6: two cargo projects with real registry crates survive pack, restore and an
//! offline build against their own vendored trees.
//!
//! The ordinary nested-workspace tests vendor through a fake cargo wrapper that copies a
//! canned tree, so they prove transport identity and restore layout but not the runtime claim.
//! This deliberately ignored proof closes that gap with real `cargo vendor` output: the root
//! project depends on `itoa`, the nested workspace on `memchr` — two distinct `Cargo.lock`
//! files, two vendor trees in one transport. The connected-side steps (lockfile generation,
//! vendoring) need crates.io and run with normal egress; every post-restore cargo command runs
//! inside a fresh Linux network namespace with an isolated HOME and cache:
//!
//! 1. `cargo build --offline` succeeds in BOTH projects, each reading its own
//!    `.pixi-sandbox/cargo-home/config.toml` and its own vendor directory.
//! 2. The built binaries run and print output that proves the registry crates really linked.
//! 3. A cold negative control — one vendor tree removed, fresh target and cache, egress denied
//!    — must fail, proving the build read the restored tree rather than a warm cache.
//!
//! Executed by CI's `test-vendor-offline` gate; the network namespace, crates.io access and the
//! cargo/rustc toolchain are Linux CI assets (see `tests/nested_offline.rs` for the pixi half).

#![cfg(target_os = "linux")]

mod support;

use assert_cmd::cargo::cargo_bin;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::tempdir;

#[test]
#[ignore = "needs crates.io at pack time plus a sudo unshare network namespace; CI's vendor offline gate"]
#[allow(clippy::too_many_lines)]
fn two_lockfiles_pack_restore_and_build_offline_against_their_own_trees() {
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
            eprintln!("::error title=vendor-offline::{message}");
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

    // The committed local pixi channel, copied into test-owned scratch (ADR-10).
    support::copy_tree(
        &support::crate_dir().join("tests/fixtures/native-pack/channel"),
        &channel,
    );
    let channel_url = url::Url::from_file_path(&channel)
        .expect("channel path is a file URL")
        .to_string();

    // Prove the namespace has no external route before any offline claim depends on it.
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
        None,
    );
    assert!(
        !probe.status.success(),
        "the egress-denial probe unexpectedly reached the internet"
    );

    write_pixi_project(&repo, "offline-root", &channel_url);
    write_pixi_project(&repo.join("subproject"), "offline-subproject", &channel_url);
    write_cargo_project(&repo, "offline-root", ROOT_MAIN);
    write_cargo_project(
        &repo.join("subproject"),
        "offline-subproject",
        SUBPROJECT_MAIN,
    );

    let pixi = executable_on_path("pixi").expect("CI installs Pixi on PATH");
    let cargo = executable_on_path("cargo").expect("the pixi task environment provides cargo");

    // Pixi half, inside the denied namespace: a local file:// channel needs no route.
    assert_success(
        &run_egress_denied(
            &pixi,
            &["install".to_string()],
            &repo,
            &home,
            &temporary_dir,
            &home.join(".cache"),
            None,
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
            None,
        ),
        "nested pixi install from the local channel with egress denied",
    );

    // Cargo half, connected side: real lockfiles against crates.io, the only network-requiring
    // step of the proof. The two locks deliberately pin different crates.
    assert_success(
        &cargo_command(&cargo, &repo, &home, &temporary_dir)
            .args(["generate-lockfile"])
            .output()
            .expect("run root cargo generate-lockfile"),
        "root cargo generate-lockfile",
    );
    assert_success(
        &cargo_command(&cargo, &repo.join("subproject"), &home, &temporary_dir)
            .args(["generate-lockfile"])
            .output()
            .expect("run nested cargo generate-lockfile"),
        "nested cargo generate-lockfile",
    );

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

    // One transport carries both pixi environments and both vendor trees.
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
            "--cargo-vendor",
        ])
        .args(["--config"])
        .arg(repo.join(".pixi-sandbox.toml"));
    assert_success(&pack.output().expect("run pack"), "two-tree vendor pack");

    let manifest = pixi_sandbox_core::manifest::Manifest::load(
        &pixi_sandbox_core::manifest::Manifest::path_in(&transport),
    )
    .expect("packed manifest loads");
    assert!(manifest.vendor.is_some(), "the root lock is vendored");
    assert_eq!(manifest.workspace_vendors.len(), 1, "one nested tree");
    assert_eq!(manifest.workspace_vendors[0].slug, "subproject");
    assert!(
        glob_dir_exists(&transport.join(".pixi-sandbox/vendor"), "itoa-"),
        "the root tree carries the real itoa crate"
    );
    assert!(
        glob_dir_exists(
            &transport.join(".pixi-sandbox/vendor/workspaces/subproject"),
            "memchr-"
        ),
        "the nested tree carries the real memchr crate"
    );

    // The consumer starts with source manifests, locks and sources only, the way an airlocked
    // checkout does — no target/ directory, no cargo cache.
    for relative in ["pixi.toml", "pixi.lock", "Cargo.toml", "Cargo.lock"] {
        fs::copy(repo.join(relative), consumer.join(relative)).expect("root project file");
        fs::create_dir_all(consumer.join("subproject")).expect("nested consumer dir");
        fs::copy(
            repo.join("subproject").join(relative),
            consumer.join("subproject").join(relative),
        )
        .expect("nested project file");
    }
    copy_sources(&repo.join("src"), &consumer.join("src"));
    copy_sources(
        &repo.join("subproject/src"),
        &consumer.join("subproject/src"),
    );

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
    assert_success(&restore_output, "two-tree restore");
    assert!(
        String::from_utf8_lossy(&restore_output.stdout).contains("restore complete"),
        "restore reports completion"
    );

    // Each project owns a complete tree and a cargo-home naming that exact directory.
    let root_vendor = consumer.join(".pixi-sandbox/vendor");
    let nested_vendor = consumer.join("subproject/.pixi-sandbox/vendor");
    assert!(root_vendor.join("itoa-1.0.0").is_dir() || glob_dir_exists(&root_vendor, "itoa-"));
    assert!(
        nested_vendor.join("memchr-2.0.0").is_dir() || glob_dir_exists(&nested_vendor, "memchr-")
    );
    let root_home = fs::read_to_string(consumer.join(".pixi-sandbox/cargo-home/config.toml"))
        .expect("root sandbox cargo-home config");
    assert!(
        root_home.contains(&root_vendor.display().to_string()),
        "{root_home}"
    );
    let nested_home =
        fs::read_to_string(consumer.join("subproject/.pixi-sandbox/cargo-home/config.toml"))
            .expect("nested sandbox cargo-home config");
    assert!(
        nested_home.contains(&nested_vendor.display().to_string()),
        "{nested_home}"
    );

    // A cache that has never held either crate, so the offline builds must read the trees.
    let runtime_cache = home.join("runtime-cache");
    fs::create_dir_all(&runtime_cache).expect("fresh runtime cache");

    let root_build = run_egress_denied(
        &cargo,
        &["build".to_string(), "--offline".to_string()],
        &consumer,
        &home,
        &temporary_dir,
        &runtime_cache,
        Some(consumer.join(".pixi-sandbox/cargo-home")),
    );
    assert_success(&root_build, "root cargo build --offline with egress denied");
    let root_run = run_egress_denied(
        &consumer.join("target/debug/offline-root"),
        &[],
        &consumer,
        &home,
        &temporary_dir,
        &runtime_cache,
        None,
    );
    assert_success(&root_run, "run the root binary");
    assert!(
        String::from_utf8_lossy(&root_run.stdout).contains("root: 2026"),
        "the itoa crate really linked: {}",
        String::from_utf8_lossy(&root_run.stdout)
    );

    let nested_build = run_egress_denied(
        &cargo,
        &[
            "build".to_string(),
            "--offline".to_string(),
            "--manifest-path".to_string(),
            "subproject/Cargo.toml".to_string(),
        ],
        &consumer,
        &home,
        &temporary_dir,
        &runtime_cache,
        Some(consumer.join("subproject/.pixi-sandbox/cargo-home")),
    );
    assert_success(
        &nested_build,
        "nested cargo build --offline with egress denied",
    );
    let nested_run = run_egress_denied(
        &consumer.join("subproject/target/debug/offline-subproject"),
        &[],
        &consumer,
        &home,
        &temporary_dir,
        &runtime_cache,
        None,
    );
    assert_success(&nested_run, "run the nested binary");
    assert!(
        String::from_utf8_lossy(&nested_run.stdout).contains("sub: Some(2)"),
        "the memchr crate really linked: {}",
        String::from_utf8_lossy(&nested_run.stdout)
    );

    // Negative control, last: the nested tree removed, a never-used cache and target dir,
    // egress denied. The build must fail — otherwise the success above was cache theatre.
    fs::remove_dir_all(&nested_vendor).expect("remove the nested vendor tree");
    fs::remove_dir_all(consumer.join("subproject/target")).expect("remove nested target");
    let cold_cache = home.join("cold-cache");
    fs::create_dir_all(&cold_cache).expect("fresh cold cache");
    let cold = run_egress_denied(
        &cargo,
        &[
            "build".to_string(),
            "--offline".to_string(),
            "--manifest-path".to_string(),
            "subproject/Cargo.toml".to_string(),
        ],
        &consumer,
        &home,
        &temporary_dir,
        &cold_cache,
        Some(consumer.join("subproject/.pixi-sandbox/cargo-home")),
    );
    assert!(
        !cold.status.success(),
        "an offline build with its vendor tree removed must not silently succeed"
    );
}

const ROOT_MAIN: &str = "fn main() {\n    let mut buffer = itoa::Buffer::new();\n    println!(\"root: {}\", buffer.format(2026u64));\n}\n";
const SUBPROJECT_MAIN: &str =
    "fn main() {\n    println!(\"sub: {:?}\", memchr::memchr(b'x', b\"pixox\"));\n}\n";

fn write_cargo_project(project: &Path, name: &str, main_rs: &str) {
    let dependency = if name == "offline-root" {
        "itoa = \"1\""
    } else {
        "memchr = \"2\""
    };
    fs::write(
        project.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n{dependency}\n"
        ),
    )
    .expect("cargo manifest");
    fs::create_dir_all(project.join("src")).expect("cargo src dir");
    fs::write(project.join("src/main.rs"), main_rs).expect("cargo main");
}

fn write_pixi_project(project: &Path, name: &str, channel_url: &str) {
    let manifest = format!(
        "[workspace]\n\
         name = \"{name}\"\n\
         channels = [\"{channel_url}\"]\n\
         platforms = [\"linux-64\"]\n\n\
         [dependencies]\n\
         native-fixture = \"==1.0\"\n\n\
         [tasks]\n\
         hello = \"cat $CONDA_PREFIX/share/hello.txt\"\n"
    );
    fs::write(project.join("pixi.toml"), manifest).expect("pixi manifest");
}

fn copy_sources(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("consumer src dir");
    for entry in fs::read_dir(source).expect("read source dir") {
        let entry = entry.expect("source entry");
        fs::copy(entry.path(), destination.join(entry.file_name())).expect("copy source file");
    }
}

fn glob_dir_exists(root: &Path, prefix: &str) -> bool {
    fs::read_dir(root).is_ok_and(|entries| {
        entries
            .flatten()
            .any(|entry| entry.file_name().to_string_lossy().starts_with(prefix))
    })
}

/// Connected-side cargo invocation with the test's isolated HOME and cache.
fn cargo_command(cargo: &Path, working_dir: &Path, home: &Path, temporary_dir: &Path) -> Command {
    let mut command = Command::new(cargo);
    command
        .current_dir(working_dir)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("TMPDIR", temporary_dir)
        .env("XDG_CACHE_HOME", home.join(".cache"));
    command
}

/// Same namespace shape as the pixi offline proof: a fresh `unshare -n` network namespace,
/// root dropped back to the calling user, isolated HOME/TMPDIR/cache and an explicit
/// `CARGO_HOME` pointing at the restore-written sandbox cargo-home when given.
fn run_egress_denied(
    program: &Path,
    args: &[String],
    working_dir: &Path,
    home: &Path,
    temporary_dir: &Path,
    cache_home: &Path,
    cargo_home: Option<PathBuf>,
) -> Output {
    let user = env::var("USER").expect("the Linux CI runner sets USER");
    let path = env::var_os("PATH").expect("task PATH");
    let mut command = Command::new("sudo");
    command
        .args(["-n", "unshare", "-n", "--", "sudo", "-n", "-u"])
        .arg(user)
        .args(["--", "/usr/bin/env"])
        .arg(format!("HOME={}", home.display()))
        .arg(format!("USERPROFILE={}", home.display()))
        .arg(format!("TMPDIR={}", temporary_dir.display()))
        .arg(format!("XDG_CACHE_HOME={}", cache_home.display()))
        .arg("PIXI_NO_CONFIG=true");
    if let Some(cargo_home) = cargo_home {
        command.arg(format!("CARGO_HOME={}", cargo_home.display()));
    }
    command
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
