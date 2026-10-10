---
id: TASK-88
title: Stop recording uv-regenerated install metadata in the per-file oracle
status: In Progress
assignee: []
created_date: '2026-10-10 14:20'
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

The cause is D13's per-file oracle meeting a package kind it does not model. `pixi-pack`
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
- [x] #7 D13's honest-limits record in `.knowledge/decisions.md` names the new exemption, why presence is the sound check for it, and the consumer-facing consequence: **an already-packed transport still fails until it is re-packed**, because its recorded digest was computed over bytes the restore path regenerates by design.
- [ ] #8 The fix is exercised against a real `pixi-unpack` reinstall of a pypi wheel. Recorded as evidence; if it cannot be produced on this machine, the AC stays open with the missing proof named rather than satisfied by the fixture.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 pixi run --frozen fmt
- [x] #2 pixi run --frozen lint
- [x] #3 pixi run --frozen test
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#1–#7 are implemented and locally proven; AC#8 is the one this machine cannot close and is
carried deliberately.

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

**AC#8 is NOT met and is carried.** The connected proof needs a real `pixi-pack` →
`pixi-unpack` round trip of a wheel. This machine cannot produce one: `pixi lock` fails
against `https://prefix.dev/conda-forge/linux-64/repodata_shards.msgpack.zst` with
`tls handshake eof`, and an airlocked consumer cannot relock at all. The fixture is honest
about uv's file layout and the seam is the real `verify_restored`, but a fixture is not a
wheel. The proof should be run against a re-packed `qgis-rust` transport — the repository in
the report — once a release carrying this fix exists.

**Left undone, deliberately.** Issue #128's third suggestion — registering user tools even when
the final tree check fails — is **not** implemented here. It is a separate, deliberate
question: it changes when `restore` mutates `$PATH`, which is a different invariant from the
oracle's soundness, and it deserves its own task and decision rather than riding along with
this one. Carried in `CONTEXT.md` § Session scratchpad.

Suite: 928 → 935 passing / 1 skipped. `pixi run --frozen lint` 11/11,
`xtask check-repository` clean, `pixi run --frozen fmt` clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
D13's per-file oracle now models the one package kind it was blind to. uv's install
metadata — `*.dist-info/uv_cache.json`, and the row of `*.dist-info/RECORD` that hashes it —
is a function of when and where the install ran, so it is presence-checked and row-collapsed
rather than content-digested, through one canonical form that pack and verify share. Every
other file in a `.dist-info`, and every file outside one, keeps its full digest. An already
packed transport still fails until it is re-packed; that is a property of the digests it
recorded, not of this code, and it is recorded rather than hidden.
<!-- SECTION:SUMMARY:END -->