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

/// TASK-131: give both projects a `Cargo.lock` and a fake cargo that copies the canned vendor
/// tree, then assert pack produces two independently recorded trees in one transport.
fn install_cargo_locks(project: &NestedProject) {
    let source = support::demo_project();
    for destination in [&project.root, &project.root.join("subproject")] {
        fs::copy(source.join("Cargo.lock"), destination.join("Cargo.lock")).unwrap();
        fs::copy(source.join("Cargo.toml"), destination.join("Cargo.toml")).unwrap();
    }
}

/// A fake cargo/rustc pair that emulates `cargo vendor` by copying the committed reference
/// tree into the requested directory — no network, no real registry (ADR-10 hermetic seam).
fn fake_vendor_toolchain(home: &Path) -> PathBuf {
    let tools = home.join("vendor-tools");
    fs::create_dir_all(&tools).unwrap();
    let canned = support::fixture_transport()
        .join(".pixi-sandbox/vendor")
        .display()
        .to_string();
    support::write_executable(
        &tools.join("cargo"),
        &format!(
            "#!/bin/sh\nset -eu\nif [ \"${{1:-}}\" = '--version' ]; then echo 'cargo 1.90.0 (fixture)'; exit 0; fi\ntest \"$1\" = vendor\ntest \"$2\" = --locked\ntest \"$3\" = --versioned-dirs\nmkdir -p \"$4\"\ncp -R \"{canned}\"/. \"$4\"/\nfind \"$4\" -exec touch -t 200001010000.00 {{}} \\;\n"
        ),
    );
    support::write_executable(
        &tools.join("rustc"),
        "#!/bin/sh\necho 'rustc 1.90.0 (fixture)'\n",
    );
    tools
}

/// Pack the two-lockfile project with `--cargo-vendor` through the fake toolchain, returning
/// the transport directory. The restore tests below all start from this one packed branch.
fn pack_both_vendor_trees(project: &NestedProject, tools: &Path) -> PathBuf {
    let transport = project.home.path().join("transport-vendor");
    support::bin()
        .env("PATH", support::path_with_fake_tools(tools))
        .env("CARGO_NET_OFFLINE", "true")
        .args(["pack", "--repo-root", project.root.to_str().unwrap()])
        .arg("--output-dir")
        .arg(&transport)
        .args([
            "--envs",
            "default,subproject/default",
            "--platform",
            host_platform(),
            "--cargo-vendor",
        ])
        .arg("--config")
        .arg(project.root.join(".pixi-sandbox.toml"))
        .assert()
        .success();
    transport
}

/// Restore `transport` into a fresh `restored/` under the test home, skipping tool
/// registration so nothing leaves the tempdir.
fn restore_transport(
    project: &NestedProject,
    transport: &Path,
    extra: &[&str],
) -> assert_cmd::assert::Assert {
    let restored = project.home.path().join("restored");
    fs::create_dir_all(&restored).unwrap();
    support::bin()
        .args([
            "restore",
            "--branch-location",
            transport.to_str().unwrap(),
            "--output-path",
            restored.to_str().unwrap(),
            "--user-tools",
            "skip",
        ])
        .args(extra)
        .assert()
}

