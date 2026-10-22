---
id: TASK-128
title: Extract shared staged-prefix promotion from command wiring
status: Done
assignee: []
created_date: '2026-10-22 15:39'
updated_date: '2026-10-22 15:46'
labels:
  - refactor
  - rust
  - restore
  - unpack
  - audit
dependencies: []
references:
  - crates/pixi-sandbox/src/commands/unpack.rs
  - crates/pixi-sandbox/src/commands/restore.rs
  - crates/pixi-sandbox/tests/native_pack.rs
documentation:
  - >-
    backlog/docs/adr/ADR-31 - Native format-1 transports replace helper
    acquisition.md
priority: medium
type: task
ordinal: 128000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rust code-quality audit: behavior-preserving refactor, not a bug fix or a new restore guarantee.

Evidence: crates/pixi-sandbox/src/commands/unpack.rs:292-340 owns destination refusal plus same-parent backup, promotion, rollback, and backup retention after rollback failure. Both unpack.rs:119 and commands/restore.rs:627 use it, with restore reaching through the sibling unpack command module. lib.rs does not expose this shared concern. AGENTS.md requires commands to be thin ambient-input wiring with reusable product logic in the library. Rule: core-separation-concerns.

Concrete cost: changing shared replacement safety requires editing a command-specific binary-private module; recovery branches cannot be characterized directly through a public library boundary without an entire install lifecycle. Existing native_pack integration tests cover corrupt input and incorrect staged oracles, but do not substitute for direct characterization of promotion failure and rollback.

Bounded remedy: move the existing destination check and prefix-promotion operation into one small library-owned module, expose only the agreed testable boundary, and have both commands delegate. Retain early unpack destination refusal and the promotion-time recheck. Keep raw-unpack oracle/fingerprint policy and restore marker policy at their existing call sites.

Non-goals: no generic filesystem transaction framework, all-environment transaction, vendor/tool/self-update migration, filesystem-policy change, diagnostics cleanup, native package redesign, or TASK-125 completion claim. Related historical precedent: completed TASK-82; TASK-125 retains its own native-artifact proof obligations. TASK-126 and TASK-127 workflow optimization are unrelated.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Before writing tests, agree with the user on public test seams: propose a minimal library promotion API over real tempdir paths plus existing black-box restore/unpack CLI boundaries; do not test private helpers or mock filesystem internals.
- [x] #2 Characterize current destination and promotion behavior at the agreed seams before extraction: absent and empty destinations, nonempty refusal without force, successful forced replacement, and deterministic failed promotion restoring previous contents. Assert independently specified content and error observations rather than implementation call order.
- [x] #3 Move destination checking and staged-prefix promotion into one library-owned concern and delegate from both commands; retain early unpack refusal and the promotion-time recheck, with no sibling-command import for this shared policy.
- [x] #4 Preserve current symlink treatment, force behavior, exact diagnostic contexts, same-parent backup placement and naming, rollback attempt, and retained backup plus recovery path when rollback fails; do not move verification or Pixi marker decisions into the helper.
- [x] #5 Keep characterization and extraction in separate stages; make small behavior-preserving changes and run focused agreed-seam tests after each. For any actual new behavior discovered, stop and scope it separately with a failing behavior test first.
- [x] #6 Run relevant pixi-sandbox package tests and lint, then the repository test baseline without silently reducing it. Record commands, test counts, cached versus executed results, and exact environmental or platform blockers; do not claim unrun rollback-failure or native-platform checks passed.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Focused public-seam characterization and existing native pack/restore/unpack integration tests pass, with baseline comparison recorded.
- [x] #2 Formatting check and relevant package lint pass; no source or test changes outside the bounded refactor without separate authorization.
- [x] #3 Task stays open until implementation and evidence exist; no change to TASK-125 native-artifact completion status.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Approved seams: existing restore/unpack CLI plus public library promotion over real temporary directories. First characterize existing CLI replacement behavior; then extract the unchanged filesystem operation, extend public API characterization for failure/rollback, and run focused and package gates. Keep changes separate from TASK127.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-22: user approved both proposed public test boundaries. Baseline 1154 passed, 2 skipped; low-disk build flags required and must pass through Turbo.

User-approved seams implemented. Before extraction, four CLI destination characterization cases passed against the original code. Extracted check_output/promote_prefix unchanged into public prefix module; early refusal and promotion recheck remain, and restore no longer imports its sibling command. Eight real-filesystem library cases cover allowed/refused destinations, failed-rename rollback, missing-stage diagnostics, and Unix live/dangling symlinks. Native/prefix focused gate: 38 passed. cargo fmt --all and scoped cargo clippy -p pixi-sandbox --all-targets --offline -- -D warnings passed. Full uncached suite with Turbo loose env and debug/incremental disabled: 1166 passed, 2 skipped (baseline 1154/2). No behavioral red is claimed for a structural extraction: characterization passed before and after. Double-failure backup-retention branch copied unchanged; it was not fault-injected or demonstrated on Windows. TASK125 native artifact proof remains unrun.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Shared staged-prefix replacement policy now belongs to public prefix library module, used by restore and unpack. Existing diagnostics, rechecks, same-parent backup, rollback and retained recovery path are preserved. Added twelve characterization cases and passed focused tests, package clippy and the uncached workspace baseline (1166 passed, 2 skipped).
<!-- SECTION:FINAL_SUMMARY:END -->
