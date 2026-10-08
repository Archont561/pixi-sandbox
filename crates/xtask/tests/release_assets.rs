//! `release_assets` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

use pixi_sandbox_core::shard::sha256_file;
use std::fs;
use std::path::{Path, PathBuf};
use xtask::release_assets::*;
use xtask::release_assets::{SUMS_NAME, built_binary_path, host_triple};

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, bytes).expect("write");
}

#[test]
fn staged_names_carry_the_exe_suffix_only_for_windows_targets() {
    assert_eq!(
        staged_name("x86_64-unknown-linux-musl"),
        "pixi-sandbox-x86_64-unknown-linux-musl"
    );
    assert_eq!(
        staged_name("aarch64-apple-darwin"),
        "pixi-sandbox-aarch64-apple-darwin"
    );
    assert_eq!(
        staged_name("x86_64-pc-windows-msvc"),
        "pixi-sandbox-x86_64-pc-windows-msvc.exe"
    );
}

/// `staged_name`/`binary_name` are pure string transforms over *any* target triple (see
/// the doc comment on the next test for why that stays true rather than narrowing to the
/// five published platforms). This ranges over realistic triple components — never the
/// literal word `windows` unless the case means to be a Windows target — and checks the
/// one invariant both functions exist to guarantee: the `.exe` suffix appears exactly when
/// the triple's OS component is `windows`, never otherwise. Bounded to 128 cases.
mod staged_and_binary_name_properties {
    use proptest::prelude::*;
    use xtask::release_assets::{binary_name, staged_name};

    /// Components a real target triple is built from, deliberately excluding "windows" so
    /// the non-Windows branch can never accidentally spell it.
    fn triple_component() -> impl Strategy<Value = String> {
        prop::sample::select(
            [
                "x86_64", "aarch64", "i686", "unknown", "pc", "apple", "gnu", "musl", "msvc",
                "linux", "darwin", "freebsd",
            ]
            .as_slice(),
        )
        .prop_map(str::to_string)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn the_exe_suffix_appears_exactly_when_the_os_component_is_windows(
            arch in triple_component(),
            vendor in triple_component(),
            non_windows_os in triple_component(),
            windows in any::<bool>(),
        ) {
            let os = if windows { "windows".to_string() } else { non_windows_os };
            let target = format!("{arch}-{vendor}-{os}");

            let staged = staged_name(&target);
            prop_assert!(staged.starts_with("pixi-sandbox-"), "{}", staged);
            prop_assert_eq!(staged.ends_with(".exe"), windows, "{}", staged);
            prop_assert_eq!(
                staged.clone(),
                format!("pixi-sandbox-{target}{}", if windows { ".exe" } else { "" })
            );

            let binary = binary_name(&target);
            prop_assert_eq!(binary == "pixi-sandbox.exe", windows, "{}", binary);
            prop_assert_eq!(binary == "pixi-sandbox", !windows, "{}", binary);
        }
    }
}

/// `staged_name` stays pure over *any* target triple on purpose (D10: testable without a
/// Windows host, and tolerant of a target this project does not yet publish for). It is
/// deliberately not migrated onto `Platform` (task-55) the way `conda_platforms.rs` and
/// `airlock.rs` were (task-59) — doing so would narrow it to the five known platforms. This
/// test is the structural guarantee that keeps the two independent implementations from
/// drifting apart for the platforms they do share: run it instead of a doc comment's promise.
#[test]
fn staged_name_agrees_with_platform_asset_name_for_every_known_platform() {
    use pixi_sandbox_core::platform::Platform;

    for platform in Platform::ALL {
        assert_eq!(
            staged_name(platform.target_triple()),
            platform.asset_name(),
            "staged_name({:?}) must agree with Platform::asset_name for {platform:?}",
            platform.target_triple()
        );
    }
}

