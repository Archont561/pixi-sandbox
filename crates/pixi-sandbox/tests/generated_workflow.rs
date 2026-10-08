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
use std::fs;
use std::process::Command;
use tempfile::tempdir;

const VERSION: &str = "9.8.7";
const CONFIG_PATH: &str = "config/pixi-sandbox.toml";
const WORKFLOW_PATH: &str = ".github/workflows/publish-sandbox.yml";
const RELOCK_PATH: &str = ".github/workflows/relock.yml";
const RELOCK_CI_WORKFLOW: &str = "ci.yml";
const SCRIPT_PATH: &str = "restore.sh";
const BRANCH: &str = "sandbox/developer-linux-64";

fn base_options() -> GithubWorkflowOptions<'static> {
    GithubWorkflowOptions {
        version: VERSION,
        config_path: CONFIG_PATH,
        workflow_path: WORKFLOW_PATH,
        relock_workflow_path: RELOCK_PATH,
        relock_ci_workflow: RELOCK_CI_WORKFLOW,
        script_path: SCRIPT_PATH,
        branch: BRANCH,
        push_paths: &[],
        scoped_permissions: false,
        concurrency: None,
        plan_timeout_minutes: None,
        publish_timeout_minutes: None,
        pixi_version: None,
        setup_pixi_cache: None,
    }
}

