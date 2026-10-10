//! Contracts for the GitHub workflow emitted into consumer repositories.
//!
//! actionlint validates GitHub's full schema through xtask. These tests own the contracts it
//! cannot evaluate: indentation-derived matrix shape, agreement with the planner's JSON
//! keys, and — since TASK-76 — the shape of the workflow as the tool's entrypoint: every
//! logic-bearing step is a single `pixi-sandbox <verb>` invocation, with no multi-line
//! `run:` block and no PowerShell twin left in the render.

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

/// TASK-76 AC#1/AC#3: the crisp end state — no literal `run: |` block survives, and no
/// `shell: pwsh` exists anywhere. Every logic-bearing step is one command (a folded scalar
/// or a one-liner), so the former Bash/PowerShell twins are gone and the render satisfies
/// the workflow-shape rule (check 9) without any exemption.
#[test]
fn the_render_carries_no_multiline_run_blocks_and_no_pwsh() {
    let workflow = workflow();
    assert!(
        !workflow.contains("run: |"),
        "a literal run block survives:\n{workflow}"
    );
    assert!(
        !workflow.contains("shell: pwsh"),
        "a pwsh step survives:\n{workflow}"
    );
    assert!(!workflow.contains("shell: powershell"));
}

/// TASK-76 AC#7: non-comment embedded shell falls from 218 lines (inside 9 literal `run:`
/// blocks in two dialects) to the single commands alone. Every remaining `run:` scalar is
/// one command: the package install (the bootstrap chain), the `fetch-release` download (the
/// bootstrap exception), and the single pixi-sandbox invocations.
#[test]
fn non_comment_shell_lines_fall_to_the_single_commands_alone() {
    let workflow = workflow();
    let mut shell_lines = 0;
    let mut in_block = false;
    let mut block_indent = 0;
    for line in workflow.lines() {
        let stripped = line.trim_start();
        if let Some(rest) = stripped.strip_prefix("run:") {
            let value = rest.trim();
            if matches!(value, "|" | ">" | "|-" | ">-" | "|+" | ">+") {
                in_block = true;
                block_indent = line.len() - stripped.len();
            } else {
                in_block = false;
                if !value.is_empty() && !value.starts_with('#') {
                    shell_lines += 1;
                }
            }
            continue;
        }
        if in_block {
            let indent = line.len() - stripped.len();
            if !stripped.is_empty() && indent > block_indent {
                if !stripped.starts_with('#') {
                    shell_lines += 1;
                }
                continue;
            }
            in_block = false;
        }
    }
    // plan: install(4) + plan(1) · publish: install(4) + fetch-release(1) + pipeline(9) ·
    // upgrade: install(4) + fetch-release(1) + self-update(2) + upgrade(9).
    assert_eq!(
        shell_lines, 35,
        "every remaining shell line must be one command"
    );
}

/// The publish lane enforces the reviewed size budgets before publishing. TASK-76: the
/// enforcement moved from the render's shell into `pipeline`'s doctor phase (asserted with
/// the exact argv in tests/pipeline.rs); what the render owns is that the step hands the
/// reviewed config to the verb.
#[test]
fn publish_lane_passes_the_reviewed_config_to_the_pipeline_before_publishing() {
    let workflow = workflow();
    let pipeline_step = workflow
        .split("      - name: Pack, verify, and publish\n")
        .nth(1)
        .expect("the pipeline step exists");
    assert!(
        pipeline_step.contains("--config config/pixi-sandbox.toml"),
        "the pipeline step must receive the sandbox config: {pipeline_step}"
    );
    assert!(
        !pipeline_step.contains("--budget-config"),
        "budget enforcement is the verb's policy, not the render's: {pipeline_step}"
    );
}

