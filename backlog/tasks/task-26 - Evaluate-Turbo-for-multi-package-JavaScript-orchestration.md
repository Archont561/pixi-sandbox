---
id: TASK-26
title: Evaluate Turbo for multi-package JavaScript orchestration
status: Done
assignee: []
created_date: '2026-10-01 09:49'
updated_date: '2026-10-01 18:10'
labels:
  - bun
  - turbo
  - tooling
milestone: m-0
dependencies:
  - TASK-25
references:
  - .knowledge/v1-evolution-plan.md
documentation:
  - backlog/docs/plans/v1-platform-workflow-transport/doc-1
  - backlog/docs/spikes/turbo-orchestration/doc-8
priority: low
type: spike
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Do not add Turbo merely for the current single docs workspace. Re-evaluate after multiple JS packages exist; if adopted, make Bun own dependencies, Turbo own the JS task graph, and Pixi provide the environment facade.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Adoption threshold and task graph are documented
- [x] #2 Turbo is a Bun dev dependency, not a Conda runtime dependency
- [x] #3 Cache inputs/outputs and CI behavior are specified
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decision spike only — no production code changed, per the task's own instruction not to add
Turbo for a single docs workspace. The evaluation lives in
`backlog/docs/spikes/turbo-orchestration/doc-8`: §1 measures the current state (one bun
workspace package, `docs`; no cross-package JS edge; docs CI build job ~27 s on
`ubuntu-latest` including runner bootstrap); §2 sets the adoption threshold — ≥ 3 JS packages
**and** a real cross-package import edge **and** ≥ ~60 s of repeated JS work on the CI path
(or a local loop past ~10 s for unchanged packages); §3 drafts the `turbo.json` task graph for
adoption day (build/check/lint/dev mirroring the pixi task names, lockfiles in every hash);
§4 fixes ownership — Turbo as a root `package.json` devDependency pinned via `bun.lock`
(never conda/pixi, so the sandbox payload budget is untouched), invoked as `bun x turbo …`
in the `web` environment; §5 fixes cache policy — local `.turbo` for developers, `--force`
for published builds, CI caches `.turbo` keyed on `bun.lock` + OS, remote caching off as an
airlock posture. The decision — **defer** — is recorded through the Backlog decision API as
decision-3 (status `deferred`) and compressed into `.knowledge/decisions.md` as D15.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
Turbo stays out: the repository's JS surface is one docs package building in ~27 s, which
meets none of the three documented adoption triggers. The threshold, the task graph, the
Bun-not-Conda ownership rule, and the cache/CI policy are written down in doc-8 and decision-3
(D15), so the next evaluation is a lookup, not a re-spike.
<!-- SECTION:SUMMARY:END -->
