//! TASK-78 / ADR-28 Slice 2: staging a reviewable recipe directory.
//!
//! The staged tree is what the build tool will read, so the assertions are about the bytes on
//! disk: the recipe, the provenance, and a payload copy that keeps modes, links and empty
//! directories. Nothing here runs `rattler-build` or touches this checkout.

use std::path::Path;

use pixi_sandbox::wrap_payload::{RECIPE_FILE, stage_recipe_dir};
use pixi_sandbox_core::payload::{
    PAYLOAD_DIR, PROVENANCE_FILE, PayloadSpec, WrapPlan, WrapRequest, plan_wrap, read_payload_tree,
};
use pixi_sandbox_core::platform::Platform;
use pixi_sandbox_core::transport_budget::TransportBudgets;

const SHA256: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn request(payload_root: &Path) -> WrapRequest {
    WrapRequest {
        spec: PayloadSpec {
            name: "demo-payload".to_string(),
            version: "1.0.0".to_string(),
            platform: Platform::Linux64,
            noarch: false,
            entry_point: None,
        },
        entries: read_payload_tree(payload_root).expect("the fixture tree is readable"),
        source_url: "https://example.invalid/demo-1.0.0.tar.gz".to_string(),
        source_sha256: SHA256.to_string(),
        budgets: TransportBudgets::default(),
    }
}

fn build_fixture_payload(root: &Path) {
    std::fs::create_dir_all(root.join("bin")).expect("bin");
    std::fs::create_dir_all(root.join("empty")).expect("empty");
    std::fs::write(root.join("bin/tool"), b"#!/bin/sh\necho hi\n").expect("tool");
    std::fs::write(root.join("data.bin"), b"payload data").expect("data");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(
            root.join("bin/tool"),
            std::fs::Permissions::from_mode(0o755),
        )
        .expect("mode");
        std::os::unix::fs::symlink("bin/tool", root.join("alias")).expect("symlink");
    }
}

#[test]
fn staging_writes_the_recipe_the_provenance_and_a_faithful_payload_copy() {
    let dir = tempfile::tempdir().expect("tempdir");
    let payload_root = dir.path().join("source-payload");
    build_fixture_payload(&payload_root);
    let plan: WrapPlan = plan_wrap(&request(&payload_root)).expect("the fixture plans cleanly");
    let out_dir = dir.path().join("build");

    let staged = stage_recipe_dir(&plan, &payload_root, &out_dir).expect("staging succeeds");

    assert_eq!(staged.recipe_dir, out_dir);
    assert_eq!(
        std::fs::read_to_string(&staged.recipe_path).expect("recipe is readable"),
        plan.recipe
    );
    assert_eq!(staged.recipe_path, out_dir.join(RECIPE_FILE));
    assert_eq!(staged.provenance_path, out_dir.join(PROVENANCE_FILE));
    assert_eq!(staged.payload_dir, out_dir.join(PAYLOAD_DIR));

    let on_disk: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&staged.provenance_path).expect("provenance is readable"),
    )
    .expect("provenance is JSON");
    assert_eq!(
        on_disk,
        serde_json::to_value(&plan.provenance).expect("provenance serialises")
    );

    assert_eq!(
        std::fs::read(out_dir.join("payload/data.bin")).expect("data copied"),
        b"payload data"
    );
    assert!(
        out_dir.join("payload/empty").is_dir(),
        "an empty directory survives staging"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(out_dir.join("payload/bin/tool"))
            .expect("tool copied")
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111, "the executable bit survives staging");
        let link = std::fs::read_link(out_dir.join("payload/alias")).expect("link copied");
        assert_eq!(
            link,
            Path::new("bin/tool"),
            "the link target is copied verbatim"
        );
    }
}

#[test]
fn staging_removes_a_stale_payload_from_a_previous_run() {
    let dir = tempfile::tempdir().expect("tempdir");
    let payload_root = dir.path().join("source-payload");
    build_fixture_payload(&payload_root);
    let plan = plan_wrap(&request(&payload_root)).expect("the fixture plans cleanly");
    let out_dir = dir.path().join("build");
    std::fs::create_dir_all(out_dir.join("payload")).expect("stale payload dir");
    std::fs::write(out_dir.join("payload/stale.bin"), b"old").expect("stale file");

    stage_recipe_dir(&plan, &payload_root, &out_dir).expect("staging succeeds");

    assert!(
        !out_dir.join("payload/stale.bin").exists(),
        "a file the new plan dropped must not survive a restage"
    );
    assert!(out_dir.join("payload/data.bin").is_file());
}
