---
id: TASK-64
title: 'tests: formalize transport, scratch, request helpers as rstest fixtures'
status: Done
assignee: []
created_date: '2026-10-03 09:27'
updated_date: '2026-10-03 10:05'
labels:
  - testing
  - rstest
dependencies: []
priority: low
ordinal: 63000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Convert the manual builder functions in pixi-sandbox-core tests verify.rs transport, pixi-sandbox tests self_update.rs scratch and request, and pixi-sandbox-git tests publish.rs remaining hand-built setup into fixture functions, and convert the near-duplicate test bodies they feed into rstest plus case tables. See doc-9 section B.3.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Tests that called them directly are converted to #[rstest] consuming the fixture
- [x] #2 Near-duplicate test bodies become #[case] tables where applicable
- [x] #3 pixi run --frozen test passes with count >= 516/1
- [x] #4 verify.rs transport and pixi-sandbox-git tests publish.rs bare_remote become #[fixture] functions with no lifetime borrowing; self_update.rs scratch becomes a #[fixture]; self_update.rs request stays a plain helper (documented rstest limitation: #[fixture] cannot thread an owned fixture's data into another #[fixture] by reference -- only #[by_ref] on an #[rstest] test parameter supports that, and Request<'a> borrows a Path/str so it cannot be a composed fixture without changing production code)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Converted three hand-built test helpers to genuine rstest fixtures and their callers to #[rstest]: (1) pixi-sandbox-core tests/verify.rs transport(dir, sha) -> Manifest became #[fixture] fn transport(#[default(sha256_of(BLOB_BODY))] sha: String) -> (TempDir, Manifest); its 3 example tests became #[rstest], and the two corruption-shape tests (tampered/missing) merged into one #[rstest]+#[case] table since they shared the exact same corrupt-then-assert-Kind shape; the proptest module's direct call site was updated to the new signature. (2) pixi-sandbox tests self_update.rs scratch(body) -> Scratch became #[fixture] fn scratch(#[default(&b"the 0.4.4 binary"[..])] body: &[u8]) -> Scratch; its 18 callers became #[rstest] consuming scratch: Scratch (with #[with(...)] overrides for the two non-default bodies). A new #[fixture] fn pixi_managed_destination() -> (TempDir, PathBuf) dedupes an identical .pixi/envs/default + conda-meta setup block two tests had hand-built independently; both converted to #[rstest] consuming it. (3) pixi-sandbox-git tests publish.rs bare_remote() -> (TempDir, String) became #[fixture]; its 8 callers became #[rstest] consuming bare_remote.

request() in self_update.rs (and the analogous helpers in publish.rs) were deliberately NOT converted into #[fixture] functions. Empirically verified via a throwaway compiled-then-reverted experiment that rstest's #[fixture] macro cannot thread an owned fixture's value into another #[fixture] by reference: a #[fixture] parameter typed as &'a T when the dependency fixture returns owned T fails with E0308. #[by_ref] (which does support local-lifetime references) only applies to #[rstest] TEST parameters, not fixture-to-fixture composition (confirmed against rstest 0.26.1's vendored source). Since Request<'a> and Snapshot<'a> are real production structs borrowing &'a Path/&'a str, and request()'s destination is not uniformly sourced from scratch across the file, the honest design keeps scratch as a true fixture and request(...) as a plain helper invoked inline in the now-#[rstest] bodies. AC wording was amended to record this.

self_update.rs and publish.rs were audited for further #[case]-table opportunities beyond verify.rs's merge; none were forced since the remaining tests call different functions or each document a distinct, separately-named design claim.

Verification: clippy workspace --all-targets -D warnings clean; fmt clean; pixi run --frozen test = 556 passed / 1 skipped; pixi run --frozen lint exit 0.
<!-- SECTION:NOTES:END -->
