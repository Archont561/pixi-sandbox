//! Contracts for the GitHub workflow emitted into consumer repositories.
//!
//! actionlint validates GitHub's full schema through xtask. These tests own the contracts it
//! cannot evaluate: indentation-derived matrix shape and agreement with the planner's JSON keys.

use pixi_sandbox::generated::{GithubWorkflowOptions, parse_version_stamp, render_github_workflow};
use pixi_sandbox_core::platform::Platform;
use pixi_sandbox_core::sandbox_config::plan_override;
use rstest::rstest;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const VERSION: &str = "9.8.7";
const CONFIG_PATH: &str = "config/pixi-sandbox.toml";
const WORKFLOW_PATH: &str = ".github/workflows/publish-sandbox.yml";
const RELOCK_PATH: &str = ".github/workflows/relock.yml";
const RELOCK_CI_WORKFLOW: &str = "ci.yml";
const SCRIPT_PATH: &str = "restore.sh";
const BRANCH: &str = "sandbox/developer-linux-64";

fn workflow() -> String {
    render_github_workflow(GithubWorkflowOptions {
        version: VERSION,
        config_path: CONFIG_PATH,
        workflow_path: WORKFLOW_PATH,
        relock_workflow_path: RELOCK_PATH,
        relock_ci_workflow: RELOCK_CI_WORKFLOW,
        script_path: SCRIPT_PATH,
        branch: BRANCH,
    })
}

/// The generated subset uses mappings, sequences of mappings, flow arrays, comments, and plain
/// scalars. Index its mapping structure from YAML indentation so assertions address paths rather
/// than matching an ambiguous snippet. actionlint remains the authoritative full YAML/GitHub
/// parser; this deliberately small parser makes the object-vs-array contract visible to Rust.
#[derive(Debug)]
struct YamlMappings {
    mappings: BTreeSet<String>,
    scalars: BTreeMap<String, String>,
}

fn parse_yaml_mappings(input: &str) -> YamlMappings {
    let mut mappings = BTreeSet::new();
    let mut scalars = BTreeMap::new();
    let mut parents: Vec<(usize, String)> = Vec::new();
    let mut block_scalar_at = None;

    for (line_number, line) in input.lines().enumerate() {
        assert!(
            !line.contains('\t'),
            "YAML line {} uses a tab",
            line_number + 1
        );
        let indent = line.len() - line.trim_start_matches(' ').len();
        let mut body = line.trim();
        if body.is_empty() {
            continue;
        }
        if block_scalar_at.is_some_and(|level| indent > level) {
            continue;
        }
        block_scalar_at = None;
        if body.starts_with('#') {
            continue;
        }
        assert_eq!(
            indent % 2,
            0,
            "YAML line {} has odd indentation: {line}",
            line_number + 1
        );
        if let Some(item) = body.strip_prefix("- ") {
            body = item;
        }
        let (key, value) = body.split_once(':').unwrap_or_else(|| {
            panic!(
                "YAML line {} is not a mapping entry: {line}",
                line_number + 1
            )
        });
        assert!(
            !key.is_empty(),
            "YAML line {} has an empty key",
            line_number + 1
        );

        while parents.last().is_some_and(|(level, _)| *level >= indent) {
            parents.pop();
        }
        let path = parents
            .iter()
            .map(|(_, key)| key.as_str())
            .chain(std::iter::once(key))
            .collect::<Vec<_>>()
            .join(".");
        let value = value.trim();
        if value.is_empty() {
            mappings.insert(path);
            parents.push((indent, key.to_string()));
        } else {
            scalars.insert(path, value.to_string());
            if matches!(value, "|" | "|-" | ">" | ">-") {
                block_scalar_at = Some(indent);
            }
        }
    }

    YamlMappings { mappings, scalars }
}

#[test]
fn generated_workflow_matches_the_reviewed_golden_file() {
    assert_eq!(
        workflow(),
        include_str!("fixtures/generated/publish-sandbox.yml"),
        "the generated consumer workflow changed; review and update the golden file intentionally"
    );
}

/// task-47 AC#3: the publisher workflow carries a parseable version stamp naming the exact CLI
/// release `init` rendered it with, within the marker window the ownership check already reads.
#[test]
fn generated_workflow_carries_a_parseable_version_stamp() {
    assert_eq!(parse_version_stamp(&workflow()), Some(VERSION));
}

