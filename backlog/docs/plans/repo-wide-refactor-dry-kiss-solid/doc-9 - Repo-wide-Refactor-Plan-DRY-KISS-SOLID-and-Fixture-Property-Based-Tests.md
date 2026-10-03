---
id: doc-9
title: 'Repo-wide Refactor Plan: DRY, KISS, SOLID, and Fixture/Property-Based Tests'
type: specification
created_date: '2026-10-03 09:18'
updated_date: '2026-10-03 09:18'
tags:
  - refactor
  - dry
  - kiss
  - solid
  - testing
  - proptest
  - rstest
---
# Repo-wide Refactor Plan: DRY, KISS, SOLID, and Fixture/Property-Based Tests

## Status

**Complete.** All 8 sequenced tasks below landed as separate backlog tasks (task-55 through
task-62), each its own focused conventional commit with tests green before and after, per the
`.agents/skills/refactor/SKILL.md` identify-then-fix process. Item 9 (`split cli.rs`) was
dropped before implementation — see "Correction after checking the backlog" below. Final test
count: `pixi run --frozen test` → 556 passed, 1 skipped (up from the 516/1 baseline this plan
started from), with `clippy -D warnings`, `fmt`, and `lint` all clean as of the task-62 commit.

## Source context and method

Scanned: all four crates (`pixi-sandbox-core`, `pixi-sandbox-git`, `pixi-sandbox`, `xtask`) — 16,734 src lines + 9,169 test lines across 29 test files. Method: `wc -l` per file, a brace-depth heuristic to rank function length (two outliers from char-literal braces discarded — `asset_name_of` and `bash32_surface` are not actually 700-line functions, see the real sizes below), targeted `grep`/`rg` for duplicated literals and patterns, and manual reading of the highest-signal hits. No `cargo clippy --all-targets -- -D warnings` pedantic pass was run as part of this scan (the default lint set is already clean per `AGENTS.md`); running `clippy::pedantic` once is listed as a cheap follow-up in Workstream 4.

Baseline to protect while doing any of this: **`pixi run --frozen test` → 516 passed, 1 skipped** (2026-10-03, commit `08ea7ea`). Every workstream below must leave that number equal or higher, never lower, per the session skill's rule.

## Principles / guardrails for every item below

1. **Behavior preserved, one thing at a time.** No item here bundles a feature change; each is pure restructuring.
2. **No new crate dependencies.** `cargo` only builds against the vendored tree under `.pixi-sandbox/vendor` (D-series transport decisions; see `AGENTS.md` "Pixi is the only environment entrypoint"). A new proc-macro crate (`strum`, `derive_more`, …) cannot be fetched on this airlocked host and would have to be vendored through a full restore/relock cycle. Every suggestion below uses only what the workspace already depends on (`proptest`, `rstest`, plain `std`).
3. **Generated files are rendered, not edited.** `crates/pixi-sandbox/src/generated/{github_workflow,relock_workflow}.rs` and `.github/workflows/{publish-sandbox,relock}.yml` are templates and their committed renders. Where a finding touches one, the fix is in the Rust template function, followed by re-rendering — never a hand edit of the `.yml`.
4. **Tests still target fixtures/tempdirs, never this checkout** (D10). Every "add a fixture / property test" item below builds on `tempfile::tempdir()` or in-memory data, matching what's already there.
5. **Respect `.knowledge/decisions.md` (D1–D15).** Nothing here proposes a different transport, schema, or git-access path — this plan only touches internal code shape, not the design.

---

## Part A — `src`: DRY and SOLID findings

### A1. Platform identity is stringly-typed and hand-duplicated in 8+ places (highest-value finding)

The five Pixi platform strings (`linux-64`, `linux-aarch64`, `osx-64`, `osx-arm64`, `win-64`) and everything derived from them (Rust target triple, release asset name, GitHub Actions runner label) have no single source of truth. The same facts are re-declared, independently, as:

