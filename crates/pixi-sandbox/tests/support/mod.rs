//! Shared integration-test fixtures.
//!
//! This is the only home for the filesystem, process, fixture-payload, and temporary-git setup
//! that more than one integration test needs. Every path starts under a test-owned tempdir and
//! every project payload comes from `tests/fixtures`, never the repository checkout (ADR-10).
//!
//! ## API convention
//!
//! Ask for a zero-argument setup value directly in an `#[rstest]` test (`bin`,
//! `fixture_transport`, `isolated_home`, and `host_platform`). Helpers that need a path or
//! caller-selected argument stay ordinary functions, paired with a `*_fixture` that returns a
//! typed function pointer. This is the rstest-compatible way to expose a parameterised helper:
//! annotating a function such as `copy_tree(source, destination)` directly as a fixture makes
//! rstest interpret those runtime arguments as fixture names. Keep properties bounded, add their
//! committed `.proptest-regressions` file, and use `#[case]` rather than assertion-table loops.

pub mod bdd;

use assert_cmd::Command;
use rstest::fixture;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

#[fixture]
pub fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[fixture]
pub fn bin() -> Command {
    Command::cargo_bin("pixi-sandbox").expect("binary builds")
}

/// An owned home and temporary directory for any restore or launcher test (ADR-10).
#[fixture]
pub fn isolated_home() -> tempfile::TempDir {
    tempfile::tempdir().expect("isolated test home")
}

/// A synthetic, already-packed payload committed for integration tests.
#[fixture]
pub fn fixture_transport() -> PathBuf {
    crate_dir().join("tests/fixtures/transport")
}

/// The complete fixture Pixi project that pack-oriented tests may copy into a tempdir.
#[fixture]
pub fn demo_project() -> PathBuf {
    crate_dir().join("tests/fixtures/demo-project")
}

/// The duplicate-source fixture for the pack-time cargo-vendor rejection.
#[fixture]
pub fn duplicate_source_project() -> PathBuf {
    crate_dir().join("tests/fixtures/duplicate-source-project")
}

/// task-33: a restore must never observe a real user's home or shell profile.
pub fn isolated_bin(home: &Path) -> Command {
    let temporary = home.join("tmp");
    fs::create_dir_all(&temporary).expect("isolated test temporary directory");

    let mut command = bin();
    command
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("SHELL", "/usr/bin/bash")
        .env("TMPDIR", temporary);
    command
}

/// Copy a fixture out of the checkout before a test writes it, preserving executable bits.
pub fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap().flatten() {
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path).unwrap();
        if metadata.is_dir() {
            copy_tree(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).unwrap();
            fs::set_permissions(&destination_path, metadata.permissions()).unwrap();
        }
    }
}

/// The host's Pixi platform name, matching the generated launchers and `restore.sh`.
#[fixture]
pub fn host_platform() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-64",
        ("linux", "aarch64") => "linux-aarch64",
        ("macos", "aarch64") => "osx-arm64",
        ("macos", "x86_64") => "osx-64",
        ("windows", "x86_64") => "win-64",
        (os, arch) => panic!("unsupported test host {os}-{arch}"),
    }
}

/// Run a git command in a temporary repository and return trimmed stdout.
pub fn git(dir: &Path, args: &[&str]) -> String {
    let output = StdCommand::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} failed in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// The argument order used by the CLI tests: command first, repository second.
pub fn run_git(args: &[&str], cwd: &Path) -> String {
    git(cwd, args)
}

/// Commit a temporary repository with a deterministic test identity.
pub fn commit(dir: &Path, message: &str) {
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

#[cfg(unix)]
pub fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(permissions.mode() | 0o111);
    fs::set_permissions(path, permissions).unwrap();
}

/// Fixtures returning parameterised helpers let tests compose setup declaratively while keeping
/// their call sites explicit about the paths they mutate.
#[fixture]
pub fn isolated_bin_fixture() -> fn(&Path) -> Command {
    isolated_bin
}

#[fixture]
pub fn copy_tree_fixture() -> fn(&Path, &Path) {
    copy_tree
}

#[fixture]
pub fn git_fixture() -> fn(&Path, &[&str]) -> String {
    git
}

#[fixture]
pub fn run_git_fixture() -> fn(&[&str], &Path) -> String {
    run_git
}

