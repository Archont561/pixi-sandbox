---
id: TASK-47
title: Give consumers a version-drift gate and reviewed upgrade path for generated files
status: To Do
assignee: []
created_date: '2026-10-02 20:05'
updated_date: '2026-10-02 20:05'
labels:
  - ci
  - init
  - release
dependencies:
  - TASK-45
references:
  - crates/pixi-sandbox/src/commands/init.rs
  - crates/pixi-sandbox/src/generated/github_workflow.rs
  - crates/pixi-sandbox/src/generated/relock_workflow.rs
  - crates/pixi-sandbox/src/commands/tools/update.rs
  - crates/pixi-sandbox/src/commands/doctor.rs
  - docs/src/content/docs/guides/ci-publishing.mdx
priority: medium
type: enhancement
ordinal: 48000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A new pixi-sandbox release leaves every consumer silently stale. `publish-sandbox.yml` pins
`PIXI_SANDBOX_VERSION` to the CLI that rendered it and embeds the whole template as of that
version; `relock.yml` stamps pixi from that CLI's embedded tools lock; and the published
transport keeps being packed with the old released `--self-bin`, so `manifest.tool.version`
never moves. The only remedy documented today is prose ("regenerate with `pixi-sandbox
init`"), and nothing detects the drift in the first place.

Give consumers the same drift policy this repository already enforces on itself
(check-repository's committed-render check) and the same bot precedent the relock workflow
set (task-39), under the three-domain split recorded as decision-4 / D16: generated files
**regenerate**, config **migrates explicitly behind review**, and the transport **repacks**
when the regenerated workflow lands on main. Version pins never float to latest.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every file `init` writes carries a machine-readable version stamp beside the ownership marker (a `pixi-sandbox-version: X.Y.Z` line, within the first three lines the marker check already reads) for the publisher workflow, the relock workflow, and the launcher; fixture tests cover presence and parsing on every render
- [ ] #2 `pixi-sandbox init --check` renders fresh, writes nothing, and exits non-zero naming each drifted file with its remedy (`pixi-sandbox init`) and each foreign-owned file separately (`--force`); a clean tree exits 0 — fixture-tested in both directions (D10)
- [ ] #3 `init` and `--check` never rewrite an existing config; a config whose `schema` is older than the CLI's is reported as a finding naming the explicit migration path (no `config migrate` command is built while config schema is 1 — it exists only once a schema bump gives it something to do), and config readers keep accepting older schemas for a deprecation window, mirroring the manifest reader policy
- [ ] #4 The generated `publish-sandbox.yml` gains a scheduled and manually dispatchable upgrade job: install the latest released CLI, run `init --check`, and on drift regenerate the marker-carrying files and open a pull request — never pushing to main, never touching the config; the render obeys the house step rule (SHA-pinned `uses:` or one-line `run:`) and stays actionlint-clean under the generated-workflow test
- [ ] #5 A human merging the upgrade PR triggers the normal `on: push` publish run, so the transport's manifest names the new `tool.version` with the new released `--self-bin`; because a `github.token` push starts no `on: push` workflows (task-44's lesson), any automated-merge path the job offers ends in an explicit `gh workflow run publish-sandbox.yml` dispatch, and the workflow/docs say which path needs it
- [ ] #6 The airlock side is unchanged — `restore.sh`/`restore.ps1` stay version-agnostic, doctor keeps reading older manifests — and the docs (guides/ci-publishing, reference/cli, README's generated-publisher section) describe the upgrade flow and state that pins never float
<!-- AC:END -->

## Implementation Plan
<!-- SECTION:PLAN:BEGIN -->
Stamp first: extend the three renderers plus the launcher templates with the version line
(the marker stays within the first three lines `ensure_replaceable` reads) and update the
generated-workflow YAML-mapping tests to assert it. Then `--check` as a render-and-compare
over the same four paths `init` writes, reporting in the `tools update --check` shape (full
list of findings, non-zero exit, zero writes) and reusing `ensure_replaceable`'s marker
logic to separate owned drift from foreign files. Then the publisher template's upgrade
job: latest CLI from the canonical channel, `init --check`, on drift `pixi-sandbox init` +
branch + `gh pr create` with `github.token` (creating a PR is allowed; merging it is what
does not trigger publish — hence the explicit-dispatch remedy for any auto-merge). Close
with the docs pass and a regeneration of this repository's own committed renders, since
the repo's drift check holds it to the same policy the new stamp enables for consumers.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
<!-- SECTION:SUMMARY:END -->