#[rstest]
fn pack_vendors_root_and_nested_lockfiles_into_qualified_trees(nested_project: NestedProject) {
    let project = nested_project;
    install_cargo_locks(&project);
    let tools = fake_vendor_toolchain(project.home.path());
    let transport = project.home.path().join("transport");

    support::bin()
        .env("PATH", support::path_with_fake_tools(&tools))
        .env("CARGO_NET_OFFLINE", "true")
        .args(["pack", "--repo-root", project.root.to_str().unwrap()])
        .arg("--output-dir")
        .arg(&transport)
        .args([
            "--envs",
            "default,subproject/default",
            "--platform",
            host_platform(),
            "--cargo-vendor",
        ])
        .arg("--config")
        .arg(project.root.join(".pixi-sandbox.toml"))
        .assert()
        .success();

    let manifest = pixi_sandbox_core::manifest::Manifest::load(
        &pixi_sandbox_core::manifest::Manifest::path_in(&transport),
    )
    .unwrap();
    let root_vendor = manifest.vendor.expect("the root lock is vendored");
    assert_eq!(
        root_vendor.directory.as_deref(),
        Some(".pixi-sandbox/vendor")
    );
    assert!(!root_vendor.blobs.is_empty(), "root blobs are recorded");

    assert_eq!(
        manifest.workspace_vendors.len(),
        1,
        "one tree per workspace"
    );
    let tree = &manifest.workspace_vendors[0];
    assert_eq!(tree.slug, "subproject");
    assert_eq!(tree.manifest, "subproject/pixi.toml");
    assert_eq!(tree.directory, "subproject");
    assert_eq!(
        tree.vendor.directory.as_deref(),
        Some(".pixi-sandbox/vendor/workspaces/subproject")
    );
    assert_eq!(tree.vendor.mode, "loose");
    assert!(!tree.vendor.blobs.is_empty(), "nested blobs are recorded");
    assert_eq!(
        tree.vendor.blobs.len(),
        root_vendor.blobs.len(),
        "the same canned crate gives each tree its own verified blob list"
    );

    // Both trees are physically present at their qualified locations.
    assert!(
        transport
            .join(".pixi-sandbox/vendor/demo-dep-1.0.0/Cargo.toml")
            .is_file()
    );
    assert!(
        transport
            .join(".pixi-sandbox/vendor/workspaces/subproject/demo-dep-1.0.0/Cargo.toml")
            .is_file()
    );

    // The branch guide names the nested tree, not just the root one.
    let readme = fs::read_to_string(transport.join("README.md")).unwrap();
    assert!(readme.contains("subproject"), "{readme}");
    assert!(
        readme.contains("subproject/.pixi-sandbox/vendor/"),
        "{readme}"
    );
}

#[rstest]
fn pack_with_cargo_vendor_skips_a_workspace_that_has_no_lockfile(nested_project: NestedProject) {
    let project = nested_project;
    // Only the root carries a Cargo.lock: the nested project is simply not vendored.
    let source = support::demo_project();
    fs::copy(source.join("Cargo.lock"), project.root.join("Cargo.lock")).unwrap();
    fs::copy(source.join("Cargo.toml"), project.root.join("Cargo.toml")).unwrap();
    let tools = fake_vendor_toolchain(project.home.path());
    let transport = project.home.path().join("transport");

    support::bin()
        .env("PATH", support::path_with_fake_tools(&tools))
        .env("CARGO_NET_OFFLINE", "true")
        .args(["pack", "--repo-root", project.root.to_str().unwrap()])
        .arg("--output-dir")
        .arg(&transport)
        .args([
            "--envs",
            "default,subproject/default",
            "--platform",
            host_platform(),
            "--cargo-vendor",
        ])
        .arg("--config")
        .arg(project.root.join(".pixi-sandbox.toml"))
        .assert()
        .success();

    let manifest = pixi_sandbox_core::manifest::Manifest::load(
        &pixi_sandbox_core::manifest::Manifest::path_in(&transport),
    )
    .unwrap();
    assert!(manifest.vendor.is_some(), "the root tree is still packed");
    assert!(
        manifest.workspace_vendors.is_empty(),
        "no lock means no tree, not an empty placeholder"
    );
}

