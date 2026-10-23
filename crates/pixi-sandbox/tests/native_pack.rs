//! Native pack writer and installation at the public CLI and library seams (TASK-125).
mod support;

use rstest::rstest;
use support::native_transport_fixture;
use support::{
    NativePackFixture, file_tree_fixture, isolated_bin_fixture, isolated_home,
    native_locked_project_fixture, native_pack_fixture, native_wheel_pack_fixture,
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

#[rstest]
fn a_locked_project_packs_without_external_helpers(
    native_locked_project_fixture: NativePackFixture,
    isolated_bin_fixture: fn(&std::path::Path) -> assert_cmd::Command,
) {
    let fixture = native_locked_project_fixture;
    let output = fixture.home.path().join("transport");
    isolated_bin_fixture(fixture.home.path())
        .env("PATH", "")
        .args(["pack", "--repo-root"])
        .arg(&fixture.pack)
        .arg("--output-dir")
        .arg(&output)
        .args(["--envs", "default", "--platform", support::host_platform()])
        .assert()
        .success();
    assert_eq!(
        std::fs::read(
            output.join(".pixi-sandbox/envs/default/pack/pypi/native_demo-1.0-py3-none-any.whl")
        )
        .unwrap(),
        std::fs::read(
            fixture
                .home
                .path()
                .join("native-pack/pypi/native_demo-1.0-py3-none-any.whl")
        )
        .unwrap()
    );
    assert_eq!(
        std::fs::read(
            output.join(
                ".pixi-sandbox/envs/default/pack/channel/noarch/native-fixture-1.0-0.tar.bz2"
            )
        )
        .unwrap(),
        std::fs::read(
            fixture
                .home
                .path()
                .join("native-pack/channel/noarch/native-fixture-1.0-0.tar.bz2")
        )
        .unwrap()
    );
    let manifest = pixi_sandbox_core::manifest::Manifest::load(
        &pixi_sandbox_core::manifest::Manifest::path_in(&output),
    )
    .unwrap();
    assert_eq!(manifest.schema, pixi_sandbox_core::manifest::SCHEMA_VERSION);
    assert!(
        manifest.tools.is_empty(),
        "native packing must not implicitly bundle tools"
    );
    let restored = fixture.home.path().join("restored");
    std::fs::create_dir(&restored).unwrap();
    isolated_bin_fixture(fixture.home.path())
        .env("PATH", "")
        .args(["restore", "--branch-location"])
        .arg(&output)
        .arg("--output-path")
        .arg(&restored)
        .args(["--user-tools", "skip", "--no-vendor"])
        .assert()
        .success();
    assert_eq!(
        std::fs::read(restored.join(".pixi/envs/default/share/hello.txt")).unwrap(),
        b"native fixture\n"
    );
    assert!(
        restored
            .join(".pixi/envs/default/conda-meta/pixi_env_prefix")
            .is_file()
    );
    let raw = fixture.home.path().join("raw-prefix");
    isolated_bin_fixture(fixture.home.path())
        .env("PATH", "")
        .args(["unpack", "--input-dir"])
        .arg(&output)
        .arg("--output-dir")
        .arg(&raw)
        .assert()
        .success();
    assert_eq!(
        std::fs::read(raw.join("share/hello.txt")).unwrap(),
        b"native fixture\n"
    );
}

#[rstest]
fn an_empty_locked_environment_is_transportable(
    native_locked_project_fixture: NativePackFixture,
    isolated_bin_fixture: fn(&std::path::Path) -> assert_cmd::Command,
) {
    let fixture = native_locked_project_fixture;
    let path = fixture.pack.join("pixi.lock");
    let mut lock: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    lock["environments"]["default"]["packages"][support::host_platform()] = serde_json::json!([]);
    lock["packages"] = serde_json::json!([]);
    std::fs::write(path, serde_json::to_vec(&lock).unwrap()).unwrap();
    let output = fixture.home.path().join("empty-transport");
    isolated_bin_fixture(fixture.home.path())
        .env("PATH", "")
        .args(["pack", "--repo-root"])
        .arg(&fixture.pack)
        .arg("--output-dir")
        .arg(&output)
        .args(["--envs", "default", "--platform", support::host_platform()])
        .assert()
        .success();
    let prefix = fixture.home.path().join("empty-prefix");
    isolated_bin_fixture(fixture.home.path())
        .env("PATH", "")
        .args(["unpack", "--input-dir"])
        .arg(output)
        .arg("--output-dir")
        .arg(&prefix)
        .assert()
        .success();
    assert!(prefix.join("conda-meta/history").is_file());
}

#[rstest]
#[case("--fetch-tools")]
#[case("--tools-lock")]
#[case("--tools-cache")]
fn pack_no_longer_advertises_helper_management(
    isolated_bin_fixture: fn(&std::path::Path) -> assert_cmd::Command,
    isolated_home: tempfile::TempDir,
    #[case] flag: &str,
) {
    let output = isolated_bin_fixture(isolated_home.path())
        .args(["pack", "--help"])
        .assert()
        .success();
    let text = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        !text.contains(flag),
        "retired flag still advertised: {flag}"
    );
}

