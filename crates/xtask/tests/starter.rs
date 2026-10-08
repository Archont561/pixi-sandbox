//! `starter` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

use std::path::Path;
use xtask::starter::*;

const TAG: &str = "v1.2.3";
const COMMIT: &str = "8193c58068c3245e5267c4c604f989237e4d6e49";
const SOURCE: &str = "Archont561/pixi-sandbox";
const STARTER: &str = "Archont561/pixi-sandbox-starter";

#[test]
fn an_empty_starter_with_default_main_is_refused_until_an_owner_seeds_the_ref() {
    let error = parse_main_ref(
        STARTER,
        false,
        b"",
        b"gh: Git Repository is empty. (HTTP 409)",
    )
    .unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("refs/heads/main"), "{message}");
    assert!(message.contains("owner must seed main"), "{message}");
    assert!(
        message.contains("default branch does not prove"),
        "{message}"
    );
}

#[test]
fn an_existing_main_ref_returns_its_commit_sha() {
    let response = serde_json::json!({
        "ref": "refs/heads/main",
        "object": { "sha": COMMIT },
    });
    let sha = parse_main_ref(STARTER, true, response.to_string().as_bytes(), b"").unwrap();
    assert_eq!(sha, COMMIT);
}

#[test]
fn an_empty_github_error_still_gives_the_owner_a_remedy() {
    let error = parse_main_ref(STARTER, false, b"", b"").unwrap_err();
    let message = format!("{error:#}");
    assert!(
        message.contains("GitHub did not provide an error detail"),
        "{message}"
    );
    assert!(message.contains("owner must seed main"), "{message}");
}

#[test]
fn a_response_for_a_different_ref_is_rejected() {
    let response = serde_json::json!({
        "ref": "refs/heads/trunk",
        "object": { "sha": COMMIT },
    });
    let error = parse_main_ref(STARTER, true, response.to_string().as_bytes(), b"").unwrap_err();
    let message = format!("{error:#}");
    assert!(
        message.contains("did not return refs/heads/main"),
        "{message}"
    );
    assert!(message.contains("owner must seed main"), "{message}");
}

#[test]
fn a_main_ref_without_a_commit_sha_is_rejected() {
    let response = serde_json::json!({
        "ref": "refs/heads/main",
        "object": { "sha": "  " },
    });
    let error = parse_main_ref(STARTER, true, response.to_string().as_bytes(), b"").unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("returned no commit"), "{message}");
    assert!(message.contains("owner must seed main"), "{message}");
}

#[test]
fn malformed_main_ref_json_is_reported() {
    let error = parse_main_ref(STARTER, true, b"not json", b"").unwrap_err();
    assert!(
        format!("{error:#}").contains("parsing the starter main-ref response"),
        "{error:#}"
    );
}

#[test]
fn starter_workflow_checks_the_real_main_ref_without_creating_or_seeding_the_repo() {
    const WORKFLOW: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../.github/workflows/starter.yml"
    ));
    assert!(WORKFLOW.contains("xtask starter-check-main"), "{WORKFLOW}");
    assert!(!WORKFLOW.contains(".default_branch // empty"), "{WORKFLOW}");
    assert!(!WORKFLOW.contains("--method POST"), "{WORKFLOW}");
    assert!(!WORKFLOW.contains("--method PUT"), "{WORKFLOW}");
}

/// A starter whose publisher came from a different release is the one failure the whole
/// publication contract exists to prevent, so the marker check is exact, not a substring.
#[test]
fn a_publisher_from_another_release_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    write_valid_starter(dir.path(), "9.9.9");
    let findings = verify(dir.path(), TAG).unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f.contains("pixi-sandbox-version: 1.2.3")),
        "{findings:?}"
    );
}

#[test]
fn a_complete_starter_for_its_own_tag_has_no_findings() {
    let dir = tempfile::tempdir().unwrap();
    write_valid_starter(dir.path(), "1.2.3");
    assert_eq!(verify(dir.path(), TAG).unwrap(), Vec::<String>::new());
}

#[test]
fn every_required_file_is_reported_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    let findings = verify(dir.path(), TAG).unwrap();
    for required in REQUIRED_FILES {
        assert!(
            findings.iter().any(|f| f.contains(required)),
            "{required} not reported in {findings:?}"
        );
    }
}

