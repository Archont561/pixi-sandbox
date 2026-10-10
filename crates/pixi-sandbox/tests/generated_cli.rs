//! TASK-76 AC#5: generator/CLI compatibility, enforced against the binary the workflow pins.
//!
//! The generated workflow invokes pixi-sandbox verbs (`plan`, `fetch-release`, `pipeline`,
//! `self-update`, `upgrade`). Every one of those invocations runs against the *released*
//! binary the workflow pins as `PIXI_SANDBOX_VERSION` — never against whatever happens to be
//! on a developer's PATH. The honest local enforcement has two halves:
//!
//! 1. The pin is the rendering binary's own version: render with `CARGO_PKG_VERSION`, assert
//!    the workflow pins exactly that, and assert the binary under test reports it. A newly
//!    built local CLI is therefore never *treated* as proof that an older released binary
//!    supports new verbs — the pin only advances when a release of this same code ships, and
//!    the template and the binary release together.
//! 2. Every pixi-sandbox invocation the generator emits is accepted by that binary: each
//!    extracted invocation's verb must have a working `--help`, and every flag it passes
//!    must be documented in that verb's help. The test fails the moment the generator emits
//!    a verb or flag the CLI does not accept.
//!
//! The emitted verb set is asserted exactly, so adding a verb to the template is a conscious
//! act: update this list, the golden fixture and the render tests together.

mod support;

use pixi_sandbox::generated::{GithubWorkflowOptions, parse_version_stamp, render_github_workflow};
use rstest::rstest;
use std::collections::BTreeSet;

const EXPECTED_VERBS: &[&str] = &[
    "fetch-release",
    "pipeline",
    "plan",
    "self-update",
    "upgrade",
];

fn render_with_the_running_version() -> String {
    render_github_workflow(GithubWorkflowOptions {
        version: env!("CARGO_PKG_VERSION"),
        config_path: "config/pixi-sandbox.toml",
        workflow_path: ".github/workflows/publish-sandbox.yml",
        relock_workflow_path: ".github/workflows/relock.yml",
        relock_ci_workflow: "ci.yml",
        script_path: "restore.sh",
        branch: "sandbox/developer-linux-64",
        push_paths: &[],
        scoped_permissions: false,
        concurrency: None,
        plan_timeout_minutes: None,
        publish_timeout_minutes: None,
        pixi_version: None,
        setup_pixi_cache: None,
    })
}

/// Every command line the render puts in a `run:` scalar: folded blocks are unfolded (a
/// folded scalar is one command however it wraps), one-line runs are themselves. Comments
/// and blanks inside blocks are not commands.
fn run_commands(render: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut in_block = false;
    let mut block_indent = 0;
    let mut folded: Vec<String> = Vec::new();
    for line in render.lines() {
        let stripped = line.trim_start();
        if let Some(rest) = stripped.strip_prefix("run:") {
            let value = rest.trim();
            in_block = matches!(value, "|" | ">" | "|-" | ">-" | "|+" | ">+");
            block_indent = line.len() - stripped.len();
            folded = Vec::new();
            if !in_block && !value.is_empty() && !value.starts_with('#') {
                commands.push(value.to_string());
            }
            continue;
        }
        if in_block {
            let indent = line.len() - stripped.len();
            if !stripped.is_empty() && indent > block_indent {
                if !stripped.starts_with('#') {
                    folded.push(stripped.to_string());
                }
                continue;
            }
            if !folded.is_empty() {
                commands.push(folded.join(" "));
            }
            in_block = false;
        }
    }
    if !folded.is_empty() {
        commands.push(folded.join(" "));
    }
    commands
}

/// Normalize one invocation's tokens for clap-shaped checking: GitHub expressions become a
/// placeholder value, shell variables become a placeholder value, quotes and trailing shell
/// punctuation are stripped.
fn normalize(invocation: &str) -> Vec<String> {
    let mut text = invocation.to_string();
    // GitHub expressions `${{ … }}` → `true` (a valid value for every value-taking flag in
    // the render, including the bool-valued `--cargo-vendor`).
    while let Some(start) = text.find("${{") {
        match text[start..].find("}}") {
            Some(close) => text = format!("{}true{}", &text[..start], &text[start + close + 2..]),
            None => break,
        }
    }
    // Shell parameters `${VAR}` → `var`.
    while let Some(start) = text.find("${") {
        match text[start..].find('}') {
            Some(close) => text = format!("{}var{}", &text[..start], &text[start + close + 1..]),
            None => break,
        }
    }
    // Shell variables `$VAR` → `var`.
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(dollar) = rest.find('$') {
        out.push_str(&rest[..dollar]);
        let after = &rest[dollar + 1..];
        let name_len = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(after.len());
        out.push_str("var");
        rest = &after[name_len..];
    }
    out.push_str(rest);
    out.split_whitespace()
        .map(|token| {
            token
                .trim_matches('"')
                .trim_end_matches(')')
                .trim_matches('"')
                .to_string()
        })
        .filter(|token| !token.is_empty())
        .filter(|token| !matches!(token.as_str(), ">>" | "&" | "|" | ";" | "&&" | "||"))
        .collect()
}

