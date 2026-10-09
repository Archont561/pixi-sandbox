//! Pack preflight and the per-file oracle, through the promoted library boundary.

mod support;

use pixi_sandbox::pack::plan_layout;
use rstest::rstest;
use support::isolated_home;
#[cfg(unix)]
use support::{pack_reference, synthetic_pack_fixture};
use tempfile::TempDir;

#[rstest]
fn a_missing_pixi_lock_is_refused_before_anything_is_created(isolated_home: TempDir) {
    let output = isolated_home.path().join("out");
    let error = plan_layout(isolated_home.path(), &output, &["default".into()], 95.0)
        .expect_err("no pixi.lock");
    assert!(format!("{error:#}").contains("pixi.lock"), "{error:#}");
    assert!(!output.exists(), "a refused plan must create nothing");
}

#[rstest]
fn an_existing_output_dir_is_refused(isolated_home: TempDir) {
    let root = isolated_home.path();
    std::fs::write(root.join("pixi.lock"), "").unwrap();
    let output = root.join("out");
    std::fs::create_dir_all(&output).unwrap();
    let error =
        plan_layout(root, &output, &["default".into()], 95.0).expect_err("stale output dir");
    assert!(format!("{error:#}").contains("already exists"), "{error:#}");
}

#[rstest]
#[case::unsafe_name(vec!["..".into()], "unsafe environment name")]
#[case::duplicate_name(vec!["default".into(), "default".into()], "requested more than once")]
fn unsafe_and_duplicate_env_names_are_refused(
    isolated_home: TempDir,
    #[case] envs: Vec<String>,
    #[case] refusal: &str,
) {
    let root = isolated_home.path();
    std::fs::write(root.join("pixi.lock"), "").unwrap();
    let output = root.join("out");
    let error = plan_layout(root, &output, &envs, 95.0).expect_err("refused selection");
    assert!(format!("{error:#}").contains(refusal), "{error:#}");
    assert!(!output.exists(), "a refused plan must create nothing");
}

#[rstest]
fn valid_args_resolve_to_absolute_paths_and_a_byte_shard_limit(isolated_home: TempDir) {
    let root = isolated_home.path();
    std::fs::write(root.join("pixi.lock"), "").unwrap();
    let output = root.join("out");
    let plan = plan_layout(root, &output, &["default".into(), "web".into()], 95.0).unwrap();
    assert!(plan.root.is_absolute(), "{}", plan.root.display());
    assert!(plan.out.is_absolute(), "{}", plan.out.display());
    assert_eq!(plan.payload, plan.out.join(".pixi-sandbox"));
    assert_eq!(plan.shard_limit, 99_614_720);
    assert!(!output.exists(), "a valid plan still creates nothing");
}

/// TASK-75 AC#2: the manifest section is resolved from a real config file, for the platform
/// being packed, and is `None` whenever there is nothing to carry.
mod host_requirements {
    use pixi_sandbox::pack::resolve_host_requirements;

    fn write_config(project: &std::path::Path, body: &str) -> std::path::PathBuf {
        let path = project.join("pixi-sandbox.toml");
        std::fs::write(&path, body).expect("config written");
        path
    }

    const BASE: &str = "schema = 1\n\n[[bundle]]\nname = \"app\"\nenvironments = [\"default\"]\nplatforms = [\"linux-64\"]\n";

    #[test]
    fn no_config_means_no_section() {
        assert_eq!(resolve_host_requirements(None, "linux-64").unwrap(), None);
    }

    #[test]
    fn a_config_without_the_table_means_no_section() {
        let project = tempfile::tempdir().unwrap();
        let config = write_config(project.path(), BASE);
        assert_eq!(
            resolve_host_requirements(Some(&config), "linux-64").unwrap(),
            None
        );
    }

