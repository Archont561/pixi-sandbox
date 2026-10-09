//! `starter` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

use xtask::starter::*;

mod support;
use support::{commit_all, run_git, starter_tree};

const TAG: &str = "v1.2.3";
const COMMIT: &str = "8193c58068c3245e5267c4c604f989237e4d6e49";
const SOURCE: &str = "Archont561/pixi-sandbox";
const STARTER: &str = "Archont561/pixi-sandbox-starter";

/// The lane that publishes the starter, read as text. The two assertions below are about step
/// *order*, which no unit test can see: they are the reason the workflow still runs the proof on
/// the artifact a user gets, and refuses a bad dispatch before it touches anything.
const STARTER_WORKFLOW: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../.github/workflows/starter.yml"
));

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
    assert!(STARTER_WORKFLOW.contains("xtask starter-check-main"));
    assert!(!STARTER_WORKFLOW.contains(".default_branch // empty"));
    assert!(!STARTER_WORKFLOW.contains("--method POST"));
    assert!(!STARTER_WORKFLOW.contains("--method PUT"));
}

/// The template's promise is that a *clone* works, so the proof runs on a clone: assembling it
/// through `git` is the only way to hand the dev task the tree that would be published rather
/// than the directory the earlier steps wrote into (task-85 AC#1).
#[test]
fn the_starter_dev_proof_runs_in_a_fresh_clone_of_the_would_be_committed_tree() {
    assert!(
        STARTER_WORKFLOW.contains("xtask starter-clone -- --dir starter --out"),
        "{STARTER_WORKFLOW}"
    );
    assert!(
        STARTER_WORKFLOW.contains(
            r#"pixi run --manifest-path "$RUNNER_TEMP/starter-clone/starter/pixi.toml" dev"#
        ),
        "the dev task must run in the clone, not in starter/"
    );
    assert!(
        !STARTER_WORKFLOW.contains("pixi run --manifest-path starter/pixi.toml dev"),
        "the old proof ran against the tree the tooling had just written into"
    );
}

/// A mismatched `source-commit` used to be discovered after a full job start. As its own job it
/// costs the lane nothing: the starter repository is never fetched and no file is scaffolded
/// before the guard has agreed (task-85 AC#4).
#[test]
fn the_dispatch_guard_is_its_own_job_and_precedes_every_starter_write() {
    let guard = STARTER_WORKFLOW
        .find("xtask starter-check-dispatch")
        .expect("the guard verb the first job runs");
    let needs = STARTER_WORKFLOW
        .find("needs: dispatch")
        .expect("the starter job must wait for the guard");
    let checkout = STARTER_WORKFLOW
        .find("repository: Archont561/pixi-sandbox-starter")
        .expect("the starter checkout, which must come after the guard");
    let scaffold = STARTER_WORKFLOW
        .find("xtask starter-scaffold")
        .expect("the step that first writes into the starter");
    assert!(guard < needs, "the guard belongs to its own job");
    assert!(
        needs < checkout && checkout < scaffold,
        "{needs} {checkout} {scaffold}"
    );
}

/// A starter whose publisher came from a different release is the one failure the whole
/// publication contract exists to prevent, so the marker check is exact, not a substring.
#[test]
fn a_publisher_from_another_release_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    starter_tree(dir.path(), "9.9.9");
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
    starter_tree(dir.path(), "1.2.3");
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
///
/// This is the conservative half of the rule: with no git work tree to ask, existence decides,
/// because a scratch directory cannot prove that anything in it is left out of a commit.
#[test]
fn committed_runtime_state_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    starter_tree(dir.path(), "1.2.3");
    std::fs::create_dir_all(dir.path().join(".pixi")).unwrap();
    std::fs::write(dir.path().join("SHA256SUMS"), "x").unwrap();
    let findings = verify(dir.path(), TAG).unwrap();
    assert!(findings.iter().any(|f| f.contains(".pixi")), "{findings:?}");
    assert!(
        findings.iter().any(|f| f.contains("SHA256SUMS")),
        "{findings:?}"
    );
}

