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

## 2026-10-25 — native evidence and closure (v0.9.0 era)

ADR-30 names the close-out conditions: native consumer runs, the complete online consumer
lifecycle, and before/after timing evidence for docs-only PRs, manifest PRs and consecutive merges.
All three are now observable on native runners. The repository is public, so Actions minutes are
not metered — billed-minute comparisons are structurally N/A, and the ADR-30 position ("no
automatic one-minute saving is claimed") stands rather than being reversed.

### 1. Verified standalone bootstrap on native runners (AC#1)

The generator's `release_bootstrap()` (curl over HTTPS-only, OS/arch case from `Platform`,
`SHA256SUMS` extraction with duplicate/malformed-entry refusal, `sha256sum`/`shasum` fallback,
chmod and PATH registration) shipped with ADR-30, the golden fixture and fake-download execution
tests. Native executions of the identical download-and-checksum primitive:

| Run | Step | Wall time |
| --- | --- | ---: |
| [consumer-proof 40206748749](https://github.com/Archont561/pixi-sandbox/actions/runs/40206748749) | Download and verify standalone release asset | 2s (7s job) |
| [starter 40206750498](https://github.com/Archont561/pixi-sandbox/actions/runs/40206750498) | Download the released binary + verify against SHA256SUMS | 1s + 1s |
| [airlock 40208159519](https://github.com/Archont561/pixi-sandbox/actions/runs/40208159519) | Download the static released binary to embed | 96s |

The airlock row is a different shape: a tag-keyed self-bin cache whose first fetch for a new tag
misses (96s) and whose later runs restore from cache — not a bootstrap regression and not compared
against the 2s rows. The historical "before" stays TASK-118's recorded ~10s conda-install
overhead (32% of a 31s publish job): historical context, not a newly measured saving. Observable
boundary, stated plainly: the exact generated consumer YAML executes in `init` consumer
repositories that are not visible from here; in-repo coverage is the native primitive runs above
plus the generator's ten-case fake-download execution suite.

### 2. Plan job on the standalone binary, dynamic matrix retained (AC#2)

The golden render (`fixtures/generated/publish-sandbox.yml`) shows all three jobs — plan, publish,
upgrade — opening with "Fetch the pinned pixi-sandbox release"; the plan job contains no
conda install. The dynamic-matrix decision and its billing caveat are documented in ADR-30
("a faster dynamic plan still consumes a VM and may still round to the same billed minute"), and
the static-matrix alternative is deliberately rejected there so the reviewed config stays
authoritative. Supporting current-revision timing for planning work: this repository's
source-built planner runs at 24s / 34s / 47s across three consecutive main pushes
(`40144875662`, `40199680995`, `40205605553`) against the 152s pre-optimization baseline recorded
in the TASK-127 ledger — a build-profile and rust-cache effect, not a consumer-billing claim.

### 3. Docs-only, code and manifest PRs on real runners (AC#4)

| PR / run | Diff class | ci span | Rust gates | Docs/xtask gates | Extra jobs |
| --- | --- | ---: | --- | --- | --- |
| [#212 / 40202531135](https://github.com/Archont561/pixi-sandbox/actions/runs/40202531135) | backlog markdown | 50s | all skipped | ran | MSRV/Windows skipped |
| [#213 / 40210180657](https://github.com/Archont561/pixi-sandbox/actions/runs/40210180657) | backlog markdown | 63s | all skipped | ran | MSRV/Windows skipped |
| [#211 / 40198959531](https://github.com/Archont561/pixi-sandbox/actions/runs/40198959531) | Rust code | 123s | all ran (1,277 tests) | docs-only steps skipped | MSRV 113s, Windows 94s ran |

Lock/manifest safety: the relock guard fired on the `pixi.toml`-touching PR run
[40198959456](https://github.com/Archont561/pixi-sandbox/actions/runs/40198959456) (17s guard,
no relock needed), while no relock run exists for the markdown-only SHAs `c2644bd`, `ae535ed` or
`22ed25f` — the TASK-121 allowlist still schedules on manifests and lockfiles and nothing else.
The filter remains repository policy in the hand-maintained `ci.yml` (there is no CI-workflow
generator to carry a config flag), with the `PIXI_SANDBOX_FULL_CI=true` escape hatch and full
gates on every push event, so main integration is never filtered.

### 4. Full verification and consecutive merges (AC#5)

- fmt, workspace clippy, the 1,277-test suite, coverage and every xtask gate are green on current
  main (ci `40203617419`, `40205605597`, `40210681131`); the cargo-deny advisories that blocked
  the 2026-10-22 lint were remediated under TASK-133.
- Native five-platform release `40205607244` is green end to end, including the Windows
  `x86_64-pc-windows-msvc` leg (672s), both macOS legs (740s / 508s) and the per-platform conda
  smoke tests; the starter dispatch `40206750498` proves byte-identical regeneration and a fresh
  clone's dev task.
- Consecutive merge stability, publish job wall time on `ded3bd62` → `823a4b84` → `603166a4`:
  215s / 201s / 213s; planner spans in the same series: 24s / 34s / 47s. No billing comparison is
  attached because, again, the public repository has no metered Actions minutes.

No release, dispatch or protection change was performed to collect this evidence; every run cited
belongs to sanctioned release-week work or ordinary pull-request activity.