/// task-47 AC#6-#8: the generated upgrade job, scheduled and manually dispatchable, never
/// floats a production pin and never pushes to main directly.
mod upgrade_job {
    use super::workflow;

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
    /// binary — `fetch-release` verifies against the release's SHA256SUMS inside the tool —
    /// and only that binary's own `self-update` ever decides the new version. The version
    /// conditional is two `if:`-gated one-line steps, so the pinned binary never sees an
    /// empty `--version`.
    #[test]
    fn the_upgrade_job_bootstraps_a_verified_binary_before_self_updating_it() {
        let workflow = workflow();
        assert!(
            workflow.contains("Fetch the pinned pixi-sandbox release"),
            "{workflow}"
        );
        assert!(
            workflow.contains(
                "pixi-sandbox fetch-release --dest \"$PIXI_SANDBOX_BIN\" --version \"$PIXI_SANDBOX_VERSION\""
            ),
            "{workflow}"
        );
        assert!(
            workflow.contains(
                "- name: Self-update to the requested version\n        if: inputs.upgrade != ''"
            ),
            "{workflow}"
        );
        assert!(
            workflow.contains(
                "\"$PIXI_SANDBOX_BIN\" self-update --dest \"$PIXI_SANDBOX_BIN\" --version \"${{ inputs.upgrade }}\""
            ),
            "a manual dispatch must pass the requested exact version through: {workflow}"
        );
        assert!(
            workflow.contains(
                "- name: Self-update to the latest release\n        if: inputs.upgrade == ''"
            ),
            "{workflow}"
        );
        assert!(
            workflow
                .contains("run: '\"$PIXI_SANDBOX_BIN\" self-update --dest \"$PIXI_SANDBOX_BIN\"'"),
            "the weekly schedule takes the newest release: {workflow}"
        );
    }

