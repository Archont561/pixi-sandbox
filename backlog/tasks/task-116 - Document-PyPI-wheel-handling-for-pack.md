---
id: TASK-116
title: Document PyPI wheel handling for pack
status: In Progress
assignee: []
created_date: '2026-10-16 00:00'
updated_date: '2026-10-16 07:06'
labels:
  - docs
  - pack
  - pypi
dependencies:
  - TASK-114
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/157'
  - backlog/tasks/task-114 - Make-pack-refuse-a-non-wheel-PyPI-dependency-before-it-creates-the-transport.md
  - backlog/docs/adr/ADR-21 - uv-regenerated install metadata is presence-checked, not digested (issue #128, TASK-88, TASK-93).md
  - docs/src/content/docs/reference/cli/create-verify.mdx
  - docs/src/content/docs/troubleshooting.mdx
priority: medium
type: docs
ordinal: 116000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-114 made `pack` reject, before creating output, selected-environment PyPI lock entries whose location is not a wheel. The behavior is not yet explained on either the `pack` reference or the symptom-first troubleshooting page. Document the exact wheel rule and accepted/refused location shapes, explain the pack-time (not plan-time) check, give the four recovery options in their recommended order, and make the upstream pixi-pack escape hatches and their unavailability through `pixi-sandbox pack` explicit. Preserve the ADR-21 distinction between uv-regenerated install metadata and wheel payloads. This is documentation only: no preflight change and no pixi-pack argument passthrough.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `reference/cli/create-verify.mdx` § `## pack` explains that the check runs during `pack`, not `plan`, and applies to the selected environments and platform before output is created.
- [x] #2 The reference documents the accepted wheel-location rule and examples, the refused source/path shapes, and that wheels can be platform-tagged rather than only universal.
- [x] #3 The reference gives the four recovery options in order, explains both upstream pixi-pack escape hatches are not exposed by `pixi-sandbox pack`, and does not imply a flag passthrough exists.
- [x] #4 The reference accurately explains that uv regenerates install metadata during unpack and that ADR-21 presence-checks the relevant metadata rather than digesting it; this does not widen the wheel acceptance rule.
- [x] #5 `troubleshooting.mdx` adds symptom 9 in the same six-field shape as entries 1–8, covering the refusal, safe action, evidence, recovery and escalation without changing existing entries.
- [ ] #6 All requested docs gates pass: frozen docs install once; docs build is warning-free; `test-doc` and `lint` pass including link and accessibility checks; the workspace test result remains 1,191 passing / 2 skipped.
<!-- AC:END -->

## Definition of Done

<!-- DOD:BEGIN -->
- [x] #1 `pixi run --frozen docs-install`
- [ ] #2 `pixi run --frozen bun --filter=pixi-sandbox-docs run build` (zero warnings)
- [x] #3 `pixi run --frozen test-doc`
- [x] #4 `pixi run --frozen lint`
- [x] #5 `pixi run --frozen test` (1,191 passing / 2 skipped)
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Add one PyPI-wheel subsection inside the pack reference section and one ninth numbered symptom in the troubleshooting page. Keep the preflight and CLI unchanged. Validate with the listed docs and workspace gates, then close the task with the evidence in house format.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-16: Added the requested PyPI wheel subsection under `## pack` and troubleshooting symptom 9; no runtime/preflight behavior or flag passthrough changed. `docs-install`, `test-doc`, `lint`, and the direct docs frontmatter/link/accessibility/snippet checks pass (2,231 internal links, 29 pages; zero link and accessibility failures). The workspace test run remains 1,191 passing / 2 skipped (xtask 242, core 316, sandbox 579 + 2 skipped, git 54).

AC#6 / DoD#2 remain open: the requested Astro build exits 0 and builds 29 pages, but reports one route-conflict warning for the pre-existing `/404` content entry versus Starlight's higher-priority `/404` route. The warning is unrelated to either changed docs page. No unrelated 404-page/config change was made because this slice is limited to TASK-116 and its two specified documentation surfaces; the warning-free-build requirement is therefore not claimed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
<!-- SECTION:FINAL_SUMMARY:END -->