/// The publish lane's own steps (`pixi lock`, `pixi run dev`) necessarily leave `.pixi/` behind
/// in the assembled tree, and the starter's own `.gitignore` keeps those bytes out of the
/// revision. Verdict on the working directory instead of on what git would commit is the
/// false positive that left the starter unpublishable (task-85 AC#2).
#[test]
fn runtime_state_the_ignore_file_keeps_out_of_the_revision_is_not_a_finding() {
    let dir = tempfile::tempdir().unwrap();
    starter_tree(dir.path(), "1.2.3");
    run_git(dir.path(), &["init", "-q"]);
    std::fs::create_dir_all(dir.path().join(".pixi/envs/default")).unwrap();
    std::fs::write(dir.path().join(".pixi/envs/default/rg"), "restored state").unwrap();
    std::fs::create_dir_all(dir.path().join(".pixi-sandbox/vendor")).unwrap();
    std::fs::write(dir.path().join(".pixi-sandbox/vendor/serde.crate"), "x").unwrap();
    std::fs::write(
        dir.path().join("SHA256SUMS"),
        "downloaded, never committed\n",
    )
    .unwrap();
    assert_eq!(verify(dir.path(), TAG).unwrap(), Vec::<String>::new());
}

/// The other side of the same rule: once a path is in the index, the ignore file no longer
/// protects anyone, and a revision that carries runtime state is still refused.
#[test]
fn runtime_state_that_git_would_commit_is_still_a_finding() {
    let dir = tempfile::tempdir().unwrap();
    starter_tree(dir.path(), "1.2.3");
    run_git(dir.path(), &["init", "-q"]);
    std::fs::create_dir_all(dir.path().join(".pixi/envs/default")).unwrap();
    std::fs::write(
        dir.path().join(".pixi/envs/default/rg"),
        "stale environment",
    )
    .unwrap();
    // `-f` is the accident this criterion is about: an ignore rule does not survive a staged file.
    run_git(dir.path(), &["add", "-f", "-A"]);
    commit_all(dir.path(), "seed");
    let findings = verify(dir.path(), TAG).unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f.contains(".pixi") && f.contains("must not be committed")),
        "{findings:?}"
    );
}

/// `git add -A` would take a `.env` in an otherwise clean tree — no ignore rule covers it —
/// so the git-aware verdict must stay loud. This is the case a blanket "if it is ignored, skip
/// the check" shortcut would silently drop.
#[test]
fn a_credential_git_would_take_is_a_finding_even_untracked() {
    let dir = tempfile::tempdir().unwrap();
    starter_tree(dir.path(), "1.2.3");
    run_git(dir.path(), &["init", "-q"]);
    std::fs::write(dir.path().join(".env"), "TOKEN=leak\n").unwrap();
    let findings = verify(dir.path(), TAG).unwrap();
    assert!(findings.iter().any(|f| f.contains(".env")), "{findings:?}");
}

/// The dispatch guard exists because a mismatched `source-commit` used to surface only after a
/// full job start (task-85 AC#4). Its contract is small: agree, or name both commits and say
/// which one to pass.
#[test]
fn a_dispatch_naming_the_commit_the_tag_points_at_is_accepted() {
    assert_eq!(
        dispatch_findings(TAG, COMMIT, Some(COMMIT)),
        Vec::<String>::new()
    );
}

#[test]
fn an_abbreviated_dispatch_matches_the_full_commit_the_tag_names() {
    // Parity with `release_findings`: a dispatch may carry the short form the UI offers.
    assert_eq!(
        dispatch_findings(TAG, &COMMIT[..7], Some(COMMIT)),
        Vec::<String>::new()
    );
}

#[test]
fn a_mismatched_dispatch_names_both_commits_and_the_one_to_pass() {
    let other = "00000000000000000000000000000000000000aa";
    let findings = dispatch_findings(TAG, other, Some(COMMIT));
    assert_eq!(findings.len(), 1, "{findings:?}");
    let message = &findings[0];
    assert!(message.contains(TAG), "{message}");
    assert!(
        message.contains(other) && message.contains(COMMIT),
        "both commits have to appear: {message}"
    );
    assert!(
        message.contains(other),
        "the remedy names the tag's commit: {message}"
    );
}

#[test]
fn an_unresolvable_tag_is_a_finding_that_names_the_dispatched_commit() {
    let other = "00000000000000000000000000000000000000aa";
    let findings = dispatch_findings(TAG, other, None);
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].contains("does not resolve"), "{findings:?}");
    assert!(findings[0].contains(other), "{findings:?}");
}

#[test]
fn a_loose_release_tag_is_refused_before_anything_is_dispatched() {
    let findings = dispatch_findings("latest", COMMIT, Some(COMMIT));
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].contains("strict semver"), "{findings:?}");
}

/// "Nothing in the workflow tells the operator which commit the tag names" is half the complaint
/// the task was written from, so the accepted case has to say it out loud too.
#[test]
fn the_operator_is_told_which_commit_the_tag_names() {
    let report = dispatch_report(TAG, COMMIT);
    assert!(report.contains(TAG) && report.contains(COMMIT), "{report}");
    assert!(report.contains("names"), "{report}");
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