#[rstest]
fn the_retired_tools_subcommand_is_rejected(
    isolated_bin_fixture: fn(&std::path::Path) -> assert_cmd::Command,
    isolated_home: tempfile::TempDir,
) {
    isolated_bin_fixture(isolated_home.path())
        .args(["tools", "list"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("unrecognized subcommand"));
}

#[rstest]
fn invalid_locked_selections_leave_no_transport_directory(
    native_locked_project_fixture: NativePackFixture,
    isolated_bin_fixture: fn(&std::path::Path) -> assert_cmd::Command,
) {
    let fixture = native_locked_project_fixture;
    std::fs::write(
        fixture.pack.join("pixi.lock"),
        "version: 7\nenvironments: {}\npackages: []\n",
    )
    .unwrap();
    let output = fixture.home.path().join("refused-transport");
    isolated_bin_fixture(fixture.home.path())
        .env("PATH", "")
        .args(["pack", "--repo-root"])
        .arg(fixture.pack)
        .arg("--output-dir")
        .arg(&output)
        .args(["--envs", "default", "--platform", support::host_platform()])
        .assert()
        .failure();
    assert!(!output.exists(), "selection preflight wrote output");
}

#[rstest]
#[case::restore(false)]
#[case::unpack(true)]
fn a_verified_but_incorrect_oracle_cannot_replace_an_existing_environment(
    native_transport_fixture: NativePackFixture,
    isolated_bin_fixture: fn(&std::path::Path) -> assert_cmd::Command,
    #[case] raw_unpack: bool,
) {
    use pixi_sandbox_core::{
        files_manifest::FilesDoc,
        manifest::{MANIFEST_DIR, Manifest},
        shard,
    };
    let fixture = native_transport_fixture;
    let manifest_path = Manifest::path_in(&fixture.pack);
    let mut manifest = Manifest::load(&manifest_path).unwrap();
    let files = manifest
        .envs
        .get_mut("default")
        .unwrap()
        .files
        .as_mut()
        .unwrap();
    let path = fixture.pack.join(MANIFEST_DIR).join(&files.blob.path);
    let mut doc = FilesDoc::parse(&std::fs::read(&path).unwrap()).unwrap();
    doc.files
        .iter_mut()
        .find(|entry| entry.p == "share/hello.txt")
        .unwrap()
        .h = Some("0".repeat(64));
    let bytes = doc.to_bytes().unwrap();
    std::fs::write(&path, &bytes).unwrap();
    files.blob.sha256 = shard::sha256_bytes(&bytes);
    files.blob.size = bytes.len() as u64;
    std::fs::write(manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    let project = fixture.home.path().join("receiver");
    let prefix = project.join(".pixi/envs/default");
    std::fs::create_dir_all(&prefix).unwrap();
    std::fs::write(prefix.join("keep.txt"), b"previous environment").unwrap();
    let mut command = isolated_bin_fixture(fixture.home.path());
    command.env("PATH", "");
    if raw_unpack {
        command
            .args(["unpack", "--input-dir"])
            .arg(&fixture.pack)
            .arg("--output-dir")
            .arg(&prefix)
            .arg("--force");
    } else {
        command
            .args(["restore", "--branch-location"])
            .arg(&fixture.pack)
            .arg("--output-path")
            .arg(&project)
            .args(["--force", "--no-vendor", "--user-tools", "skip"]);
    }
    command.assert().failure();
    assert_eq!(
        std::fs::read(prefix.join("keep.txt")).unwrap(),
        b"previous environment"
    );
    assert!(!prefix.join("share/hello.txt").exists());
}

#[rstest]
#[case(1)]
#[case(2)]
fn legacy_manifest_schemas_can_restore_a_real_pack_without_external_helpers(
    native_transport_fixture: NativePackFixture,
    isolated_bin_fixture: fn(&std::path::Path) -> assert_cmd::Command,
    #[case] schema: u32,
) {
    use pixi_sandbox_core::manifest::Manifest;
    let fixture = native_transport_fixture;
    let path = Manifest::path_in(&fixture.pack);
    let mut manifest = Manifest::load(&path).unwrap();
    manifest.schema = schema;
    std::fs::write(path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    let project = fixture.home.path().join("receiver");
    std::fs::create_dir(&project).unwrap();
    isolated_bin_fixture(fixture.home.path())
        .env("PATH", "")
        .args(["restore", "--branch-location"])
        .arg(&fixture.pack)
        .arg("--output-path")
        .arg(&project)
        .args(["--no-vendor", "--user-tools", "skip"])
        .assert()
        .success();
    assert_eq!(
        std::fs::read(project.join(".pixi/envs/default/share/hello.txt")).unwrap(),
        b"native fixture\n"
    );
    let raw = fixture.home.path().join("raw");
    isolated_bin_fixture(fixture.home.path())
        .env("PATH", "")
        .args(["unpack", "--input-dir"])
        .arg(fixture.pack)
        .arg("--output-dir")
        .arg(&raw)
        .assert()
        .success();
    assert_eq!(
        std::fs::read(raw.join("share/hello.txt")).unwrap(),
        b"native fixture\n"
    );
}

// TASK-128: characterize the existing command before moving its promotion policy.
#[rstest]
#[case::absent(false, false, false)]
#[case::empty(true, false, false)]
#[case::refused(true, true, false)]
#[case::forced(true, true, true)]
fn unpack_preserves_destination_replacement_policy(
    native_pack_fixture: NativePackFixture,
    isolated_bin_fixture: fn(&std::path::Path) -> assert_cmd::Command,
    #[case] existing: bool,
    #[case] populated: bool,
    #[case] force: bool,
) {
    let fixture = native_pack_fixture;
    let output = fixture.home.path().join("prefix");
    if existing {
        std::fs::create_dir(&output).unwrap();
    }
    if populated {
        std::fs::write(output.join("keep.txt"), b"previous environment").unwrap();
    }
    let mut command = isolated_bin_fixture(fixture.home.path());
    command
        .env("PATH", "")
        .args(["unpack", "--input-dir"])
        .arg(&fixture.pack)
        .arg("--output-dir")
        .arg(&output);
    if force {
        command.arg("--force");
    }
    if populated && !force {
        command.assert().failure().stderr(predicates::str::contains(
            "already exists and is not empty — pass --force to replace it",
        ));
        assert_eq!(
            std::fs::read(output.join("keep.txt")).unwrap(),
            b"previous environment"
        );
        assert!(!output.join("share/hello.txt").exists());
    } else {
        command.assert().success();
        assert_eq!(
            std::fs::read(output.join("share/hello.txt")).unwrap(),
            b"native fixture\n"
        );
        assert!(!output.join("keep.txt").exists());
        assert!(!output.join("conda-meta/pixi_env_prefix").exists());
    }
}

#[rstest]
#[case("0", None, 0, "generic")]
#[case("hfixture_102", None, 102, "generic")]
#[case("7", None, 7, "generic")]
#[case("20_gnu", Some(20), 20, "generic")]
#[case("custom", None, 0, "generic")]
// stale-ref-allowed: Conda runtime noarch kind, not the retired prototype.
#[case("pyhd_5", None, 5, "python")]
fn compact_locks_preserve_build_numbers_and_noarch_in_the_local_channel(
    native_locked_project_fixture: NativePackFixture,
    #[case] build: &str,
    #[case] explicit_number: Option<u64>,
    #[case] expected_number: u64,
    #[case] expected_noarch: &str,
) {
    let fixture = native_locked_project_fixture;
    let channel = support::crate_dir().join("tests/fixtures/compact-lock/channel/noarch");
    let repodata: serde_json::Value =
        serde_json::from_slice(&std::fs::read(channel.join("repodata.json")).unwrap()).unwrap();
    let filename = format!("compact-fixture-1.0-{build}.tar.bz2");
    let location = url::Url::from_file_path(channel.join(&filename))
        .unwrap()
        .to_string();
    let mut record = repodata["packages"][&filename].clone();
    let fields = record.as_object_mut().unwrap();
    for field in [
        "name",
        "version",
        "build",
        "build_number",
        "subdir",
        "noarch",
    ] {
        fields.remove(field);
    }
    if let Some(number) = explicit_number {
        record["build_number"] = number.into();
    }
    record["conda"] = location.clone().into();
    std::fs::write(fixture.pack.join("pixi.lock"), serde_json::to_vec(&serde_json::json!({
        "version": 7,
        "environments": {"default": {"packages": {support::host_platform(): [{"conda": location}]}}},
        "packages": [record],
    })).unwrap()).unwrap();
    let output = fixture.home.path().join("compact-pack");
    pixi_sandbox::native_pack::write(&fixture.pack, "default", support::host_platform(), &output)
        .unwrap();
    pixi_sandbox::native_pack::verify(&output).unwrap();
    let actual: serde_json::Value = serde_json::from_slice(
        &std::fs::read(output.join("channel/noarch/repodata.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        actual["packages"][&filename]["build_number"],
        expected_number
    );
    assert_eq!(actual["packages"][&filename]["noarch"], expected_noarch);
}

/// Rewrite every locked Conda location through `rewrite`, in the records and in the selected
/// environment alike, and leave the lock as the only thing the writer sees.
fn rewrite_conda_locations(project: &std::path::Path, rewrite: impl Fn(&str) -> String) {
    let path = project.join("pixi.lock");
    let mut lock: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let mut replacements = std::collections::BTreeMap::new();
    for record in lock["packages"].as_array_mut().unwrap() {
        if let Some(location) = record["conda"].as_str().map(str::to_string) {
            replacements.insert(location.clone(), rewrite(&location));
        }
    }
    for record in lock["packages"].as_array_mut().unwrap() {
        if let Some(location) = record["conda"].as_str().map(str::to_string)
            && let Some(new) = replacements.get(&location)
        {
            record["conda"] = new.clone().into();
        }
    }
    for environment in lock["environments"].as_object_mut().unwrap().values_mut() {
        for packages in environment["packages"]
            .as_object_mut()
            .unwrap()
            .values_mut()
        {
            for entry in packages.as_array_mut().unwrap() {
                if let Some(location) = entry["conda"].as_str().map(str::to_string)
                    && let Some(new) = replacements.get(&location)
                {
                    entry["conda"] = new.clone().into();
                }
            }
        }
    }
    std::fs::write(&path, serde_json::to_vec_pretty(&lock).unwrap()).unwrap();
}

/// Serve `files` once each over loopback HTTP, so an `http://` mirror can be exercised
/// without touching a network the tests are not allowed to use.
fn serve_http_files(
    files: std::collections::BTreeMap<String, Vec<u8>>,
) -> (String, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        let mut served = 0;
        while served < files.len() && std::time::Instant::now() < deadline {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            };
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            let mut request = [0_u8; 2048];
            let read = stream.read(&mut request).unwrap_or(0);
            let line = String::from_utf8_lossy(&request[..read])
                .lines()
                .next()
                .unwrap_or_default()
                .to_string();
            let path = line
                .split_whitespace()
                .nth(1)
                .unwrap_or("/")
                .trim_start_matches('/')
                .to_string();
            match files.get(&path) {
                Some(bytes) => {
                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        bytes.len()
                    );
                    stream.write_all(header.as_bytes()).unwrap();
                    stream.write_all(bytes).unwrap();
                    served += 1;
                }
                None => stream
                    .write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .unwrap(),
            }
        }
    });
    (format!("http://127.0.0.1:{port}"), handle)
}

// TASK-132: pixi writes a bare absolute path for a `file://` channel, so the writer must read
// that shape rather than demanding a URL it never produced.
#[rstest]
fn a_lock_with_bare_absolute_paths_packs_and_is_not_rewritten(
    native_locked_project_fixture: NativePackFixture,
) {
    let fixture = native_locked_project_fixture;
    rewrite_conda_locations(&fixture.pack, |location| {
        url::Url::parse(location)
            .unwrap()
            .to_file_path()
            .unwrap()
            .display()
            .to_string()
    });
    let lock_before = std::fs::read(fixture.pack.join("pixi.lock")).unwrap();
    let output = fixture.home.path().join("bare-path-pack");
    pixi_sandbox::native_pack::write(&fixture.pack, "default", support::host_platform(), &output)
        .unwrap();
    pixi_sandbox::native_pack::verify(&output).unwrap();
    assert_eq!(
        std::fs::read(fixture.pack.join("pixi.lock")).unwrap(),
        lock_before,
        "packing rewrote the consumer's lock"
    );
    assert!(
        output
            .join("channel/noarch/native-fixture-1.0-0.tar.bz2")
            .is_file()
    );
}

// TASK-132: an unsupported scheme must name the entry and the forms that would work, rather
// than reporting only the scheme's name.
#[rstest]
fn an_unsupported_scheme_names_the_entry_and_the_accepted_forms(
    native_locked_project_fixture: NativePackFixture,
) {
    let fixture = native_locked_project_fixture;
    rewrite_conda_locations(&fixture.pack, |location| {
        let name = location.rsplit('/').next().unwrap();
        format!("ftp://mirror.invalid/noarch/{name}")
    });
    let output = fixture.home.path().join("refused-pack");
    let error = pixi_sandbox::native_pack::write(
        &fixture.pack,
        "default",
        support::host_platform(),
        &output,
    )
    .expect_err("an ftp artifact cannot be acquired");
    let text = format!("{error:#}");
    assert!(text.contains("ftp://mirror.invalid/noarch/"), "{text}");
    assert!(text.contains("file://"), "{text}");
    assert!(text.contains("https://"), "{text}");
    assert!(
        !output.exists(),
        "a refused selection created transport output"
    );
}

// TASK-132: a mirror served over plain HTTP is acquired and still digest-verified.
#[rstest]
fn an_http_mirror_is_acquired_and_digest_verified(
    native_locked_project_fixture: NativePackFixture,
) {
    let fixture = native_locked_project_fixture;
    // The archives the fixture's lock already points at, served under their channel subdir.
    let lock: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture.pack.join("pixi.lock")).unwrap()).unwrap();
    let mut files = std::collections::BTreeMap::new();
    for record in lock["packages"].as_array().unwrap() {
        let Some(location) = record["conda"].as_str() else {
            continue;
        };
        let archive = url::Url::parse(location).unwrap().to_file_path().unwrap();
        let name = archive.file_name().unwrap().to_string_lossy().to_string();
        files.insert(format!("noarch/{name}"), std::fs::read(&archive).unwrap());
    }
    let (base, server) = serve_http_files(files);
    rewrite_conda_locations(&fixture.pack, |location| {
        let name = location.rsplit('/').next().unwrap();
        format!("{base}/noarch/{name}")
    });
    let output = fixture.home.path().join("mirror-pack");
    pixi_sandbox::native_pack::write(&fixture.pack, "default", support::host_platform(), &output)
        .unwrap();
    pixi_sandbox::native_pack::verify(&output).unwrap();
    server.join().unwrap();
    assert!(
        output
            .join("channel/noarch/native-fixture-1.0-0.tar.bz2")
            .is_file()
    );
}

