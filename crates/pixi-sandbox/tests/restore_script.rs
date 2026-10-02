//! `scripts/restore.sh` — the repository's own airlock bootstrap — under test.
//!
//! Until now nothing executed this script. `tests/cli.rs` and `tests/user_tools.rs` cover the
//! `restore.sh` that `init` *generates* into a user's project (a different file, rendered from
//! a template), and `xtask check-repository` only lints this one's Bash 3.2 surface. The two
//! defects fixed on 2026-10-01 — a report that announced a user-tool registration that never
//! happened, and a `--version` call against a worktree that had already been removed — were
//! both found by hand, after the script had been green in CI for weeks. A static lint cannot
//! see either: both are behaviour.
//!
//! So these tests run the real script, the way an airlocked developer runs it:
//!
//! * a git repository whose branch carries the committed transport fixture, so a restore is a
//!   real restore (same payload `e2e.rs` uses — no packer, no pixi, no network);
//! * the bundled `pixi-sandbox` in that branch is a `#!` shim around the binary this test
//!   built, because the fixture's own stub cannot restore anything. The manifest's declared
//!   size follows the shim, and a shim is `linkage = "script"`, so the airlock's
//!   dynamically-linked-tool rule is satisfied honestly rather than bypassed;
//! * `HOME` and `TMPDIR` point into the test's tempdir, so a test that would touch the
//!   developer's home fails instead of succeeding quietly (D10, user level).
//!
//! The reach out of this crate to `../../scripts/restore.sh` is deliberate and is one of the
//! two exceptions recorded in `tests/fixtures.rs`: the script *is* the artifact under test, and
//! a copy of it would prove nothing about the script a developer runs.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The script under test, in this repository (see the module comment and `tests/fixtures.rs`).
fn restore_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/restore.sh")
}

fn fixture_transport() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/transport")
}

/// The same host → Pixi platform table the script derives with `uname`, stated independently
/// here. `the_script_derives_the_same_platform_name_rust_does` holds the two sides together:
/// the script's comment claims the mapping matches the generated launchers', and a claim in a
/// comment is not a guarantee.
fn host_platform() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-64",
        ("linux", "aarch64") => "linux-aarch64",
        ("macos", "aarch64") => "osx-arm64",
        ("macos", "x86_64") => "osx-64",
        (os, arch) => panic!("unsupported test host: {os}-{arch}"),
    }
}