| Site | What it (re)computes | Evidence |
|---|---|---|
| `crates/pixi-sandbox/src/commands/init.rs::current_platform` (line 187) | `(os, arch) → platform` | 5-arm match |
| `crates/pixi-sandbox/src/standalone.rs::platform` (line 327) | `(os, arch) → platform` | **byte-for-byte identical** 5-arm match to the one above, just `Option` instead of `Result` |
| `crates/pixi-sandbox/src/self_update/assets.rs::SUPPORTED_HOSTS` (line 12) | `(os, arch) → release asset name` | 5-row table; its own doc comment says it must stay in sync "with what `xtask stage-release-binary` writes" — a documented manual invariant, not an enforced one |
| `crates/xtask/src/airlock.rs::static_asset_for_platform` (line 225) | `platform → release asset name` | another 5-arm match, same five asset-name string literals as `self_update/assets.rs`, written independently |
| `crates/xtask/src/release_assets.rs::staged_name`/`binary_name` (lines 18, 29) | `target triple → asset name` | the one *computed* (not hardcoded) version — the right shape, but not reused by the two tables above |
| `crates/pixi-sandbox/src/generated/github_workflow.rs::render_github_workflow` (lines 102–105, 122) | `platform → asset name`, rendered into generated shell *and* PowerShell | same five literals a third/fourth time, inside a format! template |
| `crates/pixi-sandbox-core/src/sandbox_config.rs` (`runner_for`, line 248) | `platform → GitHub runner label` | its own 4-arm match (win-64/osx-64/osx-arm64/linux-64) |
| `crates/xtask/src/conda_platforms.rs::SUPPORTED_PLATFORMS` (line 24) | the platform list itself | fifth independent copy of the 5-item array |
| `crates/pixi-sandbox-core/src/tools_lock.rs::PlatformPin` | per-platform tool pins keyed by the same strings | sixth place that must stay in lock-step |

None of these call each other. `self_update/assets.rs` even depends on `pixi-sandbox-core`, and `xtask` already depends on **all three** of `pixi-sandbox`, `pixi-sandbox-core`, and `pixi-sandbox-git` (`crates/xtask/Cargo.toml`) — so `xtask/airlock.rs::static_asset_for_platform` could call `pixi_sandbox::self_update::assets::asset_for` *today*, with no new wiring, and doesn't.

