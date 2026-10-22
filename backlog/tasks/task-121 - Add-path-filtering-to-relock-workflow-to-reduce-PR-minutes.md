---
id: TASK-121
title: Add path filtering to relock workflow to reduce PR minutes
status: To Do
assignee: []
created_date: '2026-10-22 07:34'
labels:
  - ci
  - workflow
  - optimization
  - relock
dependencies: []
priority: medium
type: enhancement
ordinal: 121000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Issue #179 section 4: unfiltered relock.yml triggers on PRs without manifest changes

Current relock.yml:
- on: pull_request (no paths filter) + workflow_dispatch with repaired_sha input
- guard job runs on every PR, even if PR only modifies README.md, docs, non-code files
- cargo fetch --locked runs without caching, downloading index on every invocation

Proposed fix from issue:
on:
  pull_request:
    paths:
      - **/Cargo.toml
      - Cargo.lock
      - pixi.toml
      - pixi.lock
      - package.json
      - bun.lock
      - .cargo/config.toml

When manifest files have not changed, lock files cannot be out of sync.

Evaluate against current design comment in relock_workflow.rs:
"The guard job reports on every pull request; the relock job runs only when the guard found drift, which is why this workflow carries no paths: filter — a stale lock arriving by any route (a transitive manifest, a hand-edited lock, a deleted one) is caught the same way."

Consider:
- If PR deletes pixi.lock, paths filter including pixi.lock would still trigger? GitHub paths filter triggers on changed files, including deletions, so yes it would trigger if lock file itself is in filter. But if filter only includes manifests, deletion of lock would not trigger guard, yet lock is missing and should be caught.
- Hand-edited lock: if someone edits pixi.lock directly without manifest change, should guard catch? Yes, but paths filter including lock files would catch.
- Transitive manifest: pixi.toml includes dependencies, but what about included files? Pixi manifest could be in subdir? Current filter uses **/Cargo.toml to catch workspace members.
- Cost: guard runs pixi lock --check and cargo fetch --locked, which are cheap (no solve, just check). But setup-pixi + checkout still costs VM boot + minute billing.
- Could we make paths filter configurable via [workflow] table like publish workflow push_paths? Or default-on with safe set?
- TASK-66 already makes guard produce authoritative verdict on repaired head; paths filter would reduce noise but must not hide drift.

Reference: https://github.com/Archont561/pixi-sandbox/issues/179 section 4
Reference: crates/pixi-sandbox/src/generated/relock_workflow.rs comment
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Measure relock guard job time and cost on docs-only PR vs manifest-changing PR
- [ ] #2 Document current no-paths design rationale and what cases it catches that paths filter would miss
- [ ] #3 Propose safe default paths filter that includes manifests and lockfiles and does not miss hand-edited or deleted locks
- [ ] #4 Implement paths filter in relock template as optional via config or default, with tests
- [ ] #5 Verify actionlint, check-repository, and that stale lock arriving by any route is still caught when filter matches
<!-- AC:END -->
