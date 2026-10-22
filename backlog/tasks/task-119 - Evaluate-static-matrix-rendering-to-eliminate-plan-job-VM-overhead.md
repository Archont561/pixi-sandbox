---
id: TASK-119
title: Evaluate static matrix rendering to eliminate plan job VM overhead
status: In Progress
assignee: []
created_date: '2026-10-22 07:34'
updated_date: '2026-10-22 07:49'
labels:
  - ci
  - workflow
  - optimization
dependencies: []
priority: medium
type: enhancement
ordinal: 119000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Issue #179 section 2: dedicated plan job VM overhead.

Current publish-sandbox.yml has a plan job that runs ubuntu-latest VM solely to run:
  pixi-sandbox plan --config pixi-sandbox.toml --json
which takes <10ms but incurs ~30s VM boot + checkout + setup-pixi + conda install, billed as full minute.

Proposed: for single-bundle or static platform targets (e.g. platforms = [linux-64] in pixi-sandbox.toml), allow pixi-sandbox init to render static matrix directly into publish-sandbox.yml when dynamic matrix generation is not strictly needed.

Evaluate:
- When is matrix truly dynamic vs static? plan --json reads pixi-sandbox.toml and emits include array based on bundles, platforms, cargo_vendor, branch, runner mapping. For many projects this is static given config.
- Could init render matrix at generation time instead of runtime? Tradeoff: if config changes, workflow must be regenerated via upgrade lane or init --force, but that is already required for push_paths etc.
- Alternative: merge plan into publish job as first step, using matrix from file or inline? But GitHub requires matrix from job outputs for include.
- Alternative: make plan job cheaper: use container, cache, or avoid conda install by using standalone binary directly?
- Measure: current plan job takes X seconds, costs 1 billed minute per push
- Ensure plan --json validation still happens (lint runs sandbox-plan --json)

This is optimization, not correctness. Do not break existing consumers whose matrix is dynamic.

Reference: https://github.com/Archont561/pixi-sandbox/issues/179 section 2
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Measure plan job wall time and billed minutes in current template and in this repo publish-sandbox.yml
- [x] #2 Document when matrix is static vs dynamic, and what inputs affect it
- [x] #3 Propose design: static rendering option, or cheaper plan job, or merged job
- [ ] #4 If static rendering, implement in generated workflow with backward compatibility and tests
- [ ] #5 Verify existing consumers still work, golden file updated, actionlint clean
<!-- AC:END -->

## Implementation Notes

### Measurements

Consumer template (starter run 38804893239):
- plan job wall: 4s (checkout 1s + setup-pixi 0s + conda install 2s + plan <1s)
- billed: 1 minute (GitHub bills per minute, minimum 1)
- pipeline not run in plan, only plan --json

This repo own publish (run 39132270578):
- plan job wall: 64s (checkout 2s + setup-pixi 1s + Build source planner 56s + plan <1s)
- billed: 2 minutes (build from source is expensive)
- Reason: this repo dogfoods by building planner from source, not using conda/standalone

### When matrix is static vs dynamic

`SandboxConfig::plan()` in `crates/pixi-sandbox-core/src/sandbox_config.rs` reads only `pixi-sandbox.toml`:
- inputs: `[[bundle]]` (name, environments, platforms, cargo_vendor), `[runners]` mapping, `branch_prefix`, `cargo_vendor` top-level
- outputs: `PublishPlan { schema: 1, include: [{platform, runner, branch, environments, cargo_vendor, ...}] }`

It does NOT read `pixi.toml`, `pixi.lock`, or any other file. Therefore matrix is fully determined by `pixi-sandbox.toml` at init time.

It is static given config, dynamic only if config changes without re-rendering workflow.

Current dynamic plan provides safety net: if consumer edits `pixi-sandbox.toml` but forgets `init`, plan job at runtime still emits correct matrix from latest config, so publish works even with stale workflow file. With static matrix rendered at init time, stale workflow would publish wrong matrix (e.g. missing new bundle) until next `init`.

`init --check` already audits drift: it compares committed workflow against what current config would render. So static matrix would be caught by `init --check` in CI, but not by publish workflow itself.

### Proposed designs

1. **Static rendering (eliminate plan job)**:
   - `init` renders `strategy.matrix.include` directly into `publish-sandbox.yml` at generation time, using `plan()` output.
   - Pros: eliminates 1 VM per push (1 billed minute), eliminates plan job complexity, matches how `push_paths` derivation already works (rendered at init time)
   - Cons: loses runtime safety net for stale workflow, requires ADR update, breaks consumers who rely on dynamic plan for config changes without init (but check-repository should catch)
   - Implementation: in `github_workflow.rs`, replace `needs: plan` and `fromJSON(needs.plan.outputs.matrix).include` with static JSON array rendered from `plan()`. Must keep `plan` feature for backward compat behind flag.

2. **Cheaper plan job (keep dynamic, reduce cost)**:
   - Replace conda install + build with curl + standalone binary (same as TASK-118 proposal)
   - Consumer plan job becomes: setup-pixi 0s + curl fetch 1s + plan <1s = ~2s vs 4s, still 1 billed minute but less wall time
   - This repo plan job: build from source 56s could be replaced by using standalone binary in CI as well, saving 56s (1 billed minute)
   - Pros: preserves runtime safety, minimal change, no ADR
   - Cons: still 1 VM overhead, still 1 billed minute

3. **Merged job (plan as first step of publish)**:
   - Not possible with GitHub's matrix: `strategy.matrix` must be known before job starts, cannot be computed in same job's steps. Could use reusable workflow or `fromJSON` from file, but still needs separate job.

Recommendation: Option 2 (cheaper plan) as immediate win, Option 1 (static) as follow-up with opt-in flag `[workflow] static_matrix = true` to preserve backward compat. For now, document that plan job is intentional safety net, and optimize it via curl (TASK-118).

### Billing impact

- Consumer: plan job 1 minute billed per push, regardless of wall time. Eliminating it saves 1 minute per push. For 100 pushes/month, 100 minutes saved.
- This repo: plan job 2 minutes billed (due to 56s build), saving 2 minutes per push if using standalone binary, or 2 minutes if eliminating job entirely.

Decision: Keep dynamic plan for safety, but make it cheaper via TASK-118 curl optimization. Re-evaluate static rendering if billing becomes significant or if consumers request it.
