use pixi_sandbox_core::manifest::{
    Blob, Env, EnvFiles, Manifest, Source, ToolEntry, ToolInfo, Vendor,
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
            },
        )]),
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
