//! TASK-78 / ADR-28 Slice 3: the real `rattler-build` adapter.
//!
//! The subprocess is faked, so the argv, the package discovery and the missing-tool remedy are
//! checked without a binary, the network or this checkout. The connected first build is a manual
//! proof recorded in the commit, not a test.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use pixi_sandbox::rattler_build::{
    ProcessRunner, RattlerBuildToolchain, RealProcessRunner, TOOL_NAME, TOOL_VERSION, build_args,
    find_package, missing_tool_remedy, pins, publish_args, resolve_tool,
};
use pixi_sandbox::tool_fetch::ToolDownload;
use pixi_sandbox_core::payload::{
    PackageIdentity, PayloadError, PayloadSpec, PayloadToolchain, PayloadTotals, Provenance,
    SourceArtifact, ValidatedPayload, WrapPlan,
};
use pixi_sandbox_core::platform::Platform;

/// A plan good enough to drive the adapter: only the platform and identity matter to argv.
fn plan(platform: Platform) -> WrapPlan {
    WrapPlan {
        spec: PayloadSpec {
            name: "demo".to_string(),
            version: "1.0.0".to_string(),
            platform,
            noarch: false,
            entry_point: None,
        },
        recipe: "package:\n  name: demo\n".to_string(),
        provenance: Provenance {
            schema: 1,
            package: PackageIdentity {
                name: "demo".to_string(),
                version: "1.0.0".to_string(),
                platform: platform.as_str().to_string(),
            },
            source: SourceArtifact {
                url: "https://example.invalid/demo-1.0.0.bin".to_string(),
                sha256: "0".repeat(64),
            },
            payload: PayloadTotals { files: 1, bytes: 1 },
        },
        payload: ValidatedPayload {
            file_count: 1,
            directory_count: 0,
            symlink_count: 0,
            total_bytes: 1,
        },
    }
}

/// Records argv, and — when `write_package` — stages the `.conda` a real build would have left,
/// so [`find_package`] has something to discover.
#[derive(Default)]
struct FakeRunner {
    calls: RefCell<Vec<Vec<String>>>,
    write_package: bool,
}

impl ProcessRunner for FakeRunner {
    fn run(&self, _program: &Path, args: &[String]) -> Result<String, PayloadError> {
        self.calls.borrow_mut().push(args.to_vec());
        if self.write_package && args.first().map(String::as_str) == Some("build") {
            let out = value_after(args, "--output-dir").expect("build args carry --output-dir");
            let platform =
                value_after(args, "--target-platform").expect("build args carry a platform");
            let dir = PathBuf::from(out).join(platform);
            std::fs::create_dir_all(&dir).expect("output dir");
            std::fs::write(dir.join("demo-1.0.0-h1234_0.conda"), b"package").expect("package");
        }
        Ok(String::new())
    }
}

fn value_after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let index = args.iter().position(|arg| arg == flag)?;
    args.get(index + 1).map(String::as_str)
}

#[test]
fn the_pins_review_one_digest_for_every_supported_platform() {
    let lock = pins();

    assert!(lock.tools.contains_key(TOOL_NAME), "the tool is pinned");
    let platforms = lock.platforms_of(TOOL_NAME);
    assert_eq!(platforms.len(), 5, "all five platforms are pinned");
    for platform in ["linux-64", "linux-aarch64", "osx-64", "osx-arm64", "win-64"] {
        assert!(platforms.contains(&platform), "{platform} is pinned");
    }
    for platform in &platforms {
        let pin = lock.pin(TOOL_NAME, platform).expect("a pin");
        assert_eq!(pin.sha256.len(), 64, "a lowercase hex digest");
        let url = lock.url(TOOL_NAME, platform).expect("a url");
        assert!(
            url.contains(TOOL_VERSION) && url.contains("rattler-build-"),
            "the url names the version and the asset: {url}"
        );
    }
    assert_eq!(
        lock.pin(TOOL_NAME, "win-64").expect("win pin").target,
        "x86_64-pc-windows-msvc.exe",
        "the Windows asset keeps its extension"
    );
}

#[test]
fn build_args_name_the_recipe_the_output_dir_and_the_platform() {
    let args = build_args(
        Path::new("/work/recipe.yaml"),
        Path::new("/work/out"),
        Platform::OsxArm64,
    );

    assert_eq!(
        args,
        [
            "build",
            "--recipe",
            "/work/recipe.yaml",
            "--output-dir",
            "/work/out",
            "--target-platform",
            "osx-arm64",
            "--test",
            "skip",
            "--no-config",
        ]
    );
}

#[test]
fn publish_args_target_an_absolute_filesystem_channel() {
    let args = publish_args(
        Path::new("/channel/linux-64/demo-1.0.0-0.conda"),
        Path::new("/channel"),
    );

    assert_eq!(args[0], "publish");
    assert_eq!(args[1], "--to");
    assert_eq!(
        args[2], "file:///channel",
        "pixi needs an absolute file:// url"
    );
    assert_eq!(args[3], "--no-config");
    assert_eq!(args[4], "/channel/linux-64/demo-1.0.0-0.conda");
}

