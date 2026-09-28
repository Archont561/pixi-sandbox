---
id: TASK-3
title: Implement publish --keep N history rotation (design §2)
status: To Do
assignee: []
created_date: '2026-09-28 22:05'
labels:
  - cli
  - publish
dependencies: []
references:
  - crates/pixi-sandbox/src/commands/publish.rs
  - .knowledge/design.md
priority: high
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
publish.rs returns not_yet for --keep. Every publish appends a new tree to the orphan branch and a force-push provably does not reclaim server space (measured, design §2), so a long-lived branch grows unboundedly toward the ~1 GB soft repo budget. Rotation must rebuild history: push a fresh tree keeping at most N snapshots and let the operator prune.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 publish --keep N rebuilds the branch so it carries at most N snapshot trees per design §2
- [ ] #2 A FakeGit test proves the rotation contract and a ShellGit test proves the pushed history actually shrank
- [ ] #3 Rotation is opt-in: without --keep the current replace-history behaviour is unchanged
- [ ] #4 docs/reference/cli.mdx documents the flag and the prune caveat
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Extend pixi-sandbox-git with a history-rebuilding push behind the existing --keep flag, cover the contract with FakeGit, prove the shrunken history with ShellGit, document in cli.mdx.
<!-- SECTION:PLAN:END -->
