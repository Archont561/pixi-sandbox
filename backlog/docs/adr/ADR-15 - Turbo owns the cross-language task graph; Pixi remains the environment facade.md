---
id: ADR-15
title: "Turbo owns the cross-language task graph; Pixi remains the environment facade"
status: accepted
tags:
  - decision
---

# ADR-15 — Turbo owns the cross-language task graph; Pixi remains the environment facade

**Decision.** Adopt Archont561/pathway's current orchestration model (task-70), superseding the
JS-only deferral recorded by task-26 and ADR-25. Bun owns the workspace dependencies;
Turbo owns build/check ordering, affected selection and the local task cache; Pixi remains the
only environment and command entrypoint. The `default` environment therefore carries Bun with
the Rust and repository tools, and the published developer bundle contains that one environment
instead of parallel `default` and `web` prefixes.

Each Cargo crate is a private Bun workspace package only for `test` and `lint`: its scripts run
`cargo nextest -p …` plus doctests and crate-scoped clippy, and its workspace dependencies mirror
the Cargo path edges so `turbo --affected` includes dependants. Cargo is still authoritative for
compilation and dependency resolution. Workspace-wide operations that cannot be split honestly
(rustfmt, cargo-deny and llvm-cov) live in the single `@repo/rust` package. Pixi's public verbs
(`test`, `lint`, `fmt`, `coverage`, `docs`) are thin facades over those scripts; Cargo flags have
one owner rather than being copied into Pixi tasks.

**Why this reverses the earlier decision.** Task-26 measured only the JavaScript surface and was
correct that one docs package did not earn an orchestrator. Pathway supplied a new, measured
use case: Rust crates can participate as task packages without replacing Cargo, allowing docs-
only and unrelated-crate changes to replay successful checks while retaining nextest process
isolation. pixi-sandbox has four Rust crates with real dependency edges, so the relevant graph is
no longer the one-package JS graph task-26 evaluated.

**Cache policy.** `.turbo/cache` is local and restored by CI using OS/ref/commit keys; Turbo
rehashes declared inputs, including Cargo and Pixi lockfiles, before accepting a hit. Remote
caching remains off because a third-party cache round-trip conflicts with the airlock posture.
Mutable tasks and release-producing commands are not cacheable. The full suite remains the
merge gate; `--affected` is a focused developer command, not permission to skip required CI.

**Evidence ✅.** The adopted graph parses at the public dry-run seam, selects the four Rust
packages from `@repo/xtask...`, and `pixi run test` executes the same 626-test baseline through
four crate-scoped nextest invocations (1 skipped). `pixi run lint` covers per-crate clippy plus
the workspace and repository gates, and `pixi run docs build` produces the Astro site through
the same graph. Pathway commit `4d2ebac` is the reference shape; pixi-sandbox keeps its stricter
Pixi-only entrypoint and generated-workflow checks.

**What would change it.** Incorrect invalidation, a measurable regression from Cargo target-lock
contention that outweighs affected/cache wins, or inability to keep the Cargo and workspace
edges mechanically aligned would collapse Rust back to one opaque `@repo/rust` task package.