#[fixture]
pub fn commit_fixture() -> fn(&Path, &str) {
    commit
}

#[cfg(unix)]
#[fixture]
pub fn make_executable_fixture() -> fn(&Path) {
    make_executable
}

/// Which packed bootstrap the script-based restore tests should simulate.
#[cfg(unix)]
#[allow(dead_code)] // This module is compiled into every integration-test binary; only restore_script uses it.
#[derive(Clone, Copy)]
pub enum Bundled {
    /// 0.3.7 or newer: honours `PIXI_SANDBOX_USER_TOOLS`.
    Current,
    /// A branch packed before 0.3.7: it does not understand the registration policy.
    PreUserTools,
}

/// A temporary repository whose named branch carries the fixture transport and a shim around the
/// binary this test built. The checkout is reset to a developer-looking project after committing
/// the payload branch, matching a real airlock repository.
#[cfg(unix)]
pub fn transport_repo(root: &Path, branch: &str, bundled: Bundled) -> PathBuf {
    fs::create_dir_all(root).unwrap();
    copy_tree(&fixture_transport(), root);

    let real = assert_cmd::cargo::cargo_bin("pixi-sandbox");
    let tool = root.join(".pixi-sandbox/tools/linux-64/pixi-sandbox");
    let shim = match bundled {
        // The packed 0.3.7+ bootstrap, with optional hostile behaviours the restore.sh tests
        // switch on through `RESTORE_SHIM_MODE` (no mode set: straight delegation, so every
        // existing test drives the real binary). `RESTORE_SHIM_LOG` records each invocation,
        // which is how the retry-discipline tests count restore attempts. Both variables are
        // test-only controls the script under test neither knows nor strips.
        Bundled::Current => format!(
            r#"#!/bin/sh
if [ -n "${{RESTORE_SHIM_LOG:-}}" ]; then printf '%s\n' "$*" >> "$RESTORE_SHIM_LOG"; fi
case "${{RESTORE_SHIM_MODE:-}}" in
  fail-restore)
    case " $* " in *" restore "*) echo "shim: restore exploded" >&2; exit 3 ;; esac ;;
  fail-verify-restored)
    case " $* " in *" --verify-restored "*) echo "shim: restored tree does not verify" >&2; exit 1 ;; esac ;;
  fail-doctor-verify)
    case " $* " in *" --verify "*) echo "shim: doctor verify failed (fixture)" >&2; exit 1 ;; esac ;;
  old-flags)
    case " $* " in *" --output-path "*) echo "error: unexpected argument '--output-path' found" >&2; exit 2 ;; esac ;;
esac
exec "{real}" "$@"
"#,
            real = real.display()
        ),
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
    make_executable(&tool);

    // The fixture manifest verifies tool size before restore writes anything, so its declared
    // size follows the shim while every other fixture byte stays unchanged.
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

    git(root, &["rm", "-qrf", "."]);
    fs::write(root.join("README.md"), "the developer's checkout\n").unwrap();
    git(root, &["add", "."]);
    commit(root, "main");

    root.to_path_buf()
}

#[cfg(unix)]
#[fixture]
pub fn transport_repo_fixture() -> fn(&Path, &str, Bundled) -> PathBuf {
    transport_repo
}

