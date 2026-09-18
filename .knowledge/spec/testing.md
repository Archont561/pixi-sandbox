---
type: Design Spec
title: "Testing Without a Compiler"
description: The strategy that keeps a Rust CLI trustworthy when the authoring box cannot compile it: fixtures, snapshot plans, and CI-only proofs.
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, testing]
status: stable
confidence: mixed
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`DESIGN.md`], sections: ["14"] }
---

# Testing Without a Compiler

## 14. Testing strategy under a "no compiler here" constraint

The honest structure, given that `cargo` cannot run in this sandbox:

1. **Pure-logic unit tests** (runnable in CI on the first push): platform↔triple mapping, kebab↔snake
   serde round-trips, `deny_unknown_fields` rejection, path safety (no `..`, no absolute escape from
   `artifacts-dir`), lockfile upsert/dedup, sha256 vectors, `parse_sha256sums` layouts, `sha256 of "" and
   "abc"` known constants, tar-name determinism, `--platform` passthrough of named platforms, semver-ish
   `min-version` compare. No process spawning in these.
2. **`Cmd` assembly tests — the highest-value tests in the repo.** Assert *exact argv* for every
   subcommand against golden JSON in `tests/golden/`. No pixi, no cargo, no network: `--dry-run --json`
   prints the plan, and the test compares the plan. This converts "I couldn't compile" into "the command
   lines are pinned", which is where this kind of tool actually breaks.
3. **`insta` snapshot tests** for `--json` and `--help` output (upstream uses insta ✅ verified) — cheap
   CLI regressions, including the `explain` table.
4. **Integration tests behind `#[ignore]` + `--ignored`**, gated on `PIXI_SANDBOX_ITEST=1`, that require
   real `pixi`/`cargo`/`bun`. Run in CI's `pack`/`build` jobs only, never in unit-test jobs.
5. **A `contract` job that pins the *world's* behaviour**, because our assumptions are external: assert
   `pixi-pack --version` is within the supported range, that `--ignore-pypi-non-wheel` still exists, that
   `bun --version` ≥ 1.2 (text lockfile), and that conda-forge's `bun` still lacks `win-64` — i.e.
   **turn today's manual verification into a failing test the day upstream moves.** ⚠️ Fetching that
   metadata needs network, so it runs on a scheduled CI job, not per-PR.
6. **What is *not* tested, explicitly**: real `pixi-pack` byte-for-byte reproducibility (upstream has a
   `test_reproducible_shasum` test ✅ verified — we assert our manifest records the hash, not that the
   hash is stable) and Windows path handling in CI without a Windows self-hosted runner. 🚧

**In-sandbox verification available right now** (useful for the Node half, measured in this session):
`bun install`/`bun ci`/`bun.lock` all work, so `node sync`/`node check` logic can be *exercised* here even
though the Rust half can only be reviewed. 🚧 Proposal: keep an `examples/js-fixture/` (package.json +
lockfile) so `node` verbs have something real to run against in CI.

---