/// `bash scripts/restore.sh …`, with every inherited `PIXI_SANDBOX_*` variable cleared and
/// `HOME`/`TMPDIR` inside the test's own tempdir. A developer running the suite must not have
/// their environment decide what the script does, and the script must not be able to reach
/// their home even when a test is wrong.
fn script_in(cwd: &Path, home: &Path) -> Command {
    let mut command = Command::new("bash");
    command
        .arg(restore_script())
        .current_dir(cwd)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("SHELL", "/usr/bin/bash")
        .env("TMPDIR", home.join("tmp"))
        .env_remove("PIXI_SANDBOX_BRANCH")
        .env_remove("PIXI_SANDBOX_BUNDLE")
        .env_remove("PIXI_SANDBOX_CONFIG")
        .env_remove("PIXI_SANDBOX_USER_TOOLS")
        .env_remove("PIXI_SANDBOX_USER_BIN");
    fs::create_dir_all(home.join("tmp")).unwrap();
    command
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

// ---------------------------------------------------------------------------------------
// Branch resolution: `--print-branch` reads the reviewed publish plan and touches nothing.
// ---------------------------------------------------------------------------------------

/// Resolve a branch against `config`, with no git repository, no network and no payload.
fn print_branch(config: &str, args: &[&str], env: &[(&str, &str)]) -> (Option<i32>, String) {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join(".pixi-sandbox.toml");
    fs::write(&config_path, config).unwrap();

    let mut command = script_in(temp.path(), temp.path());
    command.arg("--print-branch").args(args);
    command.env("PIXI_SANDBOX_CONFIG", &config_path);
    for (key, value) in env {
        command.env(key, value);
    }
    let output = command.output().unwrap();
    (output.status.code(), combined(&output))
}

/// One bundle for this host: the branch is `<branch_prefix>/<bundle>-<platform>`, and the
/// prefix defaults to `sandbox` exactly as `sandbox_config.rs` defaults it.
#[test]
fn print_branch_derives_the_branch_declared_for_this_host() {
    let platform = host_platform();

    let (code, out) = print_branch(
        &format!("schema = 1\n[[bundle]]\nname = \"developer\"\nplatforms = [\"{platform}\"]\n"),
        &[],
        &[],
    );
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(out.trim(), format!("sandbox/developer-{platform}"));

    let (code, out) = print_branch(
        &format!(
            "schema = 1\nbranch_prefix = \"xport\"\n[[bundle]]\nname = \"developer\"\nplatforms = [\"{platform}\"]\n"
        ),
        &[],
        &[],
    );
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(out.trim(), format!("xport/developer-{platform}"));
}

/// The config is parsed with `sed`, not a TOML parser (the binary that could parse it is the
/// thing being unpacked), so the shapes a human actually writes have to keep working: an array
/// spread over several lines, a trailing comma, a comment after a value.
#[test]
fn print_branch_reads_a_config_written_the_way_people_write_toml() {
    let platform = host_platform();
    let config = format!(
        "# the reviewed publish plan\n\
         schema = 1  # schema 1 branch naming\n\
         branch_prefix = \"sandbox\"  # prefix for every orphan branch\n\
         cargo_vendor = true\n\
         \n\
         [[bundle]]\n\
         name = \"developer\"  # the only bundle\n\
         environments = [\"default\", \"web\"]\n\
         platforms = [\n  \"win-64\",\n  \"{platform}\",\n]\n"
    );
    let (code, out) = print_branch(&config, &[], &[]);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(out.trim(), format!("sandbox/developer-{platform}"));
}

/// The script's `uname` table and Rust's view of the host must name the same platform, or a
/// restore on that host looks for a branch nobody publishes. The config declares every
/// supported platform, so whichever one the script picks is the one it believes it is on.
#[test]
fn the_script_derives_the_same_platform_name_rust_does() {
    let (code, out) = print_branch(
        "schema = 1\n[[bundle]]\nname = \"all\"\nplatforms = [\"linux-64\", \"linux-aarch64\", \"osx-arm64\", \"osx-64\", \"win-64\"]\n",
        &[],
        &[],
    );
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(out.trim(), format!("sandbox/all-{}", host_platform()));
}

/// Precedence, in the order the usage block promises: an explicit argument beats the
/// environment, the environment beats the config, and `auto` forces derivation even when the
/// environment is set (the documented escape hatch).
#[test]
fn an_explicit_branch_beats_the_environment_which_beats_the_config() {
    let platform = host_platform();
    let config =
        format!("schema = 1\n[[bundle]]\nname = \"developer\"\nplatforms = [\"{platform}\"]\n");

    let (code, out) = print_branch(&config, &["given/branch"], &[]);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(out.trim(), "given/branch");

    let (code, out) = print_branch(&config, &[], &[("PIXI_SANDBOX_BRANCH", "env/branch")]);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(out.trim(), "env/branch");

    let (code, out) = print_branch(&config, &["auto"], &[("PIXI_SANDBOX_BRANCH", "env/branch")]);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(out.trim(), format!("sandbox/developer-{platform}"));
}

/// Several bundles publish this platform: the script refuses to guess, names them, and says
/// how to choose. `PIXI_SANDBOX_BUNDLE` then selects one, and an unknown name is an error that
/// lists what is declared rather than a branch nobody will ever be able to fetch.
#[test]
fn an_ambiguous_platform_must_be_disambiguated_by_the_operator() {
    let platform = host_platform();
    let config = format!(
        "schema = 1\n[[bundle]]\nname = \"developer\"\nplatforms = [\"{platform}\"]\n[[bundle]]\nname = \"minimal\"\nplatforms = [\"{platform}\"]\n"
    );

    let (code, out) = print_branch(&config, &[], &[]);
    assert_eq!(code, Some(2), "{out}");
    assert!(out.contains("declares several bundles"), "{out}");
    assert!(out.contains("developer minimal"), "{out}");
    assert!(out.contains("PIXI_SANDBOX_BUNDLE=<name>"), "{out}");

    let (code, out) = print_branch(&config, &[], &[("PIXI_SANDBOX_BUNDLE", "minimal")]);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(out.trim(), format!("sandbox/minimal-{platform}"));

    let (code, out) = print_branch(&config, &[], &[("PIXI_SANDBOX_BUNDLE", "ghost")]);
    assert_eq!(code, Some(2), "{out}");
    assert!(out.contains("has no bundle 'ghost'"), "{out}");
    assert!(
        out.contains(&format!("sandbox/developer-{platform}")),
        "the error must list what *is* declared: {out}"
    );
}

/// Every way the plan can fail to name a branch for this host ends in exit 2 with a remedy —
/// never in a guessed branch name, because a guess becomes a confusing `cannot fetch` three
/// steps later.
#[test]
fn a_config_that_cannot_name_a_branch_fails_with_a_remedy() {
    let platform = host_platform();
    let other = if platform == "win-64" {
        "linux-64"
    } else {
        "win-64"
    };

    let (code, out) = print_branch(
        &format!("schema = 1\n[[bundle]]\nname = \"developer\"\nplatforms = [\"{other}\"]\n"),
        &[],
        &[],
    );
    assert_eq!(code, Some(2), "{out}");
    assert!(
        out.contains(&format!("publishes nothing for this host ({platform})")),
        "{out}"
    );
    assert!(
        out.contains(&format!("add {platform} to a [[bundle]]")),
        "{out}"
    );

    let (code, out) = print_branch(
        &format!("schema = 2\n[[bundle]]\nname = \"developer\"\nplatforms = [\"{platform}\"]\n"),
        &[],
        &[],
    );
    assert_eq!(code, Some(2), "{out}");
    assert!(out.contains("declares schema 2"), "{out}");
    assert!(out.contains("pass the branch explicitly"), "{out}");

    let (code, out) = print_branch("schema = 1\nbranch_prefix = \"sandbox\"\n", &[], &[]);
    assert_eq!(code, Some(2), "{out}");
    assert!(out.contains("declares no [[bundle]] targets"), "{out}");
}

/// No config at all: the remedy is the one-liner that does not need one.
#[test]
fn a_missing_config_says_how_to_restore_without_one() {
    let temp = tempfile::tempdir().unwrap();
    let output = script_in(temp.path(), temp.path())
        .arg("--print-branch")
        .env("PIXI_SANDBOX_CONFIG", temp.path().join("absent.toml"))
        .output()
        .unwrap();
    let out = combined(&output);
    assert_eq!(output.status.code(), Some(2), "{out}");
    assert!(out.contains("to derive the branch from"), "{out}");
    assert!(out.contains("bash scripts/restore.sh <branch>"), "{out}");
}

/// The branch name is interpolated straight into `git fetch` and `git worktree add`. Anything
/// that is not a plain ref is refused before it reaches git — including a leading `-`, which
/// git would read as an option.
#[test]
fn a_branch_name_that_is_not_ref_safe_is_refused() {
    let config = format!(
        "schema = 1\n[[bundle]]\nname = \"developer\"\nplatforms = [\"{}\"]\n",
        host_platform()
    );
    for unsafe_name in [
        "../evil",
        "with space",
        "colon:ref",
        "tilde~1",
        "caret^2",
        "reflog@{1}",
        "-rf",
        "glob*",
    ] {
        let (code, out) = print_branch(&config, &[unsafe_name], &[]);
        assert_eq!(code, Some(2), "{unsafe_name} was not refused: {out}");
        assert!(
            out.contains(&format!("refusing unsafe branch name: {unsafe_name}")),
            "{out}"
        );
    }
}

/// `--print-branch` is the dry run: it resolves and exits. Nothing is fetched, no worktree is
/// added, nothing is written — which is what makes it safe to call from a prompt or a hook.
#[test]
fn print_branch_writes_nothing_and_touches_no_git() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let cwd = temp.path().join("cwd");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&cwd).unwrap();
    let config = cwd.join(".pixi-sandbox.toml");
    fs::write(
        &config,
        format!(
            "schema = 1\n[[bundle]]\nname = \"developer\"\nplatforms = [\"{}\"]\n",
            host_platform()
        ),
    )
    .unwrap();

    let output = script_in(&cwd, &home)
        .arg("--print-branch")
        .env("PIXI_SANDBOX_CONFIG", &config)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", combined(&output));

    let mut left = fs::read_dir(&cwd)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    left.sort();
    assert_eq!(left, [".pixi-sandbox.toml"], "print mode wrote something");
    assert!(
        !home.join(".local").exists() && !home.join(".profile").exists(),
        "print mode touched HOME"
    );
    assert!(
        fs::read_dir(home.join("tmp")).unwrap().next().is_none(),
        "print mode left a worktree behind"
    );
}

