---
id: ADR-30
title: Generated CI bootstraps directly from verified standalone releases
status: accepted
date: '2026-10-22'
tags:
  - decision
---

# ADR-30 — Generated CI bootstraps directly from verified standalone releases

## Context

TASK-126 follows RFC #179 and TASK-118–122. The consumer publisher installed the conda CLI
only to download a second, standalone CLI through `fetch-release`. TASK-118 recorded 10s
for the package install in a 31s publish job, and 2s in the consumer plan job. These are
historical wall-clock observations, not measurements of this implementation or billing savings.
The conda binary is not the Linux musl binary required in an offline transport (ADR-4).

## Decision

The generated plan, publish and upgrade jobs share one pre-CLI Bash bootstrap. It downloads
the exact workflow-pinned release asset and `SHA256SUMS` from `release::PIXI_SANDBOX_REPO`
over HTTPS using curl. Runner OS **and architecture** select an asset; the renderer derives
asset spellings from `Platform::asset_name`, including musl on both Linux architectures.
Unknown runners fail before a download. There is no package-manager resolve or execution
of an unverified binary.

The bootstrap stages into a fresh temporary directory, selects the exact manifest filename
(not a substring), requires exactly one 64-hex-digit digest, and checks the staged binary
with `sha256sum --check` or macOS's `shasum -a 256 --check`. A missing/malformed/duplicate
entry, failed download, or checksum mismatch fails before installation and PATH registration.
Only the verified bytes are moved to the private `pixi-sandbox-bin` directory. The staging
directory is removed on exit. The trust boundary remains the HTTPS release and its checksum
manifest: this does not add an independently signed checksum or supply-chain attestation.

This is the narrowly reviewed exception to TASK-76's no-embedded-policy rule. A single Rust
renderer emits the identical Bash 3.2-compatible body in all three jobs; each block has a
`multiline-run-allowed` justification. Everything after bootstrap remains a CLI verb.
`fetch-release` remains available to other callers; the canonical conda channel remains the
interactive installation route. Repository check 5 distinguishes these two surfaces.

Keep the matrix dynamic: the standalone planner reads the current config on every run.
The plan job needs neither setup-pixi nor conda. Publish and upgrade retain setup-pixi for
subsequent environment operations. This repository's deliberately source-built publisher
continues to build the checked-out source; replacing it with a release would weaken dogfooding.
Static matrix rendering is deferred because changing a bundle without regenerating the workflow
must not silently publish an old matrix.

## CI filtering and concurrency

Do **not** path-filter `ci.yml`'s workflow trigger: required checks otherwise remain Pending,
and every main push must retain ADR-29's guard. Instead a SHA-pinned paths-filter action uses
the PR files API and excludes only `docs/**`, `backlog/**`, `CONTEXT.md`, and `CHANGELOG.md`
from Rust steps. Everything else, including manifests, lockfile edits/deletions, workflow
changes and mixed PRs, runs full CI. Filter failure fails CI, and an absent filter output
runs the full checks. The repository variable `PIXI_SANDBOX_FULL_CI=true` disables filtering.
Docs-only PRs still lint docs and build the site; their Windows Rust job is skipped. Pushes
and manual runs execute full CI, and docs-site artifact upload is push-only.

TASK-120 already implemented repository-wide publish concurrency, `cancel-in-progress: false`.
Retain it: never cancel an active pack/force-push. GitHub can still discard superseded pending
runs; this is not a FIFO queue or a promise every intermediate commit publishes. This updates
ADR-18's historical statement that concurrency is only explicit-field opt-in. Relock path
filtering from TASK-121 is retained, including manifests **and lockfiles**.

## Evidence and limits

Execution tests run the rendered bootstrap with fake downloads across all five runner mappings,
including corrupt bytes, missing/duplicate/malformed entries, substring-only filenames, a failed
download, an unsupported runner, CRLF manifests, and the shasum fallback. Removing the checksum
command makes the corrupt-byte test fail. Golden rendering and actionlint cover the YAML.
The consumer-proof assertion now checks the actual checksum command instead of the retired
`fetch-release` invocation.

A faster dynamic plan still consumes a VM and may still round to the same billed minute.
No automatic one-minute saving is claimed. TASK-126 stays In Progress until native consumer
runs, the complete online consumer lifecycle, and before/after timing evidence for docs-only
PRs, manifest PRs and consecutive merges are recorded. No release, starter dispatch, or
concurrency experiment is implied by this implementation.
