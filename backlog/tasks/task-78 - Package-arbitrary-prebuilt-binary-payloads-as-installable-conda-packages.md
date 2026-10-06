---
id: TASK-78
title: Package arbitrary prebuilt binary payloads as installable conda packages
status: To Do
assignee: []
created_date: '2026-10-06 21:01'
labels:
  - packaging
  - conda
  - channels
  - airlock
  - consumer
dependencies: []
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/115'
  - crates/pixi-sandbox/pixi.toml
  - crates/xtask/src/smoke.rs
priority: high
type: feature
ordinal: 78000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Pixi channels are a good transport into restricted environments, but some dependencies ship only as prebuilt binary trees rather than source packages. Issue #115 names the concrete case: Playwright browser builds, an executable plus a few hundred MB of shared libraries, resources, locales and data files that normally come from a vendor CDN. Where Git and package APIs are reachable but arbitrary downloads are not, the artifact has to be fetched on a connected host and republished through a channel.

Make an arbitrary prebuilt file or directory a first-class package payload: fetch it connected, wrap it into a platform-correct conda package, publish it to a channel, and let an air-gapped project run pixi add and execute it. The feature must be generic; browsers are the example, not the subject.

This repository is already most of the way there in machinery and none of the way there in generality. crates/pixi-sandbox/pixi.toml builds a real conda package for five platforms, publish-conda uploads the set, xtask check-conda-platforms refuses a release where a platform contributed nothing, and xtask smoke-conda-package installs the built package and runs it. What is missing is a payload path: that manifest uses the pixi-build-rust backend, which compiles a crate, and nothing in the repository wraps bytes that were built elsewhere.

Two constraints decide the shape before any code is written. pixi-build is still an upstream preview feature, declared in that manifest as preview = pixi-build, so the backend choice is a risk to name rather than assume. And the vendored tree cannot fetch a new crate without a full relock cycle, so a solution that leans on rattler-build and a generated recipe is cheaper than one that implements an archive writer here.

Keep the boundary against the sandbox transport explicit. The transport carries a whole environment on an orphan branch; this carries one dependency through a channel so it participates in the lockfile and in pixi add. They are not substitutes, and the documentation has to say which to reach for.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A reviewed decision records the implementation shape before code lands: a new pixi-sandbox verb, an xtask subcommand, or a documented rattler-build recipe pattern. It names the upstream preview status of pixi-build, the rattler-build dependency, and confirms no new vendored crate is added without a relock cycle; the decision goes in .knowledge/decisions.md with a D-number.
- [ ] #2 A payload directory or single file becomes a conda package for a named platform subdir such as linux-64 or linux-aarch64, built from inputs already on disk with no network access at build time; a native payload declared as noarch is refused with a named error rather than silently mispackaged.
- [ ] #3 Executable bits, symlinks including relative and dangling ones, nested directories and empty directories survive the round trip, proven by packing a generated fixture tree, installing it into a temporary prefix, and comparing modes and link targets entry by entry.
- [ ] #4 The payload installs at a deterministic prefix-relative location, and an optional entry point can be exposed on PATH as a launcher that resolves the real executable inside the payload rather than a copy of it, so relative runtime lookups inside the payload still work.
- [ ] #5 An air-gapped consumer can run pixi add against a channel carrying the package, install it from the lockfile with no network, and execute the exposed entry point successfully in a clean temporary prefix.
- [ ] #6 Large payloads are handled deliberately: a payload that exceeds the configured blob or transport budget fails with the budget name, the measured size and a remediation, instead of producing a package the channel will reject at upload time.
- [ ] #7 The published package records the provenance of what was wrapped, at minimum the upstream source URL and the SHA256 of the fetched artifact, so an auditor can tell which external bytes entered the channel and verify them independently.
- [ ] #8 No payload byte is committed to Git. Tests build their fixture tree in a tempdir and never point at this checkout or a real HOME per D10, and the documented workflow fetches on a connected host and publishes rather than vendoring the artifact into the repository.
- [ ] #9 Nothing in the implementation is browser-specific or Playwright-specific. Firefox and Playwright appear only as a worked example in documentation and tests; the code paths take a payload, a name, a version and a platform.
- [ ] #10 A user-facing guide documents the connected fetch, build and publish steps and the air-gapped pixi add step, and states the boundary against the sandbox transport: a channel package carries one dependency into the lockfile, the transport branch carries a whole environment, and neither replaces the other.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
<!-- DOD:END -->
