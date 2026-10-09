---
id: TASK-86
title: Add BDD features with rstest-bdd for pixi-sandbox behaviour
status: Done
assignee: []
created_date: '2026-10-08 23:07'
updated_date: '2026-10-09 15:55'
labels:
  - testing
  - bdd
dependencies: []
priority: medium
type: enhancement
ordinal: 86000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The suite proves behaviour with descriptive test names (rstest fixtures, proptest properties) but carries no executable specification: `a_failed_restore_is_reported_once_and_never_retried` reads as one assertion chain, not as the contract in the words a product discussion would use. A Gherkin feature states the behaviour first — scenario by scenario, in Given/When/Then — and the suite binds those sentences to the same fixtures and assertions the house already trusts.

rstest-bdd (`leynos/rstest-bdd`) is the binding layer: `.feature` files are parsed at compile time (via the `gherkin` crate), steps are ordinary Rust functions annotated `#[given]`/`#[when]`/`#[then]`, and `#[scenario]`/`scenarios!` generate one `#[rstest]` test per scenario — so every scenario appears in the ordinary `cargo test` count and failures name the scenario, with rstest fixture injection available underneath. It is a dev-only, compile-time dependency: nothing changes in the shipped product.

Intended behaviour: a BDD layer at `crates/pixi-sandbox/tests/features/` that states the product's observable behaviour in Gherkin — exit statuses, files and bytes on disk, report lines an operator reads — with step definitions under `tests/` driving the public API through the fixture builders `tests/support` already exposes. First slice, three features: restore verification (a verified restore writes the project; a failed verification writes nothing and keeps its evidence), transport integrity (a packed transport round-trips through verification; a tampered byte is refused before anything is written), and the user-tools policy (register, skip, and the honest-reporting paths). The layer complements the suite; it does not replace it.

Seam to agree before tests are written: per feature, the scenario list and which fixture builder each step binds to, so a step body stays glue and the assertions stay where the house puts them. Scenarios state outcomes, never internals.

