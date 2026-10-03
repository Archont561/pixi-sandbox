//! `Platform` is the single source of truth for everything keyed by the five Pixi platform
//! strings: the pixi platform id itself, the Rust target triple, the published release asset
//! name, and the GitHub-hosted runner label. Every call site that used to hand-roll one of
//! these mappings (`commands/init.rs`, `standalone.rs`, `self_update/assets.rs`,
//! `xtask/airlock.rs`, `xtask/release_assets.rs`, `xtask/conda_platforms.rs`,
//! `sandbox_config.rs::runner_for`) migrates to this type in follow-on tasks; this file only
//! proves the type itself.

use pixi_sandbox_core::platform::Platform;
use proptest::prelude::*;
use rstest::rstest;

/// A `Strategy` over `Platform::ALL` rather than its own enum — the point of the property
/// below is to exercise `Display`/`FromStr` agreement for every current and future member of
/// `ALL` without hand-listing them a second time here.
fn any_platform() -> impl Strategy<Value = Platform> {
    (0..Platform::ALL.len()).prop_map(|index| Platform::ALL[index])
}

#[test]
fn from_os_arch_covers_every_supported_host() {
    assert_eq!(
        Platform::from_os_arch("linux", "x86_64"),
        Some(Platform::Linux64)
    );
    assert_eq!(
        Platform::from_os_arch("linux", "aarch64"),
        Some(Platform::LinuxAarch64)
    );
    assert_eq!(
        Platform::from_os_arch("macos", "x86_64"),
        Some(Platform::Osx64)
    );
    assert_eq!(
        Platform::from_os_arch("macos", "aarch64"),
        Some(Platform::OsxArm64)
    );
    assert_eq!(
        Platform::from_os_arch("windows", "x86_64"),
        Some(Platform::Win64)
    );
}

#[test]
fn from_os_arch_rejects_an_unknown_host() {
    assert_eq!(Platform::from_os_arch("plan9", "x86_64"), None);
    assert_eq!(Platform::from_os_arch("linux", "riscv64"), None);
}

#[rstest]
#[case(Platform::Linux64, "linux-64")]
#[case(Platform::LinuxAarch64, "linux-aarch64")]
#[case(Platform::Osx64, "osx-64")]
#[case(Platform::OsxArm64, "osx-arm64")]
#[case(Platform::Win64, "win-64")]
fn as_str_matches_the_pixi_platform_id(#[case] platform: Platform, #[case] expected: &str) {
    assert_eq!(platform.as_str(), expected);
}

#[rstest]
#[case(Platform::Linux64, "x86_64-unknown-linux-musl")]
#[case(Platform::LinuxAarch64, "aarch64-unknown-linux-musl")]
#[case(Platform::Osx64, "x86_64-apple-darwin")]
#[case(Platform::OsxArm64, "aarch64-apple-darwin")]
#[case(Platform::Win64, "x86_64-pc-windows-msvc")]
fn target_triple_matches_the_release_build_matrix(
    #[case] platform: Platform,
    #[case] expected: &str,
) {
    assert_eq!(platform.target_triple(), expected);
}

#[rstest]
#[case(Platform::Linux64, "pixi-sandbox-x86_64-unknown-linux-musl")]
#[case(Platform::LinuxAarch64, "pixi-sandbox-aarch64-unknown-linux-musl")]
#[case(Platform::Osx64, "pixi-sandbox-x86_64-apple-darwin")]
#[case(Platform::OsxArm64, "pixi-sandbox-aarch64-apple-darwin")]
#[case(Platform::Win64, "pixi-sandbox-x86_64-pc-windows-msvc.exe")]
fn asset_name_matches_the_published_release_assets(
    #[case] platform: Platform,
    #[case] expected: &str,
) {
    assert_eq!(platform.asset_name(), expected);
}

#[rstest]
#[case(Platform::Linux64, Some("ubuntu-latest"))]
#[case(Platform::OsxArm64, Some("macos-14"))]
#[case(Platform::Osx64, Some("macos-13"))]
#[case(Platform::Win64, Some("windows-latest"))]
// No GitHub-hosted runner exists for linux-aarch64; a project publishing it must supply an
// explicit `runners.linux-aarch64` override (sandbox_config.rs), so there is no safe default.
#[case(Platform::LinuxAarch64, None)]
fn gh_runner_matches_the_hosted_default_or_is_absent(
    #[case] platform: Platform,
    #[case] expected: Option<&str>,
) {
    assert_eq!(platform.gh_runner(), expected);
}

#[test]
fn display_matches_as_str() {
    for platform in Platform::ALL {
        assert_eq!(platform.to_string(), platform.as_str());
    }
}

#[test]
fn from_str_parses_every_display_form() {
    for platform in Platform::ALL {
        assert_eq!(platform.as_str().parse::<Platform>().unwrap(), platform);
    }
}

#[test]
fn from_str_rejects_garbage() {
    assert!("not-a-platform".parse::<Platform>().is_err());
}

#[test]
fn all_lists_every_variant_exactly_once() {
    let mut seen = Platform::ALL.to_vec();
    seen.sort_by_key(|p| p.as_str());
    seen.dedup();
    assert_eq!(
        seen.len(),
        Platform::ALL.len(),
        "Platform::ALL has a duplicate"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// Every member of `Platform::ALL`, sampled by the strategy above rather than a hand-typed
    /// list, survives `as_str` then `FromStr`, and its asset name always carries the exact
    /// prefix/suffix convention the release matrix publishes under.
    #[test]
    fn platform_as_str_and_from_str_agree(platform in any_platform()) {
        prop_assert_eq!(platform.as_str().parse::<Platform>().unwrap(), platform);
        prop_assert!(platform.asset_name().starts_with("pixi-sandbox-"));
        prop_assert_eq!(platform.asset_name().ends_with(".exe"), platform == Platform::Win64);
    }
}
