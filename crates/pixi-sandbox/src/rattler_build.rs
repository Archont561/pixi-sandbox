//! The on-demand `rattler-build` toolchain (TASK-78, ADR-28).
//!
//! This is the real [`PayloadToolchain`](pixi_sandbox_core::payload::PayloadToolchain): it runs
//! `rattler-build` as a subprocess to build one `.conda` from the staged recipe, then publishes
//! it into a filesystem channel with `rattler-build publish --to file://…`, which is also the
//! indexing step (ADR-28 chose rattler-build's own indexing over a separate indexer).
//!
//! The tool is fetched on demand through the existing tool-fetch machinery (checksum, cache,
//! version check) and is never linked, never embedded and never in `tools.lock.json` (AC#12).
//! The pins are reviewed data in this module, not the embedded catalogue, because the tool is
//! 31 MiB and the catalogue is packed into every transport (ADR-28).
//!
//! The subprocess is behind [`ProcessRunner`] so the argv and the package discovery are tested
//! without a binary, the network or this checkout.

use crate::build_tool_fetch::{ToolDownload, fetch_tool_with};
use crate::pack::support;
use crate::wrap_payload::RECIPE_FILE;
use pixi_sandbox_core::build_tool_pins::{BuildToolPins, LOCK_SCHEMA, PlatformPin, Tool};
use pixi_sandbox_core::payload::{PayloadError, PayloadToolchain, WrapPlan};
use pixi_sandbox_core::platform::Platform;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The tool's identity in the tools cache.
pub const TOOL_NAME: &str = "rattler-build";
/// The reviewed version. Bumping it means re-deriving every pin in [`PINS`].
pub const TOOL_VERSION: &str = "0.76.1";

/// Release-asset template: `{version}` and `{target}` are the only substitutions (ADR-28).
const URL_TEMPLATE: &str = "https://github.com/prefix-dev/rattler-build/releases/download/v{version}/rattler-build-{target}";

/// One reviewed pin: the pixi platform, the release-asset target, the asset's SHA256 and its
/// linkage. The Windows target keeps its `.exe` because that is the release asset's name.
struct Pin {
    platform: &'static str,
    target: &'static str,
    sha256: &'static str,
    linkage: &'static str,
}

/// Reviewed from the `v0.76.1` release on 2026-10-12: the GitHub release `digest` field, and the
/// linux-64 asset downloaded and `sha256sum`-checked against it. The digests are data, so a bump
/// is a reviewable diff, exactly like `tools.lock.json`.
const PINS: &[Pin] = &[
    Pin {
        platform: "linux-64",
        target: "x86_64-unknown-linux-musl",
        sha256: "38359a75f4a880442432923d6e984d349bb4134d330edd712eb1cfb2fa80581a",
        linkage: "static",
    },
    Pin {
        platform: "linux-aarch64",
        target: "aarch64-unknown-linux-musl",
        sha256: "8f5ae1c3faf08c50007bbfe896cee84dcf5f4b8e8cc3a5fa400695c82abed2ae",
        linkage: "static",
    },
    Pin {
        platform: "osx-64",
        target: "x86_64-apple-darwin",
        sha256: "4b0183a60e8d2a93ddaf34d0def0f3a27effcf973d097c016250dfbdb7f15c1b",
        linkage: "system",
    },
    Pin {
        platform: "osx-arm64",
        target: "aarch64-apple-darwin",
        sha256: "3809453e95380d598f2c4264480396029781fc64a9b940f3e19f84485fad75bf",
        linkage: "system",
    },
    Pin {
        platform: "win-64",
        target: "x86_64-pc-windows-msvc.exe",
        sha256: "131e3d9e6d67e7b5e873627567e74708e1722a40e78bcd3619dc4b20906fbb64",
        linkage: "system",
    },
];

