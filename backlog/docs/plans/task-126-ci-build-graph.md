# TASK126: log-backed CI build graph follow-up

## Baseline and scope

Authorized 2026-10-22 after reviewing merge
`97bb24c68475d7e29e519cb2fffb22abcd145a57` and successful CI
[39813564355](https://github.com/Archont561/pixi-sandbox/actions/runs/39813564355).
Linux elapsed 707s: lint ~273s, normal tests ~139s, coverage ~190s, payload ~12s,
Rust-cache post ~43s. These are step timings, not exclusive CPU or billed time.
Turbo groups buffer output; Cargo elapsed figures include explicit lock waits.

- Lint: all 11 actual task misses despite a restored Turbo archive; multiple Clippy processes
  explicitly waited on package/build locks. Release planning incurred an optimized product build.
- Coverage: 161s compilation, 22.494s execution; 1,175 passed / 2 skipped, matching ordinary
  suites across 95 binaries. Reporting was not the dominant cost.
- Payload: 10.17s compile / 2.06s execution. Keep this connected-build/network-denied proof.
- Rust cache missed, later saved ~2.1GB compressed. Cache action fingerprinted host Rust 1.99.1
  while Pixi installed 1.98.1. Publisher's host Rust was 1.99.0.

## Changes and boundaries

1. One `cargo clippy --workspace --all-targets -- -D warnings` in `@repo/rust`; remove competing
   per-crate lint scripts. Ordinary local per-crate tests remain available and unchanged.
2. Turbo orders workspace lint -> generated workflow validation -> debug source planner ->
   repository consistency. Non-Cargo action/TOML/Biome checks can still overlap. Rooted source,
   test and manifest inputs invalidate the workspace Clippy task; automation tasks include the
   linked source inputs too. The planner remains source-fresh, not a released/cached executable
   accepted without Cargo's validation. No static release profile changes.
3. Main Linux CI uses full workspace coverage as the ordinary test gate, plus actual workspace
   doctests (`cargo test --doc --workspace`) and the separate ignored payload proof. No nextest
   filters/features or production-file coverage exclusions added. Local `pixi run test` is kept.
   Coverage remains uncached and fail-closed; only upload failure remains nonfatal as before.
4. Pin CI incremental/dev-debug/test-debug to zero; Turbo strict mode forwards **and hashes**
   Cargo profiles, incremental, build target and Rust flags. Offline/Cargo-home settings pass
   through without changing semantic cache identity. CI no longer depends on loose env mode.
5. Rust cache's supported `cmd-format: pixi run --frozen -- {0}` fingerprints the compiler Cargo
   actually uses, plus the Pixi lock in its key. The pinned cache action may additionally include
   installed rustup toolchains; this is not a claim that all unrelated host invalidation is gone.
   Instrumented coverage retains llvm-cov's separate target directory. Codecov searches only the
   explicit LCOV input, not Pixi task-cache JSON. Publisher cache-on-failure preserves compilation
   across later packing/verification failures without relaxing any publication checks.
6. Public `xtask check-ci-builds` guards the single-suite route, mandatory gates, Clippy ownership,
   debug planning, serialized graph, Pixi cache identity and hashed profile flags. Five CLI tests
   use tempdir fixture repositories, including 18 weakened configurations. They do not read this
   checkout or simulate native/offline evidence.

MSRV/Windows scheduling and product/xtask dependency extraction are deliberately unchanged: extra
runner jobs may trade billed cost for latency, while splitting runtime dependencies is broader work.
No cache/artifact from a PR is promoted into privileged publication. No permissions, protections,
release dispatches, five-platform requirements or static/offline verification gates were weakened.

## Local evidence (not CI savings)

- Native producer red: removing v7-elided build_number from the existing genuine-archive fixture
  reproduced the exact publisher error at the public CLI. Green: pack -> restore -> unpack.
- Six compact-lock channel tests initially failed for lost noarch metadata, then passed after
  implementing the v7 inference rule. Native suite: 36 pass. Explicit metadata remains authoritative.
- New automation CLI red: unknown `check-ci-builds`; green: fixture success plus refusal cases.
  The checker rejected the old real CI config before it was updated (missing doctest gate).
- Separate refactor after green: split the CI route check into a private helper, retaining public
  CLI coverage. No unrelated dependency-boundary or transport redesign.
- Full strict-mode uncached local lint: **7/7 tasks pass**, including workspace Clippy, deny,
  actionlint, generated workflows, source planning, TOML/Biome and repository consistency.
- Full strict-mode uncached local normal suite: **1,186 pass / 2 skipped** (54 git, 295 core,
  581 product, 256 xtask); workspace doctests pass (currently zero examples, including xtask).
- Turbo dry runs prove profile flag changes affect the workspace lint hash; 96 source inputs
  were present. No release-mode command or competing Cargo lint command appears in the task graph.

Additional final validation:

- Full workspace coverage: **1,186 pass / 2 skipped**, LCOV generated successfully.
- Exact JSON inventory comparison: all **1,188 entries across 96 binaries**, including identical
  ignored and filter flags, match between original per-package ordinary lists and instrumented
  workspace lists. This confirms the single-suite CI route has not dropped tests.
- After coverage/docs/test execution, warm lint: **7/7 actual Turbo hits**. A red/green CLI
  regression now guards exclusions for generated reports/logs. Dry-run hashes stay unchanged
  when only LCOV and per-package Turbo logs change; build-profile changes do invalidate them.
  Exclude generated Astro/dist/node_modules content from the Biome task hash too.
- Docs build, frontmatter, links, accessibility and CLI snippets pass: 29 pages, 62 commands,
  123 flags. Formatting, actionlint, generated workflows and repository consistency pass.
- Connected payload proof was retained, not run locally: this sandbox cannot reproduce the
  connected builder/native/network-denied runner tier. MSRV and real Windows execution likewise
  remain CI evidence, not simulated local results.
Successful post-change CI/publisher runs, controlled cold/warm/invalidation scenarios, real native
proofs and billed-minute comparisons remain open. Do not claim measured savings from warm local
checks versus cold GitHub runners or compare a failed publisher to a successful one.
