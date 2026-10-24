//! TASK-129: a bundle may carry explicitly declared nested Pixi workspaces.
//!
//! The tests here are the local half of the task's evidence. `plan` has to name what it will
//! carry and refuse what it cannot; `pack` has to put each environment under its own root; and
//! `restore` has to materialise a nested environment into the *nested project's* `.pixi/envs`,
//! which is the layout `pixi run --manifest-path …` reads. The native-runner airlock tier is not
//! claimed here — that is a CI gate, not a fixture.

#![cfg(unix)]

mod support;

use assert_cmd::Command;
use predicates::prelude::*;
use rstest::{fixture, rstest};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use support::{NativePackFixture, host_platform, native_wheel_pack_fixture};

/// A repository root with two Pixi workspaces: the root one, and `subproject/`.
///
/// Both locks point at the genuine Conda archive and wheel in the fixture's local channel, so
/// packing either one exercises the real acquisition path rather than a stub.
struct NestedProject {
    home: tempfile::TempDir,
    root: PathBuf,
    channel: PathBuf,
}

#[fixture]
fn nested_project(native_wheel_pack_fixture: NativePackFixture) -> NestedProject {
    let fixture = native_wheel_pack_fixture;
    let home = fixture.home;
    let assets = home.path().join("native-pack");
    let channel = assets.join("channel");
    let wheel = assets.join("pypi/native_demo-1.0-py3-none-any.whl");
    let root = home.path().join("nested-root");
    fs::create_dir_all(root.join("subproject")).unwrap();
    write_locked_project(&root, &channel, &wheel);
    write_locked_project(&root.join("subproject"), &channel, &wheel);
    fs::write(
        root.join(".pixi-sandbox.toml"),
        r#"
schema = 2
branch_prefix = "sandbox"
cargo_vendor = false

[[bundle]]
name = "developer"
environments = ["default"]
platforms = ["PLATFORM"]

[[bundle.workspaces]]
manifest = "subproject/pixi.toml"
"#
        .replace("PLATFORM", host_platform()),
    )
    .unwrap();
    NestedProject {
        home,
        root,
        channel,
    }
}

