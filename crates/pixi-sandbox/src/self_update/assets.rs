//! The canonical host → release-asset map.
//!
//! Reviewed data, not a computation: these five names are exactly what
//! `xtask stage-release-binary` writes and what `xtask release-checksums` covers
//! (`crates/xtask/src/release_assets.rs`). A host that is not in this table is a hard error
//! naming the host — guessing a triple would produce a 404 at best and the wrong architecture's
//! binary at worst, and decision-4 makes "never executes or installs unverified bytes" the
//! property of this command.

use anyhow::{Result, bail};

/// `(os, arch, asset name)`. The `.exe` suffix is part of the published name on Windows.
pub const SUPPORTED_HOSTS: [(&str, &str, &str); 5] = [
    ("linux", "x86_64", "pixi-sandbox-x86_64-unknown-linux-musl"),
    (
        "linux",
        "aarch64",
        "pixi-sandbox-aarch64-unknown-linux-musl",
    ),
    ("macos", "x86_64", "pixi-sandbox-x86_64-apple-darwin"),
    ("macos", "aarch64", "pixi-sandbox-aarch64-apple-darwin"),
    (
        "windows",
        "x86_64",
        "pixi-sandbox-x86_64-pc-windows-msvc.exe",
    ),
];

/// The asset name for an explicit host. A parameter rather than a `cfg!` read so every row of
/// the table is exercised on whatever machine runs the tests.
pub fn asset_for(os: &str, arch: &str) -> Result<&'static str> {
    if let Some((_, _, name)) = SUPPORTED_HOSTS
        .iter()
        .find(|(host_os, host_arch, _)| *host_os == os && *host_arch == arch)
    {
        return Ok(name);
    }
    bail!(
        "no published pixi-sandbox binary for {os}/{arch}\n\
         supported hosts: {}\n\
         build from source for this platform instead",
        SUPPORTED_HOSTS
            .iter()
            .map(|(os, arch, _)| format!("{os}/{arch}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// The host this binary is running on, in the spelling `asset_for` expects.
pub fn current_host() -> (&'static str, &'static str) {
    (std::env::consts::OS, std::env::consts::ARCH)
}
