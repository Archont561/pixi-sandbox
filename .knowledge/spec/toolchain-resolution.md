---
type: Design Spec
title: "Toolchain Resolution"
description: How a target platform is resolved and validated: P0 declared / P1 locked / P2 published, omitted lists, and compile-capability checks.
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, toolchain, platforms]
status: stable
confidence: mixed
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`DESIGN.md`], sections: ["7"] }
sources:
  - { id: anacondaorg-channels-conda-forge, resource: https://anaconda.org/channels/conda-forge/packages/bun/overview, title: anaconda.org/channels/conda-forge/packages/bun/overview }
  - { id: prefixdev-channels-conda-forge, resource: https://prefix.dev/channels/conda-forge/packages/bun, title: prefix.dev/channels/conda-forge/packages/bun }
  - { id: prefixdev-channels-conda-forge, resource: https://prefix.dev/channels/conda-forge/packages/bun, title: prefix.dev/channels/conda-forge/packages/bun }
---

# Toolchain Resolution

## 7. Toolchain resolution — the heart of it

Your premise ("pixi, because I can install rust, bun and python from conda-forge") is **almost entirely
right, with two measured exceptions**. This subsystem exists to make that premise *enforceable* rather
than hopeful.

### 7.1 Availability snapshot (what this design rests on)

This is **what `pixi sandbox doctor` prints** on 2026-09-18, not a hand-maintained wish list: the tool
regenerates it per host (L0 for presence, L3 for versions, measured reachability for `requires-hosts`),
and CI's contract job ([§14.5](/spec/testing.md#14-testing-strategy-under-a-no-compiler-here-constraint)) fails if the
table below drifts from reality.

| Tool | conda-forge | npm | PyPI | notes |
|---|---|---|---|---|
| `rust` (→ `cargo`/`rustc`) | ✅ | — | — | pixi-pack's own `pixi.toml` pins `rust = "==1.95.0"` ✅ verified |
| `python` | ✅ | — | ✅ | use conda for the interpreter, PyPI for pure-python packages |
| `bun` | ✅ **1.3.11** (updated 2026-03-18) | ✅ 1.4.2 | — | ⚠ **no `win-64`** on conda-forge; channel **lags** upstream (1.3.14/1.4.0/1.4.1 in open feedstock PRs) |
| `pixi-pack` / `pixi-unpack` | ✅ | — | — | install via `pixi global install pixi-pack pixi-unpack` or `pixi exec` ✅ |
| `node`/`npm` | ✅ | — | — | usually unnecessary if `bun` is present |

*Sources: verified against [anaconda.org/channels/conda-forge/packages/bun](https://anaconda.org/channels/conda-forge/packages/bun/overview), [prefix.dev package page](https://prefix.dev/channels/conda-forge/packages/bun), and the `bun-feedstock` PR list (2026-09-18).*

### 7.2 Model

```
Capability = (tool, version-range, platform, source)
Resolver   = for each tool, pick the FIRST source that is (a) allowed on this network,
             (b) available for the target platform, (c) satisfies the version range.
             Emit a *directives* list (`pixi add …`, `npm i -g …`, manual URL) — never install silently.
```

Three rules that make this actually useful:

1. **Per-platform availability is data, not folklore.** `bun` on `win-64` is a *known gap*, so a
   `kit build --platform win-64` must produce a **degraded kit with an explicit `omitted` section**, not
   a broken kit or a silent success. This is exactly the class of failure a "nice ergonomic" tool must
   surface.
2. **Source is recorded in provenance.** `generated.by = "conda:conda-forge/bun@1.3.11"` vs
   `"npm:bun@1.4.2"` — because those are *different runtimes with the same name* (C1/C3). Consumers
   diff kits by provenance, not just version.
3. **Fallbacks are ordered and audited, never open-ended.** `fallback = [conda, npm, manual]` is a
   closed list a reviewer can read. `manual` never *runs* anything; it prints the URL and exits `3`.

### 7.3 Where conda-forge beats pip/npm (and where it doesn't)

Use conda-forge for anything with **native/system content or ABI coupling** (rust toolchain, openssl,
cuda, gdal, geos, ffmpeg, `compilers`) — that's the reason to pick pixi at all. Use `--pypi` for pure
Python packages, and npm only for JS packages. The tool encodes this as `kind` per tool and *refuses* to
vendor a `kind = "conda"` tool from PyPI even when a same-named PyPI package exists — because that is
how you end up with `pixi` (the Pixiv downloader) instead of `pixi` (the package manager) ✅
[measured](/research/node-bun.md#5-node_modules--bun-in-a-pixi-workspace).
* **`toolchain.*` must describe every environment in the inventory, not one idealised build env.** A
  `python 3.10` env and a `python 3.13` env in the same repo are two different activation contracts, so
  `doctor` reports a *matrix* — `env × tool × present-in-lock × resolves-offline` — and `kit build`
  refuses to emit `activation.sh` if any env it is packing has a tool the matrix says is unresolved.

### 7.4 Platform validation: does this platform actually exist?

Requirement (d). `pixi sandbox platforms --env E --target P --explain` answers with *evidence* and a
*remediation*, using three layers, cheapest first, where the first veto wins:

| Layer | Question it can answer without asking anyone | Source | Verdict when it fails |
|---|---|---|---|
| **P0 declared** | is `P` a platform this workspace admits at all? | `workspace.platforms`, and per-feature `platforms = [...]` (legal ✅), parsed at L1 | `⛔ not-declared` → add `P` to `platforms`, **or** scope the offending dependency with `[feature.x] platforms = [...]` — the documented way to keep a CUDA-only or gap-only dep from constraining the whole project ✅ |
| **P1 locked** | does `pixi.lock` actually contain a package block for (`E`,`P`)? | scan `environments.<E>.packages.<P>` in `pixi.lock` (L2 heuristic, `heuristic: true`) | `⛔ not-locked` → `pixi lock` (writes the lockfile only, no install ✅), then re-check |
| **P2 published** | does the channel ship the artifacts at all? | repodata probe of `conda.anaconda.org/conda-forge/<subdir>/…`, or `[platforms].known-gaps` cached answers, or `pixi-pack`'s solver error at build time | `⛔ unavailable-upstream` → substitute provider, or accept a degraded kit |

```console
$ pixi sandbox platforms --target win-64 --env data-science --explain
⛔ data-science @ win-64: unavailable-upstream
   P0 declared ✅  P1 locked ✅ (13 packages)  P2 conda-forge → gap
   • bun 1.3.11 on conda-forge publishes linux-64, linux-aarch64, osx-64, osx-arm64 — NO win-64 ✅
   • the JS path is also gated: bun.sh documents `win32` support ✅ but via `Scoop`/`npm i -g @oven/bun-windows-x64`
   → options
     1. scope it:  [feature.bun] platforms = ["linux-64", "osx-arm64"]     (other envs keep win-64)
     2. swap the runtime for that env: `nodejs` has full platform coverage ✅
     3. accept a degraded kit: [platforms] on-missing = "warn"  → win-64 artifacts are omitted, listed in `omitted[]`, exit 5
```

Two rules make this a *feature* and not a validation nag:

* **Gaps are data.** `[platforms].known-gaps` caches P2 answers with a `checked:` date (the OKF habit of
- **A compile-capable platform is a second, stricter question.** "Does `win-arm64` exist for this env?" is
  P0–P2 above; "can the target *build*" additionally needs `rust-std-<triple>` — which is precisely how
  conda-forge splits the toolchain (one `noarch: generic` package per target, exact-pinned to `rust` ✅) — so
  `platforms --explain` under `[kit] targets-compile = true` must answer both, e.g.
  `rust 1.98.1 ✅ on linux-aarch64 · rust-std-wasm32-unknown-unknown ✅ (noarch) · rust-std-x86_64-unknown-linux-musl ❌
  no such package` ⇒ cross-compiling to musl needs a different plan (a `cargo-zigbuild`-style tool or the target
  triple's own std). Recording it as one boolean would be the classic "runs but can't build" surprise.
* Because these gaps are per-*package*, `known-gaps` entries are keyed `name × subdir` (as `bun × win-64` is
  today ✅), and the tool should be able to *learn* a new one from a failed P2 probe without a code change.
  timestamps as contract ✅), so a cold sandbox with no network can still answer "does win-64 exist for
  bun?" at L0. `pixi sandbox platforms --refresh` re-probes and rewrites the section; if a live probe
  contradicts a cached gap, CI warns — upstream availability moves, and the cache must show its age.
* **Env-level and package-level are different questions.** `--env` + `--target` answers "can *this
  environment* be packed *for that platform*"; `--package bun` answers "does *this package* exist for
  *any* platform we care about". The first is a lockfile fact, the second is a channel fact; the tool
  prints which question it answered, because conflating them is how you get a "green" plan that fails in
  `pixi-pack`'s solver three minutes later.

---
