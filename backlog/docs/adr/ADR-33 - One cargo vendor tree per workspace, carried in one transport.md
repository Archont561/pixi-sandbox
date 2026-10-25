---
id: ADR-33
title: One cargo vendor tree per workspace, carried in one transport
date: '2026-10-25'
status: accepted
tags:
  - decision
---

# ADR-33 — One cargo vendor tree per workspace, carried in one transport

**Extends:** [ADR-6](./ADR-6%20-%20Vendored%20cargo%20crates%20live%20under%20.pixi-sandbox-vendor.md)
(vendored crates live under `.pixi-sandbox/vendor/`). **Related:** ADR-32 (one branch per
bundle/platform, payload stays self-contained), TASK-129 (nested Pixi workspaces), TASK-131.

## Context

ADR-6 decided one `cargo vendor` tree keyed by the root project's single `Cargo.lock`, recorded
as one `Vendor` record in the manifest. TASK-129 then made one transport carry several *Pixi*
workspaces, each a distinct project root with its own lock and prefix. A nested Rust project
with its own `Cargo.lock` had no place in that model: `cargo vendor` keys on one lockfile, two
vendors would race on `.pixi-sandbox/vendor`, and the config loader refused the declaration
with a named "needs TASK-131" diagnostic rather than half-implementing it.

## Decision

One transport carries **one vendor tree per project that owns a `Cargo.lock`**, reusing the
ADR-6 mechanics per tree rather than inventing a second vendoring system.

1. **Layout.** The root tree keeps ADR-6's `.pixi-sandbox/vendor/` unchanged. A nested
   workspace `subproject/` gets its own tree at
   `.pixi-sandbox/vendor/workspaces/<slug>/` in the transport, restored to
   `subproject/.pixi-sandbox/vendor/` — beside that workspace's own `.pixi/envs`, never
   flattened into the root. The slug is the same TASK-129 identity the environment ids use.
2. **Manifest.** Schema 5 adds `workspace_vendors: []`, one `WorkspaceVendor` per tree carrying
   the slug, manifest path, directory and an unchanged `Vendor` payload (mode, crate count,
   size, lock hash, verified blob list). The schema bump is required even though the field is
   additive: an older binary would ignore the key and "verify" while never checking the nested
   blobs — a silent partial verification. Producers write schema 5 for every transport;
   readers keep accepting schemas 1–4.
3. **Pack.** With `--cargo-vendor`, every distinct project root holding a `Cargo.lock` gets its
   own `cargo vendor --locked --versioned-dirs` invocation, its own duplicate-source preflight
   against its own lock, and its own scratch directory. A workspace without a `Cargo.lock` is
   simply not vendored. `--no-vendor` (and not passing `--cargo-vendor`) skips every tree.
4. **Restore wiring.** Each tree is staged, verified and renamed into its owning workspace, and
   `--cargo-config` (auto/write/print/none) applies **per tree**: auto writes
   `<directory>/.pixi-sandbox/cargo-home/config.toml` plus activation hooks into that
   workspace's own env prefixes. Cargo discovers config by walking up from its working
   directory, so a build inside `subproject/` reads that project's wiring and never the root's.
5. **Shared crates are counted, not deduped.** When two lockfiles pin the same crate+version,
   `--versioned-dirs` produces two identical directories under two transport paths: two blob
   lists, two verifications, two checkouts. The payload and every ceiling count both copies.
   Git's content-addressed store keeps one underlying object, so branch *storage* is smaller;
   the manifest's overlap analysis (`vendor_overlap`) and the pack/README output name the
   shared crates and single-copy bytes explicitly, with the note that checkout and
   verification still pay twice. No cross-tree hardlinks or merged tree: either would couple
   two independently restored projects and make a partial restore ambiguous.
6. **The nested-`Cargo.lock` refusal is removed.** It was a pack-capability limit expressed as
   validation, not a property of a valid repository.

## Why these alternatives were rejected

- **One merged vendor tree for all lockfiles.** `cargo vendor` keys on one lockfile; the
  per-lockfile duplicate-source refusal is meaningful precisely because the locks differ; a
  merge would need a new conflict policy the two-project case exists to avoid.
- **Nested crates prefixed under the root tree.** Two projects would then share one directory
  and one config snippet, reintroducing cross-project coupling and making a per-workspace
  restore ambiguous.
- **A manifest map keyed by slug.** The rest of the manifest uses ordered lists
  (`workspaces[]`, env records); a list with the slug inside matches that precedent and keeps
  serialised bytes stable without map-ordering rules.
- **Silently deduping shared crates in the byte counts.** The airlock really does check out and
  hash both copies; under-reporting would make the transport ceilings lie.

## Consequences

- A root-only or non-Rust transport is byte-identical to the ADR-6 era apart from the schema
  number; the new key is skipped when empty.
- Restore space preflight counts twice the recorded size of **every** tree; the transport
  ceiling counts every vendor blob once per tree, and `plan` reports each workspace's
  vendorable crate count before a runner packs it.
- Git-source dependencies are not wired by the restore-generated config, which emits the
  crates.io source replacement ADR-6 defined; registry crates remain the supported vendor
  source. Supporting non-registry sources would amend this record.
- The decision is exercised end-to-end by an ignored Linux gate (`test-vendor-offline`): two
  cargo projects with real registry crates are packed and restored, then both build with
  `cargo build --offline` inside a fresh network namespace, and a cold negative control with
  the tree removed fails.
