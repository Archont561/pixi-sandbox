---
id: TASK-27
title: Establish Rust xtask harness and migrate generated-workflow lint
status: Done
assignee:
  - '@agent'
updated_date: '2026-10-01 15:05'
created_date: '2026-10-01 14:10'
labels:
  - rust
  - tooling
  - workflow
  - testing
milestone: m-0
dependencies: []
references:
  - scripts/lint-generated-workflow.sh
  - crates/pixi-sandbox/src/commands/init.rs
  - crates/pixi-sandbox/tests/cli.rs
  - pixi.toml
priority: high
type: enhancement
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add a workspace package at `crates/xtask` as the supported entry point for repository automation, then use it for generated GitHub workflow validation. Expose the workflow renderer through reusable Rust code so `init`, structural tests, and xtask validate the same bytes. Keep `actionlint` as the external GitHub-specific validator, but invoke it from xtask rather than maintaining orchestration in `scripts/lint-generated-workflow.sh`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `crates/xtask` is a workspace package with a typed `lint-generated-workflow` command, and `pixi run lint-generated-workflow` invokes that command
- [x] #2 The generated workflow renderer is reusable by the CLI and tests without spawning a nested `cargo run`
- [x] #3 Rust tests parse the rendered YAML and assert job, needs, permissions, configured-path, object-shaped matrix, and plan-matrix-key contracts against fixture data
- [x] #4 A deterministic reviewed golden workflow catches unintended full-document changes
- [x] #5 xtask generates into a temporary project and runs `actionlint` on the exact artifact; the check is offline and reports a useful error when the tool or generated file is missing
- [x] #6 `scripts/lint-generated-workflow.sh` is removed, and lint plus the workspace test suite pass through the new path
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Create the minimal xtask package and command dispatcher. Refactor workflow rendering behind a library API owned by the product crate, move workflow-specific assertions out of the oversized CLI test module, add YAML structure and golden-fixture tests, then port the temporary-project/actionlint adapter to xtask. Switch the Pixi task only after parity is proven and remove the shell script last.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added `crates/xtask` to the Cargo workspace with a typed Clap command. `pixi run lint-generated-workflow` now runs `cargo run -q -p xtask -- lint-generated-workflow`; xtask renders the workflow directly through the product library into a temporary project and invokes actionlint there. It uses a deterministic full SHA and package version, makes no network call, collects actionlint output on rejection, and has unit tests for both a missing artifact and a missing actionlint executable.

The workflow template moved out of the CLI command into `pixi_sandbox::generated::render_github_workflow`. `init` passes version, resolved action SHA, and its configured plan path to that pure renderer. New `tests/generated_workflow.rs` parses the generated mapping hierarchy and pins permissions, jobs, needs, object-shaped matrix, configured path, and every `matrix.*` read against the real serialized `PublishPlan` fields. A checked-in golden workflow makes all other byte changes review-visible. The old workflow-specific tests were removed from the oversized CLI module; the CLI smoke test still proves `init` writes the complete project.

Removed `scripts/lint-generated-workflow.sh` and updated repository documentation and TOML lint coverage for xtask. Release preparation now repins xtask's exact internal `pixi-sandbox` path dependency alongside the existing CLI sibling pins, so the next workspace version bump remains resolvable. Verification: `pixi run lint-generated-workflow`, actionlint, sandbox-plan lint, fmt, clippy with warnings denied, Taplo, repository consistency, cargo-deny, and all 145 nextest tests pass. The aggregate `pixi run lint` executed every default-environment check including the new xtask path successfully, then stopped only because this restored snapshot lacks the web environment and Conda failed to fetch libstdcxx with the sandbox TLS/egress error; `lint-docs` was therefore the sole unexecuted aggregate dependency, unrelated to this task.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Generated-workflow validation is now Rust-owned. The CLI, structural tests, golden fixture, and repository xtask share one pure workflow renderer; xtask supplies the temporary-project/actionlint boundary. Matrix shape and planner keys are tested where actionlint cannot evaluate them, external GitHub syntax remains actionlint-validated, and the former shell orchestration has been deleted.
<!-- SECTION:FINAL_SUMMARY:END -->