fn workflow() -> String {
    render_github_workflow(base_options())
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

#[test]
fn publish_lane_checks_size_budgets_from_the_reviewed_config_before_publishing() {
    let workflow = workflow();
    assert!(
        workflow.contains(
            "record doctor \"$SELF_BIN\" doctor --branch-location \"$TRANSPORT\" --verify --budget-config config/pixi-sandbox.toml"
        ),
        "bash publisher must enforce budgets before publish: {workflow}"
    );
    assert!(
        workflow.contains(
            "Invoke-Phase \"doctor\" { & $env:SELF_BIN doctor --branch-location $transport --verify --budget-config config/pixi-sandbox.toml"
        ),
        "Windows publisher must enforce budgets before publish: {workflow}"
    );
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
        assert!(
            workflow.contains("Download currently pinned pixi-sandbox"),
            "{workflow}"
        );
        assert!(
            workflow.contains("sha256sum --check --status"),
            "{workflow}"
        );
        assert!(
            workflow.contains("self-update --dest \"$BIN\""),
            "{workflow}"
        );
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
        assert!(
            workflow.contains("if: steps.check.outputs.drift == 'true'"),
            "{workflow}"
        );
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
        assert!(
            !workflow.contains("git push --force origin main"),
            "{workflow}"
        );
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

    #[test]
    fn the_upgrade_job_uses_an_optional_workflow_capable_token() {
        let workflow = workflow();
        assert!(
            workflow.contains("PIXI_SANDBOX_UPGRADE_TOKEN"),
            "{workflow}"
        );
        assert!(
            workflow.contains("token: ${{ secrets.PIXI_SANDBOX_UPGRADE_TOKEN || github.token }}"),
            "checkout must use the same optional token as delivery: {workflow}"
        );
        assert!(
            workflow
                .contains("GH_TOKEN: ${{ secrets.PIXI_SANDBOX_UPGRADE_TOKEN || github.token }}"),
            "gh must use the same optional token as delivery: {workflow}"
        );
    }

    #[test]
    fn the_upgrade_job_preserves_a_patch_when_delivery_is_unavailable() {
        let workflow = workflow();
        assert!(workflow.contains("git diff HEAD^ HEAD"), "{workflow}");
        assert!(workflow.contains("GITHUB_STEP_SUMMARY"), "{workflow}");
        assert!(workflow.contains("Workflows: write"), "{workflow}");
        assert!(
            workflow.contains("pixi-sandbox-upgrade-artifacts"),
            "{workflow}"
        );
        assert!(workflow.contains("actions/upload-artifact@"), "{workflow}");
        assert!(workflow.contains("if-no-files-found: ignore"), "{workflow}");
    }

    #[test]
    fn delivery_refusal_does_not_turn_the_scheduled_lane_into_a_bare_failure() {
        let workflow = workflow();
        assert!(workflow.contains("UPGRADE_TOKEN"), "{workflow}");
        assert!(
            workflow.contains("git push --force origin \"$branch\""),
            "{workflow}"
        );
        assert!(
            workflow.contains("delivery_refused=1") || workflow.contains("delivery refused"),
            "{workflow}"
        );
    }

    /// The handoff prose lives inside a double-quoted Bash argument. One escape is needed for
    /// literal Markdown quotes and backticks; two close the argument and make ShellCheck reject
    /// the generated workflow. Parse the exact rendered block rather than a copy of it.
    #[test]
    fn delivery_handoff_script_is_valid_bash() {
        use std::fs;
        use std::process::Command;
        use tempfile::tempdir;

        let workflow = workflow();
        let delivery = workflow
            .split("      - name: Prepare the upgrade patch\n")
            .nth(1)
            .expect("the delivery step exists");
        let run = delivery
            .split("        run: |\n")
            .nth(1)
            .and_then(|script| {
                script
                    .split("\n\n      - name: Upload upgrade patch")
                    .next()
            })
            .expect("the delivery step has an isolated Bash block");
        let script = run
            .lines()
            .map(|line| line.strip_prefix("          ").unwrap_or(line))
            .collect::<Vec<_>>()
            .join("\n");

        assert!(script.contains(r#"\"$VERSION\""#), "{script}");
        assert!(script.contains(r#"\`github.token\`"#), "{script}");
        assert!(!script.contains(r#"\\\"$VERSION"#), "{script}");
        assert!(!script.contains(r#"\\`github.token"#), "{script}");

        let temp = tempdir().expect("temporary Bash script directory");
        let path = temp.path().join("delivery.sh");
        fs::write(&path, script).expect("write rendered delivery script");
        let output = Command::new("bash")
            .arg("-n")
            .arg(&path)
            .output()
            .expect("bash is available on supported generated-workflow hosts");
        assert!(
            output.status.success(),
            "Bash rejected the rendered delivery script:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
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

fn unix_publish_checksum_command() -> String {
    workflow()
        .lines()
        .find(|line| {
            line.contains("SHA256 verification failed") && line.contains("sha256sum --check")
        })
        .expect("the Unix publish bootstrap carries its checksum command")
        .trim()
        .to_string()
}

fn run_unix_publish_checksum(
    asset_bytes: Option<&[u8]>,
    expected_digest: &str,
) -> std::process::Output {
    let temp = tempdir().expect("temporary runner directory");
    let asset = "pixi-sandbox-x86_64-unknown-linux-musl";
    let path = temp.path().join(asset);
    if let Some(bytes) = asset_bytes {
        fs::write(&path, bytes).expect("write release asset fixture");
    }
    fs::write(
        temp.path().join("SHA256SUMS"),
        format!("{expected_digest}  {asset}\n"),
    )
    .expect("write checksum fixture");

    Command::new("bash")
        .arg("-c")
        .arg(unix_publish_checksum_command())
        .env("RUNNER_TEMP", temp.path())
        .env("asset", asset)
        .env("path", &path)
        .output()
        .expect("execute the generated Unix checksum command")
}

#[test]
fn unix_publish_bootstrap_accepts_the_downloaded_asset_at_its_runner_temp_path() {
    // SHA256("released bytes") is an independent worked example, not computed by the code under test.
    let output = run_unix_publish_checksum(
        Some(b"released bytes"),
        "2f9e0acbd320f87ceff2b9d259c99ec87830fc87d99bf914cef87394294a6682",
    );
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[rstest]
#[case::missing_asset(None)]
#[case::mismatched_asset(Some(&b"tampered bytes"[..]))]
fn unix_publish_bootstrap_fails_closed_and_names_checksum_verification(
    #[case] asset_bytes: Option<&[u8]>,
) {
    let output = run_unix_publish_checksum(
        asset_bytes,
        "2f9e0acbd320f87ceff2b9d259c99ec87830fc87d99bf914cef87394294a6682",
    );
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("SHA256 verification failed for pixi-sandbox-x86_64-unknown-linux-musl"),
        "stdout: {stdout}"
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

/// TASK-75 AC#2 through the template: the config that carries `[host_requirements]` reaches
/// `pack`, not only `plan` and `doctor` — a consumer's transport would otherwise silently drop
/// what the reviewed config declares. Both runner dialects are checked, because the pwsh leg
/// builds its argument array separately and a one-legged fix would look green on Linux.
#[test]
fn generated_workflow_passes_the_config_to_pack_so_host_requirements_travel() {
    let workflow = workflow();
    for expected in [
        &format!("--config {CONFIG_PATH} \\\n"),
        &format!("'--config', '{CONFIG_PATH}'"),
    ] {
        assert!(
            workflow.contains(expected),
            "the pack step must receive the sandbox config ({expected}):\n{workflow}"
        );
    }
    // One reviewed config path, named by every step that reads it: plan (1), pack and doctor in
    // each runner dialect (4), and the upgrade lane's two invocations (2). The count is the
    // intentional-change detector — a new step that reads config must be added here too.
    assert_eq!(
        workflow.matches(CONFIG_PATH).count(),
        7,
        "expected plan, both pack legs, both doctor legs and the upgrade lane to name the config"
    );
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

/// task-53 (issue #79): the generated publisher's `[workflow]`-table-derived CI policy.
mod workflow_policy {
    use super::{GithubWorkflowOptions, base_options, render_github_workflow, workflow};

    /// AC#1: an unconfigured render carries none of the new blocks at all — this is the
    /// byte-identical migration path for every existing consumer, held directly by the golden
    /// fixture test above; this test names the absence explicitly, block by block.
    #[test]
    fn default_options_add_no_new_yaml() {
        let rendered = workflow();
        assert!(!rendered.contains("    paths:"));
        assert!(rendered.contains("permissions:\n  contents: write"));
        assert!(!rendered.contains("concurrency:"));
        assert!(!rendered.contains("timeout-minutes:"));
        assert!(!rendered.contains("pixi-version:"));
        assert!(!rendered.contains("cache:"));
    }

    #[test]
    fn push_paths_render_as_a_paths_allowlist_under_the_push_trigger() {
        let paths = vec!["pixi-sandbox.toml".to_string(), "pixi.toml".to_string()];
        let rendered = render_github_workflow(GithubWorkflowOptions {
            push_paths: &paths,
            ..base_options()
        });
        let push_index = rendered.find("push:\n").expect("on.push exists");
        let schedule_index = rendered.find("schedule:").expect("on.schedule exists");
        let push_block = &rendered[push_index..schedule_index];
        assert!(push_block.contains("    paths:\n"));
        for path in &paths {
            assert!(
                push_block.contains(&format!("      - {path:?}\n")),
                "missing {path:?} in {push_block}"
            );
        }
    }

    #[test]
    fn scoped_permissions_reads_top_level_and_scopes_the_publish_job() {
        let rendered = render_github_workflow(GithubWorkflowOptions {
            scoped_permissions: true,
            ..base_options()
        });
        assert!(rendered.contains("permissions:\n  contents: read\n"));
        let publish_index = rendered.find("\n  publish:\n").expect("publish job exists");
        let steps_index = rendered[publish_index..]
            .find("    steps:\n")
            .expect("publish job has steps");
        let publish_header = &rendered[publish_index..publish_index + steps_index];
        assert!(
            publish_header.contains("    permissions:\n      contents: write\n"),
            "publish job is missing its own scoped permissions: {publish_header}"
        );
        // Only the publish job gets write — the plan job stays covered by the read-only
        // workflow-level default, never regaining its own write grant.
        let plan_index = rendered.find("\n  plan:\n").expect("plan job exists");
        let plan_outputs = rendered[plan_index..]
            .find("    outputs:\n")
            .expect("plan job has outputs");
        let plan_header = &rendered[plan_index..plan_index + plan_outputs];
        assert!(!plan_header.contains("permissions:"));
    }

    #[test]
    fn concurrency_renders_a_workflow_level_group_and_cancel_policy() {
        let rendered = render_github_workflow(GithubWorkflowOptions {
            concurrency: Some(("publish-sandbox", true)),
            ..base_options()
        });
        assert!(
            rendered.contains(
                "concurrency:\n  group: \"publish-sandbox\"\n  cancel-in-progress: true\n"
            )
        );
    }

    #[test]
    fn timeouts_render_on_the_plan_and_publish_jobs_independently() {
        let rendered = render_github_workflow(GithubWorkflowOptions {
            plan_timeout_minutes: Some(15),
            ..base_options()
        });
        assert!(rendered.contains("runs-on: ubuntu-latest\n    timeout-minutes: 15\n    outputs:"));
        assert!(!rendered.contains("runs-on: ${{ matrix.runner }}\n    timeout-minutes:"));

        let rendered = render_github_workflow(GithubWorkflowOptions {
            publish_timeout_minutes: Some(60),
            ..base_options()
        });
        assert!(
            rendered.contains("runs-on: ${{ matrix.runner }}\n    timeout-minutes: 60\n    steps:")
        );
        assert!(!rendered.contains("runs-on: ubuntu-latest\n    timeout-minutes:"));
    }

    #[test]
    fn pixi_version_and_cache_render_on_every_setup_pixi_step() {
        let rendered = render_github_workflow(GithubWorkflowOptions {
            pixi_version: Some("0.81.0"),
            setup_pixi_cache: Some(false),
            ..base_options()
        });
        let occurrences = rendered.matches("prefix-dev/setup-pixi@").count();
        assert_eq!(
            occurrences, 2,
            "expected exactly plan + publish setup-pixi steps"
        );
        assert_eq!(
            rendered.matches("pixi-version: \"0.81.0\"").count(),
            occurrences,
            "every setup-pixi step must carry the pin"
        );
        assert_eq!(
            rendered.matches("cache: false").count(),
            occurrences,
            "every setup-pixi step must carry the cache policy"
        );
    }

    /// AC#4: unset, no `cache:` key at all — the action's own default, matching the pre-task-53
    /// template exactly (distinct from an explicit `setup_pixi_cache = Some(false)` above).
    #[test]
    fn unset_setup_pixi_cache_emits_no_cache_key() {
        let rendered = render_github_workflow(GithubWorkflowOptions {
            pixi_version: Some("0.81.0"),
            ..base_options()
        });
        assert!(!rendered.contains("cache:"));
    }

    /// Every combination together must still produce exactly one actionlint-parseable
    /// `on.push.paths:`, `permissions:`, `concurrency:`, `timeout-minutes:`, and `cache:` shape
    /// — xtask's `lint-generated-workflow` holds the GitHub-schema side of this across the same
    /// combination table; this holds that nothing here corrupts a neighbouring block.
    #[test]
    fn every_block_can_be_configured_at_once_without_corrupting_its_neighbours() {
        let paths = vec!["pixi-sandbox.toml".to_string()];
        let rendered = render_github_workflow(GithubWorkflowOptions {
            push_paths: &paths,
            scoped_permissions: true,
            concurrency: Some(("publish-sandbox", false)),
            plan_timeout_minutes: Some(15),
            publish_timeout_minutes: Some(60),
            pixi_version: Some("0.81.0"),
            setup_pixi_cache: Some(true),
            ..base_options()
        });
        for needle in [
            "    paths:\n",
            "permissions:\n  contents: read\n",
            "    permissions:\n      contents: write\n",
            "concurrency:\n  group: \"publish-sandbox\"\n  cancel-in-progress: false\n",
            "    timeout-minutes: 15\n",
            "    timeout-minutes: 60\n",
            "pixi-version: \"0.81.0\"",
            "cache: true",
        ] {
            assert!(
                rendered.contains(needle),
                "missing {needle:?} in:\n{rendered}"
            );
        }
    }
}

/// task-68 AC#1-#3, AC#5: the publish lane logs durable diagnostics, uploads them as a workflow
/// artifact on always(), and formats an actionable step summary on failure.
mod publish_diagnostics {
    use super::workflow;

    #[test]
    fn publish_lane_uploads_diagnostics_artifact_with_retention() {
        let workflow = workflow();
        assert!(
            workflow.contains("name: Upload publish diagnostic log"),
            "missing upload diagnostics step in:\n{workflow}"
        );
        assert!(
            workflow
                .contains("uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a"),
            "upload artifact action must use the repository-pinned version in:\n{workflow}"
        );
        assert!(
            workflow.contains("name: publish-diagnostics-${{ matrix.platform }}"),
            "missing artifact name in:\n{workflow}"
        );
        assert!(
            workflow.contains("retention-days: 7"),
            "missing artifact retention in:\n{workflow}"
        );
    }

    #[test]
    fn publish_lane_records_diagnostics_and_surfaces_step_summary_on_failure() {
        let workflow = workflow();
        assert!(
            workflow.contains("LOG_DIR=\"$RUNNER_TEMP/pixi-sandbox-logs\""),
            "missing LOG_DIR definition in:\n{workflow}"
        );
        assert!(
            workflow.contains("--log-file \"$LOG_DIR/pack.log\""),
            "missing pack --log-file in:\n{workflow}"
        );
        assert!(
            workflow.contains("--log-file \"$LOG_DIR/doctor.log\""),
            "missing doctor --log-file in:\n{workflow}"
        );
        assert!(
            workflow.contains("--log-file \"$LOG_DIR/publish.log\""),
            "missing publish --log-file in:\n{workflow}"
        );
        assert!(
            workflow.contains("GITHUB_STEP_SUMMARY"),
            "publish step must emit failure details to GITHUB_STEP_SUMMARY:\n{workflow}"
        );
    }
}