/// Runtime state is the thing a template must never ship: a committed `.pixi/` hands every
/// user a stale environment that their own lockfile then disagrees with.
#[test]
fn committed_runtime_state_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    write_valid_starter(dir.path(), "1.2.3");
    std::fs::create_dir_all(dir.path().join(".pixi")).unwrap();
    std::fs::write(dir.path().join("SHA256SUMS"), "x").unwrap();
    let findings = verify(dir.path(), TAG).unwrap();
    assert!(findings.iter().any(|f| f.contains(".pixi")), "{findings:?}");
    assert!(
        findings.iter().any(|f| f.contains("SHA256SUMS")),
        "{findings:?}"
    );
}

/// `ubuntu-latest` is a runner label and must not be mistaken for a floating version —
/// every generated workflow contains it, so a blanket `latest` search would fail them all.
#[test]
fn a_runner_label_is_not_a_floating_version() {
    assert!(floating_version_findings("    runs-on: ubuntu-latest\n", "w").is_empty());
}

#[test]
fn each_floating_version_spelling_is_reported() {
    for needle in ["releases/latest", "==latest", "--version latest", "@latest"] {
        let findings = floating_version_findings(&format!("  x {needle} y\n"), "w");
        assert_eq!(findings.len(), 1, "{needle} not reported");
    }
}

/// Idempotence is the publication contract's core promise: rerunning a verified tag must
/// produce no content change.
#[test]
fn scaffolding_twice_is_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    scaffold(dir.path(), "starter", TAG, COMMIT, SOURCE, STARTER).unwrap();
    let first: Vec<String> = SCAFFOLD_FILES
        .iter()
        .map(|f| std::fs::read_to_string(dir.path().join(f)).unwrap())
        .collect();
    scaffold(dir.path(), "starter", TAG, COMMIT, SOURCE, STARTER).unwrap();
    let second: Vec<String> = SCAFFOLD_FILES
        .iter()
        .map(|f| std::fs::read_to_string(dir.path().join(f)).unwrap())
        .collect();
    assert_eq!(first, second);
}

#[test]
fn a_loose_tag_is_refused_before_anything_is_written() {
    let dir = tempfile::tempdir().unwrap();
    for tag in ["latest", "v1.2", "1.2.3.4", ""] {
        assert!(
            scaffold(dir.path(), "starter", tag, COMMIT, SOURCE, STARTER).is_err(),
            "{tag:?} was accepted"
        );
    }
    assert!(!dir.path().join("pixi.toml").exists());
}

#[test]
fn a_non_hex_source_commit_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    assert!(scaffold(dir.path(), "starter", TAG, "not-a-sha", SOURCE, STARTER).is_err());
}

#[test]
fn the_readme_names_the_release_and_the_commit() {
    let readme = render_readme(TAG, COMMIT, SOURCE, STARTER);
    assert!(readme.contains(TAG));
    assert!(readme.contains(&COMMIT[..7]));
    assert!(readme.contains(STARTER));
    assert!(readme.contains("Use this template"));
}

/// The scaffolded project must declare a real dependency: a dependency-free workspace
/// locks to no per-platform entry and packs into an unrestorable transport.
#[test]
fn the_scaffolded_project_declares_a_real_dependency_and_a_dev_task() {
    let rendered = render_pixi_toml("starter");
    let parsed: toml::Value = toml::from_str(&rendered).unwrap();
    assert!(
        parsed["dependencies"]
            .as_table()
            .is_some_and(|d| !d.is_empty()),
        "no dependency declared"
    );
    assert!(parsed["tasks"]["dev"].is_str());
    assert_eq!(
        parsed["workspace"]["platforms"][0].as_str(),
        Some("linux-64")
    );
}

#[test]
fn the_gitignore_excludes_every_forbidden_runtime_path() {
    let ignore = render_gitignore();
    for path in [".pixi/", ".pixi-sandbox/", "SHA256SUMS"] {
        assert!(ignore.contains(path), "{path} missing from .gitignore");
    }
}

#[test]
fn a_tampered_asset_fails_its_checksum() {
    let dir = tempfile::tempdir().unwrap();
    let binary = dir.path().join("pixi-sandbox-x86_64-unknown-linux-musl");
    std::fs::write(&binary, b"real bytes").unwrap();
    let digest = pixi_sandbox_core::shard::sha256_file(&binary).unwrap();
    let sums = dir.path().join("SHA256SUMS");
    std::fs::write(
        &sums,
        format!("{digest}  pixi-sandbox-x86_64-unknown-linux-musl\n"),
    )
    .unwrap();
    verify_asset(&binary, &sums, "pixi-sandbox-x86_64-unknown-linux-musl").unwrap();

    std::fs::write(&binary, b"tampered").unwrap();
    assert!(verify_asset(&binary, &sums, "pixi-sandbox-x86_64-unknown-linux-musl").is_err());
}

