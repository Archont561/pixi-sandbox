---
id: ADR-6
title: "Vendored cargo crates live in `.pixi-sandbox/vendor/`"
status: accepted
tags:
  - decision
---

# ADR-6 — Vendored cargo crates live in `.pixi-sandbox/vendor/`

**Decision.** `pack --cargo-vendor` writes `cargo vendor` output to
`.pixi-sandbox/vendor/` — a sibling of `envs/` and `tools/`, not a child of an env — records
every vendored file as a manifest blob, and `restore` materialises it to
`<project>/.pixi-sandbox/vendor/` and wires `.cargo/config.toml` (relative `directory`).
Default mode `loose`; `tarballs` available.

**Why.** Crates are project-scoped, not env-scoped (one `Cargo.lock`); `envs/<env>/pack` must
stay byte-for-byte what `pixi-pack` produced so it can be re-packed and compared; blob
accounting and `doctor` iterate environments and would lie if a crate tree hid inside one; a
partial `restore --envs dev` still needs the crates; and the branch layout then mirrors where
the tree lands in the project. Full reasoning: `doc-2` §11.

**Evidence ✅.** 33 crates ≈ 35 MB / 1441 files loose; per-crate-bump git cost +0.07 MiB vs
+3.96 MiB for a single archive; clone of the vendored tree 390 ms (loose) vs 59 ms (tarball);
a severed-network debug build with an empty `CARGO_HOME` succeeds in 13–18 s; a registry-cache
fallback (9.7 MB) is possible but cannot be verified the same way.

**What would change it.** A project whose crates change on every commit (tarball mode), or
cargo gaining a first-class "vendor into a prefix" mode that resolves paths itself.