/// A project whose lock selects the fixture's one Conda package and one wheel, written the way
/// pixi 0.83 writes a v7 lock against a local `file://` channel.
fn write_locked_project(project: &Path, channel: &Path, wheel: &Path) {
    let transaction = channel.join("noarch/repodata.json");
    let data: Value = serde_json::from_slice(&fs::read(&transaction).unwrap()).unwrap();
    let mut packages = Vec::new();
    let mut selected = Vec::new();
    for (name, record) in data["packages"].as_object().unwrap() {
        let location = url::Url::from_file_path(channel.join("noarch").join(name))
            .unwrap()
            .to_string();
        let mut record = record.clone();
        record["conda"] = location.clone().into();
        for field in ["name", "version", "build", "build_number", "subdir"] {
            record.as_object_mut().unwrap().remove(field);
        }
        packages.push(record);
        selected.push(serde_json::json!({ "conda": location }));
    }
    let location = url::Url::from_file_path(wheel).unwrap().to_string();
    packages.push(serde_json::json!({
        "pypi": location,
        "name": "native-demo",
        "version": "1.0",
        "sha256": pixi_sandbox_core::shard::sha256_file(wheel).unwrap()
    }));
    selected.push(serde_json::json!({ "pypi": location }));
    fs::write(
        project.join("pixi.lock"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "version": 7,
            "environments": { "default": { "channels": [], "packages": { host_platform(): selected } } },
            "packages": packages
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        project.join("pixi.toml"),
        format!(
            "[workspace]\nname = 'native-fixture'\nchannels = []\nplatforms = ['{}']\n",
            host_platform()
        ),
    )
    .unwrap();
}

fn plan_json(config: &Path) -> Command {
    let mut command = support::bin();
    command.args(["plan", "--config", config.to_str().unwrap(), "--json"]);
    command
}

#[rstest]
fn plan_names_every_declared_workspace_environment_and_cost(nested_project: NestedProject) {
    let project = nested_project;
    let output = plan_json(&project.root.join(".pixi-sandbox.toml"))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let plan: Value = serde_json::from_slice(&output).unwrap();
    let workspaces = plan["workspaces"].as_array().unwrap();
    assert_eq!(workspaces.len(), 1, "one declared workspace: {plan}");
    let workspace = &workspaces[0];
    assert_eq!(workspace["bundle"], "developer");
    assert_eq!(workspace["manifest"], "subproject/pixi.toml");
    assert_eq!(workspace["directory"], "subproject");
    assert_eq!(workspace["slug"], "subproject");
    assert_eq!(workspace["environments"], serde_json::json!(["default"]));
    // Counted from the nested lock itself, so the assertion tracks the fixture rather than a
    // number someone typed once.
    let lock: Value =
        serde_json::from_slice(&fs::read(project.root.join("subproject/pixi.lock")).unwrap())
            .unwrap();
    let expected = lock["environments"]["default"]["packages"][host_platform()]
        .as_array()
        .unwrap()
        .len();
    assert_eq!(
        workspace["packages"].as_u64().unwrap(),
        expected as u64,
        "{workspace}"
    );
    assert!(
        workspace["size_bytes"].as_u64().unwrap() > 0,
        "the lock records sizes, so the estimate must not be zero: {workspace}"
    );
    // The matrix is still the only thing a runner reads.
    assert_eq!(plan["include"].as_array().unwrap().len(), 1);
}

#[rstest]
fn plan_names_the_workspace_in_its_text_output(nested_project: NestedProject) {
    let project = nested_project;
    support::bin()
        .args([
            "plan",
            "--config",
            project.root.join(".pixi-sandbox.toml").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "workspace subproject: subproject/pixi.toml · default · ",
        ))
        .stdout(predicate::str::contains("package(s)"))
        .stdout(predicate::str::contains("bundle developer"))
        // AC#3 transparency: the resolved set and the channel surface are on the same line.
        // Sorted identities for the helper's three locked packages (the wheel pulls in the
        // interpreter package).
        .stdout(predicate::str::contains(
            // stale-ref-allowed: the conda interpreter package's own name, not the retired tool.
            "set: native-demo==1.0,native-fixture==1.0=0,python==3.13.1=fixture_0",
        ))
        .stdout(predicate::str::contains("channels: -"));
}

#[rstest]
fn plan_json_names_resolved_identities_even_when_the_lock_strips_them(
    nested_project: NestedProject,
) {
    let project = nested_project;
    let output = plan_json(&project.root.join(".pixi-sandbox.toml"))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let plan: Value = serde_json::from_slice(&output).unwrap();
    let workspace = &plan["workspaces"][0];
    // The hand-built lock strips name/version/build off its Conda records, so their identities
    // must come from the archive filenames; the wheel keeps its name and version on the record.
    // The wheel's interpreter dependency makes three packages, de-duplicated and sorted.
    assert_eq!(
        workspace["identities"],
        serde_json::json!([
            "native-demo==1.0",
            "native-fixture==1.0=0",
            // stale-ref-allowed: the conda interpreter package's own name, not the retired tool.
            "python==3.13.1=fixture_0"
        ]),
        "{workspace}"
    );
    // A lock declaring no channels reports none rather than inventing one, and skips the key.
    assert!(
        workspace["channels"].is_null() || workspace["channels"] == serde_json::json!([]),
        "the fixture lock declares channels = []: {workspace}"
    );
}

#[rstest]
fn plan_json_surfaces_record_channels_and_field_based_identities(nested_project: NestedProject) {
    let project = nested_project;
    let lock_path = project.root.join("subproject/pixi.lock");
    let mut lock: Value = serde_json::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
    let platform = host_platform();
    let conda_location = lock["environments"]["default"]["packages"][platform][0]["conda"]
        .as_str()
        .unwrap()
        .to_string();
    // A lock the way a real channel solve writes it: environment channels plus identity fields
    // on the record, which must win over the filename fallback.
    lock["environments"]["default"]["channels"] = serde_json::json!([
        { "url": "https://prefix.dev/example" },
        { "url": "file:///local/mirror" }
    ]);
    for record in lock["packages"].as_array_mut().unwrap() {
        if record["conda"].as_str() == Some(conda_location.as_str()) {
            record["name"] = "native-fixture".into();
            record["version"] = "1.0".into();
            record["build"] = "0".into();
        }
    }
    fs::write(&lock_path, serde_json::to_vec_pretty(&lock).unwrap()).unwrap();

    let output = plan_json(&project.root.join(".pixi-sandbox.toml"))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let plan: Value = serde_json::from_slice(&output).unwrap();
    let workspace = &plan["workspaces"][0];
    assert_eq!(
        workspace["channels"],
        serde_json::json!(["file:///local/mirror", "https://prefix.dev/example"]),
        "channels are de-duplicated and sorted: {workspace}"
    );
    assert!(
        workspace["identities"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("native-fixture==1.0=0")),
        "record fields supply the same identity shape: {workspace}"
    );

    // The same data on the text line, including the non-empty channel join branch.
    support::bin()
        .args([
            "plan",
            "--config",
            project.root.join(".pixi-sandbox.toml").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "channels: file:///local/mirror,https://prefix.dev/example",
        ))
        .stdout(predicate::str::contains("native-fixture==1.0=0"));
}

#[rstest]
fn plan_text_uses_dashes_when_the_lock_records_no_channel_or_identity(
    nested_project: NestedProject,
) {
    let project = nested_project;
    let platform = host_platform();
    // Every selected record is unparseable as an identity or missing from the top-level list,
    // so the text line must say "-" rather than print an empty segment.
    let odd = "file:///cache/not-an-archive.zip";
    let ghost = "file:///cache/ghost-9.9-0.conda";
    let lock = serde_json::json!({
        "version": 7,
        "environments": {
            "default": {
                "channels": [],
                "packages": { platform: [ { "conda": odd }, { "conda": ghost } ] }
            }
        },
        "packages": [ { "conda": odd } ]
    });
    fs::write(
        project.root.join("subproject/pixi.lock"),
        serde_json::to_vec_pretty(&lock).unwrap(),
    )
    .unwrap();

    support::bin()
        .args([
            "plan",
            "--config",
            project.root.join(".pixi-sandbox.toml").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("channels: -"))
        .stdout(predicate::str::contains("set: -"));
}

#[rstest]
fn plan_keeps_reporting_when_lock_records_strip_or_omit_identity_data(
    nested_project: NestedProject,
) {
    let project = nested_project;
    let platform = host_platform();
    // Every defensive branch in the fact collector in one lock: a url-less channel entry,
    // a wheel record without name/version fields, a Conda location whose suffix is not an
    // archive, and a selected location no top-level record matches.
    let conda_good = "file:///cache/real-conda-2.0-1.tar.bz2";
    let wheel_bare = "file:///cache/wheelpkg-7.7-py3-none-any.whl";
    let conda_odd = "file:///cache/not-an-archive.zip";
    let ghost = "file:///cache/ghost-9.9-0.conda";
    let lock = serde_json::json!({
        "version": 7,
        "environments": {
            "default": {
                "channels": [ { "url": "https://example.com/chan" }, {} ],
                "packages": {
                    platform: [
                        { "conda": conda_good },
                        { "pypi": wheel_bare },
                        { "conda": conda_odd },
                        { "conda": ghost }
                    ]
                }
            }
        },
        "packages": [
            { "conda": conda_good, "subdir": "noarch" },
            { "pypi": wheel_bare },
            { "conda": conda_odd }
        ]
    });
    fs::write(
        project.root.join("subproject/pixi.lock"),
        serde_json::to_vec_pretty(&lock).unwrap(),
    )
    .unwrap();

    let output = plan_json(&project.root.join(".pixi-sandbox.toml"))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let plan: Value = serde_json::from_slice(&output).unwrap();
    let workspace = &plan["workspaces"][0];
    assert_eq!(
        workspace["channels"],
        serde_json::json!(["https://example.com/chan"]),
        "the url-less channel entry is skipped without failing: {workspace}"
    );
    assert_eq!(
        workspace["packages"].as_u64().unwrap(),
        4,
        "the count sums every selected entry, including ones without an identity"
    );
    let identities = workspace["identities"].as_array().unwrap();
    assert!(
        identities.contains(&serde_json::json!("real-conda==2.0=1")),
        "the Conda identity is parsed from the archive filename: {workspace}"
    );
    assert!(
        identities.contains(&serde_json::json!("wheelpkg==7.7")),
        "the wheel identity falls back to its PEP 427 filename: {workspace}"
    );
    assert!(
        !identities.iter().any(|value| value
            .as_str()
            .is_some_and(|s| s.contains("not-an-archive") || s.contains("ghost"))),
        "unparseable and unmatched records contribute no identity: {workspace}"
    );
}

#[rstest]
fn plan_refuses_a_nested_workspace_missing_a_bundle_platform(nested_project: NestedProject) {
    let project = nested_project;
    let manifest = project.root.join("subproject/pixi.toml");
    fs::write(
        &manifest,
        "[workspace]\nname = 'nested'\nchannels = []\nplatforms = ['osx-64']\n",
    )
    .unwrap();
    plan_json(&project.root.join(".pixi-sandbox.toml"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("declares only"))
        .stderr(predicate::str::contains(host_platform()));
}

#[rstest]
fn plan_refuses_a_nested_workspace_with_no_declared_platforms(nested_project: NestedProject) {
    let project = nested_project;
    fs::write(
        project.root.join("subproject/pixi.toml"),
        "[workspace]\nname = 'nested'\nchannels = []\n",
    )
    .unwrap();
    plan_json(&project.root.join(".pixi-sandbox.toml"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("names no platforms"));
}

#[rstest]
fn plan_refuses_an_environment_the_nested_lock_cannot_serve(nested_project: NestedProject) {
    let project = nested_project;
    let config = project.root.join(".pixi-sandbox.toml");
    let text = fs::read_to_string(&config).unwrap().replace(
        "manifest = \"subproject/pixi.toml\"",
        "manifest = \"subproject/pixi.toml\"\nenvironments = [\"docs\"]",
    );
    fs::write(&config, text).unwrap();
    plan_json(&config)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "cannot serve bundle environment \"docs\"",
        ));
}

/// The heart of the task: one bundle, two project roots, two destinations.
#[rstest]
fn pack_carries_a_nested_workspace_and_restore_lands_it_in_its_own_project(
    nested_project: NestedProject,
) {
    let project = nested_project;
    let transport = project.home.path().join("transport");
    support::bin()
        .args(["pack", "--repo-root", project.root.to_str().unwrap()])
        .arg("--output-dir")
        .arg(&transport)
        .args([
            "--envs",
            "default,subproject/default",
            "--platform",
            host_platform(),
        ])
        .arg("--config")
        .arg(project.root.join(".pixi-sandbox.toml"))
        .assert()
        .success();

    // The root environment's payload is byte-identical to what an un-nested pack produces.
    let archive = "channel/noarch/native-fixture-1.0-0.tar.bz2";
    let root_blob = transport
        .join(".pixi-sandbox/envs/default/pack")
        .join(archive);
    let nested_blob = transport
        .join(".pixi-sandbox/envs/workspaces/subproject/default/pack")
        .join(archive);
    assert_eq!(
        fs::read(&root_blob).unwrap(),
        fs::read(project.channel.join("noarch/native-fixture-1.0-0.tar.bz2")).unwrap()
    );
    assert_eq!(
        fs::read(&nested_blob).unwrap(),
        fs::read(&root_blob).unwrap()
    );

    let manifest = pixi_sandbox_core::manifest::Manifest::load(
        &pixi_sandbox_core::manifest::Manifest::path_in(&transport),
    )
    .unwrap();
    assert_eq!(
        manifest.schema,
        pixi_sandbox_core::manifest::SCHEMA_VERSION,
        "every new transport writes the current schema, root-only or not"
    );
    assert!(manifest.envs.contains_key("default"));
    assert!(manifest.envs.contains_key("subproject/default"));
    assert_eq!(
        manifest.envs["subproject/default"].pack_path,
        ".pixi-sandbox/envs/workspaces/subproject/default/pack"
    );
    assert!(manifest.envs["default"].workspace.is_none());
    assert_eq!(manifest.workspaces.len(), 1);
    assert_eq!(manifest.workspaces[0].slug, "subproject");
    assert_eq!(
        manifest.workspaces[0].environments,
        vec!["default".to_string()]
    );

    let restored = project.home.path().join("restored");
    fs::create_dir(&restored).unwrap();
    support::bin()
        .args(["restore", "--branch-location", transport.to_str().unwrap()])
        .arg("--output-path")
        .arg(&restored)
        .assert()
        .success()
        .stdout(predicate::str::contains("restore complete"));

    // The two destinations the task promises: the root keeps `.pixi/envs`, the nested one lands
    // in its own project, which is what `pixi run --manifest-path subproject/pixi.toml` reads.
    assert!(
        restored.join(".pixi/envs/default").is_dir(),
        "the root environment must keep its layout"
    );
    assert!(
        restored.join("subproject/.pixi/envs/default").is_dir(),
        "the nested environment must land in the nested project"
    );
    assert!(
        !restored.join(".pixi/envs/subproject").exists(),
        "nesting must not flatten into the root env directory"
    );

    // `doctor --verify-restored` is the second opinion, and it resolves destinations through the
    // same core function restore wrote through — so agreement here is a real check.
    support::bin()
        .arg("doctor")
        .arg("--verify-restored")
        .arg(&restored)
        .arg("--branch-location")
        .arg(&transport)
        .assert()
        .success();
}

#[rstest]
fn pack_refuses_a_nested_environment_naming_an_undeclared_workspace(nested_project: NestedProject) {
    let project = nested_project;
    support::bin()
        .args(["pack", "--repo-root", project.root.to_str().unwrap()])
        .arg("--output-dir")
        .arg(project.home.path().join("transport"))
        .args(["--envs", "other/default", "--platform", host_platform()])
        .arg("--config")
        .arg(project.root.join(".pixi-sandbox.toml"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not declare"));
}

#[rstest]
fn pack_refuses_a_nested_id_without_a_config_to_resolve_it(nested_project: NestedProject) {
    let project = nested_project;
    support::bin()
        .args(["pack", "--repo-root", project.root.to_str().unwrap()])
        .arg("--output-dir")
        .arg(project.home.path().join("transport"))
        .args([
            "--envs",
            "subproject/default",
            "--platform",
            host_platform(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("only `--config` can"));
}
