use pixi_sandbox_core::manifest::{
    Blob, Env, EnvFiles, Manifest, Source, ToolEntry, ToolInfo, Vendor, WorkspaceVendor,
};
use pixi_sandbox_core::transport_budget::{
    TransportBudgets, bytes_to_mib, check, measure, mib_to_bytes,
};
use std::collections::BTreeMap;

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn blob(path: &str, size: u64) -> Blob {
    Blob {
        path: path.to_string(),
        size,
        sha256: digest('a'),
        parts: Vec::new(),
    }
}

fn split_blob(path: &str, size: u64, parts: &[u64]) -> Blob {
    Blob {
        path: path.to_string(),
        size,
        sha256: digest('b'),
        parts: parts
            .iter()
            .enumerate()
            .map(|(index, size)| pixi_sandbox_core::manifest::Part {
                path: format!("{path}.part{index:03}"),
                size: *size,
                sha256: digest('c'),
            })
            .collect(),
    }
}

fn manifest() -> Manifest {
    Manifest {
        schema: 2,
        tool: ToolInfo {
            name: "pixi-sandbox".to_string(),
            version: "0.0.0".to_string(),
        },
        created_at: "2026-10-06T00:00:00Z".to_string(),
        platform: "linux-64".to_string(),
        shard_limit_bytes: 10,
        source: Source::default(),
        tools: BTreeMap::from([(
            "pixi".to_string(),
            ToolEntry {
                version: "0.0.0".to_string(),
                url: None,
                pinned_sha256: None,
                linkage: "static".to_string(),
                size_bytes: 7,
                path: Some("tools/linux-64/pixi".to_string()),
            },
        )]),
        envs: BTreeMap::from([(
            "default".to_string(),
            Env {
                platform: "linux-64".to_string(),
                pack_path: ".pixi-sandbox/envs/default/pack".to_string(),
                packed_size_bytes: 42,
                unpacked_size_bytes: 100,
                pixi_environment_fingerprint: None,
                blobs: vec![
                    blob("envs/default/pack/a.conda", 3),
                    split_blob("envs/default/pack/b.conda", 21, &[10, 10, 1]),
                ],
                files: Some(EnvFiles {
                    blob: blob("envs/default/files.json", 5),
                    entries: 1,
                }),
                workspace: None,
            },
        )]),
        workspaces: Vec::new(),
        vendor: Some(Vendor {
            mode: "loose".to_string(),
            crates: 1,
            size_bytes: 11,
            cargo_lock_sha256: None,
            directory: Some(".pixi-sandbox/vendor".to_string()),
            blobs: vec![
                blob("vendor/demo/Cargo.toml", 4),
                blob("vendor/demo/src/lib.rs", 7),
            ],
        }),
        workspace_vendors: Vec::new(),
        host_requirements: None,
    }
}

#[test]
fn measures_manifest_bytes_the_same_way_the_transport_gates_do() {
    let measurements = measure(&manifest(), Some(99));

    assert_eq!(
        measurements.largest_blob_bytes, 10,
        "split blobs count by their largest stored part"
    );
    assert_eq!(measurements.transport_bytes, 3 + 21 + 5 + 7 + 4 + 7);
    assert_eq!(measurements.repository_push_bytes, Some(99));
    assert_eq!(measurements.restore_required_bytes, 42 + 100 + 22);
}

#[test]
fn budget_violations_name_the_threshold_and_the_partition_remedy() {
    let budgets = TransportBudgets {
        max_blob_bytes: 9,
        max_transport_bytes: 40,
        max_repository_push_bytes: 90,
        max_restore_required_bytes: 150,
    };

    let report = check(&manifest(), Some(99), budgets);

    assert!(!report.ok());
    let fields = report
        .violations
        .iter()
        .map(|violation| violation.field)
        .collect::<Vec<_>>();
    assert_eq!(
        fields,
        [
            "budgets.max_blob_mib",
            "budgets.max_transport_mib",
            "budgets.max_repository_push_mib",
            "budgets.max_restore_required_mib",
        ]
    );
    assert!(
        report.violations[1].remedy.contains("[[bundle]]")
            && report.violations[1]
                .remedy
                .contains("self-contained branch"),
        "{}",
        report.violations[1].remedy
    );
}