**Why it matters (not just taste):** adding a sixth platform — which `.knowledge/decisions.md` already flags as planned work (task-1, osx-arm64 was itself a late addition) — means touching 8+ files by hand, and a missed one fails silently at the worst time (a release that builds four platforms and silently drops the fifth, the exact failure mode `conda_platforms.rs`'s own doc comment warns about for a *different* step in the same pipeline).

**Proposed refactor** (Extract Class / Replace Magic Number with Constant, scoped to avoid new deps):

```rust
// crates/pixi-sandbox-core/src/platform.rs (new, no new deps)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Platform { Linux64, LinuxAarch64, Osx64, OsxArm64, Win64 }

impl Platform {
    pub const ALL: [Platform; 5] = [ /* ... */ ];
    pub fn from_os_arch(os: &str, arch: &str) -> Option<Self> { /* the one match */ }
    pub fn as_str(self) -> &'static str { /* "linux-64", ... */ }
    pub fn target_triple(self) -> &'static str { /* "x86_64-unknown-linux-musl", ... */ }
    pub fn asset_name(self) -> String { /* "pixi-sandbox-" + triple (+ ".exe" on Windows) */ }
    pub fn gh_runner(self) -> &'static str { /* "ubuntu-latest", ... */ }
}
impl std::str::FromStr for Platform { /* parses "linux-64" etc., used where config reads it */ }
impl std::fmt::Display for Platform { /* writes as_str() */ }
```

Then: `init.rs::current_platform`, `standalone.rs::platform`, `self_update/assets.rs::asset_for`, `xtask/airlock.rs::static_asset_for_platform`, `xtask/release_assets.rs::staged_name`, `sandbox_config.rs::runner_for`, `conda_platforms.rs::SUPPORTED_PLATFORMS`, and the `generated/github_workflow.rs` template all become one-line calls into `Platform`. One property test (`for p in Platform::ALL: Platform::from_os_arch's inverse round-trips`) replaces the ~6 near-identical "every arm is covered" unit tests scattered across those files today.

This is pure-data, zero I/O, so it belongs in `pixi-sandbox-core` next to `tools_lock.rs` (which already encodes per-platform pins and is the next consumer).

### A2. `xtask::repo_checks` is a 1,078-line module doing ten unrelated jobs (SRP/OCP)

`crates/xtask/src/repo_checks.rs` implements the "check-repository" gate as ten independently-numbered checks (badge drift, stale references, conda manifest versions, mutable git refs, channel drift, literal release tags, Bash-4 surface, multi-line workflow steps, relock staleness, generated-marker exemptions — see the section banners at e.g. line 454 `// ---- 9. workflow step shape`). Each is a free function with the same ad-hoc signature `fn(&Path, &mut Vec<Failure>) -> Result<()>`, they share a few cross-cutting helpers (`lines_without_opt_out`, `rel`, the `*_ALLOWED` opt-out marker convention), and `check_repository` presumably calls all ten in sequence from one `main.rs`/dispatcher.

This is a God Module, not a god function: adding an eleventh check means growing an already-1000-line file further, and nothing stops check 3 from reaching into check 7's helpers by accident (no module boundary enforces it). It is also the single largest reason `xtask` has zero fixture/property tests (Part B.1) — ten unrelated policies sharing one file and one test module makes it hard to give any one of them a focused, parametrized test suite.

**Proposed refactor** (Extract Class, Replace Conditional with Polymorphism — lightly, no trait object ceremony needed for a Vec of ten pure functions):

- Split into `crates/xtask/src/repo_checks/{mod.rs, badges.rs, stale_refs.rs, conda_manifest.rs, mutable_refs.rs, channel_drift.rs, release_tags.rs, bash32.rs, workflow_shape.rs, relock.rs}`, one file per numbered check, each ~80–150 lines with its own `#[cfg(test)] mod tests`.
- `mod.rs` defines one small trait or just a `const CHECKS: &[fn(&Path, &mut Vec<Failure>) -> Result<()>]` array (KISS: a function-pointer table is enough here — a full `Check` trait is unnecessary ceremony for ten pure functions with an identical signature) and the shared `lines_without_opt_out`/opt-out-marker helpers move to a small `support.rs` sibling.
- Net effect: `check-repository`'s dispatch becomes `for check in CHECKS { check(root, &mut failures)?; }` — Open/Closed in the practical sense that adding check 11 means adding one file and one array entry, not editing a 1,078-line file.

### A3. Long orchestration functions mixing phases (KISS / Extract Method)

Several `run()`-shaped entry points interleave validation, I/O, and business logic across 100–350 lines where the repo's own stated architecture (`AGENTS.md` repo map: "`commands/*` ... Thin wiring ... the logic stays testable from `tests/`") says they should be thin:

| Function | Lines | What's inlined that could be a named step |
|---|---|---|
| `pack.rs::run` | 332 (27–358) | lockfile presence check, stale-output-dir guard, env-name validation, tools-lock resolution, cache setup, all inline before delegating to `build_files_oracle`/`vendor_tree`/`write_branch_docs` |
| `commands/tools/update.rs` | 763 total | not one god function, but the file mixes the pure `asset_name_of`/`owner_repo_of` helpers with the I/O-heavy update flow and ~500 lines of its own tests in one file |
| `restore.rs::run` | 142 (19–160) | branch resolution, verify, environment install, and tool registration sequenced inline |
| `restore.rs::register_user_tools` | 139 (205–343) | per-OS launcher writing and PATH-profile editing in one function |
| `doctor.rs::print_human` / `as_json` | 163 / 101 | two large, parallel renderers for the same `DoctorReport` — a case where extracting a few `fn summarize_env(...)`/`fn summarize_tool(...)` helpers shared by both would remove the duplication between the human and JSON renderers, not just shorten each |

None of these are *wrong* — `pack.rs` already delegates to well-named helpers for the heavy lifting — but the remaining top-level sequencing plus inline guards is exactly the "long method, several phases" smell from the refactor skill's own checklist. Suggested shape for `pack::run` as a template for the others:

```rust
pub fn run(args: PackArgs) -> Result<()> {
    let plan = PackPlan::from_args(args)?;   // validation + path resolution, testable alone
    plan.ensure_fresh_output()?;             // the "already exists" / lockfile guards
    let tools = plan.resolve_tools()?;       // fetch_tools / tools-lock branch
    let oracle = plan.build_files_oracle(&tools)?;
    plan.write_manifest_and_docs(&oracle)
}
```

Each extracted step is then unit-testable on its own without constructing a full pack run, which is also what makes Part B's "convert manual builders into fixtures" recommendation pay off for this file's tests specifically.

### A4. File-size hot list (proxy signal, not an automatic verdict)

For completeness, every production file over 450 lines, as a prioritized review list (not all of these need splitting — several, like `verify.rs` and `shard.rs`, are already one cohesive responsibility per `AGENTS.md`'s own repo map and should stay intact):

```
1110  commands/restore.rs       — candidate, see A3
1090  commands/pack.rs          — candidate, see A3
1078  xtask/repo_checks.rs      — candidate, see A2
1055  xtask/airlock.rs          — review: mixes plan/pack/fetch/self-bin concerns, same shape as A2 but smaller fan-out (4 concerns, not 10) — lower priority split
 859  user_tools.rs             — review only: already one coherent responsibility (install/registration), not an obvious split
 788  pixi-sandbox-git/shell.rs — review only: this *is* "all git access" by design (AGENTS.md), expected to be the largest file in that crate
 763  commands/tools/update.rs  — candidate, see A3 (file mixes pure helpers + I/O flow + its own 500 lines of tests; tests should move to tests/self_update_assets.rs-style sibling per Part B.2)
 745  pixi-sandbox-core/verify.rs — review only: single cohesive responsibility (collect every failure, D13), size is proportional to the number of failure kinds it reports, not a smell
```

---

## Part B — tests: fixtures and property-based testing

### B.1 `xtask` has zero `#[fixture]`/`#[rstest]`/`proptest!` usage across 4,631 lines

Both `rstest` and `proptest` are workspace dependencies already used in the other three crates, but `xtask`'s ~90+ inline `#[test]` functions (in `repo_checks.rs`, `airlock.rs`, `release_assets.rs`, `conda_platforms.rs`, `prepare_release.rs`, `release_refs.rs`, …) are 100% hand-written, each building its own tempdir/fixture from scratch. This is the opposite of what the module's own content suggests: `xtask` is almost entirely pure, total functions over strings and paths (regex checks, name/asset mapping, version parsing) — the textbook case for both styles of test this repo already has a house pattern for elsewhere:

- `pixi_sandbox_core::shard::split_then_join_is_identity` is the existing proptest template: round-trip a pure function over arbitrary input.
- `commands/restore.rs`'s `relocation_rewrites_only_nul_free_utf8` (the `proptest! { ... }` block at the end of the file) is the existing template for "pure function + tempdir fixture + property", already in the `pixi-sandbox` crate.

Concrete properties worth adding once `Platform` (A1) exists or even before it, directly against today's functions:

- `release_assets.rs::staged_name`/`binary_name`: *for every known target triple, the staged name starts with `pixi-sandbox-`, ends with `.exe` iff the triple contains `windows`, and is parseable back to the same triple* — replaces the current ~6 hand-picked example-based tests (lines 216–225) with one property plus the existing examples kept as a couple of named regression cases.
- `repo_checks.rs::bash32_surface`'s regex: *for every string built by concatenating a forbidden token (`mapfile`, `readarray`, `coproc`, `declare -A`, `${x,,}`, `${x^^}`, `&>>`) with random surrounding non-identifier text, the check always fires; for strings drawn only from an allow-listed vocabulary, it never does* — this is exactly a metamorphic/property test, and today the check only has hand-picked example lines.
- `pixi-sandbox-core::verify::verify`: *flipping any single byte in any one blob of a valid transport is always reported as a failure, and verifying an untouched transport never is* — a property over "which blob, which byte", generalizing the 15 hand-rolled tempdir cases already in `pixi-sandbox-core/tests/verify.rs` (607 lines, 37 tests, 0 proptest today) rather than replacing them.
- `pixi-sandbox-core::manifest::Manifest`: `tests/manifest.rs::a_valid_manifest_json_round_trips` already exists (task-38) but only varies `version`/`commit`; widening its `Strategy` to also vary `tools`, `envs`, and blob counts would catch a future field added to the struct that the two varied fields don't exercise. Lower priority than the items below — extend, don't duplicate.
- `Platform::from_os_arch` / `Platform::asset_name` (once A1 lands): round-trip and "every member of `ALL` has a non-empty, extension-correct asset name" — subsumes what are currently 4 separate, near-identical "every platform" tests spread across `self_update_assets.rs`, `airlock.rs`, `release_assets.rs`, `conda_platforms.rs`.

### B.2 `cli.rs` (2,008 lines) doesn't follow the crate's own established test-split convention

The `pixi-sandbox` crate already has a clear, good pattern for splitting a large integration surface by concern: `self_update.rs` (533 lines) is accompanied by five siblings — `self_update_assets.rs`, `self_update_checksums.rs`, `self_update_ownership.rs`, `self_update_replace.rs`, `self_update_resolver.rs` — each targeting one piece of the updater's trust boundary, matching the `self_update/{assets,checksums,ownership,replace}.rs` module split in `src/`.

`tests/cli.rs`, by contrast, is one 2,008-line file covering `pack`, `publish`, `restore`/`unpack`, `doctor`, `plan`, and `tools update` end-to-end CLI behavior together, with its own local helpers (`transport_copy`, `write_executable`, `fake_tools`, `path_with_fake_tools`, `bare_remote`, `tree_snapshot`, a `published_orphan` `#[fixture]`) that mostly apply to a subset of the file's tests. Splitting it into `tests/cli_pack.rs`, `tests/cli_publish.rs`, `tests/cli_restore.rs`, `tests/cli_doctor.rs`, `tests/cli_plan.rs`, `tests/cli_tools.rs` (sharing `tests/support/mod.rs`, which already exists and is already imported via `mod support;`) would make this file consistent with the rest of the crate, and let each file's fixtures be scoped to what it actually needs instead of six concerns sharing one 2,000-line namespace.

### B.3 Manual builder functions exist and are good — formalize them as `#[fixture]`s

This is a "keep doing this, but name it" item, not a gap: several test files already extracted the right helper functions by hand, they're just not expressed as rstest fixtures yet, which means every test repeats the call instead of declaring the dependency:

- `pixi-sandbox-core/tests/verify.rs::transport(dir, sha)` — called from most of its 37 tests.
- `pixi-sandbox/tests/self_update.rs::scratch(body)` / `request(destination, requested_version)` — called from most of its 58 tests.
- `pixi-sandbox-git/tests/publish.rs` — already uses `rstest`/`#[fixture]` in 2 places out of 43 tests; the rest still hand-build.

Converting `transport`/`scratch`/`request` into `#[fixture]` functions (rstest supports fixture *parameters*, so `transport` becomes `#[fixture] fn transport(#[default(GOOD_SHA)] sha: &str) -> (TempDir, Manifest)`) lets the growing list of near-duplicate `#[test]` bodies in these files become `#[rstest]` functions with `#[case]` tables instead — the same reduction `cli.rs` already applies in its `documents_every_verb`/`tools_update_help_documents_its_flags` tests (`#[rstest]` + `#[case]`, lines 145 and 770).

### B.4 What already meets the bar — don't touch

- `shard.rs`'s `split_then_join_is_identity` proptest and `restore.rs`'s `relocation_rewrites_only_nul_free_utf8` proptest are exactly the target shape; Part B's job is to replicate them, not replace them.
- Production code has **zero** `unwrap()` outside `#[cfg(test)]`/`proptest!` blocks in every file sampled (`restore.rs`, `pack.rs`, `init.rs`, `user_tools.rs`, `verify.rs`, `files_manifest.rs`) — the `AGENTS.md` style rule ("no `unwrap()` in library code paths that handle user input") holds today. No action needed; flagged here only so this plan doesn't miss confirming a thing that's already right.
- `sha256` has exactly one real implementation (`pixi_sandbox_core::shard::sha256_bytes`/`sha256_file`); no duplicate hashing helpers were found.
- All process spawning for tools funnels through `user_tools.rs`; all git access funnels through `pixi-sandbox-git` (confirmed, no stray `Command::new("git")` elsewhere) — both existing invariants, both intact.

---

## Correction after checking the backlog (session-skill rule: don't re-propose finished work)

`task-38` (Done, 2026-10-02) already executed most of Part B's intent for the `pixi-sandbox`
crate's integration suite: shared `tests/support/mod.rs` fixtures, a defined-once-helper guard,
every hand-rolled table converted to `#[case]`, and five new property tests (branch-name
safety, manifest round-trip, publish-plan uniqueness, files-manifest canonicalisation, NUL-free
relocation). It explicitly scoped to `cli.rs`, `e2e.rs`, `user_tools.rs`, `restore_script.rs`,
and `pixi-sandbox-core/tests/shard.rs`. That narrows Part B's real remaining scope to what
task-38 did **not** touch:

- `xtask` (B.1) — confirmed untouched, task-38 never mentions it; still the single biggest gap.
- `pixi-sandbox-core/tests/verify.rs`, `pixi-sandbox/tests/self_update*.rs` (6 files),
  `pixi-sandbox-git/tests/publish.rs` (B.3's examples) — none are in task-38's reference list;
  still open.
- `tests/cli.rs`'s split (B.2) is **downgraded from "do it" to "reconsider"**: task-38 added a
  `#[once]` fixture (`published_orphan`) specifically so an expensive publish runs once and is
  shared read-only across many listing/commit cases *in that one file*. Splitting the file would
  either duplicate that expensive setup per new file or require a cross-file fixture-sharing
  mechanism rstest doesn't give for free. Keep cli.rs as one file; if it grows past this, revisit
  then with the `#[once]` cost in view, not before.
- `tests/manifest.rs` already has a bounded manifest round-trip property (task-38); it only
  varies `version`/`commit`, so widening it is a small follow-on, not new work. `verify.rs`'s
  corruption-invariant property (any single-byte flip in any blob is always caught) is genuinely
  new — task-38's properties don't touch `verify()`.

## Sequencing (suggested backlog tasks, in dependency order) — all done

1. **`platform: introduce a Platform type in pixi-sandbox-core`** (A1). No consumers changed yet beyond the new module + its own round-trip tests. Pure addition, zero risk. — **Done, task-55 (`d5ffa89`).**
2. **`platform: migrate init.rs/standalone.rs/self_update::assets to Platform`** (A1, depends on 1). Deletes the two byte-identical matches and the `SUPPORTED_HOSTS` table's hand-copy; keeps `asset_for`'s existing error message shape (tests assert on it). — **Done, task-58 (`d916815`).**
3. **`platform: migrate xtask (airlock, release_assets, conda_platforms) to Platform`** (A1, depends on 1; natural to pair with 2 or follow it as its own commit since it's a different crate's call sites). — **Done, task-59 (`8aac6aa`).**
4. **`platform: migrate sandbox_config::runner_for and generated/github_workflow.rs template to Platform`** (A1, depends on 1). Re-render `.github/workflows/publish-sandbox.yml` via `xtask render-*`/the init path afterward so the committed file doesn't drift from its template (same discipline as `relock.yml`). — **Done, task-60 (`8dcc057`).**
5. **`xtask: split repo_checks.rs into one module per check`** (A2). Mechanical move, each check's existing tests move with it; add the `CHECKS` table last so `check_repository`'s call site changes once. — **Done, task-56 (`1521bcc`).**
6. **`pack/restore/doctor: extract pipeline steps out of long run() functions`** (A3). Do one command per task (`pack` first — it already has the most helper extraction to build on). — **Done, task-57 (`5d896bc`)**, scoped to `pack::run` per the task's own notes.
7. **`tests: add proptest coverage for release_assets, bash32_surface, verify, manifest round-trip`** (B.1). Independent of 1–6; can start immediately and is the lowest-risk, highest-learning item to do first if sequencing is flexible. — **Done, task-61 (`c53a10c`)**, covering `verify()`, `release_assets` staged/binary names, and `bash32_surface`; the manifest round-trip property already existed from task-38.
8. **`tests: formalize transport()/scratch()/bare_remote() as rstest fixtures`** (B.3). Independent, low risk, mechanical. — **Done, task-62 (`159cdcd`)**, with `self_update.rs`'s `request()` and `publish.rs`'s analogous helpers deliberately kept as plain functions rather than fixtures — see that task's notes for the rstest composition limitation this ran into.
9. ~~`tests: split cli.rs into per-command files`~~ — **dropped**, see "Correction after checking the backlog" above: task-38's `#[once]` fixture design makes the single file the right shape today.

## Acceptance strategy for any task drawn from this plan

1. `pixi run --frozen test` passes with a count ≥ 516/1 before *and* after every commit in the task.
2. `pixi run --frozen lint` (clippy `-D warnings`, `deny`, `sandbox-plan --json`, `lint-toml`, `lint-docs`) stays clean.
3. `pixi run --frozen -- convco check HEAD~1..HEAD` passes per commit (one focused conventional commit per task, per the refactor skill's golden rules).
4. For any item touching a generated file's template (item 4 above): the regenerated file is byte-identical to (or a deliberate, reviewed diff from) what's committed, verified the same way `check-repository`'s check 10 already verifies `relock.yml`.
5. No test added or changed points at this repository or a real `HOME` (D10) — every new fixture/property test builds its subject in a `tempfile::tempdir()` or in-memory, matching every existing test in the suite.

## Explicit non-goals

- Do not change the manifest schema, the transport format, or git-access strategy — this plan is internal code shape only.
- Do not add `strum`, `derive_more`, or any other proc-macro convenience crate; the `Platform` type is hand-written to avoid a vendor/relock cycle.
- Do not hand-edit `.github/workflows/publish-sandbox.yml` or `relock.yml` — re-render from the template.
- Do not reduce `xtask`'s hand-picked example-based tests when adding property tests next to them (B.1) — properties *generalize*, they don't replace the named regression cases that document a specific past incident (e.g. the v0.3.6 Bash 3.2 failure cited in `bash32_surface`'s own comment).