/// `--help` answers without a config, a repository or a network — the first thing anyone types.
#[test]
fn help_prints_the_usage_block() {
    let temp = tempfile::tempdir().unwrap();
    let output = script_in(temp.path(), temp.path())
        .arg("--help")
        .output()
        .unwrap();
    let out = combined(&output);
    assert!(output.status.success(), "{out}");
    assert!(
        out.contains("Usage: bash scripts/restore.sh [branch] [output-path]"),
        "{out}"
    );
    assert!(out.contains("PIXI_SANDBOX_USER_TOOLS"), "{out}");
}

// ---------------------------------------------------------------------------------------
// The restore itself. Gated to linux-64: the fixture transport and its manifest are linux-64,
// the same gate `user_tools.rs` and the severed-network proof use.
// ---------------------------------------------------------------------------------------

/// Which `pixi-sandbox` the branch carries. Both are `#!` shims around the binary under test —
/// the difference is what they do with the registration policy the script exports.
#[derive(Clone, Copy)]
enum Bundled {
    /// 0.3.7 or newer: honours `PIXI_SANDBOX_USER_TOOLS`.
    Current,
    /// A branch packed before 0.3.7: no `--user-tools` at all, so the environment variable is
    /// ignored and nothing is registered. This is the shape that made the old report lie.
    PreUserTools,
}

