---
id: TASK-72
title: Establish transport size budgets and environment-set partitioning
status: Done
assignee:
  - '@me'
created_date: '2026-10-04 20:38'
updated_date: '2026-10-06 10:52'
labels:
  - transport
  - airlock
  - architecture
  - measurement
dependencies: []
references:
  - 'https://github.com/Archont561/castellan/pull/13'
priority: high
type: spike
ordinal: 72000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Castellan transport is approximately 1.86 GiB and is approaching GitHub practical repository and push limits. Measure the current transport model before changing the manifest or restore protocol. Keep environment sets as the partition boundary: do not compose partial environments or introduce external payloads without evidence.

Establish reviewed budgets for the largest blob, packed transport, repository/push size, fetch/clone time, restore disk headroom, and publish behavior. Use fixture/tempdir transports plus the connected Castellan transport as measurements. If the budget requires partitioning, make the planner/configuration expose separate environment-set bundles and preserve self-contained, backward-compatible transports.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A documented measurement records current largest blob, packed transport size, repository size, fetch/clone cost, restore disk requirement, and publish/push behavior for representative fixture and Castellan transports.
- [x] #2 Reviewed hard refusal thresholds exist for blob, transport, repository/push, and restore-disk limits, with actionable errors naming the exceeded budget and the partition remedy.
- [x] #3 The planner/configuration can represent separate environment-set bundles without splitting an individual environment or weakening manifest self-containment; existing single-bundle transports remain restorable.
- [x] #4 Tempdir/fixture tests cover under-budget, over-budget, and partitioned environment-set plans; no network or this checkout is used as a test fixture.
- [x] #5 The spike records whether external immutable payloads are necessary; no payload-format migration is implemented unless the measurements demonstrate that environment-set partitioning is insufficient.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 pixi run --frozen fmt
- [x] #2 pixi run --frozen lint
- [x] #3 pixi run --frozen test
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Measure current transport size constraints from fixture/tempdir transports and any accessible connected transport evidence; codify hard budgets in configuration/planner validation with actionable errors; add tests for under-budget, over-budget, and partitioned environment-set plans; document whether external payloads are necessary; run fmt/lint/test and update task evidence.
<!-- SECTION:PLAN:END -->


## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-06: Added reviewed `[budgets]` config with defaults of max blob 95 MiB, max transport 2048 MiB, max repository push 2048 MiB, and max restore-required 8192 MiB. `doctor --budget-config <PATH>` now measures schema-2 manifests and exits non-zero before publish when any ceiling is exceeded, with diagnostics naming the failed budget and advising environment-set partitioning via separate `[[bundle]]` entries. Generated Bash and PowerShell publisher lanes, the checked-in publisher workflow, `pixi-sandbox init`, README, and reference docs all use/document the reviewed config path.

Partitioning remains at the existing planner/config boundary: separate `[[bundle]]` entries produce independent, self-contained orphan branches without splitting an individual environment or changing the manifest/restore format. Tests cover default/override validation, the 95 MiB blob cap, partitioned environment-set plans, under-budget reports, JSON budget reporting, and over-budget refusal in tempdir fixtures.

Measurements are recorded in `backlog/docs/spikes/transport-budgets/doc-11 - Transport-Size-Budgets-and-Environment-Set-Partitioning.md`. Highlights: fixture transport is 90,985 B logical / 100,001 B snapshot / 77,213 B largest blob / 265,542 B restore preflight, one local orphan publish commit, and a 0.023 s + 0.008 s local `file://` clone/checkout; current pixi-sandbox branch is 812.5 MiB declared / 814.6 MiB snapshot / 95.0 MiB largest stored blob / 2,798.5 MiB restore preflight with 11.62 s shallow clone; Castellan is 1,857.0 MiB declared / 1,862.0 MiB snapshot / 95.0 MiB largest stored blob / 7,385.3 MiB restore preflight with 19.68 s shallow clone. Dry-run publish evidence confirms orphan force-push shape for measured transports. The spike concludes external immutable payloads are not necessary for current evidence; reopen only if real consumers cannot stay within budgets after environment-set partitioning or a remote rejects below-threshold pushes.

Verification: `pixi run --frozen fmt` passed; `pixi run --frozen -- cargo check -p pixi-sandbox --offline` passed; targeted tests passed for core transport budget, sandbox config, CLI doctor, and generated workflow; `pixi run --frozen lint` passed after docs/README updates; full `pixi run --frozen test` passed with pixi-sandbox-git 21/21, pixi-sandbox-core 147/147, pixi-sandbox 388/388 with 1 skipped, and xtask 88/88; `pixi run --frozen docs build` passed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Closed TASK-72 by measuring representative and Castellan transports, codifying reviewed transport budgets, enforcing them in `doctor --budget-config` before generated publishes, documenting the environment-set partition remedy, and proving the planner/config can emit separate self-contained bundles. No payload-format migration was added because the measured transports fit the reviewed ceilings and environment-set partitioning remains sufficient.
<!-- SECTION:FINAL_SUMMARY:END -->
