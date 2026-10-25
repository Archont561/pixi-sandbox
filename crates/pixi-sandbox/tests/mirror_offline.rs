//! TASK-137 / ADR-34 spike: two cargo projects with distinct `Cargo.lock` files resolve and
//! build offline against ONE shared Cargo local registry, with egress denied.
//!
//! This is the falsifiable experiment behind the mirror proposal: ADR-33 gives each lockfile
//! its own vendor tree, so a crate both projects use (`same-file` here) is stored, checked out
//! and verified twice. The mirror under test stores every `(name, version)` once and serves
//! both locks through source replacement. The connected side needs crates.io exactly twice
//! (`generate-lockfile` + `fetch` per project); everything after runs inside a fresh Linux
//! network namespace with an isolated HOME and a config-only CARGO_HOME that has never held a
//! crate:
//!
//! 1. Both projects `cargo build --offline --locked` against the same mirror directory.
//! 2. The built binaries run and prove the registry crates really linked.
//! 3. A cold negative control — one `.crate` removed, fresh CARGO_HOME and target, egress
//!    denied — must fail, proving the builds read the mirror rather than a warm cache.
//! 4. A measurement block compares mirror bytes against what two ADR-33 `cargo vendor` trees
//!    would carry for the same two locks.
//!
//! Deliberately ignored locally: it needs crates.io plus a `sudo unshare` network namespace,
//! which are Linux CI assets. Executed by CI's `test-mirror-offline` gate. The spike has no
//! transport integration: unlike `vendor_offline.rs`, nothing is packed or restored here.

#![cfg(target_os = "linux")]

use pixi_sandbox::mirror::{MirrorProject, build_local_registry, cargo_home_in};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::tempdir;

