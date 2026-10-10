//! Tests for `pixi_sandbox::pipeline` — the publish pipeline the generated workflow runs
//! as a single `pixi-sandbox pipeline …` step (TASK-76).
//!
//! The orchestration — phase order, exact argv, outcome recording, the failure summary and
//! annotation — is asserted against a fake runner. No phase ever executes, no socket is
//! opened, and no test touches this checkout or a real HOME (D10).

use anyhow::Result;
use pixi_sandbox::pipeline::{self, PhaseCommand, PhaseRunner, Spec};
use rstest::{fixture, rstest};
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::io::Write;

/// A scripted child-process boundary: every call returns the next scripted outcome and
/// writes the scripted output to the log sink, exactly where a real tee would put it.
#[derive(Debug, Default)]
struct FakeRunner {
    script: VecDeque<(i32, Vec<u8>)>,
    fail_spawn: bool,
    seen: Vec<PhaseCommand>,
}

impl FakeRunner {
    fn script(mut self, outcomes: &[(i32, &str)]) -> Self {
        self.script = outcomes
            .iter()
            .map(|(code, output)| (*code, output.as_bytes().to_vec()))
            .collect();
        self
    }
}

impl PhaseRunner for FakeRunner {
    fn run(&mut self, command: &PhaseCommand, log: &mut (dyn Write + Send)) -> Result<i32> {
        self.seen.push(command.clone());
        if self.fail_spawn {
            anyhow::bail!("no such program: {}", command.program);
        }
        let (code, output) = self.script.pop_front().unwrap_or((0, Vec::new()));
        log.write_all(&output).expect("write to the log sink");
        Ok(code)
    }
}

struct Ctx {
    _dir: tempfile::TempDir,
    spec: Spec,
}

#[fixture]
fn ctx() -> Ctx {
    let dir = tempfile::tempdir().expect("tempdir");
    let spec = Spec {
        envs: "default, web".to_string(),
        platform: "linux-64".to_string(),
        branch: "sandbox/developer-linux-64".to_string(),
        remote: "https://example.invalid/repo.git".to_string(),
        config: dir.path().join("pixi-sandbox.toml"),
        self_bin: dir.path().join("bin/pixi-sandbox"),
        exe: dir.path().join("bin/pixi-sandbox"),
        pixi: "pixi".to_string(),
        cargo_vendor: false,
        transport_dir: dir.path().join("transport"),
        log_dir: dir.path().join("logs"),
        push_token: None,
        step_summary: None,
    };
    Ctx { _dir: dir, spec }
}

fn arg_strings(phase: &PhaseCommand) -> Vec<&str> {
    phase.args.iter().map(String::as_str).collect()
}

#[rstest]
fn plan_runs_install_pack_doctor_publish_in_order(ctx: Ctx) {
    let phases = pipeline::plan(&ctx.spec);
    let names: Vec<&str> = phases.iter().map(|phase| phase.phase).collect();
    assert_eq!(
        names,
        vec!["install", "install", "pack", "doctor", "publish"]
    );

    // The install phase runs `pixi install --frozen -e <env>` once per environment.
    assert_eq!(phases[0].program, "pixi");
    assert_eq!(
        arg_strings(&phases[0]),
        vec!["install", "--frozen", "-e", "default"]
    );
    assert_eq!(
        arg_strings(&phases[1]),
        vec!["install", "--frozen", "-e", "web"]
    );
    assert!(phases[0].env.is_empty());

    // pack: the exact argv the generated shell used to spell out in two dialects.
    let transport = ctx.spec.transport_dir.display().to_string();
    let config = ctx.spec.config.display().to_string();
    let self_bin = ctx.spec.self_bin.display().to_string();
    let logs = ctx.spec.log_dir.display().to_string();
    assert_eq!(phases[2].program, ctx.spec.exe.display().to_string());
    assert_eq!(
        arg_strings(&phases[2]),
        vec![
            "pack",
            "--repo-root",
            ".",
            "--envs",
            "default, web",
            "--output-dir",
            &transport,
            "--platform",
            "linux-64",
            "--fetch-tools",
            "--self-bin",
            &self_bin,
            "--config",
            &config,
            "--log-file",
            &format!("{logs}/pack.log"),
        ]
    );

    // doctor verifies the transport and enforces the reviewed budgets before publish.
    assert_eq!(
        arg_strings(&phases[3]),
        vec![
            "doctor",
            "--branch-location",
            &transport,
            "--verify",
            "--budget-config",
            &config,
            "--log-file",
            &format!("{logs}/doctor.log"),
        ]
    );

    // publish pushes the transport to the configured remote.
    assert_eq!(
        arg_strings(&phases[4]),
        vec![
            "publish",
            "--input-dir",
            &transport,
            "--branch-name",
            "sandbox/developer-linux-64",
            "--remote",
            "https://example.invalid/repo.git",
            "--log-file",
            &format!("{logs}/publish.log"),
        ]
    );
    assert!(phases[4].env.is_empty(), "no push token, no auth header");
}

