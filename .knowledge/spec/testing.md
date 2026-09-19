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

**What is verifiable in this sandbox, measured 2026-09-19** — the answer changes completely if the
artifact is *shell + YAML* instead of Rust, because then the author's box can execute the thing it is
shipping. Probed with a throwaway npm project (registry reachable ✅):

| Checker | Usable here? | How it was established |
|---|---|---|
| `bash -n script.sh` | ✅ | always; syntax only |
| `npx js-yaml file.yml` | ✅ | `npm i js-yaml` (pure JS) — parses `action.yml`, and **fails loudly** on malformed input (`YAMLException: deficient indentation (2:1)`) ✅ |
| `shellcheck` (npm) | ❌ | package installs (4.1.0 ✅) but its postinstall downloads the real binary → `unable to verify the first certificate` ✅ — pure-JS wrappers still fetch native code |
| `actionlint` (npm) | ❌ | 2.0.6 installs but exposes no runnable bin in this sandbox ✅ |
| `zizmor` | ❌ | not on npm ✅ |
| `pixi` / `pixi-pack` / `cargo` | ❌ | the whole point (egress matrix) |

Consequences: **(1)** YAML and shell *syntax* + a hand-written schema check are testable in the airlock;
**(2)** semantic linting (shellcheck/actionlint/zizmor) belongs in CI, which costs nothing since CI is where
the action runs anyway; **(3)** the shipped script must therefore be **self-testing** — `assemble.sh
--self-test` (stub `pixi`/`tar` on `PATH`, fixture branch, assert the file set + digests) is what lets a
sealed box prove the thing rather than trust it. Prototype run here: the script's verify→install→unpack→
cargo-wire sequence executed green against stubs, and returned the integrity failure when a checksum was
perturbed 🚧 *(single local run, not CI)*.

---
