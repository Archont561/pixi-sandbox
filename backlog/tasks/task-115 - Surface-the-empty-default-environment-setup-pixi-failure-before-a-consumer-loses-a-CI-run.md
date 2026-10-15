---
id: TASK-115
title: >-
  Surface the empty-default-environment setup-pixi failure before a consumer
  loses a CI run
status: Done
assignee: []
created_date: '2026-10-15 17:48'
updated_date: '2026-10-15 19:37'
labels:
  - docs
  - ci
  - init
dependencies: []
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/158'
priority: high
ordinal: 115000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Consumers whose pixi.toml declares every environment explicitly, with no dependencies in the implicit default feature, meet the error No packages found in default environment for linux-64 platform from the setup-pixi step and never reach their own CI. setup-pixi runs pixi info, then pixi install --locked, then pixi list --locked, and the empty default environment makes the last one exit 1 before any consumer command runs. This project own examples produce that shape and nothing documents it, so the failure names neither the environment nor the failed command. See issue 158 and polybridge PR 1. Scope is to document the caution where consumers configure CI, and to decide whether init --check or plan should report an all-explicit workspace so the pitfall is visible before a CI run is lost. The two upstream defects, pixi list exiting non-zero for an empty environment and setup-pixi not tolerating it, are out of scope and tracked upstream.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The consumer-facing CI or workflow-policy guide warns that when the workspace default environment is empty, setup-pixi must be given an explicit environments value, and shows the naming workaround.
- [x] #2 A check reachable from the CLI (init --check or plan, whichever the design lands) reports an all-explicit workspace whose implicit default environment is empty, or a recorded decision explains why no check is added.
- [x] #3 The detection, or the documented caution when no check is added, is covered by a test at the appropriate boundary.
- [x] #4 The upstream pixi and setup-pixi defects are recorded as out of scope with a pointer, so the repository does not silently absorb them.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-15 — Closed from merged PR #161 (5d8386366d7244ba764e7d28eec2293e1aed56b1). The PR closed GitHub issue #158 and added the consumer-facing caution/workaround in the CI publishing guide.

Evidence by criterion:
- AC#1: the guide names the empty implicit default environment failure and shows the explicit `setup-pixi` `environments: core` workaround.
- AC#2: the recorded decision in CONTEXT.md keeps this docs-only; pixi-sandbox does not parse consumer pixi.toml, and inferring an empty default environment would require Pixi semantics and risk false results.
- AC#3: the merged main CI run 38785621372 and docs run 38785621371 both passed for PR #161, covering the repository documentation quality/build boundary.
- AC#4: the guide identifies pixi `list` and `setup-pixi` handling as upstream behavior and points to issue #158 rather than absorbing it into pixi-sandbox.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
PR #161 documented the empty implicit default-environment setup-pixi failure before a consumer loses its CI run, showed the explicit-environment workaround, and recorded why pixi-sandbox intentionally does not add a heuristic CLI check. The upstream pixi/setup-pixi behavior remains out of scope. PR #161 closed issue #158; its main CI and docs workflows passed.
<!-- SECTION:FINAL_SUMMARY:END -->
