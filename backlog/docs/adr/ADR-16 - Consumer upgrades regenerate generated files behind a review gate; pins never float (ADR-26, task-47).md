---
id: ADR-16
title: "Consumer upgrades regenerate generated files behind a review gate; pins never float (ADR-26, task-47)"
status: accepted
tags:
  - decision
---

# ADR-16 — Consumer upgrades regenerate generated files behind a review gate; pins never float (ADR-26, task-47)

**Decision.** When pixi-sandbox releases, consumer-owned generated files are upgraded by
*regeneration*, not by editing: a version stamp beside the ownership marker makes them
self-describing, `pixi-sandbox init --check` is the drift gate (render fresh, compare,
non-zero exit, zero writes), and a scheduled job inside the *generated* workflow installs
the latest released CLI and, on drift, regenerates the marker-carrying files and opens a
PR — the task-39 relock-bot precedent applied to the publisher. Config (`pixi-sandbox.toml`)
is reviewed data and is never auto-rewritten: an older `schema` is reported with its
migration path, and readers accept older config schemas for a deprecation window, mirroring
the manifest policy. The transport repacks when the upgrade PR merges to main, carrying the
new released `--self-bin` so `manifest.tool.version` moves; an automated merge must end in
an explicit publish dispatch because a `github.token` push starts no `on: push` workflows
(task-44). Floating the workflow to latest/`@v0` is rejected.

**Why.** A version bump is not one line: the template itself changes with the release, so
the only safe upgrade unit is "re-run init with the new CLI". SHA256SUMS verification is
per-tag, so a floating pin cannot be checksum-verified the way the bootstrap requires;
exact pins keep transports reproducible and the airlock's binary/manifest pair
self-consistent. Config changes ride the same review everything else here does — the gate
is the review, not the edit.

**Evidence.** Precedents, not fresh measurement (proposed until task-47 lands):
check-repository's committed-render check already holds this repository to
"committed render must equal fresh render"; `tools update --check` already defines the
report-and-exit-non-zero gate shape; task-39's relock bot already pushes bot-authored
commits behind review; task-44 already documented the `github.token`-push-triggers-nothing
rule and the dispatch remedy; task-45 already made the generated bootstrap a per-tag,
checksum-verified download.

**What would change it.** A demonstrated need for zero-touch consumer upgrades (then an
opt-in auto-merge path ending in the explicit publish dispatch — pins still exact), or the
loss of per-tag checksum manifests (then the bootstrap design itself reopens, not just
this decision).