#[cfg(unix)]
pub fn write_executable(path: &Path, script: &str) {
    use std::os::unix::fs::PermissionsExt;

    fs::write(path, script).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

/// Minimal connected-side tools. They make the command contract testable without asking the
/// fixture project to install pixi or turning this suite into a network test.
#[cfg(unix)]
pub fn fake_tools(dir: &Path) {
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
pub fn path_with_fake_tools(dir: &Path) -> std::ffi::OsString {
    let mut paths = vec![dir.to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    std::env::join_paths(paths).unwrap()
}

#[cfg(unix)]
#[fixture]
pub fn write_executable_fixture() -> fn(&Path, &str) {
    write_executable
}

#[cfg(unix)]
#[fixture]
pub fn fake_tools_fixture() -> fn(&Path) {
    fake_tools
}

#[cfg(unix)]
#[fixture]
pub fn path_with_fake_tools_fixture() -> fn(&Path) -> std::ffi::OsString {
    path_with_fake_tools
}

/// Known-good outputs captured before TASK-82's extraction.
#[fixture]
pub fn pack_reference() -> PathBuf {
    crate_dir().join("tests/fixtures/pack-reference")
}

#[cfg(unix)]
#[allow(dead_code)] // Shared support compiles into every test binary, each using a subset.
pub struct SyntheticPackFixture {
    pub home: tempfile::TempDir,
    pub project: PathBuf,
    pub tools: PathBuf,
}

#[cfg(unix)]
#[allow(dead_code)] // Paired with synthetic_pack_fixture; not every test root packs.
impl SyntheticPackFixture {
    pub fn command(&self) -> Command {
        let mut command = isolated_bin(self.home.path());
        command
            .env("PATH", path_with_fake_tools(&self.tools))
            .env(
                "VENDOR_FIXTURE",
                fixture_transport().join(".pixi-sandbox/vendor"),
            )
            .env("CARGO_NET_OFFLINE", "true")
            .args(["pack", "--envs", "default", "--repo-root"])
            .arg(&self.project);
        command
    }

    pub fn self_bin(&self) -> PathBuf {
        let path = self.home.path().join("self-bin");
        write_executable(&path, "#!/bin/sh\necho 'pixi-sandbox 0.6.0'\n");
        path
    }
}

/// The exact external-tool inputs used for the pre-extraction pack reference.
#[cfg(unix)]
#[fixture]
pub fn synthetic_pack_fixture() -> SyntheticPackFixture {
    let home = isolated_home();
    let project = home.path().join("project");
    copy_tree(&demo_project(), &project);
    git(&project, &["init", "-q", "-b", "fixture-unborn"]);
    let tools = home.path().join("tools");
    fake_tools(&tools);
    write_executable(
        &tools.join("cargo"),
        r#"#!/bin/sh
set -eu
if [ "${1:-}" = '--version' ]; then echo 'cargo 1.90.0 (fixture)'; exit 0; fi
test "$1" = vendor
test "$2" = --locked
test "$3" = --versioned-dirs
mkdir -p "$4"
cp -R "$VENDOR_FIXTURE"/. "$4"/
find "$4" -exec touch -t 200001010000.00 {} \;
"#,
    );
    write_executable(
        &tools.join("rustc"),
        "#!/bin/sh\necho 'rustc 1.90.0 (fixture)'\n",
    );
    fs::write(
        project.join("sandbox.toml"),
        "schema = 1\n\n[[bundle]]\nname = \"demo\"\nenvironments = [\"default\"]\nplatforms = [\"linux-64\"]\n\n[host_requirements]\nlibc = \"2.34\"\npackages = [\"fontconfig\"]\ncapabilities = [\"display\"]\n",
    ).unwrap();
    SyntheticPackFixture {
        home,
        project,
        tools,
    }
}

/// An independent, complete byte-and-exec-bit snapshot; do not use production's tree walker
/// to judge whether production silently stopped recording a file.
pub type FileTree = std::collections::BTreeMap<PathBuf, (Vec<u8>, bool)>;

pub fn file_tree(root: &Path) -> FileTree {
    fn walk(root: &Path, dir: &Path, files: &mut FileTree) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.is_dir() {
                walk(root, &path, files);
            } else {
                #[cfg(unix)]
                let executable = {
                    use std::os::unix::fs::PermissionsExt;
                    metadata.permissions().mode() & 0o111 != 0
                };
                #[cfg(not(unix))]
                let executable = false;
                files.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    (fs::read(path).unwrap(), executable),
                );
            }
        }
    }
    let mut files = std::collections::BTreeMap::new();
    walk(root, root, &mut files);
    files
}

#[fixture]
pub fn file_tree_fixture() -> fn(&Path) -> FileTree {
    file_tree
}

/// A genuine Conda package/local channel, not a fake unpacker executable. Each test
/// gets its own pack copy so it can exercise refusal without modifying fixtures.
#[allow(dead_code)] // Shared support compiles into every test binary; only native_pack uses this owner.
pub struct NativePackFixture {
    pub home: tempfile::TempDir,
    pub pack: PathBuf,
}

