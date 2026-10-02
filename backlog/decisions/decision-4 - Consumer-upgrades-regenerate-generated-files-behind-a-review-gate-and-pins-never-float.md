---
id: decision-4
title: Consumer upgrades regenerate generated files behind a review gate, and version pins never float
date: '2026-10-02 20:05'
status: proposed
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

**(c), with the pins staying exact.** Three version domains, three verbs:

1. **Generated files regenerate.** A version stamp beside the ownership marker makes them
   self-describing; `pixi-sandbox init --check` is the drift gate (render fresh, compare,
   exit non-zero, write nothing); a scheduled upgrade job in the *generated* workflow
   installs the latest released CLI, runs the check, and on drift regenerates the
   marker-carrying files and opens a PR — the task-39 relock-bot precedent applied to the
   publisher. The upgrade unit is "re-run `init` with the new CLI", never a hand-edited
   `PIXI_SANDBOX_VERSION`, because the template changes with the version, not just the pin.
2. **Config migrates explicitly.** `pixi-sandbox.toml` is reviewed data ("the gate is the
   review, not the edit"); `init` never rewrites an existing config. An older `schema` is
   reported as a finding naming its migration path; a `config migrate` command is built
   only once a schema bump gives it something to do; readers accept older config schemas
   for a deprecation window, mirroring the manifest policy ("a published branch outlives
   the binary that packed it").
3. **The transport repacks on merge.** The upgrade PR merged to main triggers the normal
   publish run, whose `--self-bin` is the new released, SHA256SUMS-verified binary —
   `manifest.tool.version` moves with it. Because a `github.token` push starts no
   `on: push` workflows (the rule task-44 documented for this repo's own transport), any
   automated-merge path ends in an explicit publish dispatch; a human merge needs nothing.

Floating pins (option a) are rejected: SHA256SUMS verification is per-tag, transports stay
reproducible, and the airlock keeps a self-consistent binary/manifest pair instead of
"whatever was latest at pack time".

## Consequences

- Consumers get release-drift detection without surrendering review: one PR per release
  worth of template changes; config changes are never automated.
- The stamp, `--check`, and the upgrade job ship through the same generated templates, so
  this repository's own committed renders are regenerated with them — check-repository's
  committed-render check holds the repo to the policy it hands consumers.
- Config schema bumps now carry a compatibility obligation (read-old-for-a-window), the
  same one the manifest schema already carries.
- The airlock side gains nothing and needs nothing: the launcher is version-agnostic by
  design, doctor reads older manifests, and git-native content-addressed dedup makes the
  repack cheap when nothing but the embedded tools changed.
- An airlock that never upgrades keeps restoring its old, self-consistent transport; one
  that wants the new version gets it through an ordinary git fetch of the repacked branch.
- Stays `proposed` until task-47 lands; then `accepted` with the implementation as
  evidence.
