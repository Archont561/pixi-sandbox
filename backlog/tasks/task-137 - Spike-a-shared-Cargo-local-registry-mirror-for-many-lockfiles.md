---
id: TASK-137
title: 'Spike: a shared Cargo local-registry mirror for many lockfiles'
status: In Progress
assignee: []
created_date: '2026-10-25 17:40'
updated_date: '2026-10-25 17:40'
labels:
  - spike
  - cargo
  - vendor
  - offline
milestone: m-2
dependencies: []
references:
  - crates/pixi-sandbox/src/mirror.rs
  - crates/pixi-sandbox/tests/mirror_offline.rs
  - crates/pixi-sandbox/src/vendor.rs
documentation:
  - >-
    backlog/docs/adr/ADR-34 - Spike - one Cargo local-registry mirror for many
    lockfiles.md
  - CONTEXT.md
priority: medium
type: spike
ordinal: 137000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Timeboxed evidence spike for the owner proposal recorded in `CONTEXT.md` (2026-10-25): replace
the per-lockfile `cargo vendor` trees of ADR-6/ADR-33 with one read-only Cargo **local
registry** that any number of `Cargo.lock` files resolve against through source replacement.
The long-term proposal spans many language ecosystems; this task answers only the Cargo
question, behind an explicitly experimental module with no CLI, manifest-schema, pack or
restore surface. Existing bundles stay byte-identical when the spike is unused.

This is the evidence m-2 says ADR-33 needs before any amendment: the milestone keeps the Cargo
vendor contract in force "unless the spike proves an amendment". It is distinct from
TASK-130, which spikes Go/NuGet/Maven providers inside the existing transport model; the
adapter seam validated here (closure input, static layout, offline client config, lock-based
verification oracle) is what later ecosystems would implement.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [x] #1 A spike-only library module builds a Cargo local-registry layout (`.crate` files plus length-sharded JSON-line index) from the union of two or more `Cargo.lock` files, copying each `(name, version)` once and refusing any `.crate` whose SHA-256 disagrees with the lock checksum. No CLI verb, config schema field, pack or restore call site references it.
- [x] #2 Source-replacement `config.toml` is generated (`[source.crates-io] replace-with`, `local-registry` path), and unit tests cover index sharding, lock parsing (path members skipped, non-registry sources refused), two-lock dedup, index dependency rendering and checksum-mismatch refusal without network access.
- [ ] #3 An ignored real-runner gate (`test-mirror-offline`, changes-gated CI step in the same `sudo unshare -n` shape as `test-vendor-offline`) shows two projects with distinct lockfiles and one shared crate both executing `cargo build --offline --locked` successfully against the single mirror with egress denied, and their binaries proving the registry crates linked. Run id recorded here once the PR gate is green: _to fill from CI_.
- [ ] #4 The same gate includes a cold negative control (mirror crate removed, fresh CARGO_HOME and target, egress denied) that fails the build, plus a printed measurement block comparing mirror bytes/counts against the two ADR-33 `cargo vendor` trees for the same locks; figures recorded in the task notes, never estimated.
- [ ] #5 ADR-34 is published with `status: experimental`, the falsification criteria, the local-registry-over-proxy rationale (invariant 6, no custom CA, no daemon) and the open questions a promotion must answer (transport topology, retention, offline lock refresh, non-registry sources, real-graph parity).
<!-- AC:END -->

## Definition of Done

<!-- DOD:BEGIN -->
- [x] #1 Full ordinary suite and doc tests stay green; the new code is pedantic-clippy clean and rustfmt-clean (runner-side; no local toolchain in the dev sandbox).
- [x] #2 `cargo deny` stays green — `flate2` is already in the lock transitively and carries an accepted permissive licence; `Cargo.lock` itself is unchanged.
- [ ] #3 The `test-mirror-offline` gate is green on the pull request's real Linux runner, with its run id and measurement block transcribed into this file, before merge.
- [ ] #4 No production transport, release, dispatch or branch-protection change is performed; the spike is removable by deleting one module, one test, one pixi task and one CI step.
<!-- DOD:END -->

## Notes

- Spike scope deliberately excludes transport packaging: the gate writes the mirror to a
  scratch directory rather than packing it, so ADR-32 branch topology and the manifest schema
  are untouched. Integration is open question 1 in ADR-34.
- The connected side still needs egress exactly as today (`generate-lockfile` + `fetch`); the
  spike moves no solving offline and makes no billing or minute-saving claim.
- Promotion is a separate, reviewed decision: an accepted successor ADR must answer the five
  open questions before any product wiring exists.