/// task-47 AC#6-#8: the generated upgrade job, scheduled and manually dispatchable, never
/// floats a production pin and never pushes to main directly.
mod upgrade_job {
    use super::{CONFIG_PATH, RELOCK_PATH, SCRIPT_PATH, WORKFLOW_PATH, workflow};

    #[test]
    fn the_workflow_gains_a_schedule_and_an_opt_in_dispatch_input() {
        let workflow = workflow();
        assert!(workflow.contains("schedule:"), "{workflow}");
        assert!(workflow.contains("cron:"), "{workflow}");
        assert!(workflow.contains("inputs:\n      upgrade:"), "{workflow}");
        // An ordinary manual dispatch (today's only form) must keep working exactly as before:
        // the new input defaults to blank, which routes to the normal publish, not the upgrade.
        assert!(workflow.contains("default: \"\""), "{workflow}");
    }

    /// The normal publish lane and the upgrade lane are mutually exclusive by construction:
    /// no event can satisfy both `if:` conditions at once, so they can never double-run.
    #[test]
    fn the_publish_lane_and_the_upgrade_lane_can_never_both_fire() {
        let workflow = workflow();
        assert!(
            workflow.contains(
                "if: github.event_name == 'push' || (github.event_name == 'workflow_dispatch' && inputs.upgrade == '')"
            ),
            "{workflow}"
        );
        assert!(
            workflow.contains(
                "if: github.event_name == 'schedule' || (github.event_name == 'workflow_dispatch' && inputs.upgrade != '')"
            ),
            "{workflow}"
        );
    }

    /// The upgrade job never trusts a package manager's "latest" for anything that ends up in
    /// a committed file (decision-4 / D16): it bootstraps the exact pinned, checksum-verified
    /// binary and only that binary's own `self-update` ever decides the new version.
    #[test]
    fn the_upgrade_job_bootstraps_a_verified_binary_before_self_updating_it() {
        let workflow = workflow();
        assert!(workflow.contains("Download currently pinned pixi-sandbox"), "{workflow}");
        assert!(workflow.contains("sha256sum --check --status"), "{workflow}");
        assert!(workflow.contains("self-update --dest \"$BIN\""), "{workflow}");
        assert!(
            workflow.contains("self-update --dest \"$BIN\" --version \"$UPGRADE_VERSION\""),
            "a manual dispatch must pass the requested exact version through: {workflow}"
        );
    }

    /// `init --check` runs against the exact paths and branch this project was generated with
    /// — never defaults that could silently diverge from a customised init invocation — and
    /// only a positive drift finding triggers a real `init` run.
    #[test]
    fn drift_check_and_regeneration_use_the_exact_generation_arguments() {
        let workflow = workflow();
        let upgrade_job = workflow
            .split("\n  upgrade:\n")
            .nth(1)
            .expect("the upgrade job exists");
        for flag in [
            "--github-workflow-path .github/workflows/publish-sandbox.yml",
            "--relock-workflow-path .github/workflows/relock.yml",
            "--relock-ci-workflow ci.yml",
            "--script-path restore.sh",
            "--config config/pixi-sandbox.toml",
            "--branch sandbox/developer-linux-64",
        ] {
            assert_eq!(
                upgrade_job.matches(flag).count(),
                2,
                "expected `{flag}` in both the --check and the regenerate invocation: {upgrade_job}"
            );
        }
        assert!(workflow.contains("\"$BIN\" init --check"), "{workflow}");
        assert!(workflow.contains("if: steps.check.outputs.drift == 'true'"), "{workflow}");
    }

    /// Config is reviewed data (D16): the upgrade job's own commit never stages it, even when
    /// `init` regenerated the other three files.
    #[test]
    fn the_regenerated_commit_never_stages_the_config() {
        let workflow = workflow();
        let add_line = workflow
            .lines()
            .find(|line| line.trim_start().starts_with("git add "))
            .expect("the upgrade job stages its regenerated files");
        assert!(!add_line.contains(CONFIG_PATH), "{add_line}");
        assert!(add_line.contains(WORKFLOW_PATH), "{add_line}");
        assert!(add_line.contains(RELOCK_PATH), "{add_line}");
        assert!(add_line.contains(SCRIPT_PATH), "{add_line}");
    }

