//! Native pack installation at the public CLI and library seams (TASK-125).
mod support;

use rstest::rstest;
use support::{
    NativePackFixture, file_tree_fixture, isolated_bin_fixture, native_pack_fixture,
    native_wheel_pack_fixture,
};

#[rstest]
fn a_real_conda_pack_unpacks_without_any_helper_on_path(
    native_pack_fixture: NativePackFixture,
    isolated_bin_fixture: fn(&std::path::Path) -> assert_cmd::Command,
) {
    let fixture = native_pack_fixture;
    let output = fixture.home.path().join("prefix");
    isolated_bin_fixture(fixture.home.path())
        .env("PATH", "")
        .arg("unpack")
        .arg("--input-dir")
        .arg(&fixture.pack)
        .arg("--output-dir")
        .arg(&output)
        .assert()
        .success();
    assert_eq!(
        std::fs::read(output.join("share/hello.txt")).unwrap(),
        b"native fixture\n"
    );
}

#[rstest]
fn wheels_are_installed_without_running_an_interpreter_or_a_helper(
    native_wheel_pack_fixture: NativePackFixture,
    isolated_bin_fixture: fn(&std::path::Path) -> assert_cmd::Command,
) {
    let fixture = native_wheel_pack_fixture;
    let output = fixture.home.path().join("prefix");
    isolated_bin_fixture(fixture.home.path())
        .env("PATH", "")
        .arg("unpack")
        .arg("--input-dir")
        .arg(&fixture.pack)
        .arg("--output-dir")
        .arg(&output)
        .assert()
        .success();
    let site = if cfg!(windows) {
        "Lib/site-packages"
    } else {
        // stale-ref-allowed: real runtime-package paths in the wheel fixture.
        "lib/python3.13/site-packages"
    };
    assert_eq!(
        std::fs::read(output.join(site).join("native_demo/__init__.py")).unwrap(),
        b"VALUE = 42\n"
    );
}

#[rstest]
fn a_corrupt_native_pack_cannot_delete_an_existing_force_destination(
    native_pack_fixture: NativePackFixture,
    isolated_bin_fixture: fn(&std::path::Path) -> assert_cmd::Command,
) {
    let fixture = native_pack_fixture;
    std::fs::write(
        fixture
            .pack
            .join("channel/noarch/native-fixture-1.0-0.tar.bz2"),
        b"corrupt",
    )
    .unwrap();
    let output = fixture.home.path().join("prefix");
    std::fs::create_dir(&output).unwrap();
    std::fs::write(output.join("keep.txt"), b"original").unwrap();
    isolated_bin_fixture(fixture.home.path())
        .env("PATH", "")
        .arg("unpack")
        .arg("--input-dir")
        .arg(&fixture.pack)
        .arg("--output-dir")
        .arg(&output)
        .arg("--force")
        .assert()
        .failure();
    assert_eq!(std::fs::read(output.join("keep.txt")).unwrap(), b"original");
}

#[rstest]
#[case::build_path("build", "../escape")]
#[case::different_name("name", "other")]
#[case::different_version("version", "2.0")]
fn inconsistent_package_identity_is_refused_before_creating_a_prefix(
    native_pack_fixture: NativePackFixture,
    #[case] field: &str,
    #[case] value: &str,
) {
    let fixture = native_pack_fixture;
    let metadata = fixture.pack.join("channel/noarch/repodata.json");
    let mut document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&metadata).unwrap()).unwrap();
    document["packages"]["native-fixture-1.0-0.tar.bz2"][field] = value.into();
    std::fs::write(metadata, serde_json::to_vec(&document).unwrap()).unwrap();
    let stage = fixture.home.path().join("stage");
    let destination = fixture.home.path().join("final");
    let cache = fixture.home.path().join("cache");
    let result = pixi_sandbox::native_pack::install(&fixture.pack, &stage, &destination, &cache);
    assert!(
        result.is_err(),
        "inconsistent package identity must not install"
    );
    assert!(!stage.exists(), "refuse before creating a prefix");
    assert!(!cache.exists(), "refuse before extracting anything");
}