/// The base manifest plus one nested vendor tree of 30 bytes, so the ceiling math can be
/// judged against a transport carrying more than one vendor tree (TASK-131).
fn manifest_with_workspace_vendor() -> Manifest {
    let mut manifest = manifest();
    manifest
        .workspaces
        .push(pixi_sandbox_core::manifest::WorkspaceEntry {
            slug: "subproject".to_string(),
            manifest: "subproject/pixi.toml".to_string(),
            directory: "subproject".to_string(),
            environments: vec!["default".to_string()],
        });
    manifest.workspace_vendors.push(WorkspaceVendor {
        slug: "subproject".to_string(),
        manifest: "subproject/pixi.toml".to_string(),
        directory: "subproject".to_string(),
        vendor: Vendor {
            mode: "loose".to_string(),
            crates: 1,
            size_bytes: 30,
            cargo_lock_sha256: None,
            directory: Some(".pixi-sandbox/vendor/workspaces/subproject".to_string()),
            blobs: vec![
                blob("vendor/workspaces/subproject/demo/Cargo.toml", 10),
                blob("vendor/workspaces/subproject/demo/src/lib.rs", 20),
            ],
        },
    });
    manifest.validate().expect("fixture must validate");
    manifest
}

#[test]
fn workspace_vendor_bytes_count_in_every_ceiling() {
    let base = measure(&manifest(), None);
    let measurements = measure(&manifest_with_workspace_vendor(), None);

    // transport_bytes walks every blob, including the nested tree's 30 bytes.
    assert_eq!(measurements.transport_bytes, base.transport_bytes + 30);
    assert!(measurements.largest_blob_bytes >= 20);
    // Restore stages each tree before renaming it, so the nested tree costs twice its size —
    // exactly the rule the root tree already follows.
    assert_eq!(
        measurements.restore_required_bytes,
        base.restore_required_bytes + 60
    );
}

#[test]
fn an_over_large_workspace_tree_trips_the_restore_budget_with_a_partition_remedy() {
    let budgets = TransportBudgets {
        max_blob_bytes: u64::MAX,
        max_transport_bytes: u64::MAX,
        max_repository_push_bytes: u64::MAX,
        // The base restore estimate (164) fits; the extra 60 do not.
        max_restore_required_bytes: 200,
    };
    let report = check(&manifest_with_workspace_vendor(), None, budgets);
    assert!(!report.ok());
    let violation = report
        .violations
        .iter()
        .find(|violation| violation.field == "budgets.max_restore_required_mib")
        .expect("the nested tree's staged bytes must be judged");
    assert!(violation.remedy.contains("[[bundle]]"));
}

#[test]
fn mib_to_bytes_converts_exactly_and_saturates_at_the_bounds() {
    // exact for human-scale values, fractional included
    assert_eq!(mib_to_bytes(95.0), 95 * 1024 * 1024);
    assert_eq!(mib_to_bytes(0.5), 512 * 1024);
    // saturates instead of wrapping: a wrapped verdict would under-report a limit
    assert_eq!(mib_to_bytes(-1.0), 0);
    assert_eq!(mib_to_bytes(f64::NAN), 0);
    assert_eq!(mib_to_bytes(f64::INFINITY), u64::MAX);
    assert_eq!(mib_to_bytes(1e30), u64::MAX);
    // round-trips through the display conversion (compared by bits: 95 MiB is exact)
    assert_eq!(
        bytes_to_mib(mib_to_bytes(95.0)).to_bits(),
        95.0f64.to_bits()
    );
}
