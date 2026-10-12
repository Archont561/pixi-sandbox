---
id: TASK-105
title: Expand the composite Actions migration guide into a migration matrix
status: Done
assignee: []
created_date: '2026-10-12 07:56'
updated_date: '2026-10-12 09:12'
labels:
  - docs
  - migration
  - actions
dependencies: []
references:
  - docs/src/content/docs/guides/actions.mdx
  - docs/src/content/docs/guides/ci-publishing.mdx
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: medium
type: docs
ordinal: 105000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The "Migrating from composite Actions" page covers one retired path. Expand it into a full migration matrix for the retired composite Actions and the reusable publisher: before/after commands, generated workflow changes, and an upgrade checklist, cross-linked from CI publishing and the compatibility page.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The guide maps every retired interface (composite Actions, reusable publisher) to its replacement with before/after commands.
- [x] #2 Generated workflow changes required by the migration are spelled out with a checklist.
- [x] #3 CI publishing and the compatibility page link into the migration guide.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12: guides/migration.mdx expanded from the single-path stub into a full migration matrix: (1) a table mapping every retired interface — root composite Action, setup composite Action, publish composite Action, repository reusable publisher — to its replacement; (2) before/after: a v0.3.x consumer job calling the retired `uses:` refs versus the two-command replacement (`pixi global install` from the ecosystem channel + `pixi-sandbox init`) and the commit of the four generated paths; (3) a capability table (native matrix, checksum-verified bootstrap via fetch-release, doctor --verify gate, --cargo-vendor, orphan-branch publish, upgrade lane) pointing at the generated workflow's real shape; (4) an eight-item generated-workflow changes checklist; (5) a seven-step ordered upgrade runbook ending in a `gh workflow run ... -f upgrade=X.Y.Z` dispatch; (6) a "cannot migrate yet" note that old immutable refs keep resolving. Cross-links per AC#3: compatibility.mdx already pointed here (its retired-interface table corrected to the `pipeline` step for consistency) and guides/ci-publishing.mdx now links the guide from its migration sentence; workflow-policy.mdx already linked it. Retired interfaces are named exactly as the project's own docs name them (root/setup/publish + reusable publisher) — no interface paths were invented; the repo's shallow history contains none. ContextChips added on the connected-host and CI command blocks. Build 28 pages, zero warnings; frontmatter check clean; link-check 1300 links, 28 failures — all the pre-existing favicon defect (task-110).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The migration guide now maps every retired composite Action and the reusable publisher to its replacement with before/after commands, spells out the generated workflow changes as a checklist plus an ordered runbook, and is cross-linked from both CI publishing and the compatibility page.
<!-- SECTION:FINAL_SUMMARY:END -->
