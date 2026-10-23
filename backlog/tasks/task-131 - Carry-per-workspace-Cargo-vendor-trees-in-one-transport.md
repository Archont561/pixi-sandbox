---
id: TASK-131
title: Carry per-workspace Cargo vendor trees in one transport
status: To Do
assignee: []
created_date: '2026-10-23 18:44'
labels:
  - vendor
  - cargo
  - pack
  - restore
  - transport
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
- [ ] #1 Seams agreed before implementation: the manifest shape for several vendor trees, the restored-tree layout per workspace, and the public test boundaries (fixture projects and tempdirs, never this repository — ADR-10).
- [ ] #2 A transport can carry one vendor tree per declared workspace, each with its own lock hash, mode, size and verified blob list; a root-only transport serialises exactly as today.
- [ ] #3 Restore materialises each tree into the workspace that owns it, verifies every file before writing (invariant 1), and keeps --no-vendor, --cargo-config and the existing Cargo wiring semantics working.
- [ ] #4 Budget and provenance stay honest across trees: the transport ceiling accounts for all vendor bytes, the plan reports each tree's size, and a shared crate between two lockfiles is counted and justified rather than silently duplicated.
- [ ] #5 ADR-6 is amended, superseded or explicitly extended — the outcome is recorded as a decision rather than left implied by the code.
- [ ] #6 A fixture with two lockfiles packs, restores and builds both workspaces offline; the full suite's pass count rises or holds with fmt, lint and test green.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Focused fixture tests, the offline build proof and the repository baseline (recorded before and after) are green; unproven tiers are named rather than implied.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Agreement on the manifest shape and restored layout. 2. Manifest and writer support for several trees, with a root-only transport unchanged. 3. Restore plus verification per tree, keeping the current wiring flags. 4. Budget/provenance accounting and plan reporting. 5. ADR decision, documentation and full gates.
<!-- SECTION:PLAN:END -->
