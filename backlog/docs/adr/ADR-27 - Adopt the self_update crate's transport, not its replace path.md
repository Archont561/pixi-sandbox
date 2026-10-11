---
id: ADR-27
title: Adopt the self_update crate's transport, not its replace path
date: '2026-10-11'
status: accepted
tags:
  - decision
---
## Context

`pixi-sandbox self-update` (ADR-26) is hand-rolled: `resolver`, `assets`, `checksums`,
`ownership`, `replace`, and an orchestration in `mod.rs` (889 lines) judged by 94 integration
tests. The `self_update` crate (1.3.0) can do the download → verify → replace engine and was
declared ahead of the verdict so the airlock's relocked vendor tree would carry it. TASK-90
evaluated it against ours; the measurements and the per-file test mapping are in
`backlog/docs/spikes/self-update/doc-14 - Self-Update-Crate-Evaluation.md`.

Two properties are load-bearing and are ours, not the crate's: the refusal ladder
(`ownership::classify`) runs **before a download byte moves**, and `ReplaceStrategy` is an
**explicit parameter** so both the Unix and Windows dances are exercised on any machine.

## Decision

**Adopt the crate's release-lookup and transport half; keep ours for ownership and replacement.**

1. **Adopt.** The GitHub release lookup (`backends::github`, `ReleaseSource`), the download
   engine (`Download`) and the `SHA256SUMS` parser (`Checksum`). This removes `resolver.rs` and
   `checksums.rs` and the download internals of `fetch`, and buys the one capability we lack —
   GitHub rate-limit handling (`Error::RateLimited` with `Retry-After`, clamped) — from a
   shipped, tested client instead of our own.
2. **Keep.** `ownership.rs` (the refusal ladder, called before the crate sees a destination),
   `replace.rs` (`ReplaceStrategy`, `windows_swap`, `sweep_displaced`), and `mod.rs`'s
   plan/check/run orchestration and reporting. The crate's `update()` is **not** called: it owns
   both the replace dance (via `self-replace`, a `cfg!` read with no strategy seam) and the
   ordering of the ownership decision relative to the download.

The full adoption — calling `update()` — is rejected because it would surrender both
properties: the verify hooks run after the artifact is on disk, and `self-replace` is not
parameterisable. The crate's non-reaping of stale Windows corpses (it explicitly declines
startup cleanup) is a second reason to keep our deterministic `sweep_displaced`.

## Consequences

- `Cargo.toml`'s `self_update` line **stays**. Feature set is the measured minimum
  (`checksums`, `github`, `rustls`, `ureq`); `reqwest`, `hyper`, `tokio`, `indicatif` and the
  archive/signature features stay off and are not built, though `Cargo.lock` still records
  the optional `reqwest` tree.
- Crates gained: `self-replace` (+`fastrand`, Windows `windows-sys`) and `urlencoding`.
- **MSRV 1.88** is required to build the dependency, and is already owed by `rstest-bdd 0.6.0`,
  so adoption adds no incremental cost. Lifting `rust-version` and clearing the six
  MSRV-gated `collapsible_if` lints is **TASK-92**.
- 70 of the 94 tests are kept or re-pointed at the crate's `ReleaseSource`/`HttpClient` seams;
  24 (5 `checksums`, 19 `resolver`) move their contract onto the dependency, which owns and
  tests it. No test is dropped for a behaviour that goes untested.
- Adoption cannot be built or tested in the airlock (crates.io is refused there). The migration
  task must start on a connected host: refresh `Cargo.lock`, run one real `cargo build`, and
  exercise one real `self-update` round trip against a published release before the airlock
  receives the relocked vendor tree.
- **TASK-91** (unify the two GitHub adapters) should be revisited after the migration, since the
  crate brings its own GitHub backend.

Accepted 2026-10-11 on TASK-90's evidence (doc-14), measured on a connected host; the migration
itself is a separate task and no code was changed here.