    /// task-47 AC#8: because a `github.token` push starts no `on: push` workflow (task-44's
    /// lesson, restated here for the upgrade lane), the job opens a reviewable pull request
    /// against a side branch — never a direct push to `main` — and its own body spells out the
    /// explicit dispatch an automated merge still requires.
    #[test]
    fn the_job_opens_a_pull_request_instead_of_pushing_main() {
        let workflow = workflow();
        assert!(!workflow.contains("git push --force origin main"), "{workflow}");
        assert!(!workflow.contains("git push origin main"), "{workflow}");
        assert!(workflow.contains("gh pr create"), "{workflow}");
        assert!(workflow.contains("--base main"), "{workflow}");
        assert!(
            workflow.contains("gh workflow run .github/workflows/publish-sandbox.yml --ref main"),
            "the PR body must name the explicit post-merge dispatch: {workflow}"
        );
    }

    /// The bot identity matches the one the relock workflow already established (task-39's
    /// precedent): one recognisable automation identity across every generated bot commit.
    #[test]
    fn the_upgrade_commit_uses_the_same_bot_identity_as_relock() {
        let workflow = workflow();
        assert!(workflow.contains("pixi-sandbox[bot]"), "{workflow}");
        assert!(
            workflow.contains("41898282+github-actions[bot]@users.noreply.github.com"),
            "{workflow}"
        );
    }

    /// A job-level `permissions:` block replaces the workflow-level one rather than adding to
    /// it (the trap `relock.yml` already documents) — `pull-requests: write` must be spelled
    /// out explicitly on the upgrade job or `gh pr create` gets a 403.
    #[test]
    fn the_upgrade_job_grants_itself_pull_request_permission() {
        let workflow = workflow();
        let upgrade_job = workflow
            .split("\n  upgrade:\n")
            .nth(1)
            .expect("the upgrade job exists");
        let permissions_block = upgrade_job
            .split("permissions:\n")
            .nth(1)
            .expect("the upgrade job declares its own permissions");
        assert!(permissions_block.contains("contents: write"));
        assert!(permissions_block.contains("pull-requests: write"));
    }
}

#[rstest]
#[case("on")]
#[case("jobs")]
#[case("jobs.plan")]
#[case("jobs.publish")]
#[case("jobs.publish.strategy")]
#[case("jobs.publish.strategy.matrix")]
fn generated_workflow_has_the_required_yaml_structure(#[case] path: &str) {
    let yaml = parse_yaml_mappings(&workflow());
    assert!(yaml.mappings.contains(path), "missing YAML mapping {path}");
}

#[test]
fn generated_workflow_wires_plan_outputs_through_the_matrix() {
    let workflow = workflow();
    let yaml = parse_yaml_mappings(&workflow);

    assert_eq!(yaml.scalars["permissions.contents"], "write");
    assert_eq!(yaml.scalars["jobs.publish.needs"], "plan");
    assert_eq!(
        yaml.scalars["jobs.publish.strategy.matrix.include"],
        "${{ fromJSON(needs.plan.outputs.matrix).include }}"
    );
    assert_eq!(yaml.scalars["jobs.publish.runs-on"], "${{ matrix.runner }}");
    assert!(
        workflow.contains(&format!("pixi-sandbox plan --config {CONFIG_PATH} --json")),
        "the renderer ignored its configured sandbox plan path:\n{workflow}"
    );
    assert!(
        !workflow.contains("matrix: ${{ fromJSON("),
        "strategy.matrix must never be handed the include array directly:\n{workflow}"
    );
}

#[rstest]
#[case("pixi-sandbox plan")]
#[case("\"$SELF_BIN\" pack")]
#[case("\"$SELF_BIN\" doctor")]
#[case("\"$SELF_BIN\" publish")]
fn generated_workflow_invokes_the_expected_cli(#[case] command: &str) {
    let workflow = workflow();
    assert!(
        workflow.contains(command),
        "missing expected `{command}` invocation"
    );
}

#[test]
fn generated_workflow_downloads_a_verified_release_binary_for_publishing() {
    let workflow = workflow();
    assert!(
        !workflow.contains("uses: Archont561/pixi-sandbox"),
        "consumer workflows must not depend on repository-owned composite actions:\n{workflow}"
    );
    assert!(workflow.contains("https://prefix.dev/archont561/archont561"));
    assert!(workflow.contains("\"pixi-sandbox==${PIXI_SANDBOX_VERSION}\""));
    assert!(workflow.contains("Download released pixi-sandbox"));
    assert!(workflow.contains("releases/download/v${PIXI_SANDBOX_VERSION}"));
    assert!(workflow.contains("SHA256SUMS"));
    assert!(workflow.contains("sha256sum --check --status"));
    assert!(workflow.contains("Get-FileHash"));
    assert!(!workflow.contains("build-release-binary"));
    assert!(!workflow.contains("rustup target add"));
    assert!(workflow.contains("--self-bin \"$SELF_BIN\""));
}

