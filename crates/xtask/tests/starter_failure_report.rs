#![cfg(unix)]

//! The starter lane's post-dispatch failure report (TASK-74 AC#8, doc-10 "Credentials and
//! safety").
//!
//! doc-10 is explicit: a failed generation, lock refresh, validation or delivery leaves the last
//! known-good revision unchanged *and* is visible in a GitHub step summary that names the release
//! tag, the source commit, the failed phase and a remediation. Until 2026-10-16 only the dispatch
//! phase and the publish *success* path wrote one — the v0.7.0 refusal (run 38800327950) left the
//! starter unchanged, as promised, but its annotation named no tag, no phase and no remedy, and
//! the comment above "Upload starter evidence" promised a summary no step wrote.
//!
//! The workflow hands the summarizer its own `steps` context, so the phase it names is the step
//! that actually failed rather than a guess. Exercised through the CLI, the seam the workflow
//! calls.

use std::{fs, process::Command};

use tempfile::tempdir;

const TAG: &str = "v1.2.3";
const COMMIT: &str = "8193c58068c3245e5267c4c604f989237e4d6e49";

/// The `steps` context of a job that failed in `verify`, after `scaffold` succeeded. Real ids,
/// real outcomes, in the shape `${{ toJSON(steps) }}` produces.
const FAILED_IN_VERIFY: &str = r#"{"main":{"outputs":{},"outcome":"success","conclusion":"success"},"scaffold":{"outputs":{},"outcome":"success","conclusion":"success"},"verify":{"outputs":{},"outcome":"failure","conclusion":"failure"},"publish":{"outputs":{},"outcome":"skipped","conclusion":"skipped"}}"#;

/// Run the summarizer with a step context, a temp step summary, and the lane's own env.
fn report_failure(steps: &str) -> (std::process::Output, String) {
    report_failure_with_env(steps, &[])
}

/// The same run, with extra environment — how a test puts a credential in the job.
fn report_failure_with_env(steps: &str, extra: &[(&str, &str)]) -> (std::process::Output, String) {
    let temp = tempdir().expect("temporary directory");
    let summary = temp.path().join("summary");
    let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
    command
        .arg("starter-report-failure")
        .env("GITHUB_STEP_SUMMARY", &summary)
        .env("STEPS", steps)
        .env("TAG", TAG)
        .env("SOURCE_COMMIT", COMMIT)
        .env_remove("STARTER_TOKEN")
        .env_remove("GH_TOKEN")
        .env_remove("GITHUB_TOKEN");
    for (key, value) in extra {
        command.env(key, value);
    }
    let run = command.output().expect("run starter-report-failure");
    (run, fs::read_to_string(&summary).unwrap_or_default())
}

#[test]
fn a_failed_phase_is_summarized_with_the_tag_the_commit_the_phase_and_a_remedy() {
    let (run, summary) = report_failure(FAILED_IN_VERIFY);
    assert!(
        run.status.success(),
        "reporting a failure must not itself fail the job again: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(summary.contains("## Starter phase failed"), "{summary}");
    assert!(summary.contains(TAG), "{summary}");
    assert!(summary.contains(COMMIT), "{summary}");
    assert!(
        summary.contains("verify"),
        "the summary must name the phase that failed: {summary}"
    );
    assert!(
        summary.contains(&format!("starter-{TAG}")),
        "the summary must point at the bounded artifact the run uploads: {summary}"
    );
    assert!(
        summary.contains("unchanged"),
        "the summary must say the previous revision still stands: {summary}"
    );
}

/// A cancelled job, or a failure in a step this table has never seen, still has to report
/// *something* bounded rather than an empty file.
#[test]
fn a_context_with_no_failed_step_is_reported_rather_than_left_blank() {
    let (run, summary) = report_failure(r#"{"pub":{"outputs":{},"outcome":"skipped"}}"#);
    assert!(run.status.success());
    assert!(summary.contains("## Starter phase failed"), "{summary}");
    assert!(summary.contains(TAG), "{summary}");
    assert!(
        summary.contains("log") || summary.contains("phase"),
        "an operator must still be told what to do: {summary}"
    );
}

/// The summarizer is the last word on a failed run, so a malformed context must degrade to a
/// report and never to a second error.
#[test]
fn a_malformed_step_context_does_not_turn_into_a_second_failure() {
    let (run, summary) = report_failure("not json at all");
    assert!(
        run.status.success(),
        "a reporting step must never mask the failure it reports: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(summary.contains("## Starter phase failed"), "{summary}");
}

/// AC#8's last clause: the summary must not expose a token or secret. The summarizer echoes the
/// id of a failed step it does not know, and the job's environment holds the publication
/// credential, so that echo is exactly where a credential could ride out into a permanent,
/// world-readable run summary.
#[test]
fn a_credential_in_the_job_never_reaches_the_summary() {
    let token = "github_pat_11AAAstarter_publication_value";
    let steps = format!(r#"{{"{token}":{{"outputs":{{}},"outcome":"failure"}}}}"#);
    let (run, summary) = report_failure_with_env(
        &steps,
        &[
            ("STARTER_TOKEN", token),
            ("GH_TOKEN", "gho_aSecondCredentialValue"),
        ],
    );
    assert!(
        run.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(!summary.contains(token), "{summary}");
    assert!(!summary.contains("gho_aSecondCredentialValue"), "{summary}");
    assert!(summary.contains("## Starter phase failed"), "{summary}");
    assert!(
        summary.contains("redacted"),
        "the summary must say it withheld something rather than silently dropping the phase: {summary}"
    );
}

/// A job that stopped on a step this table has never seen still has to name something useful, and
/// the phase it names must be the failing one rather than the first alphabetically.
#[test]
fn the_failed_phase_is_taken_from_the_lane_order_not_the_json_order() {
    let (run, summary) = report_failure(
        r#"{"a-step-added-later":{"outputs":{},"outcome":"failure"},"lnit":{"outputs":{},"outcome":"failure"}}"#,
    );
    assert!(run.status.success());
    assert!(summary.contains("a-step-added-later"), "{summary}");
    assert!(
        summary.contains("no remediation on record"),
        "an unknown id has no remediation of its own: {summary}"
    );
}

/// Step ids come from the workflow, but a summary is a permanent public surface, so an id that is
/// not an identifier is not printed as one.
#[test]
fn a_step_id_that_is_not_an_identifier_is_reported_as_unknown() {
    let (run, summary) = report_failure(r#"{"":{"outputs":{},"outcome":"failure"}}"#);
    assert!(run.status.success());
    assert!(summary.contains("phase `unknown`"), "{summary}");
}

/// An unset credential arrives as an empty string, and redacting the empty string would erase the
/// summary instead of protecting it.
#[test]
fn an_empty_credential_variable_leaves_the_summary_intact() {
    let (run, summary) = report_failure_with_env(
        FAILED_IN_VERIFY,
        &[("STARTER_TOKEN", ""), ("GH_TOKEN", "  ")],
    );
    assert!(run.status.success());
    assert!(summary.contains(TAG), "{summary}");
    assert!(summary.contains("verify"), "{summary}");
}
