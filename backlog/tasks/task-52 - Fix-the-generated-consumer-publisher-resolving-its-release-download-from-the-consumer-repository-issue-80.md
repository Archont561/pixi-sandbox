---
id: TASK-52
title: >-
  Fix the generated consumer publisher resolving its release download from the
  consumer repository (issue 80)
status: Done
assignee: []
created_date: '2026-10-03 02:16'
updated_date: '2026-10-03 10:18'
labels:
  - bug
  - init
  - release
  - ci
dependencies: []
references:
  - crates/pixi-sandbox/src/generated/github_workflow.rs
  - crates/pixi-sandbox/src/release.rs
  - crates/pixi-sandbox/src/commands/init.rs
  - docs/src/content/docs/guides/ci-publishing.mdx
priority: high
type: bug
ordinal: 53000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Issue #80 (2026-10-03, <https://github.com/Archont561/pixi-sandbox/issues/80>): `pixi
sandbox init` (v0.4.3, v0.4.4, v0.5.0) writes a consumer `.github/workflows/publish-sandbox.yml`
whose "Download released pixi-sandbox" step builds the release URL from the *consumer's*
repository:

```yaml
base="$GITHUB_SERVER_URL/$GITHUB_REPOSITORY/releases/download/v${PIXI_SANDBOX_VERSION}"
```

In any consumer repo that publishes no GitHub releases, `plan` succeeds and `publish` dies
before packing anything: `curl: (22) 404` on, e.g.,
`github.com/Archont561/qgis-rs/releases/download/v0.5.0/pixi-sandbox-x86_64-unknown-linux-musl`
— the assets live on `Archont561/pixi-sandbox` (all 11 published for v0.5.0, including
`SHA256SUMS`). The pwsh twin (`$env:GITHUB_REPOSITORY`) has the identical bug; the current
tree confirms both — `crates/pixi-sandbox/src/generated/github_workflow.rs` still resolves
`$GITHUB_REPOSITORY` on lines ~108 (bash) and ~125 (pwsh).

Regression origin: the v0.3.2 template delegated to the composite
`Archont561/pixi-sandbox/setup` action, which resolved the release against the pixi-sandbox
repo; inlining that action between v0.3.2 and v0.4.3 (task-29 era) lost the repository it
downloaded from, so the generated step can only ever work inside the pixi-sandbox repo
itself. The consumer's publish history shows the switch point: runs 4–8 on the v0.3.2
composite-action template green; runs 9–11 on the inlined v0.4.4/v0.4.3/v0.5.0 templates red
on the 404.

Out of scope: the `PIXI_SANDBOX_CHANNEL` half of issue #80 is already fixed — v0.5.0 emits
the working `https://prefix.dev/archont561/archont561` (task-49's channel surface; the
consumer's channel complaint against v0.4.3 duplicates issue-71/task-48, closed by D17).
Where task-45 fixed *which* binary the bootstrap uses, this task fixes *where the released
one downloads from*. The step's `SHA256SUMS` verification must survive the fix: nothing
unverified is ever executed.