#[rstest]
fn plan_strips_whitespace_from_environment_names_but_passes_the_list_verbatim(ctx: Ctx) {
    let mut spec = ctx.spec.clone();
    spec.envs = " default ,\tweb ".to_string();
    let phases = pipeline::plan(&spec);
    assert_eq!(
        arg_strings(&phases[0]),
        vec!["install", "--frozen", "-e", "default"]
    );
    assert_eq!(
        arg_strings(&phases[1]),
        vec!["install", "--frozen", "-e", "web"]
    );
    // pack receives the matrix value verbatim, like the shell's `--envs "$ENVIRONMENTS"`.
    let pack = &phases[2];
    let envs = arg_strings(pack);
    let position = envs
        .iter()
        .position(|arg| *arg == "--envs")
        .expect("--envs");
    assert_eq!(envs[position + 1], " default ,\tweb ");
}

#[rstest]
fn plan_adds_cargo_vendor_only_when_asked(ctx: Ctx) {
    let mut spec = ctx.spec.clone();
    spec.cargo_vendor = true;
    let pack = &pipeline::plan(&spec)[2];
    assert_eq!(pack.args.last().expect("last arg"), "--cargo-vendor");

    let pack = &pipeline::plan(&ctx.spec)[2];
    assert!(!pack.args.iter().any(|arg| arg == "--cargo-vendor"));
}

#[rstest]
fn plan_gives_only_the_publish_phase_the_auth_header(ctx: Ctx) {
    let mut spec = ctx.spec.clone();
    spec.push_token = Some("secret".to_string());
    let phases = pipeline::plan(&spec);
    for phase in &phases[..4] {
        assert!(
            phase.env.is_empty(),
            "{} carries no credentials",
            phase.phase
        );
    }
    // The worked example is base64("x-access-token:secret"), computed independently of the
    // code under test.
    assert_eq!(
        phases[4].env,
        vec![
            ("GIT_CONFIG_COUNT".to_string(), "1".to_string()),
            (
                "GIT_CONFIG_KEY_0".to_string(),
                "http.extraheader".to_string()
            ),
            (
                "GIT_CONFIG_VALUE_0".to_string(),
                "AUTHORIZATION: basic eC1hY2Nlc3MtdG9rZW46c2VjcmV0".to_string()
            ),
        ]
    );
}

#[rstest]
fn base64_matches_rfc_4648() {
    // RFC 4648's own test vectors, plus the padding-free case.
    let vectors: &[(&[u8], &str)] = &[
        (b"", ""),
        (b"f", "Zg=="),
        (b"fo", "Zm8="),
        (b"foo", "Zm9v"),
        (b"foob", "Zm9vYg=="),
        (b"fooba", "Zm9vYmE="),
        (b"foobar", "Zm9vYmFy"),
        (
            b"x-access-token:ghp_testtoken123",
            "eC1hY2Nlc3MtdG9rZW46Z2hwX3Rlc3R0b2tlbjEyMw==",
        ),
    ];
    for (input, expected) in vectors {
        assert_eq!(
            &pipeline::base64_standard(input),
            expected,
            "input: {input:?}"
        );
    }
}

#[rstest]
fn a_successful_run_records_every_phase_outcome_in_the_pipeline_log(ctx: Ctx) {
    let mut runner = FakeRunner::default().script(&[
        (0, "installing default\n"),
        (0, "installing web\n"),
        (0, "packing\n"),
        (0, "verifying\n"),
        (0, "published\n"),
    ]);

    pipeline::run(&ctx.spec, &mut runner).expect("the pipeline succeeds");

    let log = std::fs::read_to_string(ctx.spec.log_dir.join(pipeline::PIPELINE_LOG))
        .expect("read the pipeline log");
    let mut expected = String::new();
    for (phase, output) in [
        ("install", "installing default\n"),
        ("install", "installing web\n"),
        ("pack", "packing\n"),
        ("doctor", "verifying\n"),
        ("publish", "published\n"),
    ] {
        let _ = writeln!(expected, "command=pipeline phase={phase} event=start");
        expected.push_str(output);
        let _ = writeln!(expected, "command=pipeline phase={phase} result=success");
    }
    assert_eq!(log, expected);
    // Every phase ran, in order.
    let names: Vec<&str> = runner.seen.iter().map(|phase| phase.phase).collect();
    assert_eq!(
        names,
        vec!["install", "install", "pack", "doctor", "publish"]
    );
}

