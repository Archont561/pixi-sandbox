---
id: TASK-52
title: >-
  Fix the generated consumer publisher resolving its release download from the
  consumer repository (issue 80)
status: To Do
assignee: []
created_date: '2026-10-03 02:16'
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
- [ ] #1 Both generated download legs (bash and pwsh) resolve the `v${PIXI_SANDBOX_VERSION}` assets from the pixi-sandbox repository via one canonical OWNER/REPO constant in the renderer — shared with `self_update`'s default `--repo` so template and updater cannot drift — and `GITHUB_REPOSITORY`/`$env:GITHUB_REPOSITORY` no longer appears in any release-download URL
- [ ] #2 The downloaded binary is still verified against the release's `SHA256SUMS` before use in every leg, and a checksum mismatch fails the job (nothing unverified is executed)
- [ ] #3 Fixture/golden-render tests assert the release-download URL names the pixi-sandbox repository in both shells, so a template edit cannot silently reintroduce consumer-repo resolution; house test conventions hold (D10 fixtures, no inline test modules)
- [ ] #4 `pixi run --frozen xtask lint-generated-workflow` (actionlint) passes on the regenerated template, and this repository's own committed workflows are untouched — the owner publisher is source-built (task-47 AC#9) and must not gain a download step
- [ ] #5 The ci-publishing guide states where the released binary is downloaded from and records the pre-fix failure mode (`curl: (22)` 404 on the consumer repo) as the remediation note for consumers who pinned a broken v0.4.3–v0.5.0 template
- [ ] #6 The fix ships in a released patch whose generated workflow passes the issue's repro on a consumer-shaped repo (`init`, then `grep releases/download` names pixi-sandbox; a publish run no longer 404s) — a connected/CI proof, or the task stays In Progress naming exactly this AC and its missing proof
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