#[test]
#[ignore = "needs crates.io at fetch time plus a sudo unshare network namespace; CI's mirror offline gate"]
#[allow(clippy::too_many_lines)]
fn two_lockfiles_share_one_local_registry_and_build_offline() {
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
            eprintln!("::error title=mirror-offline::{message}");
        }));
    }

    let scratch = tempdir().expect("test tempdir");
    let home = scratch.path().join("home");
    let temporary_dir = scratch.path().join("tmp");
    let fetch_home = scratch.path().join("fetch-cargo-home");
    let repo = scratch.path().join("repo");
    let vendor_root = scratch.path().join("vendor-root");
    let vendor_sub = scratch.path().join("vendor-sub");
    let mirror_dir = scratch.path().join("mirror");
    fs::create_dir_all(&home).expect("isolated home");
    fs::create_dir_all(&temporary_dir).expect("isolated tmp");
    fs::create_dir_all(&fetch_home).expect("fetch cargo home");
    fs::create_dir_all(repo.join("subproject")).expect("nested project dir");

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

    write_cargo_project(&repo, "mirror-root", "walkdir = \"2\"", ROOT_MAIN);
    write_cargo_project(
        &repo.join("subproject"),
        "mirror-sub",
        "memchr = \"2\"\nsame-file = \"1\"",
        SUBPROJECT_MAIN,
    );

    let cargo = executable_on_path("cargo").expect("the pixi task environment provides cargo");

    // Connected side: real locks against crates.io, then fetch so the .crate files the mirror
    // copies are actually in the isolated CARGO_HOME cache.
    for project in [&repo, &repo.join("subproject")] {
        assert_success(
            &cargo_command(&cargo, project, &fetch_home, &home, &temporary_dir)
                .args(["generate-lockfile"])
                .output()
                .expect("run cargo generate-lockfile"),
            "connected cargo generate-lockfile",
        );
        assert_success(
            &cargo_command(&cargo, project, &fetch_home, &home, &temporary_dir)
                .args(["fetch"])
                .output()
                .expect("run cargo fetch"),
            "connected cargo fetch",
        );
    }

    // The ADR-33 baseline for the measurement block: one vendor tree per lockfile.
    assert_success(
        &cargo_command(&cargo, &repo, &fetch_home, &home, &temporary_dir)
            .args(["vendor"])
            .arg(&vendor_root)
            .output()
            .expect("root cargo vendor"),
        "root cargo vendor baseline",
    );
    assert_success(
        &cargo_command(
            &cargo,
            &repo.join("subproject"),
            &fetch_home,
            &home,
            &temporary_dir,
        )
        .args(["vendor"])
        .arg(&vendor_sub)
        .output()
        .expect("sub cargo vendor"),
        "sub cargo vendor baseline",
    );

    let root_lock = repo.join("Cargo.lock");
    let sub_lock = repo.join("subproject/Cargo.lock");
    let report = build_local_registry(
        &[
            MirrorProject {
                label: "root",
                cargo_lock: &root_lock,
            },
            MirrorProject {
                label: "subproject",
                cargo_lock: &sub_lock,
            },
        ],
        &fetch_home,
        &mirror_dir,
    )
    .expect("build the local registry from both locks");

    // walkdir pulls same-file (the resolved lock also lists Windows-only winapi even on
    // Linux); the subproject declares same-file directly, so at minimum same-file is a
    // shared pair. Assert the structural invariants rather than exact upstream crate counts.
    assert_eq!(report.projects, 2);
    assert_eq!(report.per_lock.len(), 2);
    assert_eq!(report.per_lock[0].0, "root");
    assert_eq!(report.per_lock[1].0, "subproject");
    assert!(
        report.per_lock[0].1 >= 2,
        "the root lock holds walkdir plus its dependencies"
    );
    assert!(
        report.per_lock[1].1 >= 2,
        "the sub lock holds memchr and same-file"
    );
    assert_eq!(report.index_entries, report.unique_crates);
    assert_eq!(
        report.lock_packages - report.unique_crates,
        report.duplicated_pairs,
        "every repeated lock entry is a pair the mirror stores once"
    );
    assert!(
        report.duplicated_pairs >= 1,
        "same-file is declared by both projects and must be stored once"
    );
    let walkdir_index = fs::read_to_string(mirror_dir.join("index/wa/lk/walkdir"))
        .expect("the walkdir index entry exists");
    assert!(
        walkdir_index.contains("\"name\":\"same-file\""),
        "index deps are rendered from the crate manifest: {walkdir_index}"
    );
    assert!(
        glob_crate_exists(&mirror_dir, "same-file-"),
        "the shared same-file crate exists once in the mirror"
    );

    // The builder's CARGO_HOME contains config.toml only — never a fetched crate — so an
    // offline build can only succeed by reading the mirror's .crate files.
    let mirror_home = cargo_home_in(&mirror_dir);
    let config = fs::read_to_string(mirror_home.join("config.toml")).expect("mirror config");
    assert!(config.contains("replace-with = \"pixi-sandbox-mirror\""));
    assert!(config.contains("local-registry ="));
    let runtime_cache = home.join("runtime-cache");
    fs::create_dir_all(&runtime_cache).expect("fresh runtime cache");

    let root_build = run_egress_denied(
        &cargo,
        &[
            "build".to_string(),
            "--offline".to_string(),
            "--locked".to_string(),
        ],
        &repo,
        &home,
        &temporary_dir,
        &runtime_cache,
        Some(mirror_home.clone()),
    );
    assert_success(
        &root_build,
        "root cargo build --offline against the shared mirror",
    );
    let root_run = run_egress_denied(
        &repo.join("target/debug/mirror-root"),
        &[],
        &repo,
        &home,
        &temporary_dir,
        &runtime_cache,
        None,
    );
    assert_success(&root_run, "run the root binary");
    assert!(
        String::from_utf8_lossy(&root_run.stdout).contains("root: walked"),
        "the walkdir crate really linked: {}",
        String::from_utf8_lossy(&root_run.stdout)
    );

    let sub_build = run_egress_denied(
        &cargo,
        &[
            "build".to_string(),
            "--offline".to_string(),
            "--locked".to_string(),
            "--manifest-path".to_string(),
            "subproject/Cargo.toml".to_string(),
        ],
        &repo,
        &home,
        &temporary_dir,
        &runtime_cache,
        Some(mirror_home),
    );
    assert_success(
        &sub_build,
        "subproject cargo build --offline against the same shared mirror",
    );
    let sub_run = run_egress_denied(
        &repo.join("subproject/target/debug/mirror-sub"),
        &[],
        &repo.join("subproject"),
        &home,
        &temporary_dir,
        &runtime_cache,
        None,
    );
    assert_success(&sub_run, "run the subproject binary");
    assert!(
        String::from_utf8_lossy(&sub_run.stdout).contains("sub: Some(2) true"),
        "memchr and same-file really linked: {}",
        String::from_utf8_lossy(&sub_run.stdout)
    );

    // Measurement block for the ADR-34 ledger (--nocapture surfaces it in the CI log).
    let vendor_bytes = directory_size(&vendor_root) + directory_size(&vendor_sub);
    println!(
        "mirror-spike measurement: lock_entries={} unique_crates={} duplicated_pairs={} \
         mirror_bytes={} adr33_two_vendor_trees_bytes={} vendor_minus_mirror={}",
        report.lock_packages,
        report.unique_crates,
        report.duplicated_pairs,
        report.mirror_bytes,
        vendor_bytes,
        vendor_bytes.saturating_sub(report.mirror_bytes)
    );

    // Negative control, last: the memchr crate removed, a never-used CARGO_HOME and target,
    // egress denied. The build must fail — otherwise the success above was cache theatre.
    let memchr_crate = glob_crate_path(&mirror_dir, "memchr-")
        .expect("the mirror carries the memchr crate before the negative control");
    fs::remove_file(memchr_crate).expect("remove the memchr crate from the mirror");
    fs::remove_dir_all(repo.join("subproject/target")).expect("remove sub target");
    let cold_home = scratch.path().join("cold-cargo-home");
    fs::create_dir_all(&cold_home).expect("fresh cold cargo home");
    fs::create_dir_all(home.join("cold-cache")).expect("fresh cold xdg cache");
    fs::copy(
        cargo_home_in(&mirror_dir).join("config.toml"),
        cold_home.join("config.toml"),
    )
    .expect("carry only the source-replacement config into the cold home");
    let cold = run_egress_denied(
        &cargo,
        &[
            "build".to_string(),
            "--offline".to_string(),
            "--locked".to_string(),
            "--manifest-path".to_string(),
            "subproject/Cargo.toml".to_string(),
        ],
        &repo,
        &home,
        &temporary_dir,
        &home.join("cold-cache"),
        Some(cold_home),
    );
    assert!(
        !cold.status.success(),
        "an offline build with its mirror crate removed must not silently succeed"
    );
}

