---
id: TASK-42
title: Make publish-sandbox embed source-built binary
status: Done
assignee: []
created_date: '2026-10-02 15:10'
updated_date: '2026-10-02 16:36'
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
- [x] #1 publish-sandbox has no release-binary download for its self-bootstrap
- [x] #2 the workflow builds pixi-sandbox from the checked-out source on every native matrix runner
- [x] #3 the source-built binary performs pack, doctor, and publish
- [x] #4 consumer and airlock workflows continue using released packages or assets
- [x] #5 generated workflow source and committed render stay synchronized
- [x] #6 fixture tests cover source-built publishing and release-based consumers
- [x] #7 repository checks and the full test suite pass
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Add or update fixture tests
- [x] #2 Regenerate the committed workflow
- [x] #3 Run pixi run --frozen fmt, lint, and test
- [x] #4 Update task notes and final summary
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-02: Implemented source-built publisher workflow and updated generated fixture/tests. Local lint passes; full test suite passes 403/403 with 1 skipped. Native cross-platform publish proof remains CI work.

2026-10-02 audit fix: productionresults logs proved the source-built GNU binary was dynamically linked. The generated publisher now maps native Unix platforms to explicit release triples, installs the target and musl-tools on Linux, builds with --target, and embeds target/<triple>/release/pixi-sandbox. Local fmt, lint, generated-workflow checks, and 403/403 tests pass; CI publish proof remains open.

2026-10-02 native proof: PR #63 merged at b7ad517. Post-merge publish-sandbox run 37034736441 passed; snapshot a580e80 records source commit b7ad517 and embeds pixi-sandbox 0.4.1 with static linkage. CI and docs also passed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The generated publisher builds the checked-out source for an explicit native target, using musl on Linux, then uses that static source-built binary for pack, doctor, publish, and the embedded bootstrap. Consumer and airlock paths remain release-based. PR #63 merged; post-merge CI, docs, and sandbox publication passed, and the published manifest proves static pixi-sandbox 0.4.1 from b7ad517.
<!-- SECTION:FINAL_SUMMARY:END -->
