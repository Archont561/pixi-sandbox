---
type: Requirement Traceability
title: "Where each of the five stated requirements landed"
description: Traceability table mapping the five requirements (a)-(e) to the mechanisms that answer them, and an honest note on which answer is still a probe.
resource: https://github.com/Archont561/pixi-sandbox
tags: [overview, requirements]
status: stable
confidence: mixed
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`DESIGN.md`], sections: ["rev.2"] }
---

# Where each of the five stated requirements landed

## Rev. 2 — where your five requirements landed

| You asked for | Design answer | In |
|---|---|---|
| **(a)** pack an **arbitrary environment**, any manifest, no config edit | `pack [ENV…]` positional + `--all-envs`, `--manifest-path` to any repo; env names validated against the inventory, unknown names list what *was* detected | [§4.4](/spec/architecture.md#44-discovery-the-inventory), [§8.1](/spec/packagers.md#81-pack--pixi-environments--environmenttar) |
| **(b)** **detect** what's available (envs, features, toolchains, hosts) | the `Inventory`, built in **tiers L0→L3** so detection works where the tools themselves are blocked; config becomes an override | [§4.4](/spec/architecture.md#44-discovery-the-inventory), `inventory`/`environments`/`doctor` in [§5](/spec/cli.md#5-command-surface) |
| **(c)** decide what to bundle **from `Cargo.lock` / `bun.lock`** | `--components auto`: marker files *are* the spec — `pixi.lock`→env, `Cargo.lock`→vendor, a JS lockfile→node, own binary→self; absent marker ⇒ `not-applicable`, never an error | [§4.4](/spec/architecture.md#44-discovery-the-inventory), [§8](/spec/packagers.md#8-the-three-packagers), [D13](/spec/decisions.md#16-decision-log) |
| **(d)** define a **target platform** and check it exists | `--target` + `pixi sandbox platforms --explain`: **P0 declared / P1 locked / P2 published**, with `known-gaps` as dated data and the per-feature `platforms = [...]` fix | [§7.4](/spec/toolchain-resolution.md#74-platform-validation-does-this-platform-actually-exist) |
| **(e)** **ship itself + `pixi`** in the branch so `pixi run` and workspace management keep working | the dist branch carries `bin/pixi-*` + `bin/pixi-sandbox-*` + `.conda` files; a kit is a *ladder* — **R1** copied package cache → **R2** the pack's own `channel/` served as a `file://` channel (both *documented*, and R2 even allows offline `pixi add`) → **R3** `pixi-unpack` via rattler's installer → **R4** `environment.yml` + micromamba → **R5** `tar`-only rescue; remote `[mirrors]` where an org mirror exists → whose rung is chosen by probe and printed | [§9.4](/spec/artifacts.md#94-reconstruction-making-pixi-run-work-offline), [§10.5](/spec/git-registry.md#105-self-hosting-the-branch-ships-pixi-and-the-tool), [§11.1](/spec/bootstrap.md#111-the-bootstrap-revised-for-pixi-must-actually-run) |

What got *harder*, honestly: (e) is the only requirement whose **preferred** mechanism (**R1**, copying
`$PIXI_CACHE_DIR/pkgs` and letting `pixi install --frozen` do the work) is still not documented as a
supported *combination* — the pieces are verified, the composition is a probe. (Reading pixi's configuration
docs for this revision paid for itself: `file://` channels ✅ and "a local channel needs no download even in
`--offline`" ✅ are documented, so **R2 is not a gamble** and the guarantee moved there; `[mirrors]` is for
*exact-copy remote* channels ✅, never promised for a local dir.) A ladder with a CI gate ([M4.5](/spec/roadmap.md#15-roadmap-and-acceptance-gates)) rather than a promise, and why I will not claim a local
directory works as a pixi `[mirrors]` target until something proves it.

---
