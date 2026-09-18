---
type: Reference
title: "One Lockfile, One Digest"
description: The table every other workflow hangs off: which lockfile keys which component and which artifact carries its digest.
resource: https://github.com/Archont561/pixi-sandbox
tags: [workflow, lockfiles]
status: stable
confidence: reasoned
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`WORKFLOWS.md`], sections: ["0"] }
---

# One Lockfile, One Digest

## 0. One table, because everything hangs off it

| You changed… | Lockfile that must change | Who regenerates it | Offline proof | Kit component re-published |
|---|---|---|---|---|
| a conda dep (`pixi add rust`) | `pixi.lock` | `pixi add` / `pixi lock` ✅ | `pixi install --frozen` ✅ | `env` packs for **every (env × platform) whose hash moved** |
| a PyPI dep (`[pypi-dependencies]`) | `pixi.lock` (same file!) | same ✅ | same, **but** ⚠️ `uv` is not restricted to cached wheels by `--offline` ✅ | `env` packs; sdists can't be packed ✅ → `--ignore-pypi-non-wheel` |
| a JS dep (`package.json`) | `bun.lock` (or npm/pnpm/yarn equivalent) | `bun install` ✅ (`bun ci` ≡ `--frozen-lockfile` ✅) | `bun ci --frozen-lockfile` fails on drift ✅ | `node` tarball (keyed by lockfile digest) |
| a Rust dep (`Cargo.toml`) | `Cargo.lock` **and** `vendor/` | `cargo add` **before** vendoring, then `cargo vendor --locked --sync vendor` ✅ | `cargo metadata --locked --offline` ✅ | `vendor` artifact (keyed by `Cargo.lock` digest) |
| your own Rust code | nothing | — | `cargo build --offline` | `self`/`bin/*` + the injected `.conda` ([§4](/workflows/pixi-cargo-interop.md#4-pixi--cargo-interoperability-and-what-pixi-pack-does-not-carry)) |

The rule that keeps the airlock sane: **one dependency ecosystem = one lockfile = one digest = one
artifact key.** `pixi.lock` is *two* ecosystems (conda + PyPI) sharing one file, which is why a Python
bump re-packs the env too.

---
