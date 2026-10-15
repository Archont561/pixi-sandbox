---
id: TASK-88
title: Stop recording uv-regenerated install metadata in the per-file oracle
status: Done
assignee: []
created_date: '2026-10-10 14:20'
updated_date: '2026-10-15 16:45'
labels:
  - bug
  - verification
  - pypi
  - consumer
dependencies: []
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/128'
  - crates/pixi-sandbox-core/src/files_manifest.rs
  - crates/pixi-sandbox-core/src/verify.rs
priority: high
type: bug
ordinal: 88000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`pixi-sandbox restore` exits non-zero on its final restored-tree check for **every** project
whose packed environment contains a pypi dependency, and the failure is deterministic: no
consumer action can make it pass. Because `restore` exits non-zero it also skips user-tool
registration, so the documented next command then fails with `pixi: command not found` even
though every environment unpacked correctly. Reported against `Archont561/qgis-rust` with an
18-line failure output (issue #128).

The cause is ADR-13's per-file oracle meeting a package kind it does not model. `pixi-pack`
transports a pypi dependency as the **wheel**, not as installed bytes, so `pixi-unpack`
*reinstalls* it with uv at restore time. Two files come back as a function of when and where
that install ran:

- `*.dist-info/uv_cache.json` — install metadata whose first field is a wall-clock timestamp
  (`{"timestamp":{"secs_since_epoch":1791622638,...}}`)
- `*.dist-info/RECORD` — the package's own file list, whose `uv_cache.json` row carries that
  file's hash and size, so it changes whenever the timestamp does

The oracle records content digests for both, so pack-time bytes and restore-time bytes can
never agree. Two files per package is the observed shape; only the 9 of 62 `dist-info`
directories uv routed through its cache carry a `uv_cache.json`, and the 53 conda-installed
ones verify clean, which is why a project without pypi dependencies never sees this.

This repository cannot reproduce its own bug: `pixi.toml` resolves no pypi package, so its
own transport has no `dist-info` at all and restores with 0 failures across 5520 entries.
The fix therefore needs a fixture that is honest about what uv writes rather than a
transport to lean on.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [x] #1 `scan_prefix` records `*.dist-info/uv_cache.json` as presence-only — still listed, so a missing or forged install stays caught, but carrying no content digest, because no consumer could reproduce the bytes a digest would name.
- [x] #2 A `*.dist-info/RECORD` is digested with its `uv_cache.json` row collapsed to a sentinel, so a reinstall that changes only that row verifies clean while every other row keeps full content verification; a `RECORD` differing anywhere else is still an integrity failure.
- [x] #3 Pack and verify compute that canonical form through one shared entry point rather than two implementations, so the two sides cannot drift.
- [x] #4 `uv_cache.json` may appear in a restored prefix without being listed, because uv only writes it for installs it routes through its cache; a forged conda-meta record is still an unlisted-file failure, and a `uv_cache.json` outside a `.dist-info` directory keeps its content digest.
- [x] #5 A regression test reproduces issue #128 at the `verify_restored` seam: a prefix scanned at pack time, a wheel reinstalled with a different timestamp, and a clean verdict. Its complement — tampering with the package payload the wheel shipped — is still caught, so the exemption is not wider than the two files.
- [x] #6 No manifest or `files.json` schema change: a list written by an older tool still parses and still verifies under the digests it recorded, and a list written by this one is still readable by an older binary.
- [x] #7 ADR-13's honest-limits record in `backlog/docs/adr/README.md` names the new exemption, why presence is the sound check for it, and the consumer-facing consequence: **an already-packed transport still fails until it is re-packed**, because its recorded digest was computed over bytes the restore path regenerates by design.
- [x] #8 The fix is exercised against a real `pixi-unpack` reinstall of a pypi wheel. Recorded as evidence; if it cannot be produced on this machine, the AC stays open with the missing proof named rather than satisfied by the fixture.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 pixi run --frozen fmt
- [x] #2 pixi run --frozen lint
- [x] #3 pixi run --frozen test
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#1–#7 are implemented and locally proven; AC#8 was attempted against a real round trip on a
connected host and the fix was found incomplete there — see the AC#8 note at the end, and
**TASK-93**. It is carried deliberately.

**The shape.** `files_manifest::canonical_file_sha256(rel, bytes, candidates)` is the new
shared entry point. It is path-aware in exactly one way: a `*.dist-info/RECORD` has its
`uv_cache.json` row collapsed to `@pixi-sandbox-regenerated@` before hashing. Both call sites
use it — `scan_prefix` (pack) and `check_restored_entry` (verify) — which is what makes AC#3
true by construction rather than by review. The byte-level `canonicalise` /
`canonical_sha256` pair is unchanged and still used for symlink targets, where the path rule
does not apply.

`is_uv_install_metadata` requires the `.dist-info/` directory component. A file that merely
shares the name elsewhere in the prefix is transported content and keeps its digest, which
`uv_install_metadata_outside_a_dist_info_is_ordinary_content` holds.

**Why `RECORD` is normalised rather than exempted.** Only one row of a `RECORD` is
regenerated. Making the whole file presence-only would drop the content check on every
installed file of every pypi package — the largest single loss of coverage the oracle has.
`a_changed_record_row_still_changes_the_digest` is the guard against that, and it fails if the
rule is widened to the whole file.

**Compatibility (AC#6).** No schema change and no new field. Presence-only is carried by the
*absence* of `h` in a `FileEntry`, which is how `conda-meta` records already work, so an
older binary reading a list this one writes treats the entry as presence-only and passes. In
the other direction a list an older tool wrote still records a digest for `uv_cache.json`, and
this binary still compares it — which is why AC#7's re-pack consequence is stated plainly
instead of being papered over with a lenient comparison that would rescue `uv_cache.json` and
still fail `RECORD`, which is not a coherent behaviour to ship.

**Tests (AC#5).** Five new tests in `crates/pixi-sandbox-core/tests/files_manifest.rs` and two
in `crates/pixi-sandbox-core/tests/verify.rs`. Suite 928/1 → **935/1** (core 212 → 219).
Both regression tests were confirmed to **fail with the fix neutralised** — the two rules
were reverted to `false` and `a_regenerated_uv_cache_row_does_not_change_a_record_digest` and
`a_uv_reinstalled_pypi_environment_matches_the_oracle` both failed — so they test the defect
rather than restating the implementation.

The `restored::world()` fixture now carries a uv-installed wheel, so every pre-existing
restored-tree test exercises a prefix with pypi content rather than a conda-only one.

**AC#8 is NOT met. Attempted on a connected host (2026-10-11) and it FAILS — the fix does
not hold on real tooling.** The connected proof was produced: a throwaway project
(`python 3.12.*` + pypi `six`, a pure-python wheel) was `init` → `pack` → `restore`d with the
**released v0.6.1 static-musl binary** (sha256 `905ec63f9192bcd836f55c4ff4b37879a2e8454a68b94c7761a4cbc771088c6c`, verified). `restore` exits non-zero with
`the restored project does not match the manifest (1 failure(s))` on
`lib/python3.12/site-packages/six-1.17.0.dist-info/RECORD`. So the earlier "this machine cannot
produce one" no longer describes a connected host — and the real round trip is exactly what
exposes the gap below.

**The gap: uv writes a third regenerated file the fix does not model —
`*.dist-info/direct_url.json`.** Reconstructing the pack-side verification unpack and diffing
its `RECORD` against the restored one leaves **exactly two** rows out:
`direct_url.json` and `uv_cache.json`. The `uv_cache.json` row is the one the fix collapses;
the `direct_url.json` row is not, and it differs because uv records the *local wheel path*
(`file://…/pypi/six-1.17.0-py2.py3-none-any.whl`, 111 bytes at pack-verify vs 134 at restore) and
`RECORD` hashes that file. The file's own entry verifies (the path is canonicalised away) but its
`RECORD` row carries the hash of the un-canonicalised bytes, so the digest can never agree across
sides. `direct_url.json` appears for **every** wheel `pixi-unpack` installs (it always installs
from a local wheel), so this is not `six`-specific: issue #128 is **not** fixed for any packed
transport containing a pypi wheel.

**Scope check.** `grep -rn direct_url crates/pixi-sandbox-core/src` is empty, and
`git diff v0.6.1 origin/main -- crates/pixi-sandbox-core/src/files_manifest.rs` is **empty** —
main is byte-identical to the release, so the defect is present on current `main` too. AC#8 is
carried open; the follow-up is filed as **TASK-93**, which proposes treating
`*.dist-info/direct_url.json` exactly as `uv_cache.json` is treated (presence-only in
`scan_prefix`, row-collapsed in a `RECORD`) and is the change that would actually close this AC.

The fixture remains honest about uv's file layout, but it only models `uv_cache.json` + the
`RECORD` row that hashes it. A fixture is not a wheel: it now also needs `direct_url.json` in the
shape uv really writes, which is precisely what this AC exists to catch.

**Left undone, deliberately.** Issue #128's third suggestion — registering user tools even when
the final tree check fails — is **not** implemented here. It is a separate, deliberate
question: it changes when `restore` mutates `$PATH`, which is a different invariant from the
oracle's soundness, and it deserves its own task and decision rather than riding along with
this one. Carried in `CONTEXT.md` § Session scratchpad.

Suite: 928 → 935 passing / 1 skipped. `pixi run --frozen lint` 11/11,
`xtask check-repository` clean, `pixi run --frozen fmt` clean.

### 2026-10-13 — the local gap is closed; AC#8 is now only the connected proof

The third regenerated file identified above, `*.dist-info/direct_url.json`, is modelled on `main`:
`files_manifest::UV_INSTALL_METADATA` is now `["uv_cache.json", "direct_url.json"]`, so both are
presence-only in `scan_prefix` and both matching `RECORD` rows are collapsed through the shared
`canonical_file_sha256` entry point. This task's own fixture was updated with it —
`uv_installed_dist_info` (`crates/pixi-sandbox-core/tests/verify.rs`) writes `direct_url.json`
verbatim with its `RECORD` row, and `a_uv_reinstalled_pypi_environment_matches_the_oracle` passes
with a restore-time wheel path that differs from the pack-time one. The local model therefore now
reproduces the connected observation (exactly two regenerated `RECORD` rows, `direct_url.json` and
`uv_cache.json`). That work landed as **TASK-93** (AC#1–#3, on main), so the earlier note that
TASK-93 "would actually close this AC" describes history, not open work.

**AC#8 itself stays open, unchanged, and for one reason only: it names a real `pixi-unpack`
reinstall.** This airlocked host has no `pixi-pack` on `PATH`, no pypi wheel on disk, no `curl`,
and cannot resolve against `prefix.dev`/`pypi.org`, so the real round trip cannot be produced here.
The missing proof is the same one TASK-93 AC#4 records, on the same connected host. Nothing local
remains for this AC.

### 2026-10-15 — AC#8 satisfied on a connected host (real pack → unpack of a pypi wheel)

The round trip this AC names was produced end to end on a connected container
(`pixi 0.81.0`, linux-64). Consumer project `/tmp/pypi-proof`: `pixi.toml` with
`python = "3.12.*"` (conda-forge) and pypi `six = ">=1.16"`; `pixi install` resolved
`six 1.17.0`. The `pixi install` prefix carries **no** `uv_cache.json`/`direct_url.json` — those
are written only by `pixi-unpack`'s uv reinstall, which is the whole bug shape.

Packed with the fixed local build:
`pixi-sandbox pack --repo-root /tmp/pypi-proof --envs default --output-dir /tmp/pypi-transport --platform linux-64 --fetch-tools --tools-cache /tmp/tools-cache`.
`--fetch-tools` pulled `pixi-pack 0.7.11` and `pixi-unpack 0.7.11` (x86_64-unknown-linux-musl)
from the embedded pins on demand; result `default: 27 files · 42.9 MiB packed · 153.2 MiB unpacked · 6416 file entries recorded`.

The recorded oracle (`/tmp/pypi-transport/.pixi-sandbox/envs/default/files.json`) shows the fix's
shape exactly: `six-1.17.0.dist-info/direct_url.json` and `.../uv_cache.json` are **presence-only**
(no `"h"`), while `.../six-1.17.0.dist-info/RECORD` carries the collapsed digest
`dddd17af044c6c96bc70854fa99d93418a91dccce44836d487fc600167c2dcf5`.

Restore, real `pixi-unpack 0.7.11` reinstall:
`pixi-sandbox restore --branch-location /tmp/pypi-transport --output-path /tmp/pypi-restored --user-tools skip`
→ `env default: checked against the manifest's file list — 6416 entry(ies), 0 failure(s)` /
`OK — the restored tree matches the manifest`, exit 0. The restored
`direct_url.json` names the restore-time wheel path
`file:///tmp/pypi-restored/.pixi/.restore-work/pack-default/pypi/six-1.17.0-py2.py3-none-any.whl`,
which cannot equal the pack-time path, and its `RECORD` row hashes those un-canonicalised bytes —
the deterministic mismatch #128 describes.

Red→green on the same transport proved the fix is load-bearing. The pre-fix released binary
`v0.6.1` (asset `pixi-sandbox-x86_64-unknown-linux-musl`, downloaded from the GitHub release)
restored the same transport into a fresh prefix and **failed**, exit 1:
`lib/python3.12/site-packages/six-1.17.0.dist-info/RECORD: integrity: env default: content does not match the manifest's file list (expected dddd17af…, got 8993270e…)` /
`Error: the restored project does not match the manifest (1 failure(s))`. The released `v0.6.2`
(tagged 2026-10-11 18:18, after the fix `8a17a26f` landed at 15:57) and the local main build both
restore it with 0 failures, exit 0. So the fix is both proven on a real reinstall and already in a
release; the `expected` digest in the v0.6.1 failure is the collapsed one the fixed packer wrote.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
ADR-13's per-file oracle now models the one package kind it was blind to. uv's install
metadata — `*.dist-info/uv_cache.json`, and the row of `*.dist-info/RECORD` that hashes it —
is a function of when and where the install ran, so it is presence-checked and row-collapsed
rather than content-digested, through one canonical form that pack and verify share. Every
other file in a `.dist-info`, and every file outside one, keeps its full digest. An already
packed transport still fails until it is re-packed; that is a property of the digests it
recorded, not of this code, and it is recorded rather than hidden.
<!-- SECTION:SUMMARY:END -->