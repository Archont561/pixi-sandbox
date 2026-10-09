//! Cargo vendoring policy through the promoted library boundary.

mod support;

use pixi_sandbox::vendor::validate_vendorable_lockfile;
use rstest::{fixture, rstest};
use support::{demo_project, duplicate_source_project, isolated_home};
#[cfg(unix)]
use support::{pack_reference, synthetic_pack_fixture};
use tempfile::TempDir;

#[rstest]
fn cargo_vendor_requires_a_lockfile(isolated_home: TempDir) {
    let error = validate_vendorable_lockfile(isolated_home.path()).unwrap_err();
    assert!(
        format!("{error:#}").contains("--cargo-vendor needs a Cargo.lock"),
        "{error:#}"
    );
}

#[rstest]
fn duplicate_sources_name_the_crate_both_origins_and_the_remedies(
    duplicate_source_project: std::path::PathBuf,
) {
    let error = validate_vendorable_lockfile(&duplicate_source_project).unwrap_err();
    let message = format!("{error:#}");
    for required in [
        "itoa 1.0.15",
        "registry+https://github.com/rust-lang/crates.io-index",
        "git+file:///tmp/duplicate-source-gitdep",
        "make the versions differ",
        "drop one of the two dependencies",
        "<name>-<version>",
    ] {
        assert!(
            message.contains(required),
            "missing {required:?}: {message}"
        );
    }
}

#[rstest]
fn the_single_source_fixture_is_vendorable(demo_project: std::path::PathBuf) {
    validate_vendorable_lockfile(&demo_project).unwrap();
}

#[rstest]
#[case::same_source_twice(
    "version = 4\n[[package]]\nname = 'same'\nversion = '1.0.0'\nsource = 'registry+one'\n[[package]]\nname = 'same'\nversion = '1.0.0'\nsource = 'registry+one'\n"
)]
#[case::different_versions(
    "version = 4\n[[package]]\nname = 'same'\nversion = '1.0.0'\nsource = 'registry+one'\n[[package]]\nname = 'same'\nversion = '2.0.0'\nsource = 'git+two'\n"
)]
#[case::path_member(
    "version = 4\n[[package]]\nname = 'same'\nversion = '1.0.0'\n[[package]]\nname = 'same'\nversion = '1.0.0'\nsource = 'registry+one'\n"
)]
#[case::no_packages("version = 4\n")]
fn a_lockfile_without_a_source_collision_is_accepted(
    isolated_home: TempDir,
    #[case] lockfile: &str,
) {
    std::fs::write(isolated_home.path().join("Cargo.lock"), lockfile).unwrap();
    validate_vendorable_lockfile(isolated_home.path()).unwrap();
}

#[rstest]
fn a_malformed_lockfile_is_not_treated_as_an_empty_lock(isolated_home: TempDir) {
    std::fs::write(isolated_home.path().join("Cargo.lock"), "[[package\n").unwrap();
    let error = validate_vendorable_lockfile(isolated_home.path()).unwrap_err();
    assert!(format!("{error:#}").contains("parsing"), "{error:#}");
}

#[cfg(unix)]
#[rstest]
fn a_loose_vendor_tree_preserves_the_reference_bytes(
    vendor_fixture: VendorFixture,
    pack_reference: std::path::PathBuf,
) {
    let fixture = vendor_fixture.pack;
    let out = fixture.home.path().join("out");
    let payload = out.join(".pixi-sandbox");
    let (vendor, info) = pixi_sandbox::vendor::vendor_tree_with_toolchain(
        &fixture.project,
        &out,
        &payload,
        pixi_sandbox::vendor::VendorMode::Loose,
        &vendor_fixture.toolchain,
    )
    .unwrap();
    let vendor = vendor.unwrap();
    let expected = pack_reference.join("linux-loose/.pixi-sandbox");
    let expected_manifest =
        pixi_sandbox_core::manifest::Manifest::load(&expected.join("manifest.json")).unwrap();
    let expected_vendor = expected_manifest.vendor.unwrap();
    assert_eq!(
        support::file_tree(&payload.join("vendor")),
        support::file_tree(&expected.join("vendor"))
    );
    assert_eq!(vendor.mode, "loose");
    assert_eq!(vendor.crates, 1);
    assert_eq!(vendor.size_bytes, expected_vendor.size_bytes);
    assert_eq!(vendor.cargo_lock_sha256, expected_vendor.cargo_lock_sha256);
    assert_eq!(vendor.directory.as_deref(), Some(".pixi-sandbox/vendor"));
    assert!(
        vendor.blobs.is_empty(),
        "the pack coordinator records blobs afterwards"
    );
    let info = info.unwrap();
    assert_eq!(info.cargo, "cargo 1.90.0 (fixture)");
    assert_eq!(info.rustc, "rustc 1.90.0 (fixture)");
}

#[cfg(unix)]
struct VendorFixture {
    pack: support::SyntheticPackFixture,
    toolchain: pixi_sandbox::vendor::Toolchain,
}