/// Every pixi-sandbox invocation in one command line, as `<verb> <args…>` token vectors.
/// The render's invoking binaries are `pixi-sandbox` itself and `"$PIXI_SANDBOX_BIN"` (the
/// job-level alias for the downloaded standalone binary). An invoking binary only counts at
/// the start of a command (modulo the YAML quoting of a one-line `run:` scalar) or as the
/// command of a `$( … )` substitution (the plan step's `echo "matrix=$(… )"`) — the same
/// alias also appears as a flag *value* (`--dest "$PIXI_SANDBOX_BIN"`), which is not an
/// invocation.
fn invocations(command: &str) -> Vec<Vec<String>> {
    let mut found = Vec::new();
    // Only the YAML quoting of a one-line `run:` scalar (single quotes) and whitespace are
    // stripped to find the first token; the shell's own double quotes are part of the command.
    let starts_at = command.len() - command.trim_start_matches(['\'', ' ', '\t']).len();
    for needle in ["pixi-sandbox ", "\"$PIXI_SANDBOX_BIN\" "] {
        if command[..].find(needle) == Some(starts_at) {
            push_invocation(&mut found, &command[starts_at..], false);
            continue;
        }
        if let Some(at) = command.find(&format!("$({needle}")) {
            push_invocation(&mut found, &command[at + 2..], true);
        }
    }
    found
}

/// Normalize one invocation slice and keep `<verb> <args…>` (the invoking binary token is
/// dropped). A substitution slice ends at its closing paren.
fn push_invocation(found: &mut Vec<Vec<String>>, slice: &str, in_substitution: bool) {
    let slice = if in_substitution {
        match slice.find(')') {
            Some(close) => &slice[..close],
            None => slice,
        }
    } else {
        slice
    };
    let tokens = normalize(slice);
    if tokens.len() >= 2 {
        found.push(tokens[1..].to_vec());
    }
}

fn all_invocations(render: &str) -> Vec<Vec<String>> {
    run_commands(render)
        .iter()
        .flat_map(|command| invocations(command))
        .collect()
}

#[test]
fn the_workflow_pins_the_version_of_the_binary_that_rendered_it() {
    let render = render_with_the_running_version();

    // The binary under test reports its own version…
    let output = support::bin()
        .arg("--version")
        .output()
        .expect("the binary runs");
    assert!(output.status.success());
    let version_line = String::from_utf8(output.stdout).expect("UTF-8 version");
    let reported = version_line
        .split_whitespace()
        .last()
        .expect("a version token");
    assert_eq!(reported, env!("CARGO_PKG_VERSION"));

    // …the render pins exactly that version, in both the env and the version stamp.
    assert!(
        render.contains(&format!("PIXI_SANDBOX_VERSION: {reported}")),
        "the workflow must pin the rendering binary's version:\n{render}"
    );
    assert_eq!(parse_version_stamp(&render), Some(reported));
}

#[test]
fn every_emitted_invocation_is_accepted_by_the_pinned_binary() {
    let render = render_with_the_running_version();
    let invocations = all_invocations(&render);
    assert!(
        !invocations.is_empty(),
        "the extractor found no pixi-sandbox invocation in the render"
    );

    for argv in &invocations {
        let verb = &argv[0];
        let help = support::bin()
            .args([verb, "--help"])
            .output()
            .unwrap_or_else(|error| panic!("running `{verb} --help`: {error}"));
        assert!(
            help.status.success(),
            "the generator emits `{verb}`, which this CLI does not accept:\n{}",
            String::from_utf8_lossy(&help.stderr)
        );
        let help_text = String::from_utf8_lossy(&help.stdout).into_owned();
        for token in &argv[1..] {
            if let Some(flag) = token.strip_prefix("--") {
                let flag = flag.split('=').next().expect("a flag name");
                assert!(
                    help_text.contains(&format!("--{flag}")),
                    "the generator passes `--{flag}` to `{verb}`, which does not document it:\n{help_text}"
                );
            }
        }
    }
}

#[rstest]
#[case("plan")]
#[case("fetch-release")]
#[case("pipeline")]
#[case("self-update")]
#[case("upgrade")]
fn the_emitted_verb_set_is_exactly_the_reviewed_set(#[case] verb: &str) {
    let render = render_with_the_running_version();
    let invocations = all_invocations(&render);
    let emitted: BTreeSet<&str> = invocations.iter().map(|argv| argv[0].as_str()).collect();
    let expected: BTreeSet<&str> = EXPECTED_VERBS.iter().copied().collect();
    assert_eq!(
        emitted, expected,
        "the emitted verb set drifted — update this ratchet, the golden fixture and the \
         render tests together"
    );
    assert!(emitted.contains(verb));
}