/// A git repository whose `branch` holds the fixture transport, with `bundled` standing in for
/// the packed bootstrap binary. Returns the repository root — the developer's checkout, whose
/// working tree is *not* the payload (the payload lives only on the branch, as it does in
/// production).
fn transport_repo(root: &Path, branch: &str, bundled: Bundled) -> PathBuf {
    fs::create_dir_all(root).unwrap();
    copy_tree(&fixture_transport(), root);

    let real = assert_cmd::cargo::cargo_bin("pixi-sandbox");
    let tool = root.join(".pixi-sandbox/tools/linux-64/pixi-sandbox");
    let shim = match bundled {
        Bundled::Current => format!("#!/bin/sh\nexec \"{}\" \"$@\"\n", real.display()),
        Bundled::PreUserTools => format!(
            r#"#!/bin/sh
# Plays a bootstrap packed before 0.3.7: it reports that version, and because it has no
# --user-tools it registers nothing however loudly the caller asks (an unknown environment
# variable is ignored, which is the whole reason the policy travels as one).
if [ "$1" = "--version" ]; then echo "pixi-sandbox 0.3.6"; exit 0; fi
case " $* " in
  *" restore "*) exec "{real}" "$@" --user-tools skip ;;
  *) exec "{real}" "$@" ;;
esac
"#,
            real = real.display()
        ),
    };
    fs::write(&tool, shim).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();

    // The manifest declares each tool's size, and `verify` checks it before anything is
    // written: the shim is a different size from the fixture's stub, so the manifest has to
    // follow it. Everything else about the payload stays exactly as committed.
    let manifest_path = root.join(".pixi-sandbox/manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["tools"]["pixi-sandbox"]["size_bytes"] =
        serde_json::json!(fs::metadata(&tool).unwrap().len());
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    git(root, &["init", "-q", "-b", "main"]);
    git(root, &["add", "."]);
    commit(root, "payload");
    git(root, &["branch", branch]);

    // Back to a checkout that looks like a project, not a payload.
    git(root, &["rm", "-qrf", "."]);
    fs::write(root.join("README.md"), "the developer's checkout\n").unwrap();
    git(root, &["add", "."]);
    commit(root, "main");

    root.to_path_buf()
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

fn commit(dir: &Path, message: &str) {
    git(
        dir,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            message,
        ],
    );
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap().flatten() {
        let from = entry.path();
        let to = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&from, &to);
        } else {
            fs::copy(&from, &to).unwrap();
            fs::set_permissions(&to, fs::metadata(&from).unwrap().permissions()).unwrap();
        }
    }
}

/// A restored project, as the script leaves it.
struct Restored {
    output: Output,
    log: String,
    home: PathBuf,
    project: PathBuf,
}

fn restore(bundled: Bundled, policy: Option<&str>) -> (tempfile::TempDir, Restored) {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let project = temp.path().join("project");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&project).unwrap();
    let repo = transport_repo(&temp.path().join("repo"), "sandbox/demo-linux-64", bundled);

    let mut command = script_in(&repo, &home);
    command.arg("sandbox/demo-linux-64").arg(&project);
    if let Some(policy) = policy {
        command.env("PIXI_SANDBOX_USER_TOOLS", policy);
    }
    let output = command.output().unwrap();
    let log = combined(&output);
    (
        temp,
        Restored {
            output,
            log,
            home,
            project,
        },
    )
}

