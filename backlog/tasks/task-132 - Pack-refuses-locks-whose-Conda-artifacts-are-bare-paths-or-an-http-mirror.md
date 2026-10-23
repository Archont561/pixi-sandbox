---
id: TASK-132
title: Pack refuses locks whose Conda artifacts are bare paths or an http mirror
status: To Do
assignee: []
created_date: '2026-10-23 18:48'
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
- [ ] #1 A lock whose Conda artifacts are absolute paths without a scheme is packable: the reader resolves them as local files, verifies the recorded digest, and the transport records the usual blob hashes.
- [ ] #2 The accepted schemes are stated and enforced with a diagnostic naming the offending entry (currently: https and file; http is either supported for mirrors behind the same digest verification, or refused with a message that says so).
- [ ] #3 A subdir that pixi omits for a file-channel entry is derived or reported clearly, instead of failing on a field the producer never wrote.
- [ ] #4 Regression coverage packs a fixture resolved from a local directory channel with no network, and a digest mismatch or missing file still fails before any transport is written.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 The fixture pack/restore roundtrip passes offline, the existing native-pack tests are unchanged or strengthened, and the repository test baseline rises or holds with fmt, lint and test green.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Characterize the three lock shapes (bare absolute path, file URL, http URL) as tests over tempdir fixtures. 2. Resolve bare paths and decide the http policy. 3. Improve the diagnostic. 4. Prove the pack/restore roundtrip offline and update the docs that mention channel schemes.
<!-- SECTION:PLAN:END -->