#[test]
fn an_asset_absent_from_the_checksum_manifest_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let binary = dir.path().join("bin");
    std::fs::write(&binary, b"x").unwrap();
    let sums = dir.path().join("SHA256SUMS");
    std::fs::write(&sums, "deadbeef  some-other-asset\n").unwrap();
    assert!(verify_asset(&binary, &sums, "pixi-sandbox-x86_64-unknown-linux-musl").is_err());
}

#[test]
fn the_starter_tag_is_namespaced_so_it_cannot_collide_with_a_release_tag() {
    assert_eq!(starter_tag("v1.2.3"), "pixi-sandbox-v1.2.3");
}

#[test]
fn a_published_release_at_the_handed_off_commit_has_no_findings() {
    let assets = vec![
        "SHA256SUMS".to_string(),
        "pixi-sandbox-x86_64-unknown-linux-musl".to_string(),
    ];
    assert_eq!(
        release_findings(TAG, COMMIT, false, Some(COMMIT), &assets),
        Vec::<String>::new()
    );
}

/// A short sha handed off against a full sha (or the reverse) is the same commit; a
/// genuinely different commit is not. Both directions must behave.
#[test]
fn an_abbreviated_commit_still_matches_but_a_different_one_does_not() {
    let assets = vec![
        "SHA256SUMS".to_string(),
        "pixi-sandbox-x86_64-unknown-linux-musl".to_string(),
    ];
    assert!(release_findings(TAG, &COMMIT[..7], false, Some(COMMIT), &assets).is_empty());
    assert!(release_findings(TAG, COMMIT, false, Some(&COMMIT[..7]), &assets).is_empty());
    let other = "0000000000000000000000000000000000000000";
    assert!(!release_findings(TAG, COMMIT, false, Some(other), &assets).is_empty());
}

/// Every fail-closed rule in TASK-74 AC#4, each reported on its own.
#[test]
fn a_draft_an_unresolvable_tag_and_missing_assets_each_fail_closed() {
    let full = vec![
        "SHA256SUMS".to_string(),
        "pixi-sandbox-x86_64-unknown-linux-musl".to_string(),
    ];
    assert!(
        release_findings(TAG, COMMIT, true, Some(COMMIT), &full)
            .iter()
            .any(|f| f.contains("draft"))
    );
    assert!(
        release_findings(TAG, COMMIT, false, None, &full)
            .iter()
            .any(|f| f.contains("does not resolve"))
    );
    assert!(
        release_findings(
            TAG,
            COMMIT,
            false,
            Some(COMMIT),
            &["SHA256SUMS".to_string()]
        )
        .iter()
        .any(|f| f.contains("standalone"))
    );
    assert!(
        release_findings(TAG, COMMIT, false, Some(COMMIT), &[])
            .iter()
            .any(|f| f.contains("SHA256SUMS"))
    );
}

/// doc-10 forbids the automation from touching anything but the canonical starter, and
/// makes each revision immutable. Both refusals are checked before any git write.
#[test]
fn publishing_to_a_fork_or_over_an_existing_revision_is_refused() {
    assert!(
        publish_refusals(
            "https://github.com/someone-else/pixi-sandbox-starter",
            "Archont561/pixi-sandbox-starter",
            false,
            TAG,
        )
        .iter()
        .any(|r| r.contains("canonical"))
    );
    assert!(
        publish_refusals(
            "https://github.com/Archont561/pixi-sandbox-starter",
            "Archont561/pixi-sandbox-starter",
            true,
            TAG,
        )
        .iter()
        .any(|r| r.contains("immutable"))
    );
    assert!(
        publish_refusals(
            "https://github.com/Archont561/pixi-sandbox-starter",
            "Archont561/pixi-sandbox-starter",
            false,
            TAG,
        )
        .is_empty()
    );
}

fn write_valid_starter(dir: &Path, workflow_version: &str) {
    scaffold(dir, "starter", TAG, COMMIT, SOURCE, STARTER).unwrap();
    std::fs::create_dir_all(dir.join(".github/workflows")).unwrap();
    std::fs::write(dir.join("pixi.lock"), "version: 6\n").unwrap();
    std::fs::write(dir.join("pixi-sandbox.toml"), "schema = 1\n").unwrap();
    std::fs::write(
        dir.join(".github/workflows/publish-sandbox.yml"),
        format!(
            "# Generated by pixi-sandbox init\n\
             # pixi-sandbox-version: {workflow_version}\n\
             name: publish sandbox\n\
             env:\n  PIXI_SANDBOX_VERSION: {workflow_version}\n\
             jobs:\n  x:\n    runs-on: ubuntu-latest\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join(".github/workflows/relock.yml"), "name: relock\n").unwrap();
}