    #[test]
    fn the_section_resolves_for_the_packed_platform() {
        let project = tempfile::tempdir().unwrap();
        let config = write_config(
            project.path(),
            &format!(
                "{BASE}\n[host_requirements]\nlibc = \"2.34\"\npackages = [\"fontconfig\"]\n\n\
                 [host_requirements.windows]\npackages = [\"vcredist\"]\n"
            ),
        );
        let linux = resolve_host_requirements(Some(&config), "linux-64")
            .unwrap()
            .expect("linux has a section");
        assert_eq!(linux.packages, ["fontconfig"].map(String::from));
        assert_eq!(linux.libc.as_deref(), Some("2.34"));

        // Another platform resolves its own family, not the one just packed.
        let windows = resolve_host_requirements(Some(&config), "win-64")
            .unwrap()
            .expect("windows resolves");
        assert_eq!(
            windows.packages,
            ["fontconfig", "vcredist"].map(String::from)
        );
    }

    #[test]
    fn a_family_with_no_requirements_of_its_own_carries_nothing() {
        let project = tempfile::tempdir().unwrap();
        let config = write_config(
            project.path(),
            &format!("{BASE}\n[host_requirements.osx]\npackages = [\"fontconfig\"]\n"),
        );
        assert_eq!(
            resolve_host_requirements(Some(&config), "linux-64").unwrap(),
            None,
            "a Linux transport must not carry a macOS-only declaration"
        );
    }

    #[test]
    fn an_invalid_config_is_refused_before_anything_is_written() {
        let project = tempfile::tempdir().unwrap();
        let config = write_config(
            project.path(),
            &format!("{BASE}\n[host_requirements]\ncapabilities = [\"dipslay\"]\n"),
        );
        // `{error:#}` walks the context chain: the durable half of the message is the
        // cause the config loader produced, not the path this call added.
        let error = format!(
            "{:#}",
            resolve_host_requirements(Some(&config), "linux-64").unwrap_err()
        );
        assert!(error.contains("dipslay"), "got {error}");

        let missing = project.path().join("absent.toml");
        let error = format!(
            "{:#}",
            resolve_host_requirements(Some(&missing), "linux-64").unwrap_err()
        );
        assert!(error.contains("absent.toml"), "got {error}");
    }
}

#[cfg(unix)]
#[rstest]
fn the_verification_unpack_keeps_the_pack_and_records_the_same_file_oracle(
    synthetic_pack_fixture: support::SyntheticPackFixture,
    pack_reference: std::path::PathBuf,
) {
    let fixture = synthetic_pack_fixture;
    let expected = pack_reference.join("linux-no-vendor/.pixi-sandbox");
    let source = fixture.home.path().join("source-pack");
    support::copy_tree(&expected.join("envs/default/pack"), &source);
    let before = support::file_tree(&source);
    let out = fixture.home.path().join("out");
    let payload = out.join(".pixi-sandbox");
    std::fs::create_dir_all(payload.join("envs/default")).unwrap();

    let unpacker = fixture.tools.join("cache-writing-unpacker");
    support::write_executable(
        &unpacker,
        &format!(
            "#!/bin/sh\nset -eu\ntest \"$TMPDIR\" = \"$TMP\"\ntest \"$TMP\" = \"$TEMP\"\nprintf cache > \"$1/cache-written-by-unpack\"\nexec \"{}\" \"$@\"\n",
            fixture.tools.join("pixi-unpack").display(),
        ),
    );
    let (oracle, unpacked_size) = pixi_sandbox::pack::build_files_oracle(
        &out,
        &payload,
        "default",
        &source,
        &unpacker,
        u64::MAX,
    )
    .unwrap();

    assert_eq!(oracle.entries, 1);
    assert_eq!(unpacked_size, 17);
    assert_eq!(
        std::fs::read(payload.join("envs/default/files.json")).unwrap(),
        std::fs::read(expected.join("envs/default/files.json")).unwrap(),
    );
    assert_eq!(
        support::file_tree(&source),
        before,
        "unpack must use a copy"
    );
    assert!(!out.join(".pixi-sandbox-verify-default").exists());
}