One constraint came first and is now satisfied: a new crate cannot be fetched inside the airlock (egress allows github.com, not crates.io), so rstest-bdd landed the way task-38 landed rstest — declared at the workspace root (`rstest-bdd = "0.6.0"`) and as a dev-dependency of `pixi-sandbox`, with Cargo.lock refreshed by the connected relock workflow (2026-10-09). The crate's MSRV (1.88) sits below the workspace toolchain (1.98.1); the workspace's declared library MSRV 1.85 is unaffected because the crate is dev-only. A restored host compiles the suite only after the post-merge transport carries the new vendor crates (task-38's precedent: fresh restore, then `pixi run --frozen test`).

Non-goals: no replacement or rewriting of existing tests; no product behaviour change; no runtime dependency; no network; no feature-coverage requirement for every module — behaviour slices only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rstest-bdd is wired as a dev-only dependency and every Gherkin scenario runs as a plain cargo test test, visible in the suite count.
- [x] #2 At least three .feature files under crates/pixi-sandbox/tests/features/ state observable behaviour in Gherkin: restore verification, transport integrity, and the user-tools policy.
- [x] #3 Step definitions live under tests/ and drive the public API through the existing fixture builders; no step code lands in src/, and no scenario points at this repository or a real HOME.
- [x] #4 No existing test is deleted or weakened: the suite count does not drop and the descriptive tests keep the coverage they have.
- [x] #5 The test, lint and check-repository gates stay green, and AGENTS.md Test conventions describe where feature files and step definitions live.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09 — landed as **PR #126**, squash-merged into `main` at `8dc5bba`; `ci` (3m47s), `docs` (39s) and `publish sandbox` (4m0s) are green on that commit, and the transport repacked from it (pixi-sandbox 0.6.0, schema 2, static, 268 vendored crates). Closed on the merged tree, which is why this file sat `In Progress` a beat longer than the work did: the pull request carried the code and the pull-request-side proof, and the post-merge runs are the first moment the airlock claim could be told honestly.

**One deliberate deviation from the Description's mechanism sentence, recorded rather than papered over.** The Description says `#[scenario]`/`scenarios!` generate one `#[rstest]` test per scenario. What merged binds scenarios through the *vendored runtime half* of rstest-bdd instead: `.feature` files parsed by the real `gherkin` crate, steps registered with rstest-bdd's own `step!` macro into its registry (so the pattern engine, specificity ordering, `StepContext` world storage and the `StepExecution` outcome type are all the published crate's, exercised as shipped), and one thin driver in `tests/support/bdd.rs` doing what the macros' codegen does — parse, resolve each sentence in the registry, execute against an owned world cell, one plain `#[test]` per scenario, failures naming feature/scenario/step. The reason was an airlock fact, not a preference: the attribute macros live in a **separate `rstest-bdd-macros` crate** (a dev-dependency of rstest-bdd, never transitive), which was in neither `Cargo.lock` nor the vendor set, and pulling it in needed roughly twenty unlocked crates (`rstest-bdd-harness`, the `cap-std` tree, `camino`, `proc-macro-error3`, `newt-hype`, …) plus a connected relock for their crates.io checksums — which an airlocked session can neither fetch nor fabricate honestly, since a guessed checksum builds locally and dies in connected CI. All five acceptance criteria below are satisfied by the shape that merged; only the mechanism deviated. The same PR declared `rstest-bdd-macros 0.6.0`, the relock bot refreshed `Cargo.lock` (+220 lines, genuine checksums, all inside `deny.toml`'s allowlist), and the repack vendored it — **TASK-87 now retires the driver**, with the feature files and their sentences unmoved, which is the swap-in path this entry anticipated.

- **AC#1** — `rstest-bdd = "0.6.0"` at the workspace root and as a `pixi-sandbox` dev-dependency only; nothing reaches the shipped product. The `gherkin = { version = "0.16.0", default-features = false, features = ["parser"] }` edge is the same version and feature set rstest-bdd itself declares, so it added no package to the locked graph. All **eleven scenarios run as plain `#[test]` functions** and are visible in the nextest count: 4 restore verification, 3 transport integrity, 4 user-tools policy. Suite **802 → 816 passing / 1 skipped**.
- **AC#2** — three `.feature` files under `crates/pixi-sandbox/tests/features/`, exactly the three behaviour slices the Description named: `restore_verification.feature` (a verified restore writes the project and cleans its scratch; a tampered transport and a missing transport are both refused with nothing written; a verify-only run reports its verdict), `transport_integrity.feature` (a packed transport round-trips through `doctor --verify`; one tampered byte loses the healthy verdict; a tampered shard part is named in the refusal), `user_tools_policy.feature` (register by default, `skip`, the policy travelling as an environment variable, and a pre-policy bootstrap reporting honestly instead of announcing a registration that did not happen). Scenarios state outcomes only — exit statuses, bytes and files on disk, report lines an operator reads — never internals.
- **AC#3** — step definitions live in the matching `tests/bdd_<feature>.rs` integration-test root, one binary per feature so each feature owns its step registry and its scenarios stay a plain `cargo test` count. Steps drive the binary **black-box** through the existing `tests/support` fixture builders (`bin`, `isolated_bin`, `copy_tree`, `fixture_transport`, `transport_repo`, `host_platform`); no step code lands in `src/`. Every path starts under a test-owned tempdir with `HOME`/`USERPROFILE`/`SHELL`/`TMPDIR` isolated per scenario, and no scenario points at this repository or a real HOME (D10).
- **AC#4** — nothing deleted, nothing weakened: the count rose 802 → 816 and the descriptive suite keeps its coverage, because the layer complements it rather than replacing it. Where a scenario touches the same ground a descriptive test already holds, it deliberately touches the *same* fixture — the blob the transport-integrity scenarios tamper is the very `demo-pure-0.1.0-0.conda` the `cli.rs` doctor tests tamper, so the two halves cannot drift apart silently.
- **AC#5** — `fmt`, `lint` (including `check-repository` and the generated-workflow lint) and `test` green locally and on `8dc5bba` in CI, plus airlock and codecov-patch on the PR head. `AGENTS.md` § Test conventions gained the paragraph stating where the two halves live: `.feature` files in `crates/pixi-sandbox/tests/features/`, one per behaviour slice, and step definitions in the matching `tests/bdd_<feature>.rs` root.

Platform gating kept the house rule: the pre-policy user-tools scenario binds behind `#[cfg(all(target_os = "linux", target_arch = "x86_64"))]`, the same `#[cfg]` the descriptive suite uses, and its steps drive `scripts/restore.sh` against a `transport_repo` staged with the pre-0.3.7 shim — so a foreign host skips the scenario rather than failing it.

One procedural note for the record, already in `CONTEXT.md`: runs for commits authored by `pixi-sandbox[bot]` sit at `action_required`, and approving, cancelling or dispatching them are owner-token actions, so a bot-commit check that needs re-running needs the owner's click. The one real failure in the relock round was `lint:toml` — the root `rstest-bdd-macros` declaration had not been through taplo after an environment reset mid-session — fixed in `39def33`.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
task-86 gave the repository its first executable specification. Three Gherkin features live at `crates/pixi-sandbox/tests/features/` — restore verification, transport integrity, and the user-tools policy — and their eleven scenarios run as plain `cargo test` tests, bound to step definitions in one integration-test root per feature (`tests/bdd_<feature>.rs`) that drive the public CLI black-box through the `tests/support` fixture builders. Matching, the step registry, world storage and step outcomes are the published rstest-bdd crate's; the scenarios state outcomes an operator can see — exit statuses, bytes on disk, report lines — and never internals. Suite 802 → 816 passing / 1 skipped, with no existing test deleted or weakened, and `AGENTS.md` § Test conventions now records where the two halves live.

Merged as PR #126 into `main` at `8dc5bba`, with `ci`, `docs` and `publish sandbox` green and the transport repacked from that commit.

The one deviation is honest and already retired: scenarios bind through a thin driver in `tests/support/bdd.rs` rather than the `#[scenario]` attribute macro, because `rstest-bdd-macros` ships as a separate crate that the airlocked vendor set could not carry when the work was done. The same PR declared it, the relock bot refreshed `Cargo.lock`, and the repack vendored it — **TASK-87** replaces the driver with `#[scenario]`/`#[given]`/`#[when]`/`#[then]` bindings, leaving the feature files and every sentence exactly as they are.
<!-- SECTION:SUMMARY:END -->
