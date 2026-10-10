//! The canonical host → release-asset map.
//!
//! Derived from [`pixi_sandbox_core::platform::Platform`] (task-55/task-58), the single source
//! of these five names — which are exactly what `xtask stage-release-binary` writes and what
//! `xtask release-checksums` covers (`crates/xtask/src/release_assets.rs`, migrated onto the
//! same type in task-59). A host that is not in this table is a hard error naming the host —
//! guessing a triple would produce a 404 at best and the wrong architecture's binary at worst,
//! and decision-4 makes "never executes or installs unverified bytes" the property of this
//! command.

use anyhow::{Result, bail};
use pixi_sandbox_core::platform::Platform;

/// `(os, arch, asset name)`. The `.exe` suffix is part of the published name on Windows. Built
/// from `Platform::ALL` in a `const` block rather than listed by hand, so this table cannot
/// drift from `Platform`'s own asset names — the thing a doc comment used to merely promise.
pub const SUPPORTED_HOSTS: [(&str, &str, &str); Platform::ALL.len()] = {
    let mut hosts = [("", "", ""); Platform::ALL.len()];
    let mut index = 0;
    while index < Platform::ALL.len() {
        let platform = Platform::ALL[index];
        let (os, arch) = platform.os_arch();
        hosts[index] = (os, arch, platform.asset_name());
        index += 1;
    }
    hosts
};

/// The asset name for an explicit host. A parameter rather than a `cfg!` read so every row of
/// the table is exercised on whatever machine runs the tests.
///
/// # Errors
///
/// Returns an error naming the unsupported host when `(os, arch)` is not in the table.
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
#[must_use]
pub fn current_host() -> (&'static str, &'static str) {
    (std::env::consts::OS, std::env::consts::ARCH)
}
