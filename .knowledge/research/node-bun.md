---
type: Source Notes
title: "node_modules and bun in a pixi workspace"
description: How JS dependencies can (and cannot) travel with a pixi environment: bun on conda-forge, node_modules packaging, lockfile formats.
resource: https://github.com/Archont561/pixi-sandbox
tags: [research, bun, node]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`WEB_SEARCH_RESULTS.md`], sections: ["5"] }
stale_after: 2026-12-19T00:00:00Z
sources:
  - { id: wwwnpmjscom-package-pixi, resource: https://www.npmjs.com/package/pixi, title: www.npmjs.com/package/pixi }
  - { id: githubcom-pixijs-pixijs, resource: https://github.com/pixijs/pixijs/issues/4771, title: pixijs/pixijs — issues/4771 }
  - { id: githubcom-pixijs-pixijs, resource: https://github.com/pixijs/pixijs/issues/3365, title: pixijs/pixijs — issues/3365 }
  - { id: githubcom-conda-forge-bun-feedstock, resource: https://github.com/conda-forge/bun-feedstock, title: conda-forge/bun-feedstock }
  - { id: nodejslibhuntcom-bun-alternatives, resource: https://nodejs.libhunt.com/bun-alternatives, title: nodejs.libhunt.com/bun-alternatives }
  - { id: bunsh-docs-pm, resource: https://bun.sh/docs/pm/cli/install, title: bun.sh/docs/pm/cli/install }
  - { id: buncom-docs-pm, resource: https://bun.com/docs/pm/cli/install, title: bun.com/docs/pm/cli/install }
---

# node_modules and bun in a pixi workspace

> [!NOTE]
> **Outcome (2026-09-19):** the research stands, the *feature* does not. D20 retired `node_modules`
> packing from v1 — the runtimes (`nodejs`, `bun`) still travel as conda packages and a JS lockfile is still
> a CI assertion, but no JS dependency tree is pushed to a branch. Kept because it is the evidence base for
> that decision (bun on conda-forge ✅, `bun.lockb` hazards ✅, finding C2 ✅: an npm-installed bun works even
> in a GitHub-only sandbox) and because resurrecting the component means re-reading this, not re-researching it.

## 5. node_modules / bun in a pixi workspace

**First, the trap.** "pixi" is a heavily collided name: on PyPI `pixi 1.0.1` is *"a command line tool to
download images from Pixiv"* ✅ verified locally by inspecting the wheel; on npm `pixi` is the legacy
"Node Pixi Renderer" [5](https://www.npmjs.com/package/pixi) and `pixi.js` is PixiJS (a WebGL canvas
engine) with `npm install pixi.js` [4](https://www.npmjs.com/package/pixi.js/v/5.0.0-rc). Web search for
"pixi + node_modules" is dominated by PixiJS issues
[1](https://github.com/pixijs/pixijs/issues/4771)
[2](https://github.com/pixijs/pixijs/issues/3365). **Never resolve a `pixi` install via
`pip install pixi`/`npm i pixi`.**

> [!IMPORTANT]
> **Correction (2026-09-18, after user review).** My first pass claimed bun was *not* packaged on
> conda-forge, based on a 2023 staged-recipes request. That is **wrong**: `bun` is a real conda-forge
> package today — `conda install conda-forge::bun` / `pixi add bun`, **v1.3.11**, 43 files, last updated
> **2026-03-18**, feedstock [`conda-forge/bun-feedstock`](https://github.com/conda-forge/bun-feedstock).
> Two caveats that survive and still matter for a tool design: (a) the available subdirs are
> **`linux-64`, `linux-aarch64`, `osx-64`, `osx-arm64` — there is no `win-64` build**, and (b) the channel
> **lags upstream**: 1.3.14 / 1.4.0 / 1.4.1 are open feedstock PRs while bun.sh advertises **1.4.2**.
> prefix.dev also flags the artifact provenance as *"Built from bun-feedstock@7e5d297 — Not
> cryptographically verified"*. Lesson recorded in §14.

**What pixi does and does not do for JS.** pixi's documented dependency ecosystems are **conda + PyPI**
(plus its own conda-package build backends) — there is **no first-class npm/yarn/pnpm/bun dependency
ecosystem**. So "creating a pixi workspace with node_modules" is a *composition*, not an integration:
pixi supplies the **toolchain and runtimes** from conda-forge (`bun`, `nodejs`, `python`, `openssl`,
`compilers`, …) and the JS package manager owns `node_modules`. The idiomatic shape:

```toml
# pixi.toml
[workspace]
name = "hybrid"
channels = ["conda-forge"]
platforms = ["linux-64", "osx-arm64", "win-64"]

[dependencies]
nodejs = "22.*"
corepack = "*"

[tasks]
install = "bun ci"                                  # JS deps -> node_modules/
dev       = { cmd = "bun run dev", depends-on = ["install"] }
build     = { cmd = "bun run build", input = ["src"], output = { cache = "dist" } }
---
test      = "bun test"
```

**Bun specifics that make it a good (and safe) fit here.** `bun install` "creates an ordinary
`node_modules` folder… designed to be compatible with other package managers and Node.js"
[7](https://nodejs.libhunt.com/bun-alternatives); it writes **`bun.lock`** (text) — before Bun 1.2 the
lockfile was the **binary `bun.lockb`**, migrate with
`bun install --save-text-lockfile --frozen-lockfile --lockfile-only` then delete `bun.lockb`
[2](https://bun.sh/docs/pm/cli/install), [3](https://bun.com/docs/pm/cli/install).

For CI hermeticity (the bun analogue of `--frozen`/`--locked`/`cargo build --locked`):

* `bun install --frozen-lockfile` = install exactly what `bun.lock` says, **error if `package.json`
  disagrees**, never write the lockfile; requires `bun.lock` to be committed
  [3](https://bun.com/docs/pm/cli/install).
* **`bun ci`** is the equivalent alias for CI; `--production` *implies* `--frozen-lockfile` (and only
  controls what's installed — use `bun prune --production` to strip existing devDeps)
  [3](https://bun.com/docs/pm/cli/install).
* `bun install --frozen-lockfile --dry-run` validates without installing; `--offline` / `--prefer-offline`
  (or `install.offline = true` in `bunfig.toml`) drive cache-only installs, and "with a complete restored
  cache, `--offline --frozen-lockfile` makes a CI install fully deterministic and network-free"
  [3](https://bun.com/docs/pm/cli/install).
* **Bun does not execute lifecycle scripts of *installed dependencies*** (supply-chain hardening), only
  your own `pre/postinstall|prepare` [2](https://bun.sh/docs/pm/cli/install).
* Do not rely on `node_modules`/cache layout as an API: "the implementation details of Bun's install
  cache will change between versions… use `Bun.resolveSync`/`import.meta.resolve`"
  [7](https://nodejs.libhunt.com/bun-alternatives).
* Classic failure in CI: *"lockfile had changes, but lockfile is frozen"* — `package.json` changed without
  regenerating the lockfile; fix locally with `bun install`, commit the lock
  [1](https://forum.codecrafters.io/t/errors-with-bun-lockfile-had-changes-but-lockfile-is-frozen/193).

**Three-lockfile repo, therefore:** `pixi.lock` (conda+binary layer) · `bun.lock` (JS layer) ·
`Cargo.lock` (Rust layer). `.gitignore` `node_modules/` and `.pixi/` (pixi's env dir is always generated),
commit all three locks, and gate with `pixi install --frozen` + `bun ci` + `cargo build --locked`.

---
