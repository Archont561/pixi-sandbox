---
id: decision-4
title: Consumer binaries self-update only to propose reviewed regenerated files, and production pins never float
date: '2026-10-03'
status: accepted
---
## Context

Every consumer of `pixi-sandbox init` commits files whose behavior is frozen at the version
that rendered them: `publish-sandbox.yml` pins `PIXI_SANDBOX_VERSION` and embeds that
release's whole template, `relock.yml` stamps pixi from that release's embedded tools lock,
and the launcher is that release's bootstrap. When pixi-sandbox itself releases, the
question is how a consumer's published sandbox comes to contain the new version and use the
new workflow and new configuration. Options considered: (a) float the workflow to
latest/`@v0` so new releases are picked up implicitly; (b) keep the upgrade purely manual
and docs-only (the status quo — prose says "regenerate with `pixi-sandbox init`", and
nothing detects drift); (c) regenerate behind a drift gate and a reviewed pull request.

## Decision

**(c), with the pins staying exact.** Four version domains, four verbs:

1. **The standalone binary self-updates only in the upgrade lane.** `pixi-sandbox self-update`
   resolves latest by default or an exact requested version, maps the host to the canonical
   standalone asset, verifies the release's `SHA256SUMS`, and atomically replaces a managed
   standalone path. It refuses package-manager/global-trampoline ownership. Generated CI starts
   from its exact current standalone binary in runner scratch, then self-updates that disposable
   binary; production publishing never asks for latest.
2. **Generated files regenerate.** A version stamp beside the ownership marker makes them
   self-describing; `pixi-sandbox init --check` is the drift gate (render fresh, compare,
   exit non-zero, write nothing). The scheduled/manual upgrade job runs the updated binary and,
   on drift, regenerates the marker-carrying files and opens a PR — the task-39 relock-bot
   precedent applied to the publisher. The upgrade unit is "self-update, then re-run `init`",
   never a hand-edited `PIXI_SANDBOX_VERSION`, because the template changes with the version.
3. **Config migrates explicitly.** `pixi-sandbox.toml` is reviewed data ("the gate is the
   review, not the edit"); `init` never rewrites an existing config. An older `schema` is
   reported as a finding naming its migration path; a `config migrate` command is built
   only once a schema bump gives it something to do; readers accept older config schemas
   for a deprecation window, mirroring the manifest policy ("a published branch outlives
   the binary that packed it").
4. **The transport repacks on merge.** Regeneration commits the newly resolved exact version.
   The normal publish run downloads that exact checksum-verified binary and uses the same path
   for plan, pack, doctor, publish, and `--self-bin`, so `manifest.tool.version` and the embedded
   bootstrap move together. Because a `github.token` push starts no `on: push` workflows (the
   rule task-44 documented), any automated-merge path ends in an explicit publish dispatch; a
   human merge needs nothing. This repository's owner publisher remains source-built.

Floating production pins (option a) are rejected: latest is discovery for a reviewed upgrade PR,
SHA256SUMS verification is per-tag, transports stay reproducible, and the airlock keeps a
self-consistent binary/manifest pair instead of "whatever was latest at pack time".

## Consequences

- Consumers get release-drift detection without surrendering review: one PR per release
  worth of template changes; config changes are never automated.
- Self-update becomes security-sensitive release plumbing: asset selection, checksum verification,
  ownership refusal, and atomic replacement require fixture coverage on every supported platform.
- `latest` is permitted only while preparing an upgrade PR. Every committed workflow and every
  published transport still names an exact version, and manual rollback selects an exact older one.
- The stamp, `--check`, self-updater, and upgrade job ship through the generated workflow policy;
  consumer fixtures are regenerated together, while this repository's owner publisher stays on its
  separately reviewed source-built path.
- Config schema bumps now carry a compatibility obligation (read-old-for-a-window), the
  same one the manifest schema already carries.
- The airlock side gains nothing and needs nothing: the launcher is version-agnostic by
  design, doctor reads older manifests, and git-native content-addressed dedup makes the
  repack cheap when nothing but the embedded tools changed.
- An airlock that never upgrades keeps restoring its old, self-consistent transport; one
  that wants the new version gets it through an ordinary git fetch of the repacked branch.
- Accepted 2026-10-05, on task-47's evidence: the implementation is in, and the one proof that
  needed a connected host exists. Castellan PR #13 carried the regenerated owned files at a new
  exact `PIXI_SANDBOX_VERSION` (0.5.2 → 0.5.3) with every template emitted by that same binary; a
  human merge triggered the ordinary publisher, which installed and self-booted the exact 0.5.3
  release for plan, pack, doctor, publish and `--self-bin` and published a manifest naming 0.5.3.
  The credential that delivered that pull request is a separate question the decision does not
  settle — the token-less lane regenerates and then cannot push workflow files, which task-71
  AC#5 and task-73 own — but option (c) does not depend on it: the PR is the review gate, whoever
  opens it.
