---
id: TASK-130
title: 'Add opt-in Go, NuGet and Maven dependency vendoring'
status: To Do
assignee: []
created_date: '2026-10-23 16:57'
updated_date: '2026-10-23 18:29'
labels:
  - vendor
  - pack
  - restore
  - offline
  - config
dependencies: []
references:
  - crates/pixi-sandbox/src/vendor.rs
  - crates/pixi-sandbox-core/src/sandbox_config.rs
  - crates/pixi-sandbox/src/commands/plan.rs
  - crates/pixi-sandbox/src/commands/pack.rs
  - crates/pixi-sandbox/src/commands/restore.rs
documentation:
  - >-
    backlog/docs/adr/ADR-32 - A transport branch is one bundle for one platform
    and its payload stays self-contained.md
priority: medium
type: feature
ordinal: 130000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Opt-in vendoring for non-Cargo dependency ecosystems (GitHub issue #195).

**Problem.** `pixi-sandbox` bundles solved Conda environments and has dedicated Cargo crate vendoring, but polyglot projects also resolve artifacts with language-specific package managers: Go modules (`go.mod`/`go.sum`), NuGet packages (`*.csproj`) and Maven artifacts (`pom.xml`, build plugins). Including the compiler or runtime in a Pixi environment does not make those project dependencies available after an offline restore, so a restored project still tries to reach the Go proxy, nuget.org or Maven Central.

**Requested outcome.** Opt-in, lockfile-driven vendoring providers that name their ecosystem and manifest/lock inputs explicitly — no automatic scanning or blanket downloading. The plan reports the inputs and an approximate size; packed artifacts are verified by content hash; restore and the subsequent build are demonstrably offline; an unsupported or incomplete dependency graph fails clearly before anything is written. Existing Cargo vendoring and Conda-only bundles keep their current behaviour.

**Suggested order from the report.** (1) Go modules via `go.mod`/`go.sum` and Go's vendor mode; (2) NuGet via a locked restore into a bundled local package source/cache; (3) Maven, which has no standard lockfile, so the resolved artifacts and plugins need explicit handling and integrity checks.

**Relationship to TASK-129.** Deliberately separate: TASK-129 covers nested Pixi workspaces and their Conda packages, this task covers non-Conda ecosystems. Neither blocks the other, although the reporting project needs both.

**Evidence boundary.** This airlocked workspace has no `go`, `dotnet` or `mvn` and only github.com/npm/PyPI egress, so a provider's real offline proof needs a toolchain added to the development environment or a connected CI runner; no vendor claim may rest on fixture bytes alone.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Seams and the first-ecosystem scope are agreed with the user before implementation: the provider boundary, the config surface, the restored-tree layout each provider writes, and the public test boundaries (fixture projects and tempdirs, never this repository — ADR-10). Recommendation: ship Go first and leave NuGet and Maven as separately reviewed slices.
- [ ] #2 A bundle can opt into vendoring providers by naming the ecosystem and its manifest/lock inputs explicitly; an unknown ecosystem, an absent or unreadable lock input, or one input claimed by two providers is a config error, and configs without the new declaration keep parsing unchanged.
- [ ] #3 The Go provider packs exactly the modules named by the declared go.mod/go.sum, verifies every module against go.sum, and restores a tree the project can build from with GOPROXY and network access unavailable; a tampered module fails verification instead of being restored.
- [ ] #4 The NuGet and Maven providers follow the agreed scope: locked restore into a bundled local package source for NuGet, and content-hash-verified resolved artifacts plus build plugins for Maven, whose lack of a standard lockfile is documented as an explicit decision rather than papered over.
- [ ] #5 plan reports every provider's inputs and an approximate packed size before anything is written, and an unsupported or incomplete dependency graph is refused with a named diagnostic before the restored environment is touched.
- [ ] #6 Offline build proof is recorded per provider on the strongest tier actually available (denied-egress fixture, or a real runner with the toolchain installed); what was not proven — native runners, live registries, incomplete ecosystems — is stated in the task notes instead of implied.
- [ ] #7 Documentation and generated artifacts are updated through their generators (configuration guide, CLI reference, transport layout), and the full suite's pass count rises or holds with fmt, lint and test green.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Each shipped provider has a red-then-green behaviour test at an agreed public seam, plus a failure test proving a bad digest or incomplete graph refuses rather than restores.
- [ ] #2 fmt, lint and test are green, the repository test baseline is recorded before and after and never reduced, and no existing Cargo-vendoring or Conda-only behaviour changes.
- [ ] #3 The task stays open until every acceptance criterion has evidence; unproven tiers and deferred ecosystems are named as open items, not checked.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Slice order, one ecosystem at a time and each reviewed before the next:

1. Agreement (AC#1): provider boundary, config surface, restored-tree layout and the first-ecosystem scope.
2. Config + refusals (AC#2): explicit provider declarations and their named diagnostics.
3. Go provider end to end (AC#3): pack from go.sum, restore, then a build with the proxy unreachable.
4. Plan reporting and fail-closed behaviour (AC#5) across the shipped providers.
5. NuGet (AC#4) and then Maven (AC#4), only after the Go slice is proven and the layout decision holds.
6. Documentation, generated artifacts and the full gates (AC#6, AC#7).

Evidence boundary: the toolchain for each ecosystem must exist in the development environment or on a runner before any offline proof is claimed; the current restored environment provides none of go, dotnet or mvn.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Portability conclusions recorded while scoping TASK-129 (full matrix in `backlog/docs/plans/task-129-nested-workspaces.md`); they shape the provider design, not the task scope.

Rust crates.io `.crate` files and `cargo vendor` output are platform-independent: source tarballs are identical for every target, per-architecture assembly ships inside one crate, and `cargo vendor --locked --versioned-dirs` reads the target-inclusive `Cargo.lock`. What varies is the *build* (compiler, `-sys` crates needing a C toolchain, build scripts), which the per-platform Conda environment supplies. Go module zips and `go.sum` are equally platform-independent; cgo needs a C toolchain and `GOOS`/`GOARCH` are compile-time. Therefore the Go and Rust providers can ship one vendor tree that serves every platform, consistent with ADR-32.

NuGet and Maven are *mixed*: `runtime.<rid>.*` packages and per-RID native assets are platform-specific and selected from `runtimes/<rid>/native/…` at restore, while Maven classifier JARs (`-linux-x86_64`, `-natives-*`) coexist with JARs that bundle all natives and with Java plugins that may shell out to a compiler; the JDK itself is per-platform and comes from the Conda environment. Those two providers must therefore resolve per-platform inputs instead of mirroring the single-lock model, and the plan must say which platform each resolved artifact set belongs to. No vendor claim may rest on fixture bytes: this workspace has no go, dotnet or mvn, so an offline build proof needs a toolchain in the development environment or a connected runner.
<!-- SECTION:NOTES:END -->
