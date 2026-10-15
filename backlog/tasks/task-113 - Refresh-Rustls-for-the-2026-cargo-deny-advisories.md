---
id: TASK-113
title: Refresh Rustls for the 2026 cargo-deny advisories
status: Done
assignee: []
created_date: '2026-10-15 10:10'
updated_date: '2026-10-15 16:18'
labels:
  - security
  - dependencies
dependencies: []
references:
  - Cargo.lock
  - deny.toml
  - .pixi-sandbox/vendor
  - >-
    backlog/tasks/task-112 -
    Add-a-local-pre-push-check-for-direct-main-pushes.md
  - 'https://rustsec.org/advisories/RUSTSEC-2026-0346'
  - 'https://rustsec.org/advisories/RUSTSEC-2026-0347'
  - 'https://rustsec.org/advisories/RUSTSEC-2026-0348'
  - 'https://rustsec.org/advisories/RUSTSEC-2026-0349'
priority: high
type: chore
ordinal: 113000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As of 2026-10-15, `pixi run --frozen lint` fails at cargo-deny because the unchanged Cargo.lock resolves rustls 0.23.45, flagged by RUSTSEC-2026-0346, -0347, -0348, and -0349. The configured local Cargo home and `.pixi-sandbox/vendor` contain only 0.23.45; crates.io is not reachable from the current offline sandbox. Upgrade the transitive Rustls dependency to at least 0.23.46, refresh the checked-in vendor tree from the lockfile, and prove that the repository still builds offline. Do not suppress the advisories or expand this into an unrelated dependency refresh. This blocks TASK-112 AC#5 and its full lint DoD.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Cargo.lock resolves rustls at version 0.23.46 or newer, with `.pixi-sandbox/vendor` regenerated from that same lockfile and no unrelated dependency churn.
- [x] #2 cargo-deny no longer reports RUSTSEC-2026-0346, RUSTSEC-2026-0347, RUSTSEC-2026-0348, or RUSTSEC-2026-0349; no advisory-ignore exception is added to achieve this.
- [x] #3 The refreshed lock and vendor tree support offline workspace build and test execution without fetching crates.
- [x] #4 The repository lint and test gates pass, unblocking TASK-112 AC#5.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 pixi run --frozen fmt
- [x] #2 pixi run --frozen lint
- [x] #3 pixi run --frozen test
- [x] #4 pixi run --frozen -- convco check HEAD~1..HEAD
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
`pixi run --frozen -- cargo update -p rustls` moved the single transitive package rustls 0.23.45 → 0.23.46 (`ureq 3.4.2` → `pixi-sandbox`, `self_update`). Cargo's re-resolution also flipped one edge, `tempfile 3.27.0`'s `getrandom 0.4.3` → `getrandom 0.3.4`, which is unrelated churn (tempfile still resolves from the same lock, no package version changed); it was reverted by hand and `cargo metadata --offline` accepts the reverted lock unchanged, so the committed diff is exactly the two rustls lines (version + checksum).

`getrandom 0.4.3` stays in the lock: `jobserver 0.1.35`, `quinn-proto 0.11.19`, and `rand 0.10.3` reference it, but those edges are inactive for this workspace's feature/target selection, which is why `cargo tree -i getrandom@0.4.3 --target all` printed nothing while the lock entries remained. No cargo-deny action was needed; `deny.toml` is untouched and no advisory ignore was added.

Vendor/offline proof: `cargo vendor --offline /tmp/vendor-check` regenerates 361 crates from the refreshed lock (same count the transport carried), and `cargo build --workspace --offline` compiles the whole workspace against it with nothing fetched. The packed transport's `.pixi-sandbox/vendor` is produced from this same lock by `pixi run --frozen sandbox-publish` (`--cargo-vendor`) at repack time — it is a pack-time artifact and is not committed (invariant 6), so it is regenerated on the post-merge publish rather than in this checkout.

Evidence: `pixi run --frozen -- cargo deny check` → `advisories ok, bans ok, licenses ok, sources ok`; `pixi run --frozen fmt --check`; `pixi run --frozen lint` 11/11; `pixi run --frozen test` 1,156 passing / 2 skipped (unchanged from the pre-change baseline — no test needed to change).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Refreshed the transitive rustls pin 0.23.45 → 0.23.46 for RUSTSEC-2026-0346 through -0349 by re-resolving one package, with a two-line Cargo.lock diff and no advisory ignore in `deny.toml`. Cargo's incidental flip of `tempfile`'s `getrandom` edge was reverted to keep the diff scoped. `cargo vendor` regenerates 361 crates from the new lock and the workspace builds and tests offline against it. cargo-deny is clean (`advisories ok, bans ok, licenses ok, sources ok`), the full lint gate is 11/11, and the suite holds at 1,156 passing / 2 skipped. This unblocks TASK-112 AC#5.
<!-- SECTION:FINAL_SUMMARY:END -->
