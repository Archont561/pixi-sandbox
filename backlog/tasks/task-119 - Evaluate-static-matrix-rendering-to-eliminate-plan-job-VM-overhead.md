---
id: TASK-119
title: Evaluate static matrix rendering to eliminate plan job VM overhead
status: To Do
assignee: []
created_date: '2026-10-22 07:34'
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
- [ ] #1 Measure plan job wall time and billed minutes in current template and in this repo publish-sandbox.yml
- [ ] #2 Document when matrix is static vs dynamic, and what inputs affect it
- [ ] #3 Propose design: static rendering option, or cheaper plan job, or merged job
- [ ] #4 If static rendering, implement in generated workflow with backward compatibility and tests
- [ ] #5 Verify existing consumers still work, golden file updated, actionlint clean
<!-- AC:END -->
