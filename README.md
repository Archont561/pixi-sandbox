# pixi-sandbox

**Design, not implementation.** `pixi-sandbox` is a **public GitHub Action** that packs `pixi`
environments and `cargo vendor` trees into portable artifacts, publishes them on an **orphan branch of this
repo**, and ships a reconstructor alongside them so a machine whose only network access is `git` can
reconstruct a working, task-driven environment. The reconstructor is **one Rust binary**
([D21](/.knowledge/spec/decisions.md), decided): the action runs its CI verbs (`pack`/`publish`) and the kit
ships the *same binary* as its assembler (`reconstruct`) — built in CI, mirrored digest-pinned, installed by
nobody. Its behaviour is pinned by the measured `assemble.sh` oracle, which is what keeps every claim here
testable in the sandbox that wrote it.

Everything about it lives in a knowledge bundle:

```
.knowledge/            # OKF v0.2 bundle — 55 concepts, 5 areas, this repo's whole corpus
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

A git ref is an immutable, content-addressed artifact registry. The action runs the packagers that already exist
(`pixi-pack` for environments, `cargo vendor` for the crate graph), writes their outputs into an orphan
`pixi-sandbox-dist` branch with a `dist-manifest.json`, a `SHA256SUMS` and the mirrored `pixi` + `pixi-unpack`
binaries, and pins the digests in `sandbox.lock.json` on `main`. On the far side of the airlock one command —
`git clone --depth 1 --branch pixi-sandbox-dist … && sh kit/assemble.sh` — verifies digests *before executing
anything*, then reconstructs the environment so `pixi run <task>`, `pixi shell` and workspace management keep
working, with no registry, CDN or release asset needed at any point. JS dependencies are **not** part of a kit:
`node` and `npm` exist on the hosts that build the docs, and the docs build is a CI job (D20). Start with
[The one idea](/.knowledge/overview/problem.md); the mechanics are in
[The Action Shape](/.knowledge/spec/action-shape.md),
[Artifact Formats and Integrity](/.knowledge/spec/artifacts.md) and
[Reconstructing on an Airlocked Machine](/.knowledge/workflows/airlock-clone.md).

## What's in the bundle

| Area | Concepts | Contents |
|---|---|---|
| [`overview/`](/.knowledge/overview/index.md) | 4 | the problem, goals G1–G9, the five requirements traced to answers, the constraints that decide everything |
| [`spec/`](/.knowledge/spec/index.md) | 18 | architecture, command surface, config schema, toolchain/platform resolution, the three packagers, artifacts + the R1–R5 rung ladder, docs site, git registry, bootstrap, the Action shape, **the assembler binary (D21)**, CI, error taxonomy, testing + **testing strategy**, roadmap, **decision log D1–D21**, risks |
| [`workflows/`](/.knowledge/workflows/index.md) | 10 | the lockfile→digest map, create-and-pack, airlock rebuild, CI republish, **running the action (nine measured outcomes)**, **pixi ↔ cargo interop verdict**, copy-paste scripts, the unproven register, dogfooding D0–D4, publishing the docs |
| [`research/`](/.knowledge/research/index.md) | 16 | first-hand notes on pixi 0.81.0, pixi-pack 0.7.11, `cargo vendor`, bun/node, Actions limits, GitHub Markdown, AGENTS.md, clap/thiserror, OKF — plus sandbox measurements, corrections, bibliography |
| [`environment/`](/.knowledge/environment/index.md) | 7 | the reference host: inventory, egress matrix, consequences, execution and git semantics, budgets, session safety, reproduction commands |

Three lines carry the whole project: **detect, don't declare** (`Inventory` tiers L0→L3, so packing an
environment needs no config edit); **the lockfile is the spec** (which artifacts exist, and their digests, are
derived from `pixi.lock` / `Cargo.lock` — one lockfile, one digest key); and **the tool must work where it cannot
be built** (G9: answered by D19 + [D21](/.knowledge/spec/decisions.md) together — the binary comes back, but
only as a *mirrored artifact*: built in CI where network exists, shipped digest-pinned in the kit, and never
compiled or installed on the target; the design's behaviour was first proven as the `assemble.sh` oracle,
executed in the sandbox that specified it).

## Reading it as an agent

The bundle is [Open Knowledge Format v0.2](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md):
every concept has YAML front matter (`type` required, plus `title`, `description`, `tags`, `status`,
`confidence`, `generated`, `sources`, and a `legacy` key naming the pre-OKF document it came from), links are
bundle-relative (`/spec/cli.md`), and `index.md`/`log.md` are reserved for navigation. Trust is machine-readable:
`confidence: verified` ⇒ measured here or read from upstream source; `reasoned` ⇒ inferred; `open` 🚧 ⇒ needs a
real machine or a decision. **Do not smooth those labels over** — they are the reason this corpus is usable.
Editing rules are in [Bundle Conventions](/.knowledge/conventions.md).

## Status, decisions, next step

* **Decision status:** the 21 rows of the [Decision Log](/.knowledge/spec/decisions.md) are all decided —
  D1–D16 from the original design pass, D17–D20 accepted on 2026-09-19, and **D21 (one Rust binary, two
  surfaces — the action runs it, the kit ships it, nobody installs it) accepted the same day**, with its test
  plan in [Testing Strategy](/.knowledge/spec/testing-strategy.md). That acceptance closed **M0 (spec)**.
* **Next milestone:** **M1 — the binary + the action**: the `pixi-sandbox` Rust crate (`pack`/`publish` invoked
  by the action, `reconstruct` shipped in the kit), with the nine measured `assemble.sh` outcomes as its
  acceptance vectors; then **M1.5**, the end-to-end run against
  real `pixi-pack`/`pixi-unpack` in a clean container.
* **Docs site:** `.knowledge/` is the canonical markdown; an Astro Starlight site builds from it and publishes
  *twice* — GitHub Pages for browsers, a `pixi-sandbox-docs` orphan branch for sealed boxes (because
  `*.github.io` is unreachable from inside an airlock — [Publishing the Docs](/.knowledge/workflows/publishing-docs.md)).
* **How it arrives once it exists:** nobody installs anything. The action builds the `pixi-sandbox` binary in
  CI and the kit mirrors it digest-pinned into `bin/` next to `pixi`/`pixi-unpack` — so a GitHub-only machine
  just runs `git clone --depth 1 --branch pixi-sandbox-dist …` and executes the verified blob
  ([The Assembler Binary](/.knowledge/spec/assembler-binary.md)). The gap this closes: pixi's documented git
  support is **PyPI-flavoured**, so there is no `pixi add --from git+…` for a Rust binary — see
  [Bootstrap](/.knowledge/spec/bootstrap.md). Until M1 lands, this repo has no binaries, dist branches or
  releases to consume.
