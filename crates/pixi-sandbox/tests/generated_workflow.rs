//! Contracts for the GitHub workflow emitted into consumer repositories.
//!
//! actionlint validates GitHub's full schema through xtask. These tests own the contracts it
//! cannot evaluate: indentation-derived matrix shape and agreement with the planner's JSON keys.

use pixi_sandbox::generated::{GithubWorkflowOptions, render_github_workflow};
use pixi_sandbox_core::sandbox_config::plan_override;
use rstest::rstest;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const VERSION: &str = "9.8.7";
const CONFIG_PATH: &str = "config/pixi-sandbox.toml";

fn workflow() -> String {
    render_github_workflow(GithubWorkflowOptions {
        version: VERSION,
        config_path: CONFIG_PATH,
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
#[case("pixi-sandbox pack")]
#[case("pixi-sandbox doctor")]
#[case("pixi-sandbox publish")]
fn generated_workflow_invokes_the_cli_directly(#[case] command: &str) {
    let workflow = workflow();
    assert!(
        workflow.contains(command),
        "missing direct `{command}` invocation"
    );
}

#[test]
fn generated_workflow_installs_the_channel_package_with_a_verified_bootstrap() {
    let workflow = workflow();
    assert!(
        !workflow.contains("uses: Archont561/pixi-sandbox"),
        "consumer workflows must not depend on repository-owned composite actions:\n{workflow}"
    );
    assert!(workflow.contains("https://prefix.dev/archont561/pixi-sandbox"));
    assert!(workflow.contains("\"pixi-sandbox==${PIXI_SANDBOX_VERSION}\""));
    assert!(
        workflow.contains("SHA256SUMS") && workflow.contains("checksum mismatch"),
        "the standalone transport bootstrap must be checksum verified"
    );
    assert!(workflow.contains("--self-bin \"$SELF_BIN\""));
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