const ROOT_MAIN: &str = "fn main() {\n    let entries = walkdir::WalkDir::new(\".\").into_iter().count();\n    \
     println!(\"root: walked {entries}\");\n}\n";
const SUBPROJECT_MAIN: &str = "fn main() {\n    let at = memchr::memchr(b'x', b\"pixox\");\n    let \
                              manifest = same_file::is_file(\"Cargo.toml\");\n    println!(\"sub: \
                              {at:?} {manifest}\");\n}\n";

fn write_cargo_project(project: &Path, name: &str, dependencies: &str, main_rs: &str) {
    fs::write(
        project.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
             [dependencies]\n{dependencies}\n"
        ),
    )
    .expect("cargo manifest");
    fs::create_dir_all(project.join("src")).expect("cargo src dir");
    fs::write(project.join("src/main.rs"), main_rs).expect("cargo main");
}

fn cargo_command(
    cargo: &Path,
    working_dir: &Path,
    cargo_home: &Path,
    home: &Path,
    temporary_dir: &Path,
) -> Command {
    let mut command = Command::new(cargo);
    command
        .current_dir(working_dir)
        .env("CARGO_HOME", cargo_home)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("TMPDIR", temporary_dir)
        .env("XDG_CACHE_HOME", home.join(".cache"));
    command
}

/// Same namespace shape as the vendor offline proof: a fresh `unshare -n` network namespace,
/// root dropped back to the calling user, isolated HOME/TMPDIR/cache and an explicit
/// CARGO_HOME when given.
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

fn directory_size(root: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(metadata) = entry.metadata() {
                total += metadata.len();
            }
        }
    }
    total
}

fn glob_crate_path(root: &Path, prefix: &str) -> Option<PathBuf> {
    fs::read_dir(root).ok()?.flatten().find_map(|entry| {
        let path = entry.path();
        path.file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(prefix))
            .then_some(path)
    })
}

fn glob_crate_exists(root: &Path, prefix: &str) -> bool {
    glob_crate_path(root, prefix).is_some()
}
