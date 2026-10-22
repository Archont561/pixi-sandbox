---
id: TASK-118
title: Optimize publish workflow bootstrap to eliminate double installation
status: To Do
assignee: []
created_date: '2026-10-22 07:34'
labels:
  - ci
  - workflow
  - optimization
dependencies: []
priority: high
type: enhancement
ordinal: 118000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Issue #179 identifies double installation in publish-sandbox.yml publish job:
1. pixi global install pixi-sandbox==VERSION from conda
2. pixi-sandbox fetch-release --dest BIN --version VERSION which downloads standalone binary

Current design is intentional: conda install provides the tool that does checksum-verified fetch of static binary (ADR-4 static linkage, TASK-76 bootstrap exception). The standalone binary is then embedded as transport bootstrap and must be static.

Evaluate:
- Whether conda binary can be used directly as pipeline entrypoint and self-bin when static linkage is already satisfied (e.g. linux musl conda package may already be static, or dynamic is acceptable for connected publish job but not for embedded bootstrap)
- Whether fetch-release can be made to not require a prior pixi-sandbox install (e.g. curl + SHA256SUMS verification in shell, or a small standalone fetcher)
- Measure time saved: conda solve + install vs curl download, and billing minutes
- Ensure security: checksum verification must remain, no unverified execution
- Ensure static linkage guarantee for embedded bootstrap is preserved

Reference: https://github.com/Archont561/pixi-sandbox/issues/179 section 1
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Measure current publish job time breakdown: setup-pixi, conda install, fetch-release, pipeline
- [ ] #2 Document why double install exists: static linkage, checksum verification, ADR-4, TASK-76
- [ ] #3 Propose one bootstrap path that preserves verification and static guarantee while reducing minutes
- [ ] #4 Implement chosen path in generated workflow template with tests and golden file update
- [ ] #5 Verify actionlint, check-repository, and consumer-proof still pass
<!-- AC:END -->
