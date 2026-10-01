---
id: TASK-24
title: Define standalone transport bootstrap and tool deduplication
status: Done
assignee: []
created_date: '2026-10-01 09:49'
updated_date: '2026-10-01 15:40'
labels:
  - transport
  - restore
  - spike
milestone: m-0
dependencies:
  - TASK-21
references:
  - .knowledge/decisions.md
  - .knowledge/rattler-spike.md
documentation:
  - backlog/docs/plans/v1-platform-workflow-transport/doc-1
  - backlog/docs/spikes/standalone-transport/doc-7
priority: high
type: spike
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Measure the proposal to make pixi-sandbox the standalone transport/restore orchestrator. Decide how pixi-pack and pixi-unpack are replaced or retained, version the transport schema, and ensure pixi-sandbox is not shipped twice as both bootstrap tool and environment dependency.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A measured binary size and restore-disk comparison exists
- [x] #2 A proposed schema migration or compatibility policy is recorded
- [x] #3 Duplicate pixi-sandbox packaging behavior is specified
- [x] #4 An accepted or rejected decision is recorded through the Backlog decision API
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Measured spike, no production code changed. All numbers were produced on 2026-10-01 in the restored developer sandbox from the published `sandbox/developer-linux-64` branch (snapshot `7c69402`, packed by 0.3.6) plus a fresh offline `cargo build --release` of v0.3.7; every command is listed in `doc-7` §5.

Binary size (✅ measured): a fresh v0.3.7 build is 3,437,520 bytes vs the shipped static 0.3.6 bootstrap at 3,553,504; `pixi-unpack` 0.7.11 static is 15,729,744 raw / 6,794,003 gzip; `pixi` 0.81.0 is 80,335,584 raw. Restore disk (✅ measured end to end with `du` sampling): git store 505 MiB, worktree 845 MiB, restore wall 16 s, preflight 2,831.9 MiB, peak project 2,746 MiB, peak combined 4,098 MiB, final tree 2,238 MiB. Per-part transport cost from `git verify-pack` (✅): the branch stores 503.9 MiB in-pack; `pixi-unpack` costs 6.5 MiB in-pack (1.3%), `pixi` 32.9 MiB (6.5%), the bootstrap 1.7 MiB (0.34%); the vendor tree dedups from 10,254 declared files (278.1 MiB) to 9,663 git objects (142.2 MiB raw, 25.8 MiB in-pack). Four restore-disk alternatives are compared in `doc-7` §2, including the newly identified option C′ (restore from the git object store, −845 MiB combined, no format change), which is noted for the backlog rather than adopted.

Schema policy (`doc-7` §3): schema stays 2; readers accept 1..=N and refuse newer; additive `#[serde(default)]` fields may land without a bump; removals or semantic changes are a schema-3 event that must keep reading older schemas while published branches exist; git hardlinks are not a dedup mechanism.

Duplicate-packaging rule (`doc-7` §4): the `tools` entry is the canonical executable and every launcher path executes it; an environment copy is payload restored verbatim (excluding it would corrupt the prefix against `pixi.lock`); `pack` should warn about the duplication rather than refuse; promotion of the environment's copy to the bootstrap is rejected (version/schema coupling).

Decision: recorded through the Backlog decision API as `decision-2` (accepted) — retain `pixi-pack`/`pixi-unpack` as pinned helper subprocesses; `pixi-sandbox` stays the single transport bootstrap; schema 2 stands. `decision-1` (the v1 standalone orchestrator proposal) is resolved as accepted with that boundary, and the outcome is mirrored in `.knowledge/decisions.md` as D14 with the header count and `.knowledge/README.md`/`CONTEXT.md` pointers updated.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
The standalone transport design is accepted as shipped: `pixi-sandbox` is the single verified bootstrap, the helper subprocesses stay, schema 2 needs no migration, and a canonical-tools/duplication rule now specifies exactly how `pixi-sandbox` is never shipped twice. The measured evidence (doc-7) shows the absorb-the-unpacker alternative trades 6.5 MiB of in-pack branch bytes for a comparably larger binary plus a bigger vendored payload, and names option C′ (no-checkout restore) as the cheapest future disk lever.
<!-- SECTION:SUMMARY:END -->
