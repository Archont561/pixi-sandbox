---
id: TASK-44
title: Restore patch coverage for Git status boundary
status: Done
assignee:
  - '@agent'
created_date: '2026-10-02 18:05'
updated_date: '2026-10-02 18:56'
labels:
  - rust
  - testing
  - tooling
dependencies:
  - TASK-43
priority: medium
type: bug
ordinal: 45000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The task-30 Git-boundary migration added `ShellGit::worktree_status_files`, but PR #66's Codecov patch check reported 72.22% diff coverage against an 84.22% target. Cover the new status parser's empty-output and invalid-UTF-8 paths so the migration does not reduce the repository's patch-coverage gate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The new Git status operation has fixture-backed coverage for tracked changes, empty output, and invalid UTF-8 output
- [x] #2 The pixi-sandbox-git test suite passes with the added coverage cases
- [x] #3 The PR Codecov patch check passes at or above its configured target on GitHub
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added `FixedRunner` fixture coverage in `crates/pixi-sandbox-git/tests/publish.rs` for empty and invalid UTF-8 status output, alongside the existing tracked-change case. Local verification passed: 21 pixi-sandbox-git tests. The first PR check measured 72.22% diff coverage against an 84.22% target; the follow-up PR CI run passed, including the Codecov patch check (run 37049757608).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
The missing Git status parser branches are covered locally. PR #66 merged after GitHub confirmed the Codecov patch threshold in run 37049757608; the post-merge main CI run 37050382114 also passed.
<!-- SECTION:SUMMARY:END -->