// TASK-132 AC#3: a bare path whose parent is not a channel subdir must be reported by name,
// not as a generic platform mismatch.
#[rstest]
fn a_flat_artifact_path_reports_the_missing_channel_subdir(
    native_locked_project_fixture: NativePackFixture,
) {
    let fixture = native_locked_project_fixture;
    let flat = fixture.home.path().join("flat-cache");
    std::fs::create_dir(&flat).unwrap();
    let archive = fixture
        .home
        .path()
        .join("native-pack/channel/noarch/native-fixture-1.0-0.tar.bz2");
    std::fs::copy(&archive, flat.join("native-fixture-1.0-0.tar.bz2")).unwrap();
    rewrite_conda_locations(&fixture.pack, |location| {
        let name = location.rsplit('/').next().unwrap();
        flat.join(name).display().to_string()
    });
    let output = fixture.home.path().join("flat-pack");
    let error = pixi_sandbox::native_pack::write(
        &fixture.pack,
        "default",
        support::host_platform(),
        &output,
    )
    .expect_err("an archive outside a channel subdir cannot be packed");
    let text = format!("{error:#}");
    assert!(text.contains("flat-cache"), "{text}");
    assert!(text.contains("<subdir>"), "{text}");
    assert!(
        !output.exists(),
        "a refused selection created transport output"
    );
}
