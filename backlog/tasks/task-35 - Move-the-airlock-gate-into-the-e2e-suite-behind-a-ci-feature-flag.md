---
id: TASK-35
title: Move the airlock gate into the e2e suite behind a ci feature flag
status: To Do
assignee: []
created_date: '2026-10-01 18:40'
labels:
  - ci
  - testing
  - airlock
  - tooling
milestone: m-0
dependencies:
  - TASK-1
  - TASK-10
references:
  - scripts/airlock-gate.sh
  - crates/pixi-sandbox/tests/e2e.rs
  - .github/workflows/airlock.yml
  - crates/pixi-sandbox/Cargo.toml
documentation:
  - backlog/docs/specifications/transport-and-restore/doc-2
priority: medium
type: chore
ordinal: 37000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The offline airlock gate is a 400-line Bash script (`scripts/airlock-gate.sh`) that the CI workflow runs twice per matrix leg, and that one e2e test drives through `bash` just to prove the script still works. That split costs the project twice: the assertions live in a language the rest of the gate logic left behind (design.md §6 and task-27/30/31 moved repository policy into `xtask` for exactly this reason, after a Bash 3.2 bashism killed the v0.3.6 macOS release), and the test that guards them is the slowest thing in a local `pixi run test` while proving nothing a developer needs locally.

Move the gate into the Rust integration suite as `#[cfg(feature = "ci")]` tests in `crates/pixi-sandbox/tests/e2e.rs`, so the default local suite keeps only the fixture-backed airlock proof (fast, hermetic, `unshare -rn` on Linux) and the real-transport gate runs where it belongs: on CI's cross-platform matrix, against a packed-published-restored transport, under denied egress. With the Rust gate running on every matrix leg, `scripts/airlock-gate.sh` has no remaining caller and is deleted — leaving `scripts/restore.sh` as the only shell in the repository.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A `ci` cargo feature on the `pixi-sandbox` crate gates the real-transport airlock tests; `pixi run test` (default features) neither compiles nor runs them, and the local suite still proves the fixture lifecycle including the severed-namespace restore
- [ ] #2 Every assertion the shell gate makes is carried over, each as a named Rust test with its own failure message: bundled tools first on PATH, the bundled static binary executes, each environment prefix is a real installed prefix, `doctor --verify-restored` against the manifest's per-file digests (D13) including the degraded notice for a pre-oracle bundled binary, `pixi install --frozen --offline` is a no-op within the drift budget, and `cargo check --offline` builds against the vendored tree
- [ ] #3 The restored project, the extracted transport, the environment list and the cargo-skip switch reach the tests as explicit inputs (environment variables documented in the test module), and a missing input fails loudly rather than silently passing
- [ ] #4 `.github/workflows/airlock.yml` runs the gate tests on both tiers of every matrix leg, with the test binary built before egress is denied (Tier A must not need a compiler with the network down — e.g. `cargo nextest archive` / `--no-run`, run under `unshare -n` on Linux and `sandbox-exec` on macOS)
- [ ] #5 `scripts/airlock-gate.sh` is deleted along with its references in `AGENTS.md`, `README.md`, `.github/workflows/airlock.yml` paths filters and the `tests/fixtures.rs` reach-outside-the-crate allowance; `check-repository` passes with `scripts/restore.sh` as the only remaining shell script
- [ ] #6 The forged-conda-meta regression that task-10 added (`the_airlock_gate_rejects_a_forged_conda_meta_record`) survives the move as a test that still fails when the gate stops rejecting a fabricated record, and runs in CI
- [ ] #7 The session skill and the airlock documentation state the local-vs-CI split: fixture proof locally, real-transport egress-denied gate on the matrix
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add `[features] ci = []` to `crates/pixi-sandbox/Cargo.toml` and a `pixi.toml` task (`test-ci`) that runs the suite with it enabled, so the workflow step stays one line.
2. Port the gate body into a `mod gate` in `tests/e2e.rs`, one `#[test]` per numbered assertion, reading `PIXI_SANDBOX_GATE_PROJECT`, `PIXI_SANDBOX_GATE_TRANSPORT`, `PIXI_SANDBOX_GATE_ENVS`, `PIXI_SANDBOX_GATE_SKIP_CARGO`. Reuse `pixi-sandbox-core::verify` rather than shelling out to `doctor` where the same code is reachable in-process; keep the subprocess form where the point is that the *bundled* binary runs.
3. Keep the no-op measurement honest: the second `pixi install` is the one measured, the file list is compared exactly, and the KiB drift budget stays a named constant with the comment explaining why it exists.
4. Rewire `airlock.yml`: build/archive the test binary in the normal (networked) part of the job, then run Tier B directly and Tier A inside the egress-denied wrapper. Drop the two `bash project/scripts/airlock-gate.sh` steps.
5. Delete the script, the fixture allowance and every reference; run `pixi run lint-repo-consistency` and `pixi run lint-actions`.
6. Update `docs/` (airlock page) and `.agents/skills/session/SKILL.md` to the final state.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
<!-- SECTION:SUMMARY:END -->
