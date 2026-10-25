---
id: TASK-131
title: Carry per-workspace Cargo vendor trees in one transport
status: Done
assignee: []
created_date: '2026-10-23 18:44'
labels:
  - vendor
  - cargo
  - pack
  - restore
  - transport
milestone: m-1
dependencies:
  - TASK-129
references:
  - crates/pixi-sandbox-core/src/manifest.rs
  - crates/pixi-sandbox/src/vendor.rs
  - crates/pixi-sandbox/src/commands/restore.rs
documentation:
  - >-
    backlog/docs/adr/ADR-6 - Vendored cargo crates live under
    .pixi-sandbox-vendor.md
  - >-
    backlog/docs/adr/ADR-32 - A transport branch is one bundle for one platform
    and its payload stays self-contained.md
  - >-
    backlog/docs/adr/ADR-33 - One cargo vendor tree per workspace, carried in
    one transport.md
priority: medium
type: feature
ordinal: 131000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Per-workspace Cargo vendoring, deferred out of TASK-129 by the owner ruling on 2026-10-23.

**Problem.** `Manifest.vendor` is a single `Option<Vendor>` carrying one `mode`, `crates`, `size_bytes`, `cargo_lock_sha256` and blob list, and `--no-vendor` / the budget model are written against one payload. A nested Rust workspace with its own `Cargo.lock` therefore cannot be carried by a transport that already vendors the root project: the second lockfile has no place to live, and two vendors would race on `.pixi-sandbox/vendor`.

TASK-129 deliberately refuses such a declaration with a named diagnostic instead of half-implementing it, so this task carries the real work: mirror `envs` with a per-workspace vendor map (identity, lock hash, mode, size, blobs), teach restore to materialise each tree into the workspace that owns it, keep `--no-vendor` and the budget ceilings meaningful across several trees, and decide whether ADR-6 needs amendment rather than extension. It also has to answer what a shared crate between two lockfiles costs in branch size (git dedups identical blobs, but checkout and verification do not).

**Non-goals.** No change to how the root project vendors; no non-Cargo ecosystem vendoring (TASK-130); no change to ADR-32 branch topology.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Seams agreed before implementation: the manifest shape for several vendor trees, the restored-tree layout per workspace, and the public test boundaries (fixture projects and tempdirs, never this repository — ADR-10). Recorded in the slice plan `backlog/docs/plans/task-131-per-workspace-vendor.md`; `workspace_vendors[]` mirrors `workspaces[]`, trees restore to each owning workspace.
- [x] #2 A transport can carry one vendor tree per declared workspace, each with its own lock hash, mode, size and verified blob list; a root-only transport serialises exactly as today. Schema 5 (`workspace_vendors[]`), additive; schemas 1–4 still read; root-only bytes unchanged apart from the schema number.
- [x] #3 Restore materialises each tree into the workspace that owns it, verifies every file before writing (invariant 1), and keeps --no-vendor, --cargo-config and the existing Cargo wiring semantics working. Per-tree staging, verification and `cargo-home`/activation-hook wiring; fixture tests cover all four `--cargo-config` modes and `--no-vendor`.
- [x] #4 Budget and provenance stay honest across trees: the transport ceiling accounts for all vendor bytes, the plan reports each tree's size, and a shared crate between two lockfiles is counted and justified rather than silently duplicated. `WorkspacePlan.vendor_crates`, `vendor_overlap` analysis, pack/README overlap naming; no dedup, restore budget doubles every tree.
- [x] #5 ADR-6 is amended, superseded or explicitly extended — the outcome is recorded as a decision rather than left implied by the code. ADR-33 extends ADR-6, which remains accepted; recorded in the ADR README subsequent-records list.
- [x] #6 A fixture with two lockfiles packs, restores and builds both workspaces offline; the full suite's pass count rises or holds with fmt, lint and test green. Ignored `tests/vendor_offline.rs` gate (`test-vendor-offline` in CI): real `itoa`/`memchr` crates, both projects build `cargo build --offline` inside `unshare -n`, cold negative control fails; suite holds above the 1,277 baseline.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Focused fixture tests, the offline build proof and the repository baseline (recorded before and after) are green; unproven tiers are named rather than implied. Hermetic fixture tests run in the ordinary suite; the `unshare -n`/crates.io proof is an ignored CI-only gate (its restore-side mechanics were additionally proven locally with real crates); the three network-gated ignores remain the only unproven tier.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Agreement on the manifest shape and restored layout. 2. Manifest and writer support for several trees, with a root-only transport unchanged. 3. Restore plus verification per tree, keeping the current wiring flags. 4. Budget/provenance accounting and plan reporting. 5. ADR decision, documentation and full gates.
<!-- SECTION:PLAN:END -->
