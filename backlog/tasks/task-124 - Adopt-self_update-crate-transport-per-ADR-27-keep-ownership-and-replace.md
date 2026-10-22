---
id: TASK-124
title: Adopt self_update crate transport per ADR-27 (keep ownership and replace)
status: To Do
assignee: []
created_date: '2026-10-22'
updated_date: '2026-10-22'
labels:
  - self-update
  - dependencies
  - refactor
dependencies:
  - TASK-90
  - TASK-91
  - TASK-92
priority: medium
type: feature
ordinal: 124000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Migrate `crates/pixi-sandbox/src/self_update/` to use the `self_update` crate's release-lookup and transport half, keeping our refusal ladder and replace dance per ADR-27 (adopted from doc-14 spike TASK-90).

**Context:**
- ADR-27 decided: adopt crate's GitHub lookup (`backends::github`, `ReleaseSource`), download engine (`Download`) and SHA256SUMS parser (`Checksum`). Keep `ownership.rs` (refusal before download), `replace.rs` (`ReplaceStrategy` param, `windows_swap`, `sweep_displaced`) and `mod.rs` orchestration. Do NOT call `update()` – it owns replace via `self-replace` cfg! and runs verify after artifact on disk.
- `Cargo.toml` already declares `self_update = { version = "1.3", default-features = false, features = ["checksums","github","rustls","ureq"] }` (TASK-90). Minimal set – `reqwest/hyper/tokio/indicatif/archive-*` stay off.
- MSRV 1.88 already owed by `rstest-bdd` and lifted in TASK-92, so no incremental cost.
- Current impl is 889 lines + 94 tests across 7 files; after migration 70 kept/re-pointed, 24 dropped-and-replaced-by-dependency (5 checksums + 19 resolver).
- Airlock cannot fetch crates.io – migration must start on connected host: refresh Cargo.lock, `cargo build -p pixi-sandbox`, one real `self-update` round-trip against published release, then relock vendor tree via normal repack.

**Why not full replacement:**
1. Refusal before download – `ownership::classify` must run before any byte moves (security, cost). Crate verify hooks run after artifact on disk (`update.rs:1607` archive, `:1786` binary).
2. ReplaceStrategy as explicit param – allows Unix/Windows dances tested on any machine (24 tests + `replace-running-image` job). Crate delegates to `self-replace` with cfg! read, not parameterisable, and does NOT sweep stale `.old-*` siblings (`lib.rs:105-115` says not recommended). Ours reaps deterministic `.pixi-sandbox-old-<version>` at start of every update.

**Pixi update deps:**
- `pixi lock` / `pixi update` is for `pixi.toml` → `pixi.lock` (conda). Used in relock guard/bot. Correct for project manifests.
- `tools update` (`src/tools_update.rs`) is for embedded helper pins (`tools.lock.json`: pixi, pixi-pack, pixi-unpack) – GitHub release assets with URL templates, verified via checksums + linkage (`static` per ADR-4). Using `pixi update` for these would replace reviewed, checksum-verified standalone binaries with conda latest, breaking static guarantee.
- So: keep `tools_update` as-is; do not replace with `pixi update`.

**Work:**
- Delete `resolver.rs` and `checksums.rs`, replace fetch internals in `mod.rs`/`fetch_release.rs` with crate's `backends::github` + `Download` + `Checksum`.
- Keep `ownership.rs`, `replace.rs`, `assets.rs`, `mod.rs` orchestration, reporting.
- Re-point 70 tests to crate's `ReleaseSource`/`HttpClient` seams (test double via `.http_client(Arc<dyn HttpClient>)`).
- Ensure `pixi run --frozen test` passes, clippy clean at 1.88.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Connected-host step done: Cargo.lock refreshed for enabled feature set, `cargo build -p pixi-sandbox` succeeds, one real self-update round-trip against a published release verified (rate-limit handling exercised)
- [ ] #2 resolver.rs and checksums.rs removed; fetch uses crate backends::github ReleaseSource + Download + Checksum; ownership::classify still runs before download, ReplaceStrategy stays explicit param with sweep_displaced
- [ ] #3 70 of 94 self_update tests kept or re-pointed via crate HttpClient seam, 24 dropped with reason (contract now owned by dependency); no test dropped for untested behaviour; full suite pixi run --frozen test passes
- [ ] #4 No call to self_update::update(); no self-replace dance replaces replace.rs; windows_swap corpse handling remains deterministic sweep, not crate non-reaping; verify before write preserved per ADR-26 trust boundary
- [ ] #5 tools_update stays for helper pins (tools.lock.json) with checksum+linkage verification; pixi lock/update stays for pixi.toml manifests; no replacement of tools_update by pixi update
- [ ] #6 Cargo.toml feature set stays minimal (checksums,github,rustls,ureq); reqwest/hyper/tokio/indicatif/archive-* stay off (cargo tree -i reqwest prints nothing); MSRV 1.88 unchanged
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
<!-- DOD:END -->
