---
id: TASK-10
title: 'Verify a restored project against the manifest, not just the branch'
status: Done
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
- [x] #1 The manifest records enough to verify a restored prefix — the unpacked file list with a sha256 per file — with SCHEMA_VERSION bumped (1 → 2) and tests/manifest.rs plus the fixtures updated
- [x] #2 A doctor mode checks a restored project against the manifest and collects every mismatch instead of stopping at the first and writes nothing (`doctor --verify-restored <PROJECT>`, D13)
- [x] #3 scripts/airlock-gate.sh calls it (`--transport <dir>`) and a stub prefix with fabricated conda-meta is rejected — proven by a test (`the_airlock_gate_rejects_a_forged_conda_meta_record`), not asserted in a comment
- [x] #4 .knowledge/design.md states which layer verifies integrity and which verifies self-sufficiency (§7 "Which layer proves what")

Done 2026-09-29. Also fixed while here: the gate's first-run no-op failure (pixi writes its own
bookkeeping on first install — the gate now settles that install, then measures the second),
and the `--skip-cargo` early exit that would have skipped the integrity section on hosts with
no cargo. conda-meta records are presence-only by design (their bodies embed digests of
relocated files); task-1 (osx-arm64) needs macOS hardware and stays open.
<!-- AC:END -->
