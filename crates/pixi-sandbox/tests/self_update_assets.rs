//! Unit tests for the canonical host-to-release-asset map (`src/self_update/assets.rs`).

use pixi_sandbox::self_update::assets::*;

#[test]
fn every_supported_host_maps_to_its_published_asset_name() {
    assert_eq!(
        asset_for("linux", "x86_64").unwrap(),
        "pixi-sandbox-x86_64-unknown-linux-musl"
    );
    assert_eq!(
        asset_for("linux", "aarch64").unwrap(),
        "pixi-sandbox-aarch64-unknown-linux-musl"
    );
    assert_eq!(
        asset_for("macos", "x86_64").unwrap(),
        "pixi-sandbox-x86_64-apple-darwin"
    );
    assert_eq!(
        asset_for("macos", "aarch64").unwrap(),
        "pixi-sandbox-aarch64-apple-darwin"
    );
    assert_eq!(
        asset_for("windows", "x86_64").unwrap(),
        "pixi-sandbox-x86_64-pc-windows-msvc.exe"
    );
}

#[test]
fn only_the_windows_asset_carries_the_exe_suffix() {
    for (os, _, name) in SUPPORTED_HOSTS {
        assert_eq!(
            name.ends_with(".exe"),
            os == "windows",
            "{name} has the wrong suffix for {os}"
        );
    }
}

#[test]
fn an_unsupported_host_is_refused_and_the_message_names_it() {
    let err = asset_for("linux", "riscv64").unwrap_err().to_string();
    assert!(err.contains("linux/riscv64"), "{err}");
    assert!(err.contains("supported hosts"), "{err}");
    // 32-bit Windows is a plausible mistake and must not silently resolve to the 64-bit asset.
    assert!(asset_for("windows", "x86").is_err());
}

#[test]
fn the_running_host_is_reported_in_the_tables_spelling() {
    let (os, arch) = current_host();
    assert_eq!(os, std::env::consts::OS);
    assert_eq!(arch, std::env::consts::ARCH);
}