#[rstest]
#[case::parent("../outside", false)]
#[case::absolute("/outside", false)]
#[case::windows_parent("..\\outside", false)]
#[case::symlink("native_demo/link", true)]
fn unsafe_wheel_members_are_refused_before_any_installation(
    native_wheel_pack_fixture: NativePackFixture,
    #[case] member: &str,
    #[case] symlink: bool,
) {
    use std::io::Write;
    let fixture = native_wheel_pack_fixture;
    let path = fixture.pack.join("pypi/native_demo-1.0-py3-none-any.whl");
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    let mut wheel = zip::ZipWriter::new_append(file).unwrap();
    let options = zip::write::SimpleFileOptions::default();
    if symlink {
        wheel.add_symlink(member, "../../outside", options).unwrap();
    } else {
        wheel.start_file(member, options).unwrap();
        wheel.write_all(b"unsafe entry").unwrap();
    }
    wheel.finish().unwrap();
    let stage = fixture.home.path().join("stage");
    let destination = fixture.home.path().join("final");
    let cache = fixture.home.path().join("cache");
    let error = pixi_sandbox::native_pack::install(&fixture.pack, &stage, &destination, &cache)
        .unwrap_err();
    assert!(
        error.to_string().contains("unsafe wheel entry"),
        "{error:#}"
    );
    assert!(!stage.exists());
    assert!(!cache.exists());
}

#[rstest]
fn a_native_install_leaves_the_source_pack_unchanged_and_uses_the_final_script_prefix(
    native_wheel_pack_fixture: NativePackFixture,
    file_tree_fixture: fn(&std::path::Path) -> support::FileTree,
) {
    let fixture = native_wheel_pack_fixture;
    let before = file_tree_fixture(&fixture.pack);
    let stage = fixture.home.path().join("stage");
    let destination = fixture.home.path().join("final");
    let cache = fixture.home.path().join("cache");
    pixi_sandbox::native_pack::install(&fixture.pack, &stage, &destination, &cache).unwrap();
    assert_eq!(before, file_tree_fixture(&fixture.pack));
    #[cfg(unix)]
    {
        let script = std::fs::read_to_string(stage.join("bin/native-demo")).unwrap();
        assert!(
            // stale-ref-allowed: real runtime-package paths in the wheel fixture.
            script.starts_with(&format!("#!{}/bin/python3.13\n", destination.display())),
            "{script}"
        );
        assert!(!script.contains(&*stage.to_string_lossy()));
    }
    assert!(!destination.exists(), "promotion belongs to the caller");
}

#[rstest]
fn a_broken_archive_with_a_matching_digest_cannot_replace_an_existing_prefix(
    native_pack_fixture: NativePackFixture,
    isolated_bin_fixture: fn(&std::path::Path) -> assert_cmd::Command,
) {
    let fixture = native_pack_fixture;
    let archive = fixture
        .pack
        .join("channel/noarch/native-fixture-1.0-0.tar.bz2");
    std::fs::write(&archive, b"not a Conda archive").unwrap();
    let metadata = fixture.pack.join("channel/noarch/repodata.json");
    let mut document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&metadata).unwrap()).unwrap();
    document["packages"]["native-fixture-1.0-0.tar.bz2"]["sha256"] =
        pixi_sandbox_core::shard::sha256_file(&archive)
            .unwrap()
            .into();
    std::fs::write(metadata, serde_json::to_vec(&document).unwrap()).unwrap();
    let output = fixture.home.path().join("prefix");
    std::fs::create_dir(&output).unwrap();
    std::fs::write(output.join("keep.txt"), b"original").unwrap();
    isolated_bin_fixture(fixture.home.path())
        .env("PATH", "")
        .args(["unpack", "--input-dir"])
        .arg(&fixture.pack)
        .arg("--output-dir")
        .arg(&output)
        .arg("--force")
        .assert()
        .failure();
    assert_eq!(std::fs::read(output.join("keep.txt")).unwrap(), b"original");
}

#[rstest]
#[case::parent("../outside")]
#[case::nested("nested/escape")]
#[case::windows_parent("..\\outside")]
fn unsafe_bare_environment_names_are_refused_before_staging(
    native_pack_fixture: NativePackFixture,
    isolated_bin_fixture: fn(&std::path::Path) -> assert_cmd::Command,
    #[case] environment: &str,
) {
    let fixture = native_pack_fixture;
    let output = fixture.home.path().join("prefix");
    isolated_bin_fixture(fixture.home.path())
        .env("PATH", "")
        .args(["unpack", "--input-dir"])
        .arg(&fixture.pack)
        .arg("--output-dir")
        .arg(&output)
        .args(["--env", environment])
        .assert()
        .failure()
        .stderr(predicates::str::contains("unsafe environment name"));
    assert!(!fixture.home.path().join(".pixi").exists());
    assert!(!output.exists());
}