#[cfg(unix)]
#[rstest]
#[case::linux_no_vendor("linux-no-vendor", "linux-64", None, false, false)]
#[case::linux_loose("linux-loose", "linux-64", Some("loose"), true, true)]
#[case::windows_loose("windows-loose", "win-64", Some("loose"), true, false)]
fn pack_outputs_match_the_pre_extraction_reference(
    synthetic_pack_fixture: support::SyntheticPackFixture,
    pack_reference: std::path::PathBuf,
    #[case] reference: &str,
    #[case] platform: &str,
    #[case] vendor_mode: Option<&str>,
    #[case] self_bin: bool,
    #[case] host: bool,
) {
    let fixture = synthetic_pack_fixture;
    let output = fixture.home.path().join("out");
    let mut command = fixture.command();
    command
        .arg("--output-dir")
        .arg(&output)
        .args(["--platform", platform]);
    if let Some(mode) = vendor_mode {
        command.args(["--cargo-vendor", "--cargo-vendor-mode", mode]);
    }
    if self_bin {
        command.arg("--self-bin").arg(fixture.self_bin());
    }
    if host {
        command
            .arg("--config")
            .arg(fixture.project.join("sandbox.toml"));
    }
    command.assert().success();

    let mut actual = support::file_tree(&output);
    let manifest_path = std::path::Path::new(".pixi-sandbox/manifest.json");
    let manifest = &mut actual.get_mut(manifest_path).unwrap().0;
    let text = String::from_utf8(manifest.clone()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    let timestamp = value["created_at"].as_str().unwrap();
    *manifest = text
        .replace(
            &format!("\"created_at\": \"{timestamp}\""),
            "\"created_at\": \"2000-01-01T00:00:00Z\"",
        )
        .into_bytes();
    let readme = &mut actual.get_mut(std::path::Path::new("README.md")).unwrap().0;
    *readme = String::from_utf8(readme.clone())
        .unwrap()
        .replace(
            &format!("Built {timestamp} from commit"),
            "Built 2000-01-01T00:00:00Z from commit",
        )
        .into_bytes();

    let expected = support::file_tree(&pack_reference.join(reference));
    assert_eq!(
        actual.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>(),
        "complete file set"
    );
    for (path, bytes_and_exec) in expected {
        assert_eq!(actual[&path], bytes_and_exec, "{} changed", path.display());
    }
}

#[rstest]
#[case::zero(0.0, "positive finite number")]
#[case::negative(-1.0, "positive finite number")]
#[case::nan(f64::NAN, "positive finite number")]
#[case::infinite(f64::INFINITY, "positive finite number")]
#[case::overflow(f64::MAX, "too large")]
fn an_invalid_shard_limit_creates_nothing(
    isolated_home: TempDir,
    #[case] limit: f64,
    #[case] refusal: &str,
) {
    let root = isolated_home.path();
    std::fs::write(root.join("pixi.lock"), "").unwrap();
    let output = root.join("out");
    let error = plan_layout(root, &output, &["default".into()], limit).unwrap_err();
    assert!(format!("{error:#}").contains(refusal), "{error:#}");
    assert!(!output.exists());
}

#[cfg(unix)]
#[rstest]
fn a_failed_verification_unpack_retains_scratch_and_both_error_streams(
    synthetic_pack_fixture: support::SyntheticPackFixture,
    pack_reference: std::path::PathBuf,
) {
    let fixture = synthetic_pack_fixture;
    let unpacker = fixture.tools.join("failing-unpacker");
    support::write_executable(
        &unpacker,
        "#!/bin/sh\necho 'fixture unpack stdout'\necho 'fixture unpack stderr' >&2\nexit 42\n",
    );
    let out = fixture.home.path().join("out");
    let payload = out.join(".pixi-sandbox");
    std::fs::create_dir_all(payload.join("envs/default")).unwrap();
    let error = pixi_sandbox::pack::build_files_oracle(
        &out,
        &payload,
        "default",
        &pack_reference.join("linux-no-vendor/.pixi-sandbox/envs/default/pack"),
        &unpacker,
        u64::MAX,
    )
    .unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("fixture unpack stdout"), "{message}");
    assert!(message.contains("fixture unpack stderr"), "{message}");
    assert!(out.join(".pixi-sandbox-verify-default").is_dir());
    assert!(!payload.join("envs/default/files.json").exists());
}