#[test]
fn find_package_returns_the_single_conda_the_build_wrote() {
    let dir = tempfile::tempdir().expect("tempdir");
    let platform_dir = dir.path().join("linux-64");
    std::fs::create_dir_all(&platform_dir).expect("platform dir");
    let package = platform_dir.join("demo-1.0.0-h1234_0.conda");
    std::fs::write(&package, b"package").expect("package");

    assert_eq!(
        find_package(dir.path(), Platform::Linux64).expect("found"),
        package
    );
}

#[test]
fn find_package_refuses_a_build_that_wrote_no_package() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("linux-64")).expect("platform dir");

    let error = find_package(dir.path(), Platform::Linux64).expect_err("nothing to return");

    assert!(
        matches!(&error, PayloadError::Toolchain(message) if message.contains("no .conda")),
        "the operator is told the build wrote nothing: {error}"
    );
}

#[test]
fn find_package_names_the_platform_directory_a_build_that_never_ran_left_missing() {
    let dir = tempfile::tempdir().expect("tempdir");

    let error = find_package(dir.path(), Platform::Linux64).expect_err("no build ran");

    assert!(
        matches!(&error, PayloadError::Toolchain(message)
            if message.contains("linux-64") && message.contains("no package directory")),
        "the operator is pointed at the directory the build should have written: {error}"
    );
}

#[test]
fn find_package_refuses_to_choose_between_two_packages() {
    let dir = tempfile::tempdir().expect("tempdir");
    let platform_dir = dir.path().join("linux-64");
    std::fs::create_dir_all(&platform_dir).expect("platform dir");
    std::fs::write(platform_dir.join("demo-1.0.0-h0.conda"), b"one").expect("first");
    std::fs::write(platform_dir.join("demo-1.0.0-h1.conda"), b"two").expect("second");

    let error = find_package(dir.path(), Platform::Linux64).expect_err("two packages");

    assert!(
        matches!(&error, PayloadError::Toolchain(message)
            if message.contains("2 packages") && message.contains("expected exactly one")),
        "the caller must act, not guess which artifact was built: {error}"
    );
}

#[test]
fn the_adapter_names_the_program_it_was_given() {
    let runner = FakeRunner::default();
    let toolchain = RattlerBuildToolchain::new(PathBuf::from("/opt/tools/rattler-build"), &runner);

    assert_eq!(toolchain.program(), Path::new("/opt/tools/rattler-build"));
}

#[cfg(unix)]
#[test]
fn the_real_runner_returns_the_output_of_a_command_that_succeeded() {
    let output = RealProcessRunner
        .run(
            Path::new("/bin/sh"),
            &["-c".to_string(), "printf built".to_string()],
        )
        .expect("sh runs");

    assert!(output.contains("built"), "{output}");
}

#[cfg(unix)]
#[test]
fn the_real_runner_reports_a_command_that_failed_as_a_toolchain_error() {
    let error = RealProcessRunner
        .run(
            Path::new("/bin/sh"),
            &["-c".to_string(), "exit 3".to_string()],
        )
        .expect_err("exit 3 is a failure");

    assert!(
        matches!(&error, PayloadError::Toolchain(message) if message.contains("sh")),
        "the failure names the program that failed: {error}"
    );
}

#[test]
fn the_adapter_builds_the_staged_recipe_and_discovers_the_package() {
    let runner = FakeRunner {
        write_package: true,
        ..FakeRunner::default()
    };
    let toolchain = RattlerBuildToolchain::new(PathBuf::from("rattler-build"), &runner);
    let work = tempfile::tempdir().expect("tempdir");

    let package = toolchain
        .build(&plan(Platform::Linux64), work.path())
        .expect("the fake build succeeds");

    assert_eq!(
        package.file_name().and_then(|name| name.to_str()),
        Some("demo-1.0.0-h1234_0.conda")
    );
    let calls = runner.calls.borrow();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0][0], "build");
    assert_eq!(
        value_after(&calls[0], "--output-dir"),
        Some(work.path().join("out").to_str().unwrap())
    );
}

#[test]
fn the_adapter_publishes_the_package_into_the_channel() {
    let runner = FakeRunner::default();
    let toolchain = RattlerBuildToolchain::new(PathBuf::from("rattler-build"), &runner);

    toolchain
        .index(
            Path::new("/channel"),
            Path::new("/channel/linux-64/demo-1.0.0-0.conda"),
        )
        .expect("the fake publish succeeds");

    let calls = runner.calls.borrow();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0][0], "publish");
    assert_eq!(calls[0][2], "file:///channel");
}

#[test]
fn a_missing_tool_fails_with_a_named_remedy_not_a_bare_spawn_error() {
    #[derive(Debug)]
    struct Offline;

    impl ToolDownload for Offline {
        fn open(&self, _url: &str) -> anyhow::Result<Box<dyn std::io::Read>> {
            anyhow::bail!("the network is unreachable")
        }
    }

    let cache = tempfile::tempdir().expect("tempdir");
    let error = resolve_tool(Platform::Linux64, cache.path(), &Offline)
        .expect_err("an offline fetch cannot succeed");

    let PayloadError::Toolchain(message) = &error else {
        panic!("expected a toolchain error, got {error}");
    };
    assert!(message.contains(TOOL_NAME), "names the tool: {message}");
    assert!(
        message.contains("--build-tool"),
        "names the remedy: {message}"
    );
    assert!(
        missing_tool_remedy("boom")
            .to_string()
            .contains("--build-tool"),
        "the public remedy builder states the same fix"
    );
}