#[test]
fn built_binaries_nest_the_triple_directory_only_for_explicit_targets() {
    assert_eq!(
        built_binary_path(
            Path::new("target"),
            Some("x86_64-unknown-linux-musl"),
            "irrelevant"
        ),
        PathBuf::from("target/x86_64-unknown-linux-musl/release/pixi-sandbox")
    );
    assert_eq!(
        built_binary_path(
            Path::new("target"),
            Some("x86_64-pc-windows-msvc"),
            "irrelevant"
        ),
        PathBuf::from("target/x86_64-pc-windows-msvc/release/pixi-sandbox.exe")
    );
    // A host build: the triple names the file (exe on a Windows host) but cargo puts it
    // straight under target/release/, with no triple directory.
    assert_eq!(
        built_binary_path(Path::new("target"), None, "x86_64-pc-windows-msvc"),
        PathBuf::from("target/release/pixi-sandbox.exe")
    );
    assert_eq!(
        built_binary_path(Path::new("target"), None, "x86_64-unknown-linux-gnu"),
        PathBuf::from("target/release/pixi-sandbox")
    );
}

#[test]
fn a_host_build_stages_from_target_release_under_the_host_asset_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target_dir = dir.path().join("target");
    write(
        &target_dir.join("release/pixi-sandbox"),
        b"host build payload",
    );
    let out = dir.path().join("out");
    fs::create_dir_all(&out).expect("mkdir");

    let staged = stage_release_binary(None, &target_dir, &out, "true").expect("host staging");

    let expected_name = format!("pixi-sandbox-{}", host_triple().expect("rustc"));
    assert_eq!(staged, out.join(expected_name));
    assert_eq!(fs::read(&staged).expect("read"), b"host build payload");
}

#[test]
fn staging_copies_under_the_asset_name_and_keeps_it_executable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target_dir = dir.path().join("target");
    write(
        &target_dir.join("x86_64-unknown-linux-musl/release/pixi-sandbox"),
        b"#!/bin/true\nfake ELF payload",
    );
    let out = dir.path().join("out");
    fs::create_dir_all(&out).expect("mkdir");

    let staged = stage_release_binary(Some("x86_64-unknown-linux-musl"), &target_dir, &out, "true")
        .expect("staging succeeds");

    assert_eq!(staged, out.join("pixi-sandbox-x86_64-unknown-linux-musl"));
    assert_eq!(
        fs::read(&staged).expect("read"),
        b"#!/bin/true\nfake ELF payload"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&staged).expect("stat").permissions().mode();
        assert_ne!(mode & 0o111, 0, "a staged binary must stay executable");
    }
}

#[test]
fn a_failing_strip_is_not_fatal_because_the_release_profile_already_strips() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target_dir = dir.path().join("target");
    write(
        &target_dir.join("aarch64-apple-darwin/release/pixi-sandbox"),
        b"mach-o payload",
    );
    let staged = stage_release_binary(
        Some("aarch64-apple-darwin"),
        &target_dir,
        dir.path(),
        "false", // exits non-zero: strip must be advisory, never fatal
    )
    .expect("a failing strip must not fail the release");
    assert!(staged.is_file());
}

#[test]
fn a_missing_binary_is_an_error_naming_the_path_and_the_remedy() {
    let dir = tempfile::tempdir().expect("tempdir");
    let error = stage_release_binary(
        Some("aarch64-apple-darwin"),
        &dir.path().join("target"),
        dir.path(),
        "true",
    )
    .expect_err("nothing was built");
    let message = format!("{error:#}");
    assert!(
        message.contains("release binary not found"),
        "must say what is missing: {message}"
    );
    assert!(
        message.contains("aarch64-apple-darwin"),
        "must name the target: {message}"
    );
    assert!(
        message.contains("build-release-binary"),
        "must name the remedy: {message}"
    );
}