#[rstest]
fn a_failed_phase_stops_the_pipeline_and_reports_it(ctx: Ctx) {
    // Two environments, so two install phases; the pack phase is the one that fails.
    let mut runner = FakeRunner::default().script(&[(0, ""), (0, ""), (3, "pack exploded\n")]);

    let failure = pipeline::run(&ctx.spec, &mut runner).unwrap_err();

    assert_eq!(failure.phase, "pack");
    assert_eq!(failure.exit_code, 3);
    assert_eq!(
        failure.annotation,
        "::error::publish pipeline failed during pack (exit 3)"
    );
    // The phases after the failure never ran.
    let names: Vec<&str> = runner.seen.iter().map(|phase| phase.phase).collect();
    assert_eq!(names, vec!["install", "install", "pack"]);

    // The log records the failure.
    let log = std::fs::read_to_string(ctx.spec.log_dir.join(pipeline::PIPELINE_LOG))
        .expect("read the pipeline log");
    assert!(
        log.contains("command=pipeline phase=pack result=failure exit=3"),
        "{log}"
    );

    // The summary is the shell's shape: header, failed phase, branch, artifact name, and
    // the log excerpt.
    let summary = &failure.summary;
    assert!(
        summary.contains("### ❌ pixi-sandbox publish failed on linux-64"),
        "{summary}"
    );
    assert!(
        summary.contains("- **Failed phase**: `pack` (exit code 3)"),
        "{summary}"
    );
    assert!(
        summary.contains("- **Branch**: `sandbox/developer-linux-64`"),
        "{summary}"
    );
    assert!(
        summary.contains("publish-diagnostics-linux-64"),
        "{summary}"
    );
    assert!(
        summary.contains("<details><summary>Diagnostic log excerpt (last 50 lines)</summary>"),
        "{summary}"
    );
    assert!(summary.contains("pack exploded"), "{summary}");
    assert!(summary.contains("result=failure exit=3"), "{summary}");
}

#[rstest]
fn the_failure_summary_carries_only_the_last_50_log_lines(ctx: Ctx) {
    let mut sixty = String::new();
    for n in 1..=60 {
        let _ = writeln!(sixty, "line {n}");
    }
    let mut runner = FakeRunner::default().script(&[(0, ""), (0, ""), (1, &sixty)]);

    let failure = pipeline::run(&ctx.spec, &mut runner).unwrap_err();

    let summary = &failure.summary;
    assert!(summary.contains("line 60"), "{summary}");
    assert!(
        summary.contains("line 12"),
        "the excerpt starts 50 lines back: {summary}"
    );
    assert!(
        !summary.contains("line 1\n"),
        "line 1 fell off the excerpt: {summary}"
    );
    assert!(
        !summary.contains("line 11\n"),
        "line 11 fell off the excerpt: {summary}"
    );
    let excerpt = summary
        .split("```text\n")
        .nth(1)
        .and_then(|rest| rest.split("```\n").next())
        .expect("the excerpt is fenced");
    assert_eq!(
        excerpt.lines().count(),
        50,
        "the excerpt is exactly 50 lines"
    );
}

#[rstest]
fn the_summary_is_appended_to_the_summary_file_when_the_runner_provides_one(ctx: Ctx) {
    let mut spec = ctx.spec.clone();
    let summary_path = spec.log_dir.join("summary.md");
    spec.step_summary = Some(summary_path.clone());
    let mut runner = FakeRunner::default().script(&[(0, ""), (0, ""), (2, "")]);

    let failure = pipeline::run(&spec, &mut runner).unwrap_err();

    let written = std::fs::read_to_string(&summary_path).expect("read the summary file");
    assert_eq!(written, failure.summary);
}

#[rstest]
fn a_rerun_resets_the_log_directory(ctx: Ctx) {
    std::fs::create_dir_all(&ctx.spec.log_dir).expect("mkdir");
    std::fs::write(
        ctx.spec.log_dir.join(pipeline::PIPELINE_LOG),
        "stale evidence\n",
    )
    .expect("write stale log");
    std::fs::write(ctx.spec.log_dir.join("stale.txt"), "stale\n").expect("write stale file");
    let mut runner = FakeRunner::default();

    pipeline::run(&ctx.spec, &mut runner).expect("the pipeline succeeds");

    let log = std::fs::read_to_string(ctx.spec.log_dir.join(pipeline::PIPELINE_LOG))
        .expect("read the pipeline log");
    assert!(!log.contains("stale evidence"), "{log}");
    assert!(
        !ctx.spec.log_dir.join("stale.txt").exists(),
        "the directory was recreated"
    );
}

#[rstest]
fn a_stale_transport_directory_is_removed_before_pack(ctx: Ctx) {
    std::fs::create_dir_all(&ctx.spec.transport_dir).expect("mkdir");
    std::fs::write(ctx.spec.transport_dir.join("stale-blob"), "stale\n").expect("write");
    let mut runner = FakeRunner::default();

    pipeline::run(&ctx.spec, &mut runner).expect("the pipeline succeeds");

    assert!(
        !ctx.spec.transport_dir.join("stale-blob").exists(),
        "pack refuses a stale output directory; the pipeline removes it first"
    );
}

#[rstest]
fn a_child_that_cannot_start_fails_its_phase_with_127(ctx: Ctx) {
    let mut runner = FakeRunner {
        fail_spawn: true,
        ..FakeRunner::default()
    };

    let failure = pipeline::run(&ctx.spec, &mut runner).unwrap_err();

    assert_eq!(failure.phase, "install");
    assert_eq!(failure.exit_code, 127);
    assert_eq!(
        failure.annotation,
        "::error::publish pipeline failed during install (exit 127)"
    );
    // The reason is in the log, where an operator reads it.
    let log = std::fs::read_to_string(ctx.spec.log_dir.join(pipeline::PIPELINE_LOG))
        .expect("read the pipeline log");
    assert!(log.contains("no such program"), "{log}");
}
