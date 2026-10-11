---
id: ADR-17
title: "Init writes only files it owns; the consumer's pixi.toml is never mutated (issue #71, task-48)"
status: accepted
tags:
  - decision
---

# ADR-17 — Init writes only files it owns; the consumer's pixi.toml is never mutated (issue #71, task-48)

**Decision.** `pixi-sandbox init` no longer appends any channel to the consumer's
`[workspace].channels`. It writes only the files it owns — the publisher workflow, the relock
workflow, the launcher, and the sandbox config when absent — and leaves `pixi.toml`
byte-identical, requiring only that the manifest exists. The CLI install stays an explicit
`pixi global install --channel https://prefix.dev/archont561/archont561`, which never reads
project channels. This reverses task-34, whose premise — that the prefix.dev namespace root
`https://prefix.dev/archont561` lets a project consume other Archont561 channels — is false:
prefix.dev serves repodata only at `/<owner>/<channel>`, so the appended URL 404s and every
consumer with a dependency failed `pixi lock` right after init.

**Why.** No generated file reads the project's channel list: the publisher workflow installs
the CLI globally with explicit channels, the relock bot runs pixi against the project's own
channels, and the airlock is offline. The append bought only `pixi add <namespace tool>`
inside a project environment — speculative until a second tool ships — and cost a rewrite of
a manifest init does not own (the same ownership boundary the `GENERATED_MARKER` draws) which,
as shipped, broke the lock. An empty project locked fine — an empty solve never fetches
repodata — which is why the smoke test missed it; the fixture cover therefore runs against
dependency-carrying manifests.

**Evidence.** Issue #71 (2026-10-02): with the namespace root appended, a dependency-free
probe locks but `bun =1.3.11` 404s on
`https://prefix.dev/archont561/noarch/repodata.json` under `pixi lock --verbose`; the
wirewright consumer's four environments all failed until the entry was removed. The live
probe against the fixed binary runs on a connected host (the airlock cannot reach
prefix.dev); the fixture suite proves the byte-identical property directly, and CI proves
the suite.

**What would change it.** Real project-level demand to consume Archont561 packages from
consumer project environments. The ecosystem channel now exists — `archont561/archont561`,
created 2026-10-02, with every release publishing to it from 0.4.4 on (task-49) — so the
revisit, should that demand materialise, is appending *that* channel, recorded as a new
entry; a single package's channel is not that path, and the namespace root never was a
channel at all.
