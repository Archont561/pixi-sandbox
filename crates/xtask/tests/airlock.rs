//! `airlock` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use xtask::airlock::*;

#[test]
fn plan_argv_is_config_mode_without_envs_and_override_mode_with_them() {
    assert_eq!(
        plan_argv(
            "target/release/pixi-sandbox",
            ".pixi-sandbox.toml",
            "",
            "",
            "",
            "",
            ""
        ),
        [
            "target/release/pixi-sandbox",
            "plan",
            "--config",
            ".pixi-sandbox.toml",
            "--json"
        ]
    );
    assert_eq!(
        plan_argv("BIN", "C", "default,web", "osx-arm64", "", "", "false"),
        [
            "BIN",
            "plan",
            "--bundle",
            "custom",
            "--envs",
            "default,web",
            "--platform",
            "osx-arm64",
            "--branch-prefix",
            "sandbox",
            "--cargo-vendor",
            "false",
            "--json"
        ]
    );
    // Empty bundle/prefix/cargo-vendor fall back to the same defaults the shell applied.
    assert_eq!(
        plan_argv("BIN", "C", "default", "linux-64", "proof", "ci", ""),
        [
            "BIN",
            "plan",
            "--bundle",
            "proof",
            "--envs",
            "default",
            "--platform",
            "linux-64",
            "--branch-prefix",
            "ci",
            "--cargo-vendor",
            "true",
            "--json"
        ]
    );
}

#[test]
fn a_valid_matrix_passes_through_unchanged_and_an_empty_include_is_an_error() {
    let json = r#"{"schema":1,"include":[{"bundle":"developer"}]}"#;
    assert_eq!(validate_matrix(json).expect("valid"), json);

    for bad in [
        r#"{"schema":1,"include":[]}"#,
        r#"{"schema":1}"#,
        "not json at all",
    ] {
        let error = validate_matrix(bad).expect_err("must be rejected");
        assert!(
            format!("{error:#}").contains("include") || format!("{error:#}").contains("JSON"),
            "must name the problem: {error:#}"
        );
    }
}

#[test]
fn an_override_is_honoured_without_any_api_call() {
    let called = std::sync::atomic::AtomicBool::new(false);
    let (resolved, summary) = resolve_tag(
        "v0.3.7",
        "v0.3.6",
        &|_| {
            called.store(true, std::sync::atomic::Ordering::Relaxed);
            Some("v0.3.7".to_string())
        },
        &|| None,
    )
    .expect("override wins");
    assert_eq!(resolved, "v0.3.6");
    assert_eq!(summary, "proving v0.3.6 (SANDBOX_RELEASE_VERSION override)");
    assert!(!called.load(std::sync::atomic::Ordering::Relaxed));
}

#[test]
fn a_declared_but_unpublished_release_falls_back_to_the_newest_published_one() {
    let (resolved, summary) = resolve_tag(
        "v0.3.8",
        "",
        &|declared| {
            assert_eq!(declared, "v0.3.8");
            None // mid-release: the declared tag has no published assets yet
        },
        &|| Some("v0.3.7".to_string()),
    )
    .expect("fallback");
    assert_eq!(resolved, "v0.3.7");
    assert_eq!(summary, "proving v0.3.7 (declared v0.3.8)");
}

#[test]
fn an_unreachable_releases_api_is_an_error_and_a_garbage_tag_is_refused() {
    let error = resolve_tag("v0.3.8", "", &|_| None, &|| None).expect_err("unreachable");
    assert!(
        format!("{error:#}").contains("could not reach the releases API"),
        "{error:#}"
    );
    for garbage in ["", "latest", "{\"message\":\"Not Found\"}"] {
        let error =
            resolve_tag("v0.3.8", "", &|_| Some(garbage.to_string()), &|| None).expect_err("shape");
        assert!(
            format!("{error:#}").contains("not a vX.Y.Z tag"),
            "{garbage} must be refused: {error:#}"
        );
    }
}