The issue's suggested shapes: a template-time constant, or a `${{ github.repository
}}`-independent owner/repo pair naming the pixi-sandbox repository. Do not regress task-29
(no return to the composite action).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Both generated download legs (bash and pwsh) resolve the `v${PIXI_SANDBOX_VERSION}` assets from the pixi-sandbox repository via one canonical OWNER/REPO constant in the renderer — shared with `self_update`'s default `--repo` so template and updater cannot drift — and `GITHUB_REPOSITORY`/`$env:GITHUB_REPOSITORY` no longer appears in any release-download URL
- [x] #2 The downloaded binary is still verified against the release's `SHA256SUMS` before use in every leg, and a checksum mismatch fails the job (nothing unverified is executed)
- [x] #3 Fixture/golden-render tests assert the release-download URL names the pixi-sandbox repository in both shells, so a template edit cannot silently reintroduce consumer-repo resolution; house test conventions hold (D10 fixtures, no inline test modules)
- [x] #4 `pixi run --frozen xtask lint-generated-workflow` (actionlint) passes on the regenerated template, and this repository's own committed workflows are untouched — the owner publisher is source-built (task-47 AC#9) and must not gain a download step
- [x] #5 The ci-publishing guide states where the released binary is downloaded from and records the pre-fix failure mode (`curl: (22)` 404 on the consumer repo) as the remediation note for consumers who pinned a broken v0.4.3–v0.5.0 template
- [x] #6 The fix ships in a released patch whose generated workflow passes the issue's repro on a consumer-shaped repo (`init`, then `grep releases/download` names pixi-sandbox; a publish run no longer 404s) — a connected/CI proof, or the task stays In Progress naming exactly this AC and its missing proof
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
One constant, both shells. Reuse or lift a single canonical repository slug —
`self_update`'s resolver already defaults `--repo` to `Archont561/pixi-sandbox`; hoist that
constant into shared scope (`release.rs`) and have `generated/github_workflow.rs` render the
download base from it in both legs:
`base="$GITHUB_SERVER_URL/<slug>/releases/download/v${PIXI_SANDBOX_VERSION}"` and the
`$env:` twin. Keep `GITHUB_SERVER_URL` (still correct on GHES); drop `GITHUB_REPOSITORY`
from the URL only. Do not make the slug configurable: issue #79 / task-53 owns the config
surface, and an owner/repo override can graduate into that table if a fork ever needs one —
until then a constant is the smallest correct change.

Test first: extend the template's fixture/golden renders so both legs' download URLs name
the pixi-sandbox repository (a test reading the committed template would have caught this
regression at inline time). Then update the render, confirm the SHA256SUMS step is
byte-identical, and run `pixi run --frozen fmt`, `lint` (includes actionlint on the
generated workflows via `xtask lint-generated-workflow`), and `test`. No regeneration of
`.github/workflows/publish-sandbox.yml` in this repo: the owner publisher is source-built.
The patch release rides auto-release after merge; the connected proof (AC#6) is a consumer
publish run, or a scripted init-then-grep of the freshly rendered template on the release
artifacts.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Root cause confirmed by reading `crates/pixi-sandbox/src/generated/github_workflow.rs`: the
rendered consumer workflow's "Download released pixi-sandbox" step built its download base
from `$GITHUB_SERVER_URL/$GITHUB_REPOSITORY/...` (bash) and the `$env:` twin (pwsh) —
`GITHUB_REPOSITORY` is always the *consumer's* repository at Actions runtime, never
pixi-sandbox's, so any consumer with no releases of its own got a 404 before packing ran.
Confirmed the same bug mirrored in the committed golden fixture
(`crates/pixi-sandbox/tests/fixtures/generated/publish-sandbox.yml` lines 76 and 93).

Fix (minimal, surgical — `git diff` touches 6 lines of the renderer plus the `use` line):
- Added `pub const PIXI_SANDBOX_REPO: &str = "Archont561/pixi-sandbox";` to
  `crates/pixi-sandbox/src/release/mod.rs` as the one canonical owner/repo slug.
- Changed `self_update::DEFAULT_REPO` from its own standalone literal to
  `pub const DEFAULT_REPO: &str = crate::release::PIXI_SANDBOX_REPO;` — the CLI's `--repo`
  default (`cli.rs:68`) and the generated workflow's download URL now share one constant and
  cannot drift apart again.
- In `generated/github_workflow.rs`, replaced the two `$GITHUB_REPOSITORY`/`$env:GITHUB_REPOSITORY`
  download-base segments with a new `__RELEASE_REPO__` placeholder, substituted via
  `.replace("__RELEASE_REPO__", PIXI_SANDBOX_REPO)` at render time, following the existing
  `__VERSION__`/`__CONFIG_PATH__` pattern exactly. `GITHUB_SERVER_URL` is untouched (still
  correct on GHES); the unrelated `REMOTE: ${{ github.server_url }}/${{ github.repository }}.git`
  line (used for publish's git push, which correctly targets the consumer's own repo) was
  deliberately left alone.
- Updated the golden fixture (`tests/fixtures/generated/publish-sandbox.yml` lines 76/93) to
  match the fixed render; the byte-exact golden test
  (`generated_workflow_matches_the_reviewed_golden_file`) caught the diff as expected before
  the fixture was updated, confirming the test harness would have caught this regression at
  inline time (task-29 era) had it existed then.

Test-first: added
`generated_workflow_downloads_release_assets_from_the_pixi_sandbox_repository_not_the_consumers`
in `crates/pixi-sandbox/tests/generated_workflow.rs`, asserting (a) both the bash and pwsh
download bases name `pixi_sandbox::release::PIXI_SANDBOX_REPO` literally, (b) that constant
equals `pixi_sandbox::self_update::DEFAULT_REPO` (so the two call sites can never diverge
again), and (c) no `releases/download` occurrence in the rendered workflow is preceded by
`GITHUB_REPOSITORY` within an 80-character window (guards against the bug reappearing in any
future reshuffle of the template). Confirmed this test fails to compile (red) before
`PIXI_SANDBOX_REPO` existed, then passes (green) after the fix — true test-first.

Verification:
- `cargo test -p pixi-sandbox --test generated_workflow --offline`: 16/16 passed, including the
  golden-file test against the updated fixture.
- `pixi run --frozen test`: 557 passed / 1 skipped (up from the 556/1 baseline — the one new
  test).
- `pixi run --frozen -- cargo run -q -p xtask -- lint-generated-workflow`: actionlint clean on
  both the publisher and relock renders.
- `pixi run --frozen -- cargo run -q -p xtask -- check-repository`: all consistency checks
  green.
- `git status --short .github/`: empty — confirmed this repository's own committed
  `.github/workflows/publish-sandbox.yml` is untouched. Read `crates/xtask/src/workflow.rs` in
  full first: `WORKFLOW_PATH` (that file) is only ever rendered into a tempdir for actionlint
  validation, never byte-compared against the committed copy (unlike `RELOCK_PATH`/`relock.yml`,
  which `check-repository` does byte-check) — so leaving it alone is both correct and
  automatically enforced; it is a vestigial/stale artifact from an earlier dogfooding run, out
  of scope by the task's own AC#4 wording ("the owner publisher is source-built ... must not
  gain a download step").
- `pixi run --frozen docs build` (astro build): succeeded, 13 pages built including the edited
  `guides/ci-publishing` page; `pixi run --frozen lint-docs` (biome) clean.
- `pixi run --frozen fmt` then `lint`: full gate green (fmt, cargo-deny, actionlint,
  xtask lint-generated-workflow, sandbox-plan smoke, taplo, docs-install, lint-docs,
  check-repository).

AC#6 (ships in a released patch + connected-host repro proof) is left open and named
explicitly: this sandbox has no connected GitHub host to cut a release or run a live
init-then-publish repro against a consumer-shaped repo in this session. The fix is complete,
tested, and ready to ship on the next patch release via the repo's existing auto-release flow;
once released, the proof is a one-line repro (`pixi-sandbox init` against a fresh consumer
repo, `grep releases/download` on the generated workflow names `Archont561/pixi-sandbox`, and a
real publish run no longer 404s).

2026-10-03, release proof — v0.5.2 shipped the fix. Consumer-proof run 37137917674 downloaded and checksum-verified the released standalone binary, initialized a clean consumer-shaped fixture twice, proved byte-identical regeneration, and asserted that the generated publisher names `Archont561/pixi-sandbox/releases/download` plus `SHA256SUMS`. This is the connected proof AC#6 required; task complete. Evidence: https://github.com/Archont561/pixi-sandbox/actions/runs/37137917674
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
5 of 6 ACs done, offline and verified. The generated consumer publisher's "Download released
pixi-sandbox" step now resolves `v${PIXI_SANDBOX_VERSION}` assets from `Archont561/pixi-sandbox`
(a new `release::PIXI_SANDBOX_REPO` constant, re-exported as `self_update::DEFAULT_REPO`) in
both the bash and pwsh legs, never from the consumer's own `$GITHUB_REPOSITORY` again.
SHA256SUMS verification is untouched (AC#2). A new fixture/golden-render test locks the fix in
place (AC#3); `xtask lint-generated-workflow` passes and this repo's own committed
`publish-sandbox.yml` is confirmed untouched (AC#4); the ci-publishing guide now documents the
download source and the pre-fix 404 remediation (AC#5). Full `pixi run --frozen test`
(557/1 skipped) and `lint` are green.

AC#6 stays open: shipping in a released patch and reproducing the fix against a live
consumer-shaped repo needs a connected GitHub host and a cut release, neither available in this
sandbox session. The fix is complete and ready to ship on the next patch via the existing
auto-release flow.
<!-- SECTION:FINAL_SUMMARY:END -->