/// The whole sequence, end to end: derive nothing (the branch is explicit), find the branch
/// *locally* and do not reach for the network, add a worktree, verify the payload, restore it,
/// verify the tree that came out, remove the worktree, and report the pixi entrypoint the
/// user should run next.
#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn a_local_branch_restores_without_touching_the_network() {
    let (temp, restored) = restore(Bundled::Current, Some("register"));
    let log = &restored.log;
    assert!(restored.output.status.success(), "{log}");

    // The airlock invariant, as an assertion: objects that are already local are never
    // re-fetched. A fetch here would mean a disconnected host cannot restore.
    assert!(
        log.contains("→ using local sandbox/demo-linux-64 (no fetch needed)"),
        "{log}"
    );
    assert!(!log.contains("→ fetching"), "{log}");

    // Verified before anything was written, and again after it was written (D13).
    assert!(
        log.contains("verify    OK — every declared byte matches the manifest"),
        "{log}"
    );
    assert!(
        log.contains("OK — the restored tree matches the manifest"),
        "{log}"
    );

    // A real project came out, not just a log that says so.
    assert!(!restored.project.join(".pixi/sandbox-env.sh").exists());
    assert!(
        restored
            .project
            .join(".pixi/envs/demo/conda-meta/pixi_env_prefix")
            .is_file()
    );
    assert!(restored.project.join(".cargo/config.toml").is_file());

    // The worktree is scratch, and scratch is cleaned up — including from git's own list, or
    // the next restore trips over a stale entry.
    assert!(log.contains("→ cleanup worktree"), "{log}");
    assert!(
        fs::read_dir(restored.home.join("tmp"))
            .unwrap()
            .next()
            .is_none(),
        "the worktree survived the run"
    );
    let worktrees = Command::new("git")
        .args(["worktree", "list"])
        .current_dir(temp.path().join("repo"))
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&worktrees.stdout).lines().count(),
        1,
        "a worktree is still registered: {}",
        String::from_utf8_lossy(&worktrees.stdout)
    );

    // The script no longer sources an activation hook. It tells the operator how to put the
    // registered pixi launcher on this already-running shell's PATH, and keeps `pixi run` as
    // the only supported entrypoint.
    assert!(log.contains("current shell: export PATH="), "{log}");
    assert!(
        log.contains("then use pixi as the only entrypoint: pixi run --frozen <task>"),
        "{log}"
    );

    // And the registration report is true: it says registered, and the launchers are there.
    let user_bin = restored.home.join(".local/bin");
    assert!(
        log.contains(&format!(
            "user tools: registered pixi and pixi-sandbox in {} for new shells",
            user_bin.display()
        )),
        "{log}"
    );
    let launcher = fs::read_to_string(user_bin.join("pixi")).unwrap();
    assert!(
        launcher.contains("managed by pixi-sandbox"),
        "the script proves registration by grepping for this marker; without it the report \
         goes back to guessing: {launcher}"
    );
    assert!(user_bin.join("pixi-sandbox").is_file());
}

/// The regression the honest-reporting fix was about: a branch packed before 0.3.7 carries a
/// binary that ignores `PIXI_SANDBOX_USER_TOOLS`, registers nothing, and used to be announced
/// as a success. The script must report what happened, name the version that caused it, and
/// still exit 0 — the restore itself worked.
#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn a_bootstrap_that_ignores_the_policy_is_reported_not_announced() {
    let (_temp, restored) = restore(Bundled::PreUserTools, Some("register"));
    let log = &restored.log;
    assert!(restored.output.status.success(), "{log}");

    assert!(
        log.contains(
            "user tools: NOT registered — the bundled pixi-sandbox 0.3.6 predates --user-tools (0.3.7),"
        ),
        "the report must name the version that ignored the request: {log}"
    );
    assert!(
        log.contains("no longer sources or supports"),
        "it must also say what to do instead: {log}"
    );
    assert!(
        !log.contains("user tools: registered pixi"),
        "it must not claim a registration that did not happen: {log}"
    );

    // Nothing was registered — the report is a fact about HOME, not a hope.
    assert!(
        !restored.home.join(".local").exists(),
        "HOME was written to"
    );
    // …and the restore still succeeded, which is why this is a notice and not a failure.
    assert!(!restored.project.join(".pixi/sandbox-env.sh").exists());

    // The version came from the *restored* copy: the branch worktree is gone by the time the
    // report runs, and asking the removed path printed "unknown version" before the fix.
    assert!(!log.contains("unknown version"), "{log}");
}

