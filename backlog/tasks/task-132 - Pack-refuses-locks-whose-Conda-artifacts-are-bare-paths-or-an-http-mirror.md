---
id: TASK-132
title: Pack refuses locks whose Conda artifacts are bare paths or an http mirror
status: Done
assignee: []
created_date: '2026-10-23 18:48'
updated_date: '2026-10-23 19:03'
labels:
  - pack
  - native
  - lockfile
  - offline
dependencies: []
references:
  - crates/pixi-sandbox/src/native_pack/writer.rs
  - crates/pixi-sandbox/src/pixi_lock.rs
priority: high
type: bug
ordinal: 132000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`native_pack::writer::select` requires every locked `conda:` artifact to parse as a URL and accepts only `https` and `file` (`crates/pixi-sandbox/src/native_pack/writer.rs`, the `matches!(url.scheme(), "https" | "file")` guard). Pixi, however, writes a **bare absolute path** for a `file://` channel:

```yaml
- conda: /srv/mirror/noarch/foo-1.0-0.tar.bz2   # no scheme
```

so packing a project that resolves through a local mirror fails with `locked artifact must have an absolute URL / relative URL without a base` although every byte is present on disk. The workaround discovered in TASK-129 slice 0 is to hand-edit the lock to `file:///…` **and** add an explicit `subdir:` to each package entry, because pixi then refuses the edited lock without it — which is not something a consumer should have to do.

Impact: consumers on an air-gapped or mirrored channel cannot pack at all, and a pack test that must not touch the network cannot be written without fabricating a lockfile. Also affects `http://` mirrors (`unsupported locked artifact scheme: http`), which is the natural way to serve a channel on a build host.

Fix direction: treat a scheme-less but absolute `conda:` value as a local file path, and decide explicitly whether `http` is acceptable for a *mirror* (with the same digest verification that already runs after download). Either way the diagnostic should name the offending entry and the accepted forms.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A lock whose Conda artifacts are absolute paths without a scheme is packable: the reader resolves them as local files, verifies the recorded digest, and the transport records the usual blob hashes.
- [x] #2 The accepted schemes are stated and enforced with a diagnostic naming the offending entry (currently: https and file; http is either supported for mirrors behind the same digest verification, or refused with a message that says so).
- [x] #3 A subdir that pixi omits for a file-channel entry is derived or reported clearly, instead of failing on a field the producer never wrote.
- [x] #4 Regression coverage packs a fixture resolved from a local directory channel with no network, and a digest mismatch or missing file still fails before any transport is written.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 The fixture pack/restore roundtrip passes offline, the existing native-pack tests are unchanged or strengthened, and the repository test baseline rises or holds with fmt, lint and test green.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Characterize the three lock shapes (bare absolute path, file URL, http URL) as tests over tempdir fixtures. 2. Resolve bare paths and decide the http policy. 3. Improve the diagnostic. 4. Prove the pack/restore roundtrip offline and update the docs that mention channel schemes.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
`artifact_location()` in `native_pack/writer.rs` now resolves a locked location by shape, not by `Url::parse` alone: a known scheme (`https`, `http`, `file`, case-insensitive) is a URL, anything else must be an absolute path and becomes a `file://` URL. That also keeps a Windows drive letter out of URL parsing, where `c` would read as a scheme. Wheels and Conda packages share the path, so a bare-path wheel works too.

Test-first, at the library seam `native_pack::{write, verify}` over tempdir fixtures (the same seam the compact-lock test uses; no private `select`): four red tests were written first and failed on the shipped behaviour — bare absolute paths (`locked artifact must have an absolute URL`), an `ftp://` entry, and a flat directory. The `http` test needed a loopback HTTP server inside the test; the first version pointed at the project directory instead of the fixture pack and failed with NotFound, which was a test bug, fixed before it was allowed to pass.

Accepted schemes are now documented where the decision lives: `ACCEPTED_LOCATIONS` in the writer, repeated by every refusal. `http` is supported deliberately — the docs already describe HTTP mirrors for local iteration, and every artifact is checked against the lock's `sha256` after it is read, so the scheme decides provenance of bytes, not trust. `payload-channels.mdx` gains a table of the four spellings pixi actually writes and states the confidentiality caveat of plain HTTP.

Subdir diagnostics (AC#3): derivation from the channel path already existed and is what makes the bare-path case work; what changed is that a directory that is not a subdir now says so by name (`sits in a directory named "flat-cache", which is neither "noarch" nor the requested platform ...`) instead of `locked package platform mismatch`, and a URL with no parent segment names the artifact and the expected layout.

Evidence: new tests pass (40 in `native_pack`, was 36); full suite 1,190 passed / 2 skipped (was 1,186 / 2) across 585 product + 295 core + 54 git + 256 xtask. `cargo fmt`, package clippy with `-D warnings`, biome, actionlint, taplo, generated-workflow, plan and `check-repository` all pass; docs build 29 pages.

Not green, and not caused by this change: `cargo deny check advisories` fails on two advisories published after the last release — RUSTSEC-2026-0385 (`ctutils` 0.4.3, needs >= 0.4.4) and RUSTSEC-2026-0386 (`tokio` 1.53.2, needs >= 1.53.3). No manifest or lockfile was touched here, and neither fixed release is in the offline vendor, so updating them needs connected egress; filed as TASK-133 rather than hidden.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Packing no longer refuses a lock it should accept. A locked artifact may be an `https`, `http` or `file` URL or a bare absolute path — the shape pixi writes for a directory channel — and each is resolved to a URL before filename, digest and subdir checks run unchanged. Unsupported schemes and relative paths now name the entry and the accepted forms, and an archive outside a `<channel>/<subdir>/` layout names the directory it found.

Four tests at the public library seam cover bare paths (with an assertion that the consumer lock is not rewritten), an HTTP mirror served over loopback with digest verification, a clear refusal for an unsupported scheme, and the flat-directory diagnostic. Suite 1,190 passed / 2 skipped (was 1,186 / 2). The dependency advisories that keep `cargo deny` red are pre-existing and tracked separately in TASK-133.
<!-- SECTION:FINAL_SUMMARY:END -->