#[rstest]
fn pack_with_cargo_vendor_without_any_lockfile_names_what_it_needs(nested_project: NestedProject) {
    let project = nested_project;
    let tools = fake_vendor_toolchain(project.home.path());
    let transport = project.home.path().join("transport");

    support::bin()
        .env("PATH", support::path_with_fake_tools(&tools))
        .args(["pack", "--repo-root", project.root.to_str().unwrap()])
        .arg("--output-dir")
        .arg(&transport)
        .args([
            "--envs",
            "default,subproject/default",
            "--platform",
            host_platform(),
            "--cargo-vendor",
        ])
        .arg("--config")
        .arg(project.root.join(".pixi-sandbox.toml"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("Cargo.lock"))
        .stderr(predicate::str::contains("--cargo-vendor"));
    assert!(
        !transport.exists(),
        "a refused pack leaves no transport behind"
    );
}

// ---------------------------------------------------------------------------
// TASK-131 slice 3: restore materialises one vendor tree per owning project and
// wires Cargo inside that project's own .pixi-sandbox and env prefixes.
// ---------------------------------------------------------------------------

/// Pack both vendor trees, then restore, and assert every tree and every piece of wiring
/// lands in the project that owns the lockfile.
#[rstest]
fn restore_materialises_each_vendor_tree_into_its_own_project_with_auto_wiring(
    nested_project: NestedProject,
) {
    let project = nested_project;
    install_cargo_locks(&project);
    let tools = fake_vendor_toolchain(project.home.path());
    let transport = pack_both_vendor_trees(&project, &tools);

    restore_transport(&project, &transport, &[]).success();
    let restored = project.home.path().join("restored");

    // The two materialised trees, each inside its owning project's .pixi-sandbox.
    assert!(
        restored
            .join(".pixi-sandbox/vendor/demo-dep-1.0.0/Cargo.toml")
            .is_file()
    );
    assert!(
        restored
            .join("subproject/.pixi-sandbox/vendor/demo-dep-1.0.0/Cargo.toml")
            .is_file()
    );
    // The nested tree never lands under the root project's vendor directory.
    assert!(!restored.join(".pixi-sandbox/vendor/workspaces").exists());

    // One sandbox cargo-home per project, each naming its own vendor directory.
    let root_home =
        fs::read_to_string(restored.join(".pixi-sandbox/cargo-home/config.toml")).unwrap();
    assert!(
        root_home.contains(&format!(
            "directory = \"{}\"",
            restored.join(".pixi-sandbox/vendor").display()
        )),
        "{root_home}"
    );
    let nested_home =
        fs::read_to_string(restored.join("subproject/.pixi-sandbox/cargo-home/config.toml"))
            .unwrap();
    assert!(
        nested_home.contains(&format!(
            "directory = \"{}\"",
            restored.join("subproject/.pixi-sandbox/vendor").display()
        )),
        "{nested_home}"
    );
    assert!(!nested_home.contains("subproject/.pixi-sandbox/cargo-home"));

    // Activation hooks are written into each workspace's own env, pointing at its own home.
    let root_hook = fs::read_to_string(
        restored.join(".pixi/envs/default/etc/conda/activate.d/pixi-sandbox-cargo-home.sh"),
    )
    .unwrap();
    let nested_hook = fs::read_to_string(
        restored
            .join("subproject/.pixi/envs/default/etc/conda/activate.d/pixi-sandbox-cargo-home.sh"),
    )
    .unwrap();
    assert!(
        root_hook.contains(
            restored
                .join(".pixi-sandbox/cargo-home")
                .to_string_lossy()
                .as_ref(),
        ),
        "{root_hook}"
    );
    assert!(
        nested_hook.contains(
            restored
                .join("subproject/.pixi-sandbox/cargo-home")
                .to_string_lossy()
                .as_ref(),
        ),
        "{nested_hook}"
    );
    assert!(
        !root_hook.contains("subproject"),
        "the root hook must not cross projects: {root_hook}"
    );
    // The three shell flavours travel together, per workspace.
    assert!(
        restored
            .join("subproject/.pixi/envs/default/etc/conda/activate.d/pixi-sandbox-cargo-home.ps1")
            .is_file()
    );
    assert!(
        restored
            .join("subproject/.pixi/envs/default/etc/conda/activate.d/pixi-sandbox-cargo-home.bat")
            .is_file()
    );

    // The second opinion still passes once both trees are on disk.
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
fn restore_no_vendor_skips_every_tree_but_keeps_the_environments(nested_project: NestedProject) {
    let project = nested_project;
    install_cargo_locks(&project);
    let tools = fake_vendor_toolchain(project.home.path());
    let transport = pack_both_vendor_trees(&project, &tools);

    restore_transport(&project, &transport, &["--no-vendor"]).success();
    let restored = project.home.path().join("restored");

    // Environments still land, in both projects.
    assert!(restored.join(".pixi/envs/default").is_dir());
    assert!(restored.join("subproject/.pixi/envs/default").is_dir());
    // No vendor tree, cargo-home or activation hook anywhere.
    assert!(!restored.join(".pixi-sandbox/vendor").exists());
    assert!(!restored.join("subproject/.pixi-sandbox/vendor").exists());
    assert!(!restored.join(".pixi-sandbox/cargo-home").exists());
    assert!(
        !restored
            .join("subproject/.pixi-sandbox/cargo-home")
            .exists()
    );
    assert!(
        !restored
            .join(".pixi/envs/default/etc/conda/activate.d/pixi-sandbox-cargo-home.sh")
            .exists()
    );
    assert!(
        !restored
            .join("subproject/.pixi/envs/default/etc/conda/activate.d/pixi-sandbox-cargo-home.sh")
            .exists()
    );
}

#[rstest]
fn restore_cargo_config_write_writes_one_project_config_per_tree(nested_project: NestedProject) {
    let project = nested_project;
    install_cargo_locks(&project);
    let tools = fake_vendor_toolchain(project.home.path());
    let transport = pack_both_vendor_trees(&project, &tools);

    let output = restore_transport(&project, &transport, &["--cargo-config", "write"])
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).unwrap();
    let restored = project.home.path().join("restored");

    let root_config = fs::read_to_string(restored.join(".cargo/config.toml")).unwrap();
    let nested_config = fs::read_to_string(restored.join("subproject/.cargo/config.toml")).unwrap();
    assert!(
        root_config.contains(
            restored
                .join(".pixi-sandbox/vendor")
                .to_string_lossy()
                .as_ref(),
        ),
        "{root_config}"
    );
    assert!(
        nested_config.contains(
            restored
                .join("subproject/.pixi-sandbox/vendor")
                .to_string_lossy()
                .as_ref(),
        ),
        "{nested_config}"
    );
    assert!(!root_config.contains("subproject"));
    // Write mode never creates the sandbox cargo-home.
    assert!(!restored.join(".pixi-sandbox/cargo-home").exists());
    assert!(
        !restored
            .join("subproject/.pixi-sandbox/cargo-home")
            .exists()
    );
    // One wired summary per tree.
    assert_eq!(
        output.matches("cargo vendor: wired via").count(),
        2,
        "{output}"
    );
}

#[rstest]
fn restore_refuses_an_existing_nested_vendor_tree_without_force_and_replaces_with_it(
    nested_project: NestedProject,
) {
    let project = nested_project;
    install_cargo_locks(&project);
    let tools = fake_vendor_toolchain(project.home.path());
    let transport = pack_both_vendor_trees(&project, &tools);

    let restored = project.home.path().join("restored");
    fs::create_dir_all(restored.join("subproject/.pixi-sandbox/vendor")).unwrap();
    fs::write(
        restored.join("subproject/.pixi-sandbox/vendor/previous-owner.txt"),
        b"old",
    )
    .unwrap();

    restore_transport(&project, &transport, &[])
        .failure()
        .stderr(predicate::str::contains("already exists"))
        .stderr(predicate::str::contains("--force"))
        .stderr(predicate::str::contains("subproject"));
    // A refused restore leaves the previous tree exactly as it found it.
    assert!(
        restored
            .join("subproject/.pixi-sandbox/vendor/previous-owner.txt")
            .is_file()
    );
    assert!(
        !restored
            .join("subproject/.pixi-sandbox/vendor/demo-dep-1.0.0")
            .exists()
    );

    restore_transport(&project, &transport, &["--force"]).success();
    assert!(
        !restored
            .join("subproject/.pixi-sandbox/vendor/previous-owner.txt")
            .exists()
    );
    assert!(
        restored
            .join("subproject/.pixi-sandbox/vendor/demo-dep-1.0.0/Cargo.toml")
            .is_file()
    );
}

#[rstest]
#[case::print("print")]
#[case::none("none")]
fn restore_print_and_none_modes_leave_every_project_unwired(
    nested_project: NestedProject,
    #[case] mode: &str,
) {
    let project = nested_project;
    install_cargo_locks(&project);
    let tools = fake_vendor_toolchain(project.home.path());
    let transport = pack_both_vendor_trees(&project, &tools);

    let output = restore_transport(&project, &transport, &["--cargo-config", mode])
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).unwrap();
    let restored = project.home.path().join("restored");

    // The trees themselves are still materialised; only the wiring is withheld.
    assert!(
        restored
            .join(".pixi-sandbox/vendor/demo-dep-1.0.0/Cargo.toml")
            .is_file()
    );
    assert!(
        restored
            .join("subproject/.pixi-sandbox/vendor/demo-dep-1.0.0/Cargo.toml")
            .is_file()
    );
    assert!(!restored.join(".cargo/config.toml").exists());
    assert!(!restored.join("subproject/.cargo/config.toml").exists());
    assert!(!restored.join(".pixi-sandbox/cargo-home").exists());
    assert!(
        !restored
            .join("subproject/.pixi-sandbox/cargo-home")
            .exists()
    );
    assert!(
        !restored
            .join(".pixi/envs/default/etc/conda/activate.d/pixi-sandbox-cargo-home.sh")
            .exists()
    );

    // One NOT-wired line per tree, and print names both absolute vendor directories.
    assert_eq!(
        output
            .matches(&format!("cargo vendor: NOT wired (--cargo-config {mode})"))
            .count(),
        2,
        "{output}"
    );
    if mode == "print" {
        assert_eq!(
            output.matches("# Cargo source replacement").count(),
            2,
            "{output}"
        );
        assert!(
            output.contains(
                restored
                    .join("subproject/.pixi-sandbox/vendor")
                    .to_string_lossy()
                    .as_ref(),
            )
        );
    }
}