#[cfg(unix)]
#[fixture]
fn vendor_fixture() -> VendorFixture {
    let fixture = synthetic_pack_fixture();
    let cargo = fixture.tools.join("cargo-wrapper");
    support::write_executable(
        &cargo,
        &format!(
            "#!/bin/sh\nexport VENDOR_FIXTURE=\"{}\"\nexec \"{}\" \"$@\"\n",
            support::fixture_transport()
                .join(".pixi-sandbox/vendor")
                .display(),
            fixture.tools.join("cargo").display(),
        ),
    );
    let toolchain = pixi_sandbox::vendor::Toolchain {
        cargo,
        rustc: fixture.tools.join("rustc"),
    };
    VendorFixture {
        pack: fixture,
        toolchain,
    }
}

#[cfg(unix)]
#[rstest]
fn tarballs_keep_each_crates_reference_bytes_and_remove_scratch(
    vendor_fixture: VendorFixture,
    pack_reference: std::path::PathBuf,
) {
    use std::io::Read;
    let fixture = vendor_fixture.pack;
    let out = fixture.home.path().join("out");
    let payload = out.join(".pixi-sandbox");
    let (vendor, _) = pixi_sandbox::vendor::vendor_tree_with_toolchain(
        &fixture.project,
        &out,
        &payload,
        pixi_sandbox::vendor::VendorMode::Tarballs,
        &vendor_fixture.toolchain,
    )
    .unwrap();
    let vendor = vendor.unwrap();
    assert_eq!(vendor.mode, "tarballs");
    assert_eq!(vendor.crates, 1);
    assert!(!out.join(".cargo-vendor-tmp").exists());
    let archive = std::fs::File::open(payload.join("vendor/demo-dep-1.0.0.tar")).unwrap();
    let mut archive = tar::Archive::new(archive);
    let mut actual = std::collections::BTreeMap::new();
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        if entry.header().entry_type().is_file() {
            let path = entry.path().unwrap().into_owned();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            actual.insert(path, bytes);
        }
    }
    let expected = support::file_tree(&pack_reference.join("linux-loose/.pixi-sandbox/vendor"))
        .into_iter()
        .map(|(path, (bytes, _))| (path, bytes))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(actual, expected);
}

#[cfg(unix)]
#[rstest]
fn a_collision_is_refused_before_the_vendor_tree_is_created(vendor_fixture: VendorFixture) {
    let fixture = vendor_fixture.pack;
    std::fs::copy(
        support::duplicate_source_project().join("Cargo.lock"),
        fixture.project.join("Cargo.lock"),
    )
    .unwrap();
    let out = fixture.home.path().join("out");
    let error = pixi_sandbox::vendor::vendor_tree_with_toolchain(
        &fixture.project,
        &out,
        &out.join(".pixi-sandbox"),
        pixi_sandbox::vendor::VendorMode::Loose,
        &vendor_fixture.toolchain,
    )
    .unwrap_err();
    assert!(
        format!("{error:#}").contains("more than one source"),
        "{error:#}"
    );
    assert!(
        !out.exists(),
        "a refused vendor operation must create nothing"
    );
}

#[cfg(unix)]
#[rstest]
fn cargo_producing_no_crates_is_a_failure(vendor_fixture: VendorFixture) {
    let fixture = vendor_fixture.pack;
    support::write_executable(&fixture.tools.join("cargo"), "#!/bin/sh\nmkdir -p \"$4\"\n");
    let out = fixture.home.path().join("out");
    let error = pixi_sandbox::vendor::vendor_tree_with_toolchain(
        &fixture.project,
        &out,
        &out.join(".pixi-sandbox"),
        pixi_sandbox::vendor::VendorMode::Loose,
        &vendor_fixture.toolchain,
    )
    .unwrap_err();
    assert!(
        format!("{error:#}").contains("cargo vendor produced no crate directories"),
        "{error:#}"
    );
}

#[cfg(unix)]
#[rstest]
#[case::cargo("cargo")]
#[case::rustc("rustc")]
fn unavailable_version_reporting_keeps_the_honest_unknown_label(
    vendor_fixture: VendorFixture,
    #[case] program: &str,
) {
    let fixture = vendor_fixture.pack;
    let executable = fixture.tools.join(program);
    let original = std::fs::read_to_string(&executable).unwrap();
    support::write_executable(
        &executable,
        &format!(
            "#!/bin/sh\nif [ \"${{1:-}}\" = --version ]; then exit 7; fi\n{}",
            original.strip_prefix("#!/bin/sh\n").unwrap(),
        ),
    );
    let out = fixture.home.path().join("out");
    let (_, info) = pixi_sandbox::vendor::vendor_tree_with_toolchain(
        &fixture.project,
        &out,
        &out.join(".pixi-sandbox"),
        pixi_sandbox::vendor::VendorMode::Loose,
        &vendor_fixture.toolchain,
    )
    .unwrap();
    let info = info.unwrap();
    match program {
        "cargo" => assert_eq!(info.cargo, "cargo (unknown)"),
        "rustc" => assert_eq!(info.rustc, "rustc (unknown)"),
        _ => unreachable!("named cases"),
    }
}
