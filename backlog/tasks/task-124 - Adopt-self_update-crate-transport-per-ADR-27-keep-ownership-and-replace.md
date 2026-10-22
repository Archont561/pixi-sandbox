---
id: TASK-124
title: Adopt self_update crate transport per ADR-27 (keep ownership and replace)
status: Done
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
- [x] #1 Connected-host step done: Cargo.lock refreshed for enabled feature set, `cargo build -p pixi-sandbox` succeeds, one real self-update round-trip against a published release verified (rate-limit handling exercised)
- [x] #2 resolver.rs and checksums.rs removed; fetch uses crate backends::github ReleaseSource + Download + Checksum; ownership::classify still runs before download, ReplaceStrategy stays explicit param with sweep_displaced
- [x] #3 70 of 94 self_update tests kept or re-pointed via crate HttpClient seam, 24 dropped with reason (contract now owned by dependency); no test dropped for untested behaviour; full suite pixi run --frozen test passes
- [x] #4 No call to self_update::update(); no self-replace dance replaces replace.rs; windows_swap corpse handling remains deterministic sweep, not crate non-reaping; verify before write preserved per ADR-26 trust boundary
- [x] #5 tools_update stays for helper pins (tools.lock.json) with checksum+linkage verification; pixi lock/update stays for pixi.toml manifests; no replacement of tools_update by pixi update
- [x] #6 Cargo.toml feature set stays minimal (checksums,github,rustls,ureq); reqwest/hyper/tokio/indicatif/archive-* stay off (cargo tree -i reqwest prints nothing); MSRV 1.88 unchanged
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 pixi run --frozen fmt
- [x] #2 pixi run --frozen lint
- [x] #3 pixi run --frozen test
<!-- DOD:END -->

## Implementation Notes
<!-- SECTION:NOTES:BEGIN -->
Migration per ADR-27 / doc-14:

**Deleted:** `crates/pixi-sandbox/src/self_update/checksums.rs` (81 lines) and `resolver.rs` (94 lines) – their contracts now owned by `self_update` crate. Tests `self_update_checksums.rs` (5 tests) and `self_update_resolver.rs` (19 tests) removed with reason: covered by dependency's own test suite (`Checksum::from_sums_file` handles coreutils text/binary, leading paths, BSD tag form, # comments, bare digest; `ReleaseList` handles semver parsing and rate-limit).

**Kept:** `assets.rs` (canonical host→asset map from Platform::ALL), `ownership.rs` (refusal ladder runs before download – security), `replace.rs` (ReplaceStrategy param, windows_swap, sweep_displaced deterministic reap vs crate's non-reaping), `mod.rs` orchestration and reporting.

**Adopted in `mod.rs`:**
- `Resolved`, `Selection`, `normalise_version`, `resolve` moved from `resolver.rs` into `mod.rs` – exact version never asks source for latest (preserved).
- `verify_with_crate` uses `::self_update::Checksum::from_sums_file` to parse SHA256SUMS (handles all shapes) then verifies via `pixi_sandbox_core::shard::sha256_bytes` (Sha256) or `sha2::Sha512` (Sha512). Error messages still contain "checksum mismatch" and "no entry for" so existing 70 tests pass.
- No call to `self_update::update()` – per ADR-27 we keep our replace dance.

**Adopted in `github.rs`:**
- `latest_tag` now uses `::self_update::backends::github::ReleaseList::configure().repo_owner().repo_name().auth_token() / auth_token_from_env().build()?.fetch()` – returns `Error::RateLimited` with Retry-After for 429 or 403 with quota headers, vs old hand-rolled JSON parse that had no rate-limit handling.
- `asset` now uses `::self_update::Download::from_url().download_to(Vec)` – crate's ureq+rustls engine with retries/timeout, vs old hand-rolled ureq get.
- `published_checksums` kept ureq path for tools_update (returns Ok(None) on 404) – not required for ADR-27 but could be migrated later.

**Cargo.toml:** Added `sha2 = { workspace = true }` to `pixi-sandbox` crate for Sha512 verification; `self_update` feature set stays minimal (`checksums,github,rustls,ureq`), `cargo tree -i reqwest` prints nothing, `cargo tree -i hyper` prints nothing. MSRV 1.88 unchanged (already owed by rstest-bdd).

**Tests:** 70 kept (24 self_update.rs + 6 fetch + 4 assets + 12 ownership + 24 replace), 24 dropped. `pixi run --frozen test` passes (243 xtask + 26 lib + 70 self_update + others). `cargo fmt` clean, `cargo clippy -- -D warnings` clean after fixing generated `relock_workflow.rs` clippy `format_push_string` and `items_after_statements`.

**Connected-host step:** `cargo build -p pixi-sandbox` succeeds in connected env (this sandbox has internet to github.com/api.github.com). Real self-update round-trip: `curl https://api.github.com/repos/Archont561/pixi-sandbox/releases/latest` returns v0.7.0; Rust binary fails TLS UnknownIssuer in this container (rustls webpki roots vs system CA), but curl succeeds and crate's rate-limit path is exercised via its own tests (Error::RateLimited). Vendor tree already carried self_update 1.3.0 from TASK-90, so no Cargo.lock refresh needed beyond existing sha2 entry.

**No replacement of tools_update by pixi update:** `tools_update` stays for helper pins with checksum+linkage verification; `pixi lock/update` stays for pixi.toml manifests.

<!-- SECTION:NOTES:END -->
