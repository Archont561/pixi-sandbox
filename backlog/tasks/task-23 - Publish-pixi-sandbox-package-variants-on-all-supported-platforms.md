---
id: TASK-23
title: Publish pixi-sandbox package variants on all supported platforms
status: Done
assignee:
  - '@me'
created_date: '2026-10-01 09:49'
updated_date: '2026-10-01 14:34'
labels:
  - release
  - platform
  - packaging
milestone: m-0
dependencies: []
references:
  - .knowledge/rust-bootstrap.md
documentation:
  - backlog/docs/plans/v1-platform-workflow-transport/doc-1
priority: high
type: enhancement
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Build and publish Pixi/Conda package variants for linux-64, linux-aarch64, osx-64, osx-arm64, and win-64 using native runners; retain standalone Rust release assets for transport bootstrap.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All five package variants are produced from the same release version
- [x] #2 Each package installs and runs pixi-sandbox on its native runner
- [x] #3 Standalone binaries retain checksum verification
- [x] #4 Missing platform artifacts fail the release job
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Widen crates/pixi-sandbox/pixi.toml (the pixi-build package manifest) to all five platforms and add a platform-independent package task there, so release builds no longer need the root default env. Build + smoke-test the .conda on each native runner in the release matrix, upload per-platform artifacts, and make the release job fail unless all five .conda are present.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented on 2026-10-01. Design: packaging no longer goes through the root default environment, which cannot resolve on win-64 (conda-forge has no bun) and is not a precondition for producing a package anyway.

Changes: (1) crates/pixi-sandbox/pixi.toml — the standalone pixi-build manifest — now lists linux-64, linux-aarch64, osx-64, osx-arm64, win-64; its [workspace] platforms is deliberately wider than the root list. (2) Root pixi.toml gains an empty `package` environment; `pixi run -e package package` needs no solve and therefore runs on every native runner, and package-smoke installs the artifact with `pixi global install --path` and runs it. (3) release.yml builds + smoke-tests one .conda per native runner in the existing build matrix and uploads it as conda-<platform>; the release job downloads those artifacts UNMERGED (all five share one filename) and calls scripts/check-conda-platforms.sh, which fails the release if any platform is missing or produced more than one package. (4) scripts/lint-repo-consistency.sh now prunes .pixi/target/node_modules/vendor before scanning crates/ (the pixi-build backend leaves a vendored registry under crates/pixi-sandbox/.pixi/bld).

Verified here on linux-64: `pixi run -e package package` built .publish/out/pixi-sandbox-0.3.5-ha35fb5c_0.conda; `package-smoke` installed it and it reported 0.3.5; check-conda-platforms.sh fails on a missing platform and on a duplicate, passes on all five; `pixi run lint` (minus cargo-deny, which cannot clone the RustSec advisory DB in this sandbox) and `pixi run test` (143 passed) are green; actionlint and taplo accept the new manifest and workflow. Note the packaged binary is a normal pixi-build cargo build (dynamic on Linux); it is a connected-machine artifact and is NOT the static standalone bootstrap binary the transport keeps.

Release proof: auto-release run 36875656660 cut v0.3.7 from 910df8f and dispatched release run 36875815588. All five native matrix jobs built and smoke-tested the 0.3.7 package on linux-64, linux-aarch64, osx-64, osx-arm64, and win-64. The publish job enforced one package per platform, generated SHA256SUMS, published all packages to prefix.dev with `--generate-attestation`, and created the non-draft GitHub Release. The release contains five standalone binaries, five `.conda` packages, SHA256SUMS, and install.sh; the successful prefix.dev upload step generated the package attestation.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
Release v0.3.7 proves the complete five-platform package pipeline: every native package built, installed, and ran on its target runner, all five were published to the canonical prefix.dev channel with attestation, and the GitHub Release retained the verified standalone transport assets.
<!-- SECTION:SUMMARY:END -->
