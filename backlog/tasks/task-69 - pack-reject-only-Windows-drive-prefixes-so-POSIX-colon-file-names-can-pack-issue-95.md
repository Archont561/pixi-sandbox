---
id: TASK-69
title: >-
  pack: reject only Windows drive prefixes so POSIX colon file names can pack
  (issue 95)
status: In Progress
assignee:
  - '@agent'
created_date: '2026-10-04 15:29'
updated_date: '2026-10-04 15:36'
labels:
  - bug
  - pack
  - files-manifest
dependencies: []
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/95'
  - crates/pixi-sandbox-core/src/manifest.rs
  - crates/pixi-sandbox-core/src/files_manifest.rs
  - crates/pixi-sandbox-core/tests/manifest.rs
priority: high
type: bug
ordinal: 69000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Issue 95: check_rel_path (crates/pixi-sandbox-core/src/manifest.rs) rejects any path containing a colon, but a colon is a legal byte in POSIX file names and real conda environments carry them — perl ships man/man3/App::Cpan.3 style man pages, so an environment that resolves perl (the conda gtk/webkit stack) can never pack. Reject only an actual Windows drive prefix (C:/x, C:\x, and the drive-relative C:x) instead of any colon, and word the pack-time scan failure for the file on disk instead of for the manifest that does not exist yet.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 check_rel_path rejects a Windows drive prefix (C:/x, C:\x, drive-relative C:x, any single ASCII letter followed by a colon) and absolute paths, and accepts POSIX-legal colon-containing relative paths such as man/man3/App::Cpan.3 — for every label it guards (files manifest entry, blob path, part path, pack_path, tool path)
- [x] #2 The incident reproduces and is fixed at the public seams: scanning a staged prefix that carries perl-style Package::Name.3 man pages produces a valid per-file oracle, and the restored side (FilesDoc parse plus verify_restored) accepts those entries in a faithful round-trip
- [x] #3 A pack-time validation failure is worded for the scan, not for the manifest: the error names the scanned environment file (the diagnosis note in issue 95)
- [x] #4 The manifest wire format and SCHEMA_VERSION are unchanged — the fix relaxes validation only, and the forward-compatibility note (a transport carrying colon-named entries needs a fixed binary to restore; each transport embeds its own) is recorded in the task notes
- [x] #5 fmt, lint (clippy, deny, actionlint, repo-consistency) and the full test suite pass, with the count rising from 619 passing / 1 skipped
- [ ] #6 Consumer proof after release: castellan pushes pack their shells environment green again and its transport repacks — recorded here and on issue 95 (needs the next pixi-sandbox release; the AC stays open until then)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Test-first at three public seams: FilesDoc parse/scan_prefix (the incident), Manifest validate (drive prefixes refused for every label), verify_restored round-trip via the verify.rs World fixture. Then the check_rel_path fix + scan-time wording. Consumer release proof stays open (AC#6).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-04 — implemented test-first at the public seams. Red: a scanned perl-style prefix (man/man3/App::Cpan.3) failed scan_prefix with the incident error, and the pack-time rejection said "files manifest entry" while no manifest existed. Green: check_rel_path now rejects only a Windows drive prefix (any ASCII letter followed by a colon, covering C:/x, C:\x and the drive-relative C:x) plus the existing absolute and parent-dir refusals — a bare colon anywhere else is a legal POSIX byte. FilesDoc gained a labelled validate so the pack-time scan words its rejection "scanned environment file must be relative" while parse keeps "files manifest entry".

Tests: new tests/files_manifest.rs freezes the incident (the scan records and FilesDoc::parse accepts the man page; C:/x, C:\x, C:x, /x and \x are refused; the scan-time wording asserted), tests/manifest.rs freezes every label check_rel_path guards (blob path, part path, pack_path, tool path — drive-prefixed and absolute), and the verify.rs World fixture now carries the man page so the faithful-restore test proves the restore-side oracle round-trips colon names. Suite 623 passing / 1 skipped (was 619/1); pixi run --frozen lint green (fmt, clippy, deny, actionlint, taplo, biome, sandbox-plan, check-repository).

AC#4 note — no schema change: SCHEMA_VERSION and the wire format are untouched, the fix relaxes validation only. Forward compatibility: a files.json carrying colon-named entries parses only on binaries with this fix; every transport embeds its own pixi-sandbox (self-bin), so a packed branch restores self-consistently, but a stale standalone binary doctoring such a transport fails with the old message — that is the release dependency AC#6 records.

AC#6 stays open: it needs the next pixi-sandbox release, then a castellan push whose shells environment packs green and repacks the transport — to be recorded here and on issue #95.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Pack no longer rejects POSIX colon file names: check_rel_path refuses only a Windows drive prefix (plus absolute and escaping paths), so an environment that resolves perl — the conda gtk/webkit stack — packs again. The pack-time scan failure now names the scanned environment file instead of the manifest that does not exist yet. Five criteria proven locally (623 passing / 1 skipped, all lint gates green); the consumer proof (AC#6) waits on the next release and castellan repacking its transport.
<!-- SECTION:FINAL_SUMMARY:END -->