#[test]
fn release_assets_are_selected_from_the_pixi_platform() {
    assert_eq!(
        static_asset_for_platform("linux-64").expect("linux"),
        "pixi-sandbox-x86_64-unknown-linux-musl"
    );
    assert_eq!(
        static_asset_for_platform("osx-arm64").expect("mac"),
        "pixi-sandbox-aarch64-apple-darwin"
    );
    assert!(static_asset_for_platform("freebsd-64").is_err());
}

#[cfg(unix)]
#[test]
fn self_bin_download_uses_the_static_asset_and_marks_it_executable() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let gh = dir.path().join("gh");
    write_executable(
        &gh,
        r#"#!/bin/sh
set -eu
log="$(dirname "$0")/gh.args"
: > "$log"
for arg in "$@"; do
  printf '%s\n' "$arg" >> "$log"
done
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--output" ]; then
shift
out="$1"
  fi
  shift || true
done
printf '#!/bin/sh\nexit 0\n' > "$out"
chmod 600 "$out"
"#,
    );
    let out = dir.path().join("nested/pixi-sandbox");

    airlock_self_bin_with_gh(&gh, "owner/repo", "v0.4.0", "linux-64", &out).expect("download");

    let args = fs::read_to_string(dir.path().join("gh.args")).expect("args");
    assert!(args.contains("release\ndownload\nv0.4.0\n"), "{args}");
    assert!(args.contains("--repo\nowner/repo\n"), "{args}");
    assert!(
        args.contains("--pattern\npixi-sandbox-x86_64-unknown-linux-musl\n"),
        "{args}"
    );
    assert!(args.contains("--clobber\n"), "{args}");
    assert!(out.is_file());
    assert_ne!(
        fs::metadata(&out).expect("metadata").permissions().mode() & 0o111,
        0,
        "downloaded self binary must be executable"
    );
}

#[test]
fn env_lists_split_on_commas_and_empty_names_are_dropped() {
    assert_eq!(
        split_envs("default,web").expect("split"),
        ["default", "web"]
    );
    assert_eq!(
        split_envs(" default , web ").expect("split"),
        ["default", "web"]
    );
    for empty in ["", " , "] {
        assert!(split_envs(empty).is_err(), "'{empty}' names nothing");
    }
}

#[test]
fn pack_argv_carries_the_vendor_flag_only_when_asked() {
    let with = pack_argv(
        "default,web",
        Path::new("/t"),
        true,
        Path::new("/bin/pixi-sandbox"),
    );
    assert!(with.contains(&"--cargo-vendor".to_string()));
    let without = pack_argv(
        "default",
        Path::new("/t"),
        false,
        Path::new("/bin/pixi-sandbox"),
    );
    assert!(!without.contains(&"--cargo-vendor".to_string()));
    for argv in [&with, &without] {
        assert_eq!(
            &argv[..3],
            &[
                "pack".to_string(),
                "--repo-root".to_string(),
                ".".to_string()
            ]
        );
        assert!(argv.contains(&"--fetch-tools".to_string()));
        assert!(argv.contains(&"/bin/pixi-sandbox".to_string()));
    }
}

#[cfg(unix)]
#[test]
fn airlock_pack_installs_named_envs_and_runs_the_static_self_binary() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().join("repo");
    fs::create_dir_all(&repo).expect("repo");
    let pixi = dir.path().join("pixi");
    write_executable(
        &pixi,
        r#"#!/bin/sh
set -eu
printf '%s\n' "$@" >> "$(dirname "$0")/pixi.args"
"#,
    );
    let self_bin = dir.path().join("pixi-sandbox");
    write_executable(
        &self_bin,
        r#"#!/bin/sh
set -eu
: > "$(dirname "$0")/self.args"
for arg in "$@"; do
  printf '%s\n' "$arg" >> "$(dirname "$0")/self.args"
done
"#,
    );
    let out = dir.path().join("transport");
    fs::create_dir_all(&out).expect("out");
    fs::write(out.join("stale"), b"old").expect("stale");

    airlock_pack_with_pixi(&pixi, &repo, "default, web", &out, "false", Some(&self_bin))
        .expect("pack");

    let pixi_args = fs::read_to_string(dir.path().join("pixi.args")).expect("pixi args");
    assert!(
        pixi_args.contains("install\n--frozen\n-e\ndefault\ninstall\n--frozen\n-e\nweb\n"),
        "{pixi_args}"
    );
    assert!(
        !out.join("stale").exists(),
        "packing starts from an empty transport dir"
    );
    let self_args = fs::read_to_string(dir.path().join("self.args")).expect("self args");
    assert!(self_args.contains("pack\n--repo-root\n.\n"), "{self_args}");
    assert!(
        self_args.contains(&format!("--output-dir\n{}\n", out.display())),
        "{self_args}"
    );
    assert!(
        self_args.contains(&format!("--self-bin\n{}\n", self_bin.display())),
        "{self_args}"
    );
    assert!(
        !self_args.contains("--cargo-vendor\n"),
        "false disables cargo vendoring: {self_args}"
    );
}

