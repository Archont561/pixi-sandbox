---
id: TASK-66
title: 'relock: attach an authoritative verdict to the repaired PR head'
status: To Do
assignee: []
created_date: '2026-10-04 13:19'
labels:
  - relock
  - ci
  - github-actions
dependencies:
  - TASK-65
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/92#issuecomment-5974132039'
  - 'https://github.com/Archont561/castellan/pull/8'
  - crates/pixi-sandbox/src/generated/relock_workflow.rs
priority: high
type: enhancement
ordinal: 66000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The issue #92 consumer evidence shows that a successful generated relock leaves reviewers with one visible red guard on the old SHA and no check rollup on the bot-authored repair SHA. The detached workflow_dispatch validations pass, but are not attached to the pull request. Make the generated relock lane publish one authoritative verdict on the repaired head without weakening fork safety or treating an unvalidated repair as green.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A detected drift that is successfully repaired does not leave the relock workflow permanently red solely because its initial guard found repairable drift
- [ ] #2 The bot-authored repaired SHA receives one authoritative PR-visible verdict after lock guard, configured CI, and sandbox publish validation complete
- [ ] #3 A failed repair, failed validation, fork pull request, or missing permission fails closed and reports an actionable reason
- [ ] #4 The design avoids duplicate publish/relock loops and preserves the existing rule that dispatch occurs only after a real lock commit
- [ ] #5 Renderer tests cover clean, repaired, failed, and fork paths; the generated workflow remains actionlint-clean and init regeneration remains byte-identical
- [ ] #6 A consumer pull request proves the repaired head has a usable check rollup; record the PR and run links in this task and issue #92
<!-- AC:END -->