/// `skip` is the CI and shared-account path: the tools work in this shell and HOME is never
/// touched. The script says both halves, because the second one is the surprise.
#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn the_skip_policy_leaves_home_alone_and_says_so() {
    let (_temp, restored) = restore(Bundled::Current, Some("skip"));
    let log = &restored.log;
    assert!(restored.output.status.success(), "{log}");

    assert!(
        log.contains("user tools: not registered (PIXI_SANDBOX_USER_TOOLS=skip)"),
        "{log}"
    );
    assert!(
        log.contains("current shell unchanged"),
        "the consequence has to be spelled out: {log}"
    );
    assert!(
        log.contains("run --frozen <task> explicitly"),
        "the pixi-only fallback has to be spelled out: {log}"
    );
    assert!(
        !restored.home.join(".local").exists() && !restored.home.join(".profile").exists(),
        "skip must not create anything under HOME"
    );
    assert!(restored.project.join(".pixi/envs/demo").is_dir());
}

/// The connected case: the branch is only on `origin`, so the script fetches it — here from a
/// second repository on disk, which exercises the fetch path with no network in sight.
#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn a_branch_only_on_origin_is_fetched_then_restored() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let project = temp.path().join("project");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&project).unwrap();

    let origin = transport_repo(
        &temp.path().join("origin"),
        "sandbox/demo-linux-64",
        Bundled::Current,
    );
    let consumer = temp.path().join("consumer");
    fs::create_dir_all(&consumer).unwrap();
    git(&consumer, &["init", "-q", "-b", "main"]);
    fs::write(consumer.join("README.md"), "a fresh clone\n").unwrap();
    git(&consumer, &["add", "."]);
    commit(&consumer, "main");
    git(
        &consumer,
        &["remote", "add", "origin", origin.to_str().unwrap()],
    );

    let output = script_in(&consumer, &home)
        .arg("sandbox/demo-linux-64")
        .arg(&project)
        .env("PIXI_SANDBOX_USER_TOOLS", "skip")
        .output()
        .unwrap();
    let log = combined(&output);
    assert!(output.status.success(), "{log}");
    assert!(log.contains("→ fetching sandbox/demo-linux-64"), "{log}");
    assert!(
        log.contains("OK — the restored tree matches the manifest"),
        "{log}"
    );
    assert!(!project.join(".pixi/sandbox-env.sh").exists());
}

/// A branch that does not exist: fail with exit 1, and — because the usual cause is a typo or
/// a renamed bundle — list the sandbox branches origin actually has. Nothing is restored, and
/// no worktree is left behind.
#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn a_missing_branch_fails_and_lists_what_origin_has() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let project = temp.path().join("project");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&project).unwrap();

    let origin = transport_repo(
        &temp.path().join("origin"),
        "sandbox/demo-linux-64",
        Bundled::Current,
    );
    let consumer = temp.path().join("consumer");
    fs::create_dir_all(&consumer).unwrap();
    git(&consumer, &["init", "-q", "-b", "main"]);
    fs::write(consumer.join("README.md"), "a fresh clone\n").unwrap();
    git(&consumer, &["add", "."]);
    commit(&consumer, "main");
    git(
        &consumer,
        &["remote", "add", "origin", origin.to_str().unwrap()],
    );

    let output = script_in(&consumer, &home)
        .arg("sandbox/typo-linux-64")
        .arg(&project)
        .output()
        .unwrap();
    let log = combined(&output);
    assert_eq!(output.status.code(), Some(1), "{log}");
    assert!(
        log.contains("::error::cannot fetch sandbox/typo-linux-64 from origin"),
        "{log}"
    );
    assert!(
        log.contains("sandbox/demo-linux-64"),
        "the error must list the branches that do exist: {log}"
    );
    assert!(
        fs::read_dir(&project).unwrap().next().is_none(),
        "a failed restore must leave the output path alone"
    );
    assert!(
        fs::read_dir(home.join("tmp")).unwrap().next().is_none(),
        "a failed restore must not leave a worktree behind"
    );
}