#[cfg(unix)]
#[test]
fn self_bin_resolution_finds_an_executable_on_the_given_path_and_nothing_else() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("pixi-sandbox");
    fs::write(&bin, b"#!/bin/true\n").expect("write");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("chmod");
    let plain = dir.path().join("not-executable");
    fs::write(&plain, b"payload").expect("write");

    let path_value = std::env::join_paths([dir.path(), Path::new("/nonexistent")])
        .expect("join")
        .to_string_lossy()
        .into_owned();
    assert_eq!(
        resolve_on_path("pixi-sandbox", std::ffi::OsStr::new(&path_value)),
        Some(bin.clone())
    );
    assert_eq!(
        resolve_on_path("not-executable", std::ffi::OsStr::new(&path_value)),
        None,
        "a non-executable file is not a command"
    );
    assert_eq!(
        resolve_on_path("absent", std::ffi::OsStr::new(&path_value)),
        None
    );
}

#[cfg(unix)]
#[test]
fn pixi_global_trampolines_resolve_to_the_real_package_binary() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let exposed = dir.path().join("bin");
    let package = dir.path().join("env/bin");
    fs::create_dir_all(exposed.join("trampoline_configuration")).expect("mkdir");
    fs::create_dir_all(&package).expect("mkdir");
    let shim = exposed.join("pixi-sandbox");
    let real = package.join("pixi-sandbox");
    fs::write(&shim, b"trampoline").expect("write shim");
    fs::write(&real, b"#!/bin/true\n").expect("write real");
    fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).expect("chmod shim");
    fs::set_permissions(&real, fs::Permissions::from_mode(0o755)).expect("chmod real");
    fs::write(
        exposed.join("trampoline_configuration/pixi-sandbox.json"),
        format!(r#"{{"executable":"{}","args":[]}}"#, real.display()),
    )
    .expect("write config");

    assert_eq!(resolve_pixi_trampoline(&shim), real);
}

#[cfg(unix)]
#[test]
fn malformed_or_unhelpful_trampoline_configs_keep_the_original_binary() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let exposed = dir.path().join("bin");
    fs::create_dir_all(exposed.join("trampoline_configuration")).expect("mkdir");
    let shim = exposed.join("pixi-sandbox");
    fs::write(&shim, b"trampoline").expect("write shim");
    fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).expect("chmod shim");

    assert_eq!(resolve_pixi_trampoline(&shim), shim);
    fs::write(
        exposed.join("trampoline_configuration/pixi-sandbox.json"),
        b"not-json",
    )
    .expect("write invalid json");
    assert_eq!(resolve_pixi_trampoline(&shim), shim);
    fs::write(
        exposed.join("trampoline_configuration/pixi-sandbox.json"),
        b"null",
    )
    .expect("write null json");
    assert_eq!(resolve_pixi_trampoline(&shim), shim);
    assert_eq!(resolve_pixi_trampoline(Path::new("/")), PathBuf::from("/"));
}

