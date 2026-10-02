---
id: TASK-43
title: Route pack provenance through pixi-sandbox-git
status: Done
assignee: []
created_date: '2026-10-02 16:22'
updated_date: '2026-10-02 16:22'
labels:
  - architecture
  - git
dependencies: []
priority: high
ordinal: 44000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Remove the direct git subprocess from pack source provenance so production Git access remains inside pixi-sandbox-git.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 pack invokes no git subprocess directly,pixi-sandbox-git owns optional current-commit lookup,fmt lint and full tests pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-02: Repo-wide audit found the pack git_commit helper bypassing the Git boundary. Moved the optional lookup into pixi-sandbox-git through its Runner command model; local fmt, lint, and 403/403 tests pass.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Pack source provenance now calls pixi_sandbox_git::current_commit. The Git crate executes optional rev-parse through its Runner command representation and preserves prior best-effort behavior. Fmt, lint, and all 403 tests pass.
<!-- SECTION:FINAL_SUMMARY:END -->
