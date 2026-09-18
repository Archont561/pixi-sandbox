# pixi-sandbox

**Design proposal, not implementation.** An ergonomic Rust tool that packs `pixi` environments, `cargo vendor`
trees and optional `node_modules` into portable artifacts, publishes them on an **orphan branch of this repo**,
and reconstructs a working, task-driven environment on a machine whose only network access is `git`.

Everything about it lives in a knowledge bundle:

```
.knowledge/            # OKF v0.2 bundle — 51 concepts, 5 areas, this repo's whole corpus
  index.md             # start here (progressive disclosure: areas → groups → concepts)
  log.md               # dated history of the corpus
  conventions.md       # labels ↔ trust tiers, front-matter extensions, editing rules
```

> [!IMPORTANT]
> **No code files, on purpose.** The sandbox this was authored in reaches GitHub, npm and PyPI — but not
> crates.io, `prefix.dev`, conda channels or GitHub release assets — so any Rust written here could not be
> compiled, and `pixi` itself cannot be installed to validate a single flag. Read
> [Design Constraints](/.knowledge/overview/constraints.md) for the measurements and
> [What Is Still Unproven](/.knowledge/workflows/unproven.md) for the gap register. The design is finished
> enough to review, correct, and hand to a machine that *can* build it.

## The idea in one paragraph

A git ref is an immutable, content-addressed artifact registry. `pixi-sandbox kit build` runs the packagers
that already exist (`pixi-pack` for environments, `cargo vendor` for the crate graph, `bun install --frozen-lockfile`
for JS), writes their outputs into an orphan `pixi-sandbox-dist` branch with a `dist-manifest.json` and a
`.sha256` per artifact, and pins the digests in `sandbox.lock.json` on `main`. On the far side of the airlock,
`pixi-sandbox reconstruct` clones *that branch only* (`--filter=blob:none`), verifies digests, and reinstalls
the environment so that `pixi run <task>`, `pixi shell` and workspace management keep working — with `pixi`
itself and `pixi-sandbox` shipped from the same branch, so no registry, CDN or release asset is ever needed.
Start with [The one idea](/.knowledge/overview/problem.md); the mechanics are in
[Artifact Formats and Integrity](/.knowledge/spec/artifacts.md) and
[Reconstructing on an Airlocked Machine](/.knowledge/workflows/airlock-clone.md).

## What's in the bundle

| Area | Concepts | Contents |
|---|---|---|
| [`overview/`](/.knowledge/overview/index.md) | 4 | the problem, goals G1–G9, the five requirements traced to answers, the constraints that decide everything |
| [`spec/`](/.knowledge/spec/index.md) | 15 | architecture, command surface, config schema, toolchain/platform resolution, the three packagers, artifacts + the R1–R5 rung ladder, docs site, git registry, bootstrap, CI, error taxonomy, testing, roadmap, **decision log D1–D18**, risks |
| [`workflows/`](/.knowledge/workflows/index.md) | 9 | the lockfile→digest map, create-and-pack, airlock rebuild, CI republish, **pixi ↔ cargo interop verdict**, copy-paste scripts, the unproven register, dogfooding D0–D4, publishing the docs |
| [`research/`](/.knowledge/research/index.md) | 16 | first-hand notes on pixi 0.81.0, pixi-pack 0.7.11, `cargo vendor`, bun/node, Actions limits, GitHub Markdown, AGENTS.md, clap/thiserror, OKF — plus sandbox measurements, corrections, bibliography |
| [`environment/`](/.knowledge/environment/index.md) | 7 | the reference host: inventory, egress matrix, consequences, execution and git semantics, budgets, session safety, reproduction commands |

Three lines carry the whole project: **detect, don't declare** (`Inventory` tiers L0→L3, so packing an
environment needs no config edit); **the lockfile is the spec** (which artifacts exist, and their digests, are
derived from `pixi.lock` / `Cargo.lock` / `bun.lock` — one lockfile, one digest key); and **the tool must work
where it cannot be built** (G9: it consumes its own kits, so every release doubles as an airlock rehearsal).

## Reading it as an agent

The bundle is [Open Knowledge Format v0.2](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md):
every concept has YAML front matter (`type` required, plus `title`, `description`, `tags`, `status`,
`confidence`, `generated`, `sources`, and a `legacy` key naming the pre-OKF document it came from), links are
bundle-relative (`/spec/cli.md`), and `index.md`/`log.md` are reserved for navigation. Trust is machine-readable:
`confidence: verified` ⇒ measured here or read from upstream source; `reasoned` ⇒ inferred; `open` 🚧 ⇒ needs a
real machine or a decision. **Do not smooth those labels over** — they are the reason this corpus is usable.
Editing rules are in [Bundle Conventions](/.knowledge/conventions.md).

## Status, decisions, next step

* **Open decisions for you:** the 18 rows of [Decision Log](/.knowledge/spec/decisions.md). D1–D16 are
  `decided`/`recommended`; D17 (dogfooding tiers) and D18 (docs publishing) are the newest and still need your
  acceptance — accepting them is also the gate that closes **M0 (spec)**.
* **Next milestone:** **M1 — the walking skeleton**, read-only verbs only (`inventory`, `environments`,
  `platforms`, `plan`, `doctor`, `completions`) with argv goldens and no writes; then M2 packs and unpacks a
  `linux-64` environment, verified by `tar -xf` on a box with no conda or pixi on `PATH`. Gates are in
  [Roadmap and Acceptance Gates](/.knowledge/spec/roadmap.md).
* **Docs site:** `.knowledge/` is the canonical markdown; an Astro Starlight site builds from it and publishes
  *twice* — GitHub Pages for browsers, a `pixi-sandbox-docs` orphan branch for sealed boxes (because
  `*.github.io` is unreachable from inside an airlock — [Publishing the Docs](/.knowledge/workflows/publishing-docs.md)).
* **Install once it exists:** the default is the one that works in a GitHub-only sandbox —
  `git clone --depth 1 --branch pixi-sandbox-dist` + checksum + `chmod` (or `pixi sandbox install`, same four
  steps); offline users then `pixi add ./pixi-sandbox-0.1.0-linux-64.conda` ✅, and a networked machine can
  `cargo install --git https://github.com/Archont561/pixi-sandbox.git --bin pixi-sandbox`. Note the gap this
  design exists to close: pixi's documented git support is **PyPI-flavoured**, so there is no
  `pixi add --from git+…` for a Rust binary — see [Bootstrap](/.knowledge/spec/bootstrap.md).
  Until then, this repo has no binaries, dist branches or releases to consume.
