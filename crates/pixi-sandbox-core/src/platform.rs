//! The single source of truth for the five Pixi platform ids this project supports, and
//! everything derived from them.
//!
//! Before this module existed, `(os, arch) -> platform id`, `platform id -> Rust target
//! triple`, `platform id -> release asset name`, and `platform id -> GitHub-hosted runner`
//! were each hand-rolled independently in `commands/init.rs`, `standalone.rs`,
//! `self_update/assets.rs`, `xtask/airlock.rs`, `xtask/release_assets.rs`,
//! `xtask/conda_platforms.rs`, and `sandbox_config.rs::runner_for` — two of them
//! (`init.rs::current_platform` and `standalone.rs::platform`) byte-for-byte identical. Adding
//! a sixth platform meant editing eight-plus files by hand, with no compiler or test tying them
//! together. This type is the one place that fact now lives; call sites migrate to it in
//! follow-on tasks (task-58, task-59, task-60), file by file, with behavior unchanged.
//!
//! Deliberately hand-written rather than derived with a crate like `strum`: this workspace
//! builds offline against a vendored `Cargo.lock` (see `AGENTS.md`), so a new proc-macro
//! dependency cannot be added without a connected-side relock and a repacked transport.

use crate::{Error, Result};

/// One of the five platforms this project builds, packs, and publishes for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Platform {
    Linux64,
    LinuxAarch64,
    Osx64,
    OsxArm64,
    Win64,
}

impl Platform {
    /// Every supported platform, in the order the release build matrix lists them.
    pub const ALL: [Platform; 5] = [
        Platform::Linux64,
        Platform::LinuxAarch64,
        Platform::Osx64,
        Platform::OsxArm64,
        Platform::Win64,
    ];

    /// Rust's host spelling (`std::env::consts::{OS, ARCH}`) to a `Platform`. Pure, so every
    /// row — and the unsupported one — is exercised without needing that host.
    ///
    /// Not `const`: matching on `&str` in a const fn needs a newer `PartialEq`-as-const-trait
    /// than this workspace's pinned toolchain stabilizes. [`Platform::os_arch`] is the const,
    /// reverse direction used where a `const` context needs it (e.g.
    /// `self_update::assets::SUPPORTED_HOSTS`).
    #[must_use]
    pub fn from_os_arch(os: &str, arch: &str) -> Option<Platform> {
        match (os, arch) {
            ("linux", "x86_64") => Some(Platform::Linux64),
            ("linux", "aarch64") => Some(Platform::LinuxAarch64),
            ("macos", "x86_64") => Some(Platform::Osx64),
            ("macos", "aarch64") => Some(Platform::OsxArm64),
            ("windows", "x86_64") => Some(Platform::Win64),
            _ => None,
        }
    }

    /// The reverse of [`Platform::from_os_arch`]: Rust's `(OS, ARCH)` spelling for this
    /// platform (`std::env::consts` values), as opposed to [`Platform::as_str`]'s Pixi
    /// spelling.
    #[must_use]
    pub const fn os_arch(self) -> (&'static str, &'static str) {
        match self {
            Platform::Linux64 => ("linux", "x86_64"),
            Platform::LinuxAarch64 => ("linux", "aarch64"),
            Platform::Osx64 => ("macos", "x86_64"),
            Platform::OsxArm64 => ("macos", "aarch64"),
            Platform::Win64 => ("windows", "x86_64"),
        }
    }

    /// The current host's platform, in the spelling every other method here expects.
    #[must_use]
    pub fn current() -> Option<Platform> {
        Platform::from_os_arch(std::env::consts::OS, std::env::consts::ARCH)
    }