/// The reviewed pins, in the shape the tool-fetch machinery already consumes.
#[must_use]
pub fn pins() -> BuildToolPins {
    let platforms = PINS
        .iter()
        .map(|pin| {
            (
                pin.platform.to_string(),
                PlatformPin {
                    target: pin.target.to_string(),
                    sha256: pin.sha256.to_string(),
                    linkage: pin.linkage.to_string(),
                    note: Some(
                        "rattler-build is fetched on demand, never embedded (ADR-28)".to_string(),
                    ),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let tool = Tool {
        version: TOOL_VERSION.to_string(),
        url_template: URL_TEMPLATE.to_string(),
        platforms,
    };
    let mut tools = BTreeMap::new();
    tools.insert(TOOL_NAME.to_string(), tool);
    BuildToolPins {
        schema: LOCK_SCHEMA,
        generated_at: None,
        tools,
    }
}

/// The remedy a missing tool must carry, so a failed fetch names the fix rather than a bare
/// spawn error (AC#12).
#[must_use]
pub fn missing_tool_remedy(detail: &str) -> PayloadError {
    PayloadError::Toolchain(format!(
        "could not fetch {TOOL_NAME} {TOOL_VERSION}: {detail}; run this on a connected host, or \
         pass --build-tool <path> to an existing rattler-build"
    ))
}

/// Fetch the pinned build tool into `cache` (or return the verified copy already there).
///
/// # Errors
///
/// [`PayloadError::Toolchain`], naming the fetch failure and the remedy, when the tool cannot be
/// obtained.
pub fn resolve_tool(
    platform: Platform,
    cache: &Path,
    download: &dyn ToolDownload,
) -> Result<PathBuf, PayloadError> {
    fetch_tool_with(&pins(), TOOL_NAME, platform.as_str(), cache, download)
        .map(|fetched| fetched.path)
        .map_err(|error| missing_tool_remedy(&error.to_string()))
}

/// argv (after the program) that builds one package for `platform` into `out_dir`.
#[must_use]
pub fn build_args(recipe: &Path, out_dir: &Path, platform: Platform) -> Vec<String> {
    vec![
        "build".to_string(),
        "--recipe".to_string(),
        recipe.display().to_string(),
        "--output-dir".to_string(),
        out_dir.display().to_string(),
        "--target-platform".to_string(),
        platform.as_str().to_string(),
        // The recipe installs bytes that are already checked by the caller; running the built
        // package's test phase is not what proves this flow.
        "--test".to_string(),
        "skip".to_string(),
        // Do not read a global pixi/rattler-build config, so the build is reproducible on any host.
        "--no-config".to_string(),
    ]
}

/// argv that adds `package` to a filesystem `channel_dir` and regenerates its index. The channel
/// must be an absolute path: pixi requires absolute `file://` paths (AC#11).
#[must_use]
pub fn publish_args(package: &Path, channel_dir: &Path) -> Vec<String> {
    vec![
        "publish".to_string(),
        "--to".to_string(),
        format!("file://{}", channel_dir.display()),
        "--no-config".to_string(),
        package.display().to_string(),
    ]
}

/// The single `.conda` rattler-build wrote under `out_dir/<platform>/`.
///
/// # Errors
///
/// [`PayloadError::Toolchain`] when the directory cannot be read, or holds zero or more than one
/// package: the caller must act, not guess which artifact was built.
pub fn find_package(out_dir: &Path, platform: Platform) -> Result<PathBuf, PayloadError> {
    let dir = out_dir.join(platform.as_str());
    let entries = std::fs::read_dir(&dir).map_err(|error| {
        PayloadError::Toolchain(format!(
            "rattler-build produced no package directory at {}: {error}",
            dir.display()
        ))
    })?;
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            PayloadError::Toolchain(format!("reading {}: {error}", dir.display()))
        })?;
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "conda")
        {
            found.push(path);
        }
    }
    match found.as_slice() {
        [only] => Ok(only.clone()),
        [] => Err(PayloadError::Toolchain(format!(
            "rattler-build wrote no .conda under {}; the recipe or the build log will say why",
            dir.display()
        ))),
        several => Err(PayloadError::Toolchain(format!(
            "rattler-build wrote {} packages under {}, expected exactly one",
            several.len(),
            dir.display()
        ))),
    }
}

/// The subprocess seam: the adapter is tested with a fake that records argv and stages the
/// package a real build would have written.
pub trait ProcessRunner {
    /// Run `program` with `args`, returning combined output.
    ///
    /// # Errors
    ///
    /// [`PayloadError::Toolchain`] when the process cannot start or exits non-zero.
    fn run(&self, program: &Path, args: &[String]) -> Result<String, PayloadError>;
}

/// The real runner: a child process via the shared `support::run` (absolute program, both streams
/// kept in the failure).
#[derive(Debug, Default)]
pub struct RealProcessRunner;

impl ProcessRunner for RealProcessRunner {
    fn run(&self, program: &Path, args: &[String]) -> Result<String, PayloadError> {
        let mut command = Command::new(program);
        command.args(args);
        support::run(&mut command)
            .map_err(|error| PayloadError::Toolchain(format!("{}: {error}", program.display())))
    }
}

/// The `rattler-build` adapter. `work_dir` is the staged recipe directory (Slice 2): the recipe,
/// the payload and the provenance are already there, and the build output lands in `work_dir/out`.
pub struct RattlerBuildToolchain<'a> {
    program: PathBuf,
    runner: &'a dyn ProcessRunner,
}

impl<'a> RattlerBuildToolchain<'a> {
    #[must_use]
    pub fn new(program: PathBuf, runner: &'a dyn ProcessRunner) -> Self {
        Self { program, runner }
    }

    #[must_use]
    pub fn program(&self) -> &Path {
        &self.program
    }
}

impl PayloadToolchain for RattlerBuildToolchain<'_> {
    fn build(&self, plan: &WrapPlan, work_dir: &Path) -> Result<PathBuf, PayloadError> {
        let recipe = work_dir.join(RECIPE_FILE);
        let out_dir = work_dir.join("out");
        let args = build_args(&recipe, &out_dir, plan.spec.platform);
        self.runner.run(&self.program, &args)?;
        find_package(&out_dir, plan.spec.platform)
    }

    fn index(&self, channel_dir: &Path, package: &Path) -> Result<(), PayloadError> {
        let args = publish_args(package, channel_dir);
        self.runner.run(&self.program, &args).map(|_output| ())
    }
}