    /// The upgrade step forwards the exact generation arguments (D16) — each flag once. The
    /// check/regenerate split is the verb's phase, not the render's: no drift output and no
    /// `if:`-gated regenerate step survives in the YAML.
    #[test]
    fn the_upgrade_step_forwards_the_exact_generation_arguments() {
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
                1,
                "expected `{flag}` exactly once in the upgrade step: {upgrade_job}"
            );
        }
        assert!(
            upgrade_job.contains("\"$PIXI_SANDBOX_BIN\" upgrade"),
            "{upgrade_job}"
        );
        assert!(
            !upgrade_job.contains("drift="),
            "the render must not gate on a drift output: {upgrade_job}"
        );
        assert!(
            !upgrade_job.contains("init --check"),
            "the check is the verb's phase: {upgrade_job}"
        );
    }

    /// Config is reviewed data (D16): the upgrade step passes it to the verb, and no git
    /// staging shell remains in the render — the commit stages exactly the three owned files
    /// and never the config (asserted with `FakeGit` in tests/upgrade.rs).
    #[test]
    fn the_upgrade_commit_stages_only_the_owned_files_and_never_the_config() {
        let workflow = workflow();
        for shell in [
            "git add",
            "git commit",
            "git push",
            "git checkout",
            "git config",
        ] {
            assert!(
                !workflow.contains(shell),
                "the render still carries `{shell}` shell: {workflow}"
            );
        }
        let upgrade_job = workflow
            .split("\n  upgrade:\n")
            .nth(1)
            .expect("the upgrade job exists");
        assert!(
            upgrade_job.contains("--config config/pixi-sandbox.toml"),
            "{upgrade_job}"
        );
    }

    /// The delivery is a pull request (task-44's lesson restated): the render carries no push
    /// shell at all; the verb pushes a version-derived branch and opens the PR (the title and
    /// body shape is asserted in tests/upgrade.rs).
    #[test]
    fn the_job_opens_a_pull_request_instead_of_pushing_main() {
        let workflow = workflow();
        assert!(!workflow.contains("git push"), "{workflow}");
        let upgrade_job = workflow
            .split("\n  upgrade:\n")
            .nth(1)
            .expect("the upgrade job exists");
        assert!(
            upgrade_job.contains("--repo \"${{ github.repository }}\""),
            "{upgrade_job}"
        );
        assert!(
            upgrade_job.contains("--artifact-dir \"${{ runner.temp }}/pixi-sandbox-upgrade\""),
            "{upgrade_job}"
        );
    }

    /// The bot identity is the tool's, not the render's: `ShellGit` authors the upgrade commit
    /// as `pixi-sandbox[bot]` — the same identity relock.yml's bot uses — asserted with a real
    /// repository in the git crate's `tests/branch_commit.rs`. The render carries no
    /// `git config user.*` shell.
    #[test]
    fn the_upgrade_commit_uses_the_same_bot_identity_as_relock() {
        let workflow = workflow();
        assert!(!workflow.contains("git config user."), "{workflow}");
        assert!(
            !workflow.contains("pixi-sandbox[bot]"),
            "the identity moved into the tool: {workflow}"
        );
    }

    /// A job-level `permissions:` block replaces the workflow-level one rather than adding to
    /// it (the trap `relock.yml` already documents) — `pull-requests: write` must be spelled
    /// out explicitly on the upgrade job or the pull request gets a 403.
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

    /// The optional workflow-capable token: checkout uses it, and the upgrade step reads it
    /// natively — `PIXI_SANDBOX_UPGRADE_TOKEN` for the availability verdict,
    /// `GITHUB_TOKEN` for the pull-request API call.
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
            workflow.contains(
                "GITHUB_TOKEN: ${{ secrets.PIXI_SANDBOX_UPGRADE_TOKEN || github.token }}"
            ),
            "the pull request must authenticate with the same optional token as delivery: {workflow}"
        );
    }

    /// The patch artifact survives a refused delivery: the upgrade step names the artifact
    /// directory, and the upload runs unconditionally (`if-no-files-found: ignore`).
    #[test]
    fn the_upgrade_job_preserves_a_patch_when_delivery_is_unavailable() {
        let workflow = workflow();
        assert!(workflow.contains("--artifact-dir \"${{ runner.temp }}/pixi-sandbox-upgrade\""));
        assert!(workflow.contains("name: pixi-sandbox-upgrade-artifacts"));
        assert!(
            workflow.contains("path: ${{ runner.temp }}/pixi-sandbox-upgrade"),
            "{workflow}"
        );
        assert!(workflow.contains("actions/upload-artifact@"), "{workflow}");
        assert!(workflow.contains("if-no-files-found: ignore"), "{workflow}");
        assert!(workflow.contains("retention-days: 14"), "{workflow}");
        assert!(
            workflow.contains("if: always()\n        uses: actions/upload-artifact@"),
            "the artifact upload must not be gated on a drift output: {workflow}"
        );
    }

    /// A refused delivery never turns the scheduled lane red: the credential is an
    /// environment variable the verb reads, and the refusal semantics (handoff summary,
    /// `delivery_refused=1`, exit 0) are asserted in tests/upgrade.rs.
    #[test]
    fn delivery_refusal_does_not_turn_the_scheduled_lane_into_a_bare_failure() {
        let workflow = workflow();
        assert!(
            workflow
                .contains("PIXI_SANDBOX_UPGRADE_TOKEN: ${{ secrets.PIXI_SANDBOX_UPGRADE_TOKEN }}"),
            "{workflow}"
        );
        assert!(
            !workflow.contains("delivery_refused"),
            "the refusal output is the verb's: {workflow}"
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

/// TASK-76 AC#2: the pack-verify-publish wrapper, the upgrade sequence and the patch
/// delivery are pixi-sandbox verbs now — the render invokes exactly these, and the
/// behaviours behind them are covered by tests/pipeline.rs and tests/upgrade.rs.
#[rstest]
#[case("pixi-sandbox plan")]
#[case("fetch-release --dest \"$PIXI_SANDBOX_BIN\" --version \"$PIXI_SANDBOX_VERSION\"")]
#[case("\"$PIXI_SANDBOX_BIN\" pipeline")]
#[case("self-update --dest \"$PIXI_SANDBOX_BIN\"")]
#[case("\"$PIXI_SANDBOX_BIN\" upgrade")]
fn generated_workflow_invokes_the_expected_cli(#[case] command: &str) {
    let workflow = workflow();
    assert!(
        workflow.contains(command),
        "missing expected `{command}` invocation"
    );
}

/// TASK-76: the release download moved into the binary. The render no longer spells out a
/// URL, an asset name, or a checksum command: `fetch-release` resolves this runner's asset
/// and verifies it against the release's SHA256SUMS inside the tool (tested against a fake
/// release in `tests/self_update_fetch.rs`). The package install stays exact-pinned.
#[test]
fn generated_workflow_fetches_a_verified_release_binary_through_the_cli() {
    let workflow = workflow();
    assert!(
        !workflow.contains("uses: Archont561/pixi-sandbox"),
        "consumer workflows must not depend on repository-owned composite actions:\n{workflow}"
    );
    assert!(workflow.contains("https://prefix.dev/archont561/archont561"));
    assert!(workflow.contains("\"pixi-sandbox==${PIXI_SANDBOX_VERSION}\""));
    assert!(
        workflow.contains(
            "pixi-sandbox fetch-release --dest \"$PIXI_SANDBOX_BIN\" --version \"$PIXI_SANDBOX_VERSION\""
        ),
        "{workflow}"
    );
    // The shell download is gone: no URL, no asset case statement, no checksum command.
    // (Comments may name SHA256SUMS when explaining the bootstrap exception; shell may not.)
    let shell = workflow
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    for gone in [
        "releases/download",
        "sha256sum",
        "Get-FileHash",
        "SHA256SUMS",
        "curl ",
        "Invoke-WebRequest",
    ] {
        assert!(
            !shell.contains(gone),
            "the render still carries `{gone}` shell: {shell}"
        );
    }
    assert!(!workflow.contains("build-release-binary"));
    assert!(!workflow.contains("rustup target add"));
}

/// TASK-75 AC#2 through the template: the config that carries `[host_requirements]` reaches
/// the pipeline verb (which hands it to `pack`), not only `plan` — a consumer's transport
/// would otherwise silently drop what the reviewed config declares. TASK-76: one pipeline
/// step carries it, on every platform.
#[test]
fn generated_workflow_passes_the_config_to_the_pipeline_so_host_requirements_travel() {
    let workflow = workflow();
    assert!(
        workflow.contains("--config config/pixi-sandbox.toml\n"),
        "the pipeline step must receive the sandbox config:\n{workflow}"
    );
    // One reviewed config path, named by every step that reads it: plan (1), the pipeline
    // step (1), and the upgrade step (1). The count is the intentional-change detector — a new
    // step that reads config must be added here too.
    assert_eq!(
        workflow.matches(CONFIG_PATH).count(),
        3,
        "expected plan, the pipeline step and the upgrade step to name the config"
    );
}

/// Issue #80, restated for TASK-76: the release download names the pixi-sandbox repository,
/// never the consumer's. The URL moved into the binary: `fetch-release`'s `--repo` defaults
/// to the same constant the renderer used to embed, so the two cannot drift apart.
#[test]
fn generated_workflow_fetches_release_assets_from_the_pixi_sandbox_repository_not_the_consumers() {
    let workflow = workflow();
    assert!(
        !workflow.contains("releases/download"),
        "the download URL moved into fetch-release:\n{workflow}"
    );
    assert!(
        !workflow.contains("GITHUB_REPOSITORY"),
        "no download URL may resolve against the consumer's own repository:\n{workflow}"
    );
    assert_eq!(
        pixi_sandbox::release::PIXI_SANDBOX_REPO,
        pixi_sandbox::self_update::DEFAULT_REPO,
        "the tool's release-download default must name the pixi-sandbox repository"
    );
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

/// TASK-76: the render no longer embeds release asset names at all. The old case/switch
/// arms duplicated `Platform::asset_name` as literal text a plain runner executed before any
/// pixi-sandbox binary existed to ask; the download moved into `fetch-release`/`self-update`,
/// which resolve the host asset in Rust (pinned by the self-update and git crates' own
/// tests). The structural guarantee is therefore the *absence* of the literals.
#[test]
fn the_render_embeds_no_release_asset_names() {
    let workflow = workflow();
    for platform in Platform::ALL {
        assert!(
            !workflow.contains(platform.asset_name()),
            "the render still embeds {}'s asset name {}",
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

    /// TASK-76: the upgrade job gained a setup-pixi step (it installs the pinned CLI the
    /// fetch-release bootstrap runs from), so the pin and cache policy now render on all
    /// three setup-pixi steps.
    #[test]
    fn pixi_version_and_cache_render_on_every_setup_pixi_step() {
        let rendered = render_github_workflow(GithubWorkflowOptions {
            pixi_version: Some("0.81.0"),
            setup_pixi_cache: Some(false),
            ..base_options()
        });
        let occurrences = rendered.matches("prefix-dev/setup-pixi@").count();
        assert_eq!(
            occurrences, 3,
            "expected plan + publish + upgrade setup-pixi steps"
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

/// task-68 AC#1-#3, AC#5, restated for TASK-76: the publish lane's durable diagnostics. The
/// log directory is the pipeline verb's argument, the upload step's path matches it, and the
/// upload runs on `always()`. The outcome recording and the failure step summary are the
/// verb's, asserted in tests/pipeline.rs.
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
    fn publish_lane_records_diagnostics_and_uploads_them_on_always() {
        let workflow = workflow();
        assert!(
            workflow.contains("--log-dir \"${{ runner.temp }}/pixi-sandbox-logs\""),
            "the pipeline step must name its log directory:\n{workflow}"
        );
        assert!(
            workflow.contains("path: ${{ runner.temp }}/pixi-sandbox-logs"),
            "the upload path must match the pipeline's log directory:\n{workflow}"
        );
        assert!(
            workflow.contains("if: always()\n        uses: actions/upload-artifact@"),
            "the diagnostics upload must run on always():\n{workflow}"
        );
    }
}
