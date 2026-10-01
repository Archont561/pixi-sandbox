---
id: doc-8
title: Turbo Orchestration Spike
type: other
created_date: '2026-10-01 18:05'
updated_date: '2026-10-01 18:05'
tags:
  - spike
  - turbo
  - bun
  - tooling
  - decision
---
# Spike — Turbo for multi-package JavaScript orchestration

**Status:** decision spike, no production code changed. Produced for backlog task-26
(v1 evolution plan § "Evaluate Turbo"), which asks for a documented adoption threshold, a
proposed JS task graph, cache and CI behaviour — and a clear decision based on the
repository's *actual* JS package count.

**Question.** When should this repository adopt Turbo for its JavaScript workspaces, what
would the task graph and caching policy look like on that day, and is that day today?

## §1 Current state, measured

- The bun workspace has exactly **one** JS package: `package.json` declares
  `workspaces: ["docs"]`, and `docs/package.json` is the only member. There are no
  cross-package JS dependencies, no shared component library, no second app.
- The docs job is small: the `docs` GitHub workflow's `build` job — checkout, pixi setup with
  cache, `pixi run docs-build` (which is `bun install --frozen-lockfile` + `astro build`) —
  completes in **~27 s** wall time including runner bootstrap (run 36879920379, 2026-10-01;
  recent runs range ~30–80 s end to end including the Pages deploy job).
- Pixi already owns the task graph that exists today: `pixi.toml` expresses
  `docs-install → docs-build` and `docs-install → lint-docs` as task dependencies, and the
  pre-commit hook reuses `lint-docs-write` so the hook and the lint gate cannot drift. There
  is no JS-level fan-out for Turbo to schedule.

With one package and zero cross-package edges, Turbo has nothing to parallelise, nothing to
deduplicate, and no graph to prune: it would add a dependency, a `turbo.json`, and a second
place where "what runs when" is expressed, in exchange for caching a build that already
finishes faster than the runner takes to boot.

## §2 Adoption threshold

Re-evaluate — and adopt, per the v1 evolution plan's division of labour (Bun owns
dependencies, Turbo owns the JS task graph, Pixi provides the environment facade) — when
**all** of the following are true:

1. **≥ 3 JS packages** in the bun workspace (the `docs` site plus at least two more, e.g. a
   shared Starlight component package and a second site or tool). Two packages are still
   cheaper to express as two pixi tasks than as a graph.
2. **A real cross-package edge**: at least one package imports another workspace package, so
   build order and change-driven pruning stop being trivial. A pile of unrelated packages
   under one lockfile is a workspaces question, not a Turbo question.
3. **Cost that matters**: the combined JS task time on the docs/CI path exceeds **~60 s** of
   work that repeats when only one package changed (measured on `ubuntu-latest`, warm pixi
   cache), or a locally-feeling feedback loop (dev/build) degrades past ~10 s for unchanged
   packages.

Until all three hold, the answer is **defer**; if the repository ever commits to keeping the
docs site as the only JS package permanently, the spike may be closed as **rejected**.

## §3 Proposed task graph (for the day the threshold is met)

`turbo.json` at the root, expressed as tasks that mirror the pixi names so the facades stay
in step (Pixi tasks become thin wrappers calling `bun x turbo <task>`):

```jsonc
{
  "$schema": "https://turbo.build/schema.json",
  "tasks": {
    "build": {
      "dependsOn": ["^build"],          // workspace deps build first
      "inputs": ["$TURBO_DEFAULT$", "astro.config.mjs", "content/**"],
      "outputs": ["dist/**", ".astro/**"]
    },
    "check": {                          // astro check / tsc, when a package has it
      "dependsOn": ["^build"],
      "inputs": ["$TURBO_DEFAULT$"]
    },
    "lint": {
      "dependsOn": ["docs-install"],    // root task, see §4
      "inputs": ["src/**", "content/**", "biome.json"]
    },
    "dev": { "cache": false, "persistent": true }
  }
}
```

`bun.lock`, `package.json`, and `pixi.lock` are part of every task's hash by default
(`$TURBO_DEFAULT$`), so a lockfile roll re-runs the world — the intended behaviour for an
airlock-sensitive repo. The Pixi facade becomes `js-build = "bun x turbo build"` in the `web`
environment, replacing per-package tasks (`docs-build`, …) one for one.

## §4 Dependency ownership: Bun, not Conda

Turbo is a JS build orchestrator, not a runtime the sandbox needs at restore time. Per the
plan's ownership rule it arrives as a **root `package.json` devDependency** (version pinned
through `bun.lock`, exactly like `@biomejs/biome` today) and **never** as a pixi/conda
dependency:

- the `web` pixi environment keeps carrying only what a *user* of the site needs (bun
  itself);
- the sandbox/transport branch stays free of a tool that exists only to sequence CI work —
  the payload budget measured in doc-7 is not spent on developer tooling;
- invocation goes through bun (`bun x turbo …`), the same pattern the repo already uses for
  node-shebang bins (biome, backlog, skills), never a global or conda-faithful PATH lookup.

## §5 Cache inputs/outputs and CI behaviour

- **Local cache** (`.turbo/`, gitignored) is the default for developers; `docs-build`-style
  published builds run with `--force` so a release never ships from a warm cache.
- **CI** caches `.turbo` (and `node_modules`) with `actions/cache` keyed on
  `bun.lock` + OS, restoring across runs of the same workflow; PR CI uses the cache, release
  builds don't (`--force`).
- **Remote caching stays off.** Vercel's remote cache would send content hashes and build
  artefact identifiers to a third party, which collides with the airlock posture this
  repository is built around (docs.yml's own comment: large payloads and third-party
  round-trips are policy decisions, not conveniences). If a future need makes remote caching
  attractive, that is a new decision, not a config flip.

## §6 Decision

**Defer** — recorded as **decision-3** through the Backlog decision API. Today's repository
has one JS workspace and a ~27 s docs job; the threshold in §2 is not met, so no Turbo
dependency, no `turbo.json`, and no pixi task changes are made. The triggers above are the
re-entry point.
