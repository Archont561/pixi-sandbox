---
id: TASK-133
title: Update ctutils and tokio for the two advisories that redden cargo deny
status: Done
assignee: []
created_date: '2026-10-23 19:03'
labels:
  - dependencies
  - security
  - ci
dependencies: []
references:
  - Cargo.lock
  - deny.toml
documentation:
  - >-
    backlog/docs/adr/ADR-31 - Native format-1 transports replace helper
    acquisition.md
priority: high
type: bug
ordinal: 133000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Two advisories published after the v0.8.0 release make `cargo deny check advisories` fail on an otherwise unchanged tree, which means the next `ci` run on main goes red at the lint step.

- **RUSTSEC-2026-0385** (vulnerability): `ctutils` 0.4.3 generates non-constant-time masks for the small integer widths; fixed in 0.4.4 (`cargo update -p ctutils`).
- **RUSTSEC-2026-0386** (unsoundness): `tokio` 1.53.2 `AsyncFd::new` and related constructors violate I/O safety; fixed in >=1.53.3, <1.52.0 is excluded so 1.53.3 is the target (`cargo update -p tokio`).

Both are transitive (neither is a direct dependency in this workspace). Neither fixed release is in the restored offline vendor — the vendor carries `ctutils-0.4.3` and `tokio-1.53.2` — so the update needs connected egress to fetch and re-vendor, or a CI runner with crates.io access. Do not add advisory exceptions: both have fixed releases, unlike the narrowly approved unmaintained-advisory exception already in \`deny.toml\`.

Found during TASK-132, where the failure was observed but deliberately not fixed: that task changes neither manifests nor the lockfile, and hiding an unrelated advisory behind an unrelated change is exactly how a stale claim enters the record.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ctutils resolves to >= 0.4.4 and tokio to >= 1.53.3 in Cargo.lock, with the new sources genuinely vendored (checksums from the crates.io index, no fabricated vendor data).
- [x] #2 cargo deny check advisories/bans/licenses/sources passes with no new advisory exception; existing approved exceptions are unchanged and re-justified only if they still apply.
- [x] #3 The workspace still builds offline from the vendor tree and every suite passes without a reduced baseline; any crate whose source cannot be vendored is reported instead of being forced.
- [x] #4 The dependency change is one focused commit with the advisory ids in its body, so the release notes state why the lockfile moved.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 fmt, lint and test green on the updated graph, with the local versus connected boundary stated explicitly if the fix can only be proven on CI.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Bump the two crates and fetch sources (needs connected egress or a runner). 2. Re-vendor and verify the manifest hashes. 3. Run deny, clippy, the full suite and an offline build from the vendor tree. 4. Commit with the advisory ids.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The bump could not be produced from this machine: crates.io is outside the airlock (`index.crates.io` and `static.crates.io` both fail to connect) and the restored vendor carried only `ctutils-0.4.3` and `tokio-1.53.2`. A temporary workflow, `deps-provision.yml`, ran on a connected runner instead: `cargo update -p ctutils@0.4.3 --precise 0.4.4`, `cargo update -p tokio@1.53.2 --precise 1.53.3`, then `cargo deny check advisories`, `pixi run lint` and `pixi run test` against the result, then committed `Cargo.lock` back to the branch. Run 40012769426 is green on all three, and the same `ci` gate that was red on main passes on PR #197 (merge `2db0c937`). The workflow file was never part of the merged change; the branch carries one commit, `8a3d35b5`.

The lock moved only the two crates: `ctutils` 0.4.3 → 0.4.4 (`12218662…`) and `tokio` 1.53.2 → 1.53.3 (`ce3335fa…`), both registry checksums. Six unrelated crates (`errno`, `quinn-udp`, `rustix`, `rustls-platform-verifier`, `tempfile`, `winapi-util`) had their `windows-sys` edge rewritten `0.61.2` → `0.52.0` by the same re-resolution; both versions remain in the lock, so no package entered or left the graph, and `cargo` accepts the file without rewriting it (`--locked` succeeds).

AC#1's "genuinely vendored" and AC#3 could only be checked after the merge, because the re-vendor is the transport `publish sandbox` run that `Cargo.lock` in `push_paths` triggers on main. That run (40015243764's push, transport `1486dae2` → `244614af`) repacked with 590 crates, and a fresh `scripts/restore.sh` verified 29,954 blobs with 0 failures. Post-restore evidence, all from the new vendor tree: `ctutils-0.4.4` and `tokio-1.53.3` are present and their `.cargo-checksum.json` `package` fields equal the lock checksums byte for byte; `cargo metadata --locked --offline` resolves; `cargo deny --manifest-path Cargo.toml check` reports `advisories ok, bans ok, licenses ok, sources ok` with `deny.toml` untouched; `pixi run --frozen lint` passes 7/7 including `check-repository`; `pixi run --frozen test` passes 1,190 / 2 skipped (585 product + 295 core + 54 git + 256 xtask), exactly the pre-change baseline, so nothing was reduced to make the graph build.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
`ctutils` is 0.4.4 and `tokio` is 1.53.3 in `Cargo.lock`, closing RUSTSEC-2026-0385 and RUSTSEC-2026-0386. The two crates were bumped on a connected runner because neither fixed release existed in the offline vendor, and the sources were then genuinely re-vendored by the transport repack on main: the restored `ctutils-0.4.4` and `tokio-1.53.3` carry checksums identical to the lock's registry checksums. `cargo deny check` is green on all four checks with no advisory exception added, and the full suite still reports 1,190 passed / 2 skipped offline from the vendor tree. The pre-existing `deny.toml` exception is unchanged. Landed as PR #197 (`2db0c937`); `ci`, `publish sandbox` and `docs` are all green on main afterwards.
<!-- SECTION:FINAL_SUMMARY:END -->
