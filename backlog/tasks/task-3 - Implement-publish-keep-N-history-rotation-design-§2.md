---
id: TASK-3
title: Implement publish --keep N history rotation (design §2)
status: Done
assignee:
  - '@agent'
created_date: '2026-09-28 22:05'
updated_date: '2026-09-29 07:42'
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
- [x] #1 publish --keep N rebuilds the branch so it carries at most N snapshot trees per design §2
- [x] #2 A FakeGit test proves the rotation contract and a ShellGit test proves the pushed history actually shrank
- [x] #3 Rotation is opt-in: without --keep the current replace-history behaviour is unchanged
- [x] #4 docs/reference/cli.mdx documents the flag and the prune caveat
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
TDD, three seams: (1) GitProtocol/FakeGit contract - publishing with keep=N leaves at most N snapshot commits, newest first, oldest dropped; (2) ShellGit against a local bare remote - the pushed history is rebuilt and rev-list --count shrinks; (3) CLI - --keep N publishes and reports rotation, and without it the single-orphan-commit behaviour is byte-identical. Then docs/reference/cli.mdx documents the flag and the prune caveat (force-push is not a shrink, design 2).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Red 1 (FakeGit seam): the_mock_rotates_the_branch_to_at_most_keep_snapshots and the_mock_treats_keep_one_and_no_keep_the_same_way failed to compile - Snapshot had no keep. Green: Snapshot.keep plus Snapshot::retained(), FakeGit rebuilds history newest-first and truncates to keep.

Red 2 (ShellGit seam, local bare remote): four publishes with keep 2 served 1 commit instead of 2, and lowering keep from 3 to 1 did not shrink. Green: ShellGit::rebuilt_parent fetches the kept commits shallow with --filter=blob:none (fallback to a plain shallow fetch), re-commits them oldest-first as a fresh parentless chain, and the new snapshot is committed on top, then force-pushed as before.

Red 3 (CLI seam): publish --keep 2 still errored not_yet. Green: args.keep flows into Snapshot and a rotating publish prints the retained count plus the prune caveat. publish_pushes_the_transport_as_a_single_orphan_commit stayed green untouched, which is AC3.

Guard added after the loop: a_rotating_publish_fetches_metadata_only_and_a_default_one_does_not_fetch_at_all asserts the argv (one fetch, --filter=blob:none, --depth=1) and that a default publish never fetches - it was green on arrival, it exists so the cheapness of rotation cannot be lost silently.

Live smoke outside the suite: four publishes of the fixture transport with --keep 3 against a local bare remote left exactly 3 commits. Docs: docs/reference/cli.mdx gained a Rotation section with a caution that rotation is not a shrink; design.md step 4 and the README CLI notes updated; tools update is now the only deferred verb.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
publish --keep N rebuilds the orphan branch to the N most recent snapshots: the kept commits are fetched shallow and blobless, re-committed as a fresh chain, and the new snapshot is force-pushed on top. Proven by a FakeGit contract test, two ShellGit tests against a real bare remote (bounded history, and an existing history shrinking when keep is lowered) and a CLI test. Without --keep nothing changed. Documented with the prune caveat: rotation bounds what the branch serves, not what the server stores.
<!-- SECTION:FINAL_SUMMARY:END -->