/// Issue #80: the release-download step named the *consumer's* repository, so any consumer
/// publishing no GitHub releases of its own got a 404 before anything was packed. The
/// download base must name this project's own repository — the one that actually publishes
/// `pixi-sandbox-*` release assets and `SHA256SUMS` — in both the bash and the pwsh leg, and
/// `GITHUB_REPOSITORY` must never appear in a release-download URL again. The constant is
/// shared with `self_update::DEFAULT_REPO` so the template and the updater cannot drift apart.
#[test]
fn generated_workflow_downloads_release_assets_from_the_pixi_sandbox_repository_not_the_consumers()
{
    let workflow = workflow();
    let expected_base = format!(
        "$GITHUB_SERVER_URL/{}/releases/download/v${{PIXI_SANDBOX_VERSION}}",
        pixi_sandbox::release::PIXI_SANDBOX_REPO
    );
    let expected_pwsh_base = format!(
        "$env:GITHUB_SERVER_URL/{}/releases/download/v$env:PIXI_SANDBOX_VERSION",
        pixi_sandbox::release::PIXI_SANDBOX_REPO
    );
    assert!(
        workflow.contains(&expected_base),
        "bash leg must download from the pixi-sandbox repository:\n{workflow}"
    );
    assert!(
        workflow.contains(&expected_pwsh_base),
        "pwsh leg must download from the pixi-sandbox repository:\n{workflow}"
    );
    assert_eq!(
        pixi_sandbox::release::PIXI_SANDBOX_REPO,
        pixi_sandbox::self_update::DEFAULT_REPO,
        "the renderer and self-update's default --repo must name the same repository"
    );
    for (index, _) in workflow.match_indices("releases/download") {
        let window_start = index.saturating_sub(80);
        let window = &workflow[window_start..index];
        assert!(
            !window.contains("GITHUB_REPOSITORY"),
            "a release-download URL must never resolve against the consumer's own repository:\n{workflow}"
        );
    }
}

#[test]
fn generated_workflow_only_reads_matrix_keys_the_plan_emits() {
    let workflow = workflow();
    let plan = plan_override(
        "developer",
        &["default".to_string(), "web".to_string()],
        "linux-64",
        "sandbox",
        true,
    )
    .expect("fixture plan is valid");
    let plan: Value = serde_json::to_value(plan).expect("publish plan serializes");
    let include = plan["include"]
        .as_array()
        .expect("plan JSON emits an include array");
    assert!(!include.is_empty(), "the fixture plan publishes nothing");

    let mut referenced = BTreeSet::new();
    for (index, _) in workflow.match_indices("matrix.") {
        let key: String = workflow[index + "matrix.".len()..]
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect();
        if !key.is_empty() {
            referenced.insert(key);
        }
    }
    assert!(
        referenced.len() >= 4,
        "expected the workflow to read several matrix keys, found {referenced:?}"
    );

    for entry in include {
        let entry = entry.as_object().expect("each include entry is an object");
        for key in &referenced {
            assert!(
                entry.contains_key(key),
                "the workflow reads matrix.{key}, which plan JSON does not emit: {entry:?}"
            );
        }
    }
}

/// The embedded bash/PowerShell case/switch arms that pick a release asset name cannot call
/// into `Platform` (task-55) at render time the way other call sites were migrated (task-58,
/// task-59, task-60): they are literal text inside a workflow that a plain GitHub runner
/// executes before any pixi-sandbox binary exists to ask. Keeping that text a hand-typed
/// literal inside the render function is still a duplicate of `Platform::asset_name`, so this
/// test is the structural guarantee a doc comment used to be: every platform's asset name in
/// the rendered workflow must agree with `Platform`, for both the bash and the PowerShell
/// branch.
#[test]
fn every_rendered_asset_name_agrees_with_platform() {
    let workflow = workflow();
    for platform in Platform::ALL {
        assert!(
            workflow.contains(platform.asset_name()),
            "rendered workflow is missing {}'s asset name {}",
            platform.as_str(),
            platform.asset_name()
        );
    }
}