#[test]
fn checksums_cover_every_platform_including_the_ones_the_old_glob_dropped() {
    let dir = tempfile::tempdir().expect("tempdir");
    // All five release platforms, exactly the asset set the GitHub Release uploads — the
    // old `*-unknown-*` glob checksummed only the two musl lines of this set.
    for (name, bytes) in [
        (
            "pixi-sandbox-x86_64-unknown-linux-musl",
            b"linux x86_64" as &[u8],
        ),
        ("pixi-sandbox-aarch64-unknown-linux-musl", b"linux aarch64"),
        ("pixi-sandbox-x86_64-apple-darwin", b"darwin x86_64"),
        ("pixi-sandbox-aarch64-apple-darwin", b"darwin aarch64"),
        ("pixi-sandbox-x86_64-pc-windows-msvc.exe", b"windows"),
    ] {
        write(&dir.path().join(name), bytes);
    }

    let sums = release_checksums(dir.path()).expect("checksums succeed");

    let body = fs::read_to_string(&sums).expect("read sums");
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines.len(), 5, "every platform binary gets a line");
    let names: Vec<&str> = lines
        .iter()
        .map(|line| line.split_once("  ").expect("sha256sum line format").1)
        .collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted, "lines must be sorted by asset name");
    for line in lines {
        let (digest, name) = line
            .split_once("  ")
            .expect("sha256sum line format: two spaces");
        assert_eq!(digest.len(), 64, "lowercase hex sha256: {digest}");
        let expected = sha256_file(&dir.path().join(name)).expect("digest of the same file");
        assert_eq!(digest, expected, "digest must match the file bytes");
    }
}

#[test]
fn conda_packages_are_not_checksummer() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        &dir.path().join("pixi-sandbox-0.3.7-h4778241_0.conda"),
        b"conda payload",
    );
    write(
        &dir.path().join("pixi-sandbox-x86_64-unknown-linux-musl"),
        b"binary",
    );

    let sums = release_checksums(dir.path()).expect("checksums succeed");

    let body = fs::read_to_string(&sums).expect("read sums");
    assert_eq!(body.lines().count(), 1, "only the standalone binary");
    assert!(
        body.contains("pixi-sandbox-x86_64-unknown-linux-musl"),
        "the binary is the one covered: {body}"
    );
}

#[test]
fn an_empty_directory_is_an_error_not_an_empty_sums_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let error = release_checksums(dir.path()).expect_err("nothing to checksum");
    let message = format!("{error:#}");
    assert!(
        message.contains("no standalone release binaries"),
        "must say the matrix produced nothing: {message}"
    );
    assert!(
        !dir.path().join(SUMS_NAME).exists(),
        "the shell this replaced wrote an empty file and exited 0"
    );
}

#[test]
fn the_completeness_check_names_the_binary_a_tampered_sums_file_omits() {
    let dir = tempfile::tempdir().expect("tempdir");
    let first = dir.path().join("pixi-sandbox-x86_64-apple-darwin");
    let second = dir.path().join("pixi-sandbox-x86_64-unknown-linux-musl");
    write(&first, b"darwin");
    write(&second, b"linux");
    let sums = dir.path().join(SUMS_NAME);
    fs::write(&sums, "0000000000000000000000000000000000000000000000000000000000000000  pixi-sandbox-x86_64-apple-darwin\n")
        .expect("write sums");

    let error = verify_completeness(&sums, &[first, second]).expect_err("one is missing");
    let message = format!("{error:#}");
    assert!(
        message.contains("pixi-sandbox-x86_64-unknown-linux-musl"),
        "must name the uncovered binary: {message}"
    );
    assert!(
        message.contains("missing checksum"),
        "must say what is wrong: {message}"
    );
}

#[test]
fn stale_extra_lines_in_the_sums_file_fail_the_completeness_check() {
    let dir = tempfile::tempdir().expect("tempdir");
    let binary = dir.path().join("pixi-sandbox-x86_64-apple-darwin");
    write(&binary, b"darwin");
    let sums = dir.path().join(SUMS_NAME);
    fs::write(
        &sums,
        "0000000000000000000000000000000000000000000000000000000000000000  pixi-sandbox-x86_64-apple-darwin\n\
         0000000000000000000000000000000000000000000000000000000000000000  pixi-sandbox-deleted-target\n",
    )
    .expect("write sums");

    let error = verify_completeness(&sums, &[binary]).expect_err("stale line");
    assert!(
        format!("{error:#}").contains("stale line"),
        "must flag the surplus: {error:#}"
    );
}
