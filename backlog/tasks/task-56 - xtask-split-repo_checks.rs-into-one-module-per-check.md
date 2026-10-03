---
id: TASK-56
title: 'xtask: split repo_checks.rs into one module per check'
status: To Do
assignee: []
created_date: '2026-10-03 09:26'
updated_date: '2026-10-03 09:27'
labels:
  - refactor
  - solid
dependencies: []
priority: medium
ordinal: 57000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Split the 1,078-line crates/xtask/src/repo_checks.rs (10 unrelated lint policies in one file) into crates/xtask/src/repo_checks/{mod,badges,stale_refs,conda_manifest,mutable_refs,channel_drift,release_tags,bash32,workflow_shape,relock}.rs, each carrying its own tests, with mod.rs holding a small CHECKS function-pointer table and the shared opt-out-marker helpers in a support.rs sibling. See doc-9 section A2.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 repo_checks.rs is split into one file per numbered check under crates/xtask/src/repo_checks/, each with its own tests
- [ ] #2 mod.rs holds a CHECKS function-pointer table used by check_repository
- [ ] #3 Shared opt-out-marker helpers live in one support module, not duplicated per check
- [ ] #4 pixi run --frozen test and pixi run --frozen lint pass with the same check-repository behavior
<!-- AC:END -->