#[cfg(unix)]
#[test]
fn deny_egress_absolutizes_the_program_before_sudo_sanitizes_path() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let pixi = dir.path().join("pixi");
    fs::write(&pixi, b"#!/bin/true\n").expect("write");
    fs::set_permissions(&pixi, fs::Permissions::from_mode(0o755)).expect("chmod");
    let path_value = std::env::join_paths([dir.path()]).expect("join");

    assert_eq!(
        command_with_absolute_program(&["pixi".into(), "run".into()], &path_value)
            .expect("resolve"),
        [pixi.display().to_string(), "run".to_string()]
    );
    assert_eq!(
        command_with_absolute_program(&["/usr/bin/pixi".into()], &path_value)
            .expect("absolute unchanged"),
        ["/usr/bin/pixi".to_string()]
    );
    assert!(
        command_with_absolute_program(&[], &path_value)
            .expect_err("empty command")
            .to_string()
            .contains("needs a command")
    );
    assert!(
        command_with_absolute_program(&["missing".into()], &path_value)
            .expect_err("missing program")
            .to_string()
            .contains("not on PATH")
    );
}

#[test]
fn egress_denial_wraps_per_os_and_fails_loudly_elsewhere() {
    assert_eq!(
        denial_argv("linux", &["bash".to_string(), "gate.sh".to_string()]).expect("linux"),
        ["sudo", "unshare", "-n", "--", "bash", "gate.sh"]
    );
    assert_eq!(
        denial_argv("macos", &["/bin/bash".to_string(), "gate.sh".to_string()]).expect("macos"),
        [
            "sudo",
            "sandbox-exec",
            "-p",
            DARWIN_EGRESS_PROFILE,
            "/bin/bash",
            "gate.sh"
        ]
    );
    let error = denial_argv("windows", &["cmd".to_string()]).expect_err("unknown os");
    assert!(
        format!("{error:#}").contains("no egress-denial mechanism known for windows"),
        "{error:#}"
    );
}

/// A real fetch through a real bare remote: the point is the developer-shaped sequence
/// (init, remote add, shallow fetch, worktree), so the fixture uses real git.
#[test]
fn the_published_branch_is_fetched_into_a_worktree_the_way_a_developer_machine_would() {
    let dir = tempfile::tempdir().expect("tempdir");
    let remote_path = dir.path().join("remote.git");
    let publish_dir = dir.path().join("payload");
    fs::create_dir_all(&publish_dir).expect("mkdir");
    fs::write(publish_dir.join("blob.txt"), b"transport bytes").expect("write");
    run_git(
        &["init", "-q", "--bare"],
        dir.path(),
        &[&remote_path.to_string_lossy()],
    );
    run_git(&["init", "-q", "-b", "main"], &publish_dir, &[]);
    run_git(&["add", "."], &publish_dir, &[]);
    run_git(&["commit", "-q", "-m", "transport"], &publish_dir, &[]);
    // Remote and refspec are separate arguments: a colon inside one token is a URL
    // spelling, not a refspec, and the push silently targets the wrong thing.
    run_git(
        &["push", "-q"],
        &publish_dir,
        &[
            remote_path.to_string_lossy().as_ref(),
            "HEAD:refs/heads/sandbox/proof-linux-64",
        ],
    );

    let host = dir.path().join("host");
    let worktree = dir.path().join("checkout");
    airlock_fetch(
        &remote_path.to_string_lossy(),
        "sandbox/proof-linux-64",
        &host,
        &worktree,
    )
    .expect("fetch");

    assert!(worktree.join("blob.txt").is_file());
    assert_eq!(
        fs::read(worktree.join("blob.txt")).expect("read"),
        b"transport bytes"
    );
}

#[cfg(unix)]
fn write_executable(path: &Path, contents: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::write(path, contents).expect("write executable");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod executable");
}

fn run_git(args: &[&str], cwd: &Path, extras: &[&str]) {
    let mut command = StdCommand::new("git");
    command.args(args);
    command.args(extras);
    command
        .env("GIT_AUTHOR_NAME", "fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid");
    let out = command.current_dir(cwd).output().expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} {extras:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
