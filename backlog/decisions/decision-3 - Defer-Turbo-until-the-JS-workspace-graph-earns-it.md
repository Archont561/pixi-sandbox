---
id: decision-3
title: Defer Turbo until the JS workspace graph earns it
date: '2026-10-01 15:51'
status: deferred
---
## Context

Task-26 (v1 evolution plan) asked whether to adopt Turbo for JavaScript orchestration. The
plan's division of labour is already settled: Bun owns dependencies, Turbo would own the JS
task graph, Pixi provides the environment facade. The open question was only *when*.

The evaluation is in `backlog/docs/spikes/turbo-orchestration/doc-8`, measured 2026-10-01:
the bun workspace has exactly one JS package (`workspaces: ["docs"]`), there is no
cross-package JS dependency, and the docs workflow's build job — checkout, pixi setup,
`bun install --frozen-lockfile` + `astro build` — completes in ~27 s on `ubuntu-latest`.
Pixi already expresses the only task graph that exists (`docs-install → docs-build`,
`docs-install → lint-docs`, reused by the pre-commit hook as `lint-docs-write`).

## Decision

**Deferred — no Turbo today.** No dependency, no `turbo.json`, no pixi task changes. Revisit
when *all* of the doc-8 §2 triggers hold:

1. ≥ 3 JS packages in the bun workspace (two packages still fit two pixi tasks);
2. a real cross-package edge — some package imports another workspace package, so build
   order and change-driven pruning stop being trivial;
3. ≥ ~60 s of repeated JS work on the CI path that a cache would prune, or a local
   feedback loop past ~10 s for unchanged packages.

If the repository commits to the docs site remaining the only JS package permanently, close
this as rejected instead.

When adopted: Turbo is a **root `package.json` devDependency** pinned through `bun.lock`
(like `@biomejs/biome` today), never a conda/pixi dependency; invoked as `bun x turbo …`
inside the `web` environment; local `.turbo` cache for developers, `--force` for published
builds, `.turbo` cached across CI runs keyed on `bun.lock` + OS, and **remote caching stays
off** — third-party cache round-trips contradict the airlock posture.

## Consequences

- The single `docs` workspace keeps its three-line pixi task graph; nothing new to install
  or keep pinned, and the sandbox branch's payload budget is not spent on a CI-only tool.
- The threshold is written down and measurable, so the next "should we add Turbo?" question
  is a lookup, not a re-spike: count workspace members, check for a cross-package import,
  read the docs job duration.
- If JS packages grow, the migration is mechanical and pre-designed (doc-8 §3): pin
  `turbo` as a Bun devDependency, add `turbo.json` mirroring the pixi task names, turn the
  per-package pixi tasks into thin `bun x turbo <task>` facades.
- Deferred ≠ decided forever: any PR adding a second or third JS package should re-check
  the triggers (a §2 hit reopens this decision rather than silently accumulating pixi task
  duplication).
