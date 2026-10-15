#![cfg(unix)]

//! The starter lane's publication credential (TASK-74 AC#7 and AC#8).
//!
//! The lane pushes `.github/workflows/*` into *another* repository. The repository
//! `GITHUB_TOKEN` can do neither — it is scoped to the repository running the workflow, and
//! GitHub bars it from workflow files whatever the `permissions:` block says (issue #101). Until
//! 2026-10-16 the workflow answered a missing `PIXI_SANDBOX_STARTER_TOKEN` by falling back to
//! that token, so a run spent its whole scaffold budget before failing on a push GitHub was
//! always going to refuse, and the annotation blamed the push rather than the credential.
//!
//! These tests hold the preflight instead: it runs before the lane touches the starter
//! repository, and it names the secret, the repository and the scopes it needs. Exercised
//! through the CLI because that is the seam the workflow calls.

use std::{fs, process::Command};

use tempfile::tempdir;

const REPO: &str = "Archont561/pixi-sandbox-starter";

/// Run the preflight with `STARTER_TOKEN` set to `token` — or absent — and a temp step summary.
fn check_token(token: Option<&str>) -> (std::process::Output, String) {
    let temp = tempdir().expect("temporary directory");
    let summary = temp.path().join("summary");
    let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
    command
        .args(["starter-check-token", "--repo", REPO])
        .env("GITHUB_STEP_SUMMARY", &summary)
        .env_remove("STARTER_TOKEN");
    if let Some(token) = token {
        command.env("STARTER_TOKEN", token);
    }
    let run = command.output().expect("run starter-check-token");
    (run, fs::read_to_string(&summary).unwrap_or_default())
}

#[test]
fn a_missing_token_fails_before_any_starter_access_naming_the_secret_and_its_scopes() {
    let (run, summary) = check_token(None);
    assert!(
        !run.status.success(),
        "a lane with no publication credential must not proceed"
    );
    let error = String::from_utf8_lossy(&run.stderr);
    assert!(error.contains("::error::"), "{error}");
    assert!(error.contains("PIXI_SANDBOX_STARTER_TOKEN"), "{error}");
    assert!(error.contains(REPO), "{error}");
    assert!(error.contains("Contents"), "{error}");
    assert!(error.contains("Workflows"), "{error}");
    assert!(summary.contains("## Starter token"), "{summary}");
    assert!(summary.contains("PIXI_SANDBOX_STARTER_TOKEN"), "{summary}");
}

#[test]
fn a_token_that_is_only_whitespace_is_not_a_credential() {
    let (run, _) = check_token(Some("   "));
    assert!(!run.status.success());
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("PIXI_SANDBOX_STARTER_TOKEN"),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}

#[test]
fn a_present_token_is_acknowledged_without_ever_being_printed() {
    let (run, summary) = check_token(Some("github_pat_11AAAstarter_publication_value"));
    assert!(
        run.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let reported = format!(
        "{}{}{summary}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        !reported.contains("github_pat_11AAAstarter_publication_value"),
        "the token value must never reach a log or a summary: {reported}"
    );
    assert!(summary.contains("## Starter token"), "{summary}");
    assert!(summary.contains(REPO), "{summary}");
}
