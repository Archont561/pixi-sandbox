---
type: Goals and Non-goals
title: "Goals G1-G9 and what this project refuses to do"
description: Nine goals (G1 zero-config packing, G2 detect-first, G3 lockfile-derived bundles, G4 platform existence checks, G9 dogfooding) plus the explicitly rejected approaches.
resource: https://github.com/Archont561/pixi-sandbox
tags: [overview, goals]
status: stable
confidence: reasoned
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`DESIGN.md`], sections: ["2"] }
---

# Goals G1-G9 and what this project refuses to do

## 2. Goals / non-goals

**Goals**

1. **G1 — Single verb.** `pixi sandbox <thing>` reads like `cargo <thing>`; no flag soup, no 12-step README ritual.
2. **G2 — Git-only portability.** Every artifact consumable over `git` alone, in both directions (publish + install).
3. **G3 — Hermetic by default, explicit about it.** `--locked`/`--frozen` semantics inherited from pixi and cargo; if something could drift, the tool says so instead of silently solving.
4. **G4 — Plan/apply split.** `--dry-run` prints an exact, ordered command list; the same invocation without it executes that list. Anything that mutates must be previewable.
5. **G5 — Honest about capability.** `doctor` tells you *why* it can't do something ("crates.io is blocked", not "error").
6. **G6 — Auditable provenance.** Every artifact records which *source* produced it, not just its version (see [C1/C3](/research/corrections.md#14-corrections--method-notes): conda-forge `bun` = 1.3.11 vs upstream 1.4.2 — same name, different runtime).
7. **G7 — Zero config to be useful.** Detection (`Inventory`, [§4.4](/spec/architecture.md#44-discovery-the-inventory)) must cover: *any* environment in *any* manifest (a stranger's `pyproject.toml` is in scope), *any* target
   platform (validated, [§7.4](/spec/toolchain-resolution.md#74-platform-validation-does-this-platform-actually-exist)), and *which* artifacts
   are needed (`Cargo.lock`/`bun.lock` decide, [§8](/spec/packagers.md#8-the-three-packagers)). Config is an **override, not a
   prerequisite** — a repo with no `pixi-sandbox.toml` must still get a correct `plan`.
8. **G8 — The kit must make pixi work.** A `git clone` of the dist branch on a machine with nothing but
   `git` + `tar` must yield: the `pixi` binary, `pixi-sandbox`, a manifest + `pixi.lock`, and usable
   `.pixi/envs/<name>` prefixes — so `pixi run <task>` and workspace management behave offline
   ([§9.4](/spec/artifacts.md#94-reconstruction-making-pixi-run-work-offline), [§10.5](/spec/git-registry.md#105-self-hosting-the-branch-ships-pixi-and-the-tool)).
9. **G9 — It must build itself.** `pixi-sandbox` is developed *in* the environment it produces: a box with
   this repo's measured egress ([Host Inventory and Verdict](/environment/inventory.md): git ✅, crates.io ❌,
   conda ❌, no `rustc`) must be able to fetch, verify, reconstruct and *use* a kit that yesterday's
   `pixi-sandbox` published. Any dev-loop step that needs a host outside the allowlist is a design bug, not
   a sandbox limitation — hence the tier ladder and the seed rule in
   [Dogfooding on a Box Like This One §7](/workflows/dogfooding.md#7-dogfooding-developing-pixi-sandbox-on-a-box-like-this-one). Note the
   asymmetry: this box cannot compile *Rust*, yet it can install 227 MB of npm deps and run a real
   `astro build` ✅ — so the docs site ([§9.6](/spec/docs-site.md#96-docs-a-starlight-site-published-twice--once-for-browsers-once-for-the-airlock))
   is dogfooded locally while `pixi-sandbox` itself is built by CI and consumed here.

**Non-goals**

* ❌ Zero-config does **not** mean "we guess". Anything detected at a heuristic tier is labelled
  `heuristic: true` in `plan` output, and `--strict` turns an unverified detection into a failure.
* ❌ Not a package manager, resolver, or reimplementation of pixi/cargo/bun — it **orchestrates** them.
* ❌ No HTTP client, no TLS stack, no async runtime (this is what keeps the vendored dep tree ~4 crates; see [D3](/spec/decisions.md#16-decision-log)).
* ❌ Not a Windows-first citizen on day one (see the `bun`/`win-64` gap in [§7](/spec/toolchain-resolution.md#7-toolchain-resolution--the-heart-of-it)).
* ❌ Not an artifact *server*; a git host is enough.
* ❌ Not a replacement for OCI images in production deploys (pixi's OCI mirroring is the right tool there; we only *emit* things those images can `COPY`).

---