#[fixture]
pub fn native_pack_fixture(isolated_home: tempfile::TempDir) -> NativePackFixture {
    let pack = isolated_home.path().join("native-pack");
    copy_tree(&crate_dir().join("tests/fixtures/native-pack"), &pack);
    fs::write(
        pack.join("pixi-pack.json"),
        serde_json::to_vec(&serde_json::json!({
            "version": "1", "platform": host_platform(),
        }))
        .unwrap(),
    )
    .unwrap();
    NativePackFixture {
        home: isolated_home,
        pack,
    }
}

#[fixture]
pub fn native_wheel_pack_fixture(native_pack_fixture: NativePackFixture) -> NativePackFixture {
    copy_tree(
        // stale-ref-allowed: runtime-package fixture, not the retired implementation.
        &crate_dir().join("tests/fixtures/native-python-overlay"),
        &native_pack_fixture.pack,
    );
    native_pack_fixture
}

/// A locked project whose selected Conda/wheel archives are genuine local files.
#[fixture]
pub fn native_locked_project_fixture(
    native_wheel_pack_fixture: NativePackFixture,
) -> NativePackFixture {
    let fixture = native_wheel_pack_fixture;
    let project = fixture.home.path().join("locked-project");
    fs::create_dir(&project).unwrap();
    let data: serde_json::Value = serde_json::from_slice(
        &fs::read(fixture.pack.join("channel/noarch/repodata.json")).unwrap(),
    )
    .unwrap();
    let mut packages = Vec::new();
    let mut selected = Vec::new();
    for (name, record) in data["packages"].as_object().unwrap() {
        let location = url::Url::from_file_path(fixture.pack.join("channel/noarch").join(name))
            .unwrap()
            .to_string();
        let mut record = record.clone();
        record["conda"] = location.clone().into();
        // Pixi lock v7 elides identity fields already encoded in the URL.
        for field in ["name", "version", "build", "build_number", "subdir"] {
            record.as_object_mut().unwrap().remove(field);
        }
        packages.push(record);
        selected.push(serde_json::json!({"conda": location}));
    }
    let wheel = fixture.pack.join("pypi/native_demo-1.0-py3-none-any.whl");
    let location = url::Url::from_file_path(&wheel).unwrap().to_string();
    packages.push(serde_json::json!({"pypi": location, "name": "native-demo", "version": "1.0", "sha256": pixi_sandbox_core::shard::sha256_file(&wheel).unwrap()}));
    selected.push(serde_json::json!({"pypi": location}));
    fs::write(project.join("pixi.lock"), serde_json::to_vec_pretty(&serde_json::json!({
        "version": 7, "environments": {"default": {"channels": [], "packages": {host_platform(): selected}}}, "packages": packages
    })).unwrap()).unwrap();
    fs::write(
        project.join("pixi.toml"),
        format!(
            "[workspace]\nname = 'native-fixture'\nchannels = []\nplatforms = ['{}']\n",
            host_platform()
        ),
    )
    .unwrap();
    NativePackFixture {
        home: fixture.home,
        pack: project,
    }
}

/// An actual empty locked environment, for tests unrelated to dependency resolution.
#[allow(dead_code)] // Shared fixture support is compiled separately by each test root.
pub fn write_empty_native_lock(project: &Path, environments: &[&str]) {
    let environments: serde_json::Map<String, serde_json::Value> = environments
        .iter()
        .map(|name| {
            (
                (*name).to_string(),
                serde_json::json!({"channels": [], "packages": {host_platform(): []}}),
            )
        })
        .collect();
    fs::write(
        project.join("pixi.lock"),
        serde_json::to_vec(&serde_json::json!({
            "version": 7, "environments": environments, "packages": [],
        }))
        .unwrap(),
    )
    .unwrap();
}

/// A transport produced by the public native pack CLI from genuine local packages.
#[fixture]
pub fn native_transport_fixture(
    native_locked_project_fixture: NativePackFixture,
) -> NativePackFixture {
    let fixture = native_locked_project_fixture;
    let transport = fixture.home.path().join("transport");
    isolated_bin(fixture.home.path())
        .env("PATH", "")
        .args(["pack", "--repo-root"])
        .arg(&fixture.pack)
        .arg("--output-dir")
        .arg(&transport)
        .args(["--envs", "default", "--platform", host_platform()])
        .assert()
        .success();
    NativePackFixture {
        home: fixture.home,
        pack: transport,
    }
}
