---
id: TASK-42
title: Make publish-sandbox embed source-built binary
status: In Progress
assignee: []
created_date: '2026-10-02 15:10'
updated_date: '2026-10-02 16:21'
labels:
  - ci
  - release
  - transport
dependencies: []
priority: high
ordinal: 43000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Build pixi-sandbox from checked-out source on each native publish runner. Use that binary for packing, verification, publishing, and the embedded bootstrap. Keep consumer and airlock workflows release-based.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 publish-sandbox has no release-binary download for its self-bootstrap
- [ ] #2 the workflow builds pixi-sandbox from the checked-out source on every native matrix runner
- [ ] #3 the source-built binary performs pack, doctor, and publish
- [ ] #4 consumer and airlock workflows continue using released packages or assets
- [ ] #5 generated workflow source and committed render stay synchronized
- [ ] #6 fixture tests cover source-built publishing and release-based consumers
- [ ] #7 repository checks and the full test suite pass
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Add or update fixture tests
- [ ] #2 Regenerate the committed workflow
- [ ] #3 Run pixi run --frozen fmt, lint, and test
- [ ] #4 Update task notes and final summary
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-02: Implemented source-built publisher workflow and updated generated fixture/tests. Local lint passes; full test suite passes 403/403 with 1 skipped. Native cross-platform publish proof remains CI work.

2026-10-02 audit fix: productionresults logs proved the source-built GNU binary was dynamically linked. The generated publisher now maps native Unix platforms to explicit release triples, installs the target and musl-tools on Linux, builds with --target, and embeds target/<triple>/release/pixi-sandbox. Local fmt, lint, generated-workflow checks, and 403/403 tests pass; CI publish proof remains open.
<!-- SECTION:NOTES:END -->