    /// The Pixi platform id: `linux-64`, `osx-arm64`, etc. — what `.pixi-sandbox.toml`,
    /// `manifest.json`, and `pixi` itself call this platform.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Platform::Linux64 => "linux-64",
            Platform::LinuxAarch64 => "linux-aarch64",
            Platform::Osx64 => "osx-64",
            Platform::OsxArm64 => "osx-arm64",
            Platform::Win64 => "win-64",
        }
    }

    /// The Rust target triple this platform's binaries are built for — matches the
    /// `release.yml` build matrix.
    #[must_use]
    pub const fn target_triple(self) -> &'static str {
        match self {
            Platform::Linux64 => "x86_64-unknown-linux-musl",
            Platform::LinuxAarch64 => "aarch64-unknown-linux-musl",
            Platform::Osx64 => "x86_64-apple-darwin",
            Platform::OsxArm64 => "aarch64-apple-darwin",
            Platform::Win64 => "x86_64-pc-windows-msvc",
        }
    }

    /// The published release asset name for this platform's standalone binary:
    /// `pixi-sandbox-<target-triple>`, with the `.exe` suffix Windows assets carry. A literal
    /// match rather than `format!("pixi-sandbox-{}", self.target_triple())` so this is a
    /// `const fn`: callers that need the five names in a `const` table (e.g.
    /// `self_update::assets::SUPPORTED_HOSTS`) can build it from `Platform::ALL` instead of
    /// retyping the literals, which is the whole point of this type (task-55).
    #[must_use]
    pub const fn asset_name(self) -> &'static str {
        match self {
            Platform::Linux64 => "pixi-sandbox-x86_64-unknown-linux-musl",
            Platform::LinuxAarch64 => "pixi-sandbox-aarch64-unknown-linux-musl",
            Platform::Osx64 => "pixi-sandbox-x86_64-apple-darwin",
            Platform::OsxArm64 => "pixi-sandbox-aarch64-apple-darwin",
            Platform::Win64 => "pixi-sandbox-x86_64-pc-windows-msvc.exe",
        }
    }

    /// The default GitHub-hosted runner label for this platform, or `None` when no
    /// GitHub-hosted runner exists for it (today: `linux-aarch64`), which means a project
    /// publishing that platform must supply an explicit `runners.<platform>` override.
    #[must_use]
    pub const fn gh_runner(self) -> Option<&'static str> {
        match self {
            Platform::Linux64 => Some("ubuntu-latest"),
            Platform::LinuxAarch64 => None,
            // `macos-14` is Apple Silicon on GitHub-hosted runners.
            Platform::OsxArm64 => Some("macos-14"),
            Platform::Osx64 => Some("macos-13"),
            Platform::Win64 => Some("windows-latest"),
        }
    }
}

impl std::str::FromStr for Platform {
    type Err = Error;

    fn from_str(value: &str) -> Result<Platform> {
        Platform::ALL
            .into_iter()
            .find(|platform| platform.as_str() == value)
            .ok_or_else(|| Error::Invalid(format!("unknown platform {value:?}")))
    }
}

/// The host operating-system family a platform belongs to.
///
/// Host-level facts — a glibc floor, a package manager, the presence of a display server — are
/// properties of the *OS*, not of the architecture, so `[host_requirements]` sections and their
/// probes are scoped to a family (issue #109, TASK-75) rather than to each of the five Pixi
/// platforms. The mapping lives here, next to [`Platform`], because it is one more fact derived
/// from the platform list: adding a sixth platform means answering which family it belongs to,
/// and the exhaustive match below makes that a compile error rather than a silent omission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HostFamily {
    Linux,
    Osx,
    Windows,
}

impl HostFamily {
    /// Every family, in the order `[host_requirements]` sections are documented.
    pub const ALL: [HostFamily; 3] = [HostFamily::Linux, HostFamily::Osx, HostFamily::Windows];

    /// The spelling used in `[host_requirements.<family>]` and in JSON output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            HostFamily::Linux => "linux",
            HostFamily::Osx => "osx",
            HostFamily::Windows => "windows",
        }
    }
}

impl std::fmt::Display for HostFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Platform {
    /// The OS family this platform belongs to (`linux-64` and `linux-aarch64` → [`HostFamily::Linux`]).
    #[must_use]
    pub const fn host_family(self) -> HostFamily {
        match self {
            Platform::Linux64 | Platform::LinuxAarch64 => HostFamily::Linux,
            Platform::Osx64 | Platform::OsxArm64 => HostFamily::Osx,
            Platform::Win64 => HostFamily::Windows,
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
