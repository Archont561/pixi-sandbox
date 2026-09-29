---
id: TASK-10
title: 'Verify a restored project against the manifest, not just the branch'
status: To Do
assignee: []
created_date: '2026-09-29 10:54'
labels:
  - core
  - restore
dependencies: []
priority: medium
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doctor --verify runs over the *branch* before restore, and restore verifies each blob sha256 as it writes. Nothing checks the tree that comes out the other end, so a restore that produces a structurally plausible but wrong prefix is indistinguishable from a good one.

Measured against a real packed, published and restored transport: a prefix containing one hand-forged conda-meta record and a single payload file passes every check in `scripts/airlock-gate.sh`. Deleting the environment outright is worse still - `pixi install --frozen --offline` re-fetches it over a reachable network, reports "The default environment has been installed", and a before/after file count taken across the call can land on equal numbers, so the no-op check reads as a pass.

There is no oracle to close this with today. A manifest env entry stores `.conda` archives with one sha256 each, `unpacked_size_bytes` is a size and came out 0 in the fixture pack, and `pixi_environment_fingerprint` is the marker file pixi itself writes, not a hash of the extracted tree. The airlock gate therefore checks shape, not content, and its honest scope has to stay "self-sufficiency, not integrity".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The manifest records enough to verify a restored prefix - the unpacked file list with a sha256 per file, or a per-env digest of the extracted tree - with SCHEMA_VERSION bumped and tests/manifest.rs plus the fixtures updated,A doctor mode checks a restored project against the manifest and collects every mismatch instead of stopping at the first and writes nothing,scripts/airlock-gate.sh calls it and a stub prefix with fabricated conda-meta is rejected - proven by a test rather than asserted in a comment,.knowledge/design.md states which layer verifies integrity and which verifies self-sufficiency
<!-- AC:END -->
