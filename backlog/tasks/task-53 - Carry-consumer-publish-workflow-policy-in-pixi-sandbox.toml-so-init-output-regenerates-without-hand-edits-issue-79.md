---
id: TASK-53
title: >-
  Carry consumer publish-workflow policy in pixi-sandbox.toml so init output
  regenerates without hand edits (issue 79)
status: To Do
assignee: []
created_date: '2026-10-03 02:16'
labels:
  - init
  - ci
  - config
dependencies: []
references:
  - crates/pixi-sandbox/src/generated/github_workflow.rs
  - crates/pixi-sandbox/src/commands/init.rs
  - crates/pixi-sandbox/src/commands/plan.rs
  - >-
    backlog/tasks/task-47 -
    Give-consumers-a-self-updating-binary-and-reviewed-generated-file-upgrade-path.md
  - docs/src/content/docs/guides/ci-publishing.mdx
  - .pixi-sandbox.toml
priority: medium
type: feature
ordinal: 54000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Issue #79 (2026-10-03, <https://github.com/Archont561/pixi-sandbox/issues/79>): `pixi
sandbox init` (v0.5.0) generates a working `.github/workflows/publish-sandbox.yml` but
hardcodes policy nearly every real repository wants to change, with no way to say so in
`pixi-sandbox.toml`:

| Generated | What the consumer hand-edits | Why |
| --- | --- | --- |
| bare `push:` trigger | a `paths:` allowlist | every push to `main` repacks the transport and force-pushes an orphan branch, even a README typo |
| no `permissions:` block | `contents: read` top-level, `contents: write` only on publish | the generated token is default-permission |
| no `concurrency:` | `group: publish-sandbox`, `cancel-in-progress: false` | two publishes racing force-push the same orphan branch |
| no `timeout-minutes` | 15 on plan, 60 on publish | a hung `pack` holds a runner for the six-hour default |
| `PIXI_SANDBOX_VERSION` pinned only in `env:` | also pin `pixi-version` on each `setup-pixi`, set `cache: false` | pixi-sandbox owns the native `pixi install` while packing, so an action cache keyed on the consumer manifest can restore a solve pixi-sandbox will not reuse — a correctness hazard, not mere staleness |

Because those edits are local, the marker the generator writes ("local policy edits
follow") stays true, and the next `init` refuses to overwrite; `--force` regenerates and
the consumer re-applies five edits by ritual, with nothing in the tool saying whether the
re-applied edit still matches what the *new* generator would have produced. A generated
file that always needs post-processing is not a generated file.

Proposed shape (from the issue; all optional, current output as the default so existing
repos need no change): a `[workflow]` table in the consumer's `pixi-sandbox.toml` —
`push_paths`, `permissions`, `concurrency { group, cancel_in_progress }`,
`timeouts { plan, publish }`, `pixi_version`, `setup_pixi_cache`. `push_paths` can be
*derived*: `plan` already knows every manifest that feeds the transport, so `init` can
emit the allowlist instead of the consumer transcribing it, and `plan --json` can expose
the derivation for diffing.

Relationship, not overlap: task-47's `init --check` and upgrade lane (its AC#4/AC#6) make
regeneration *safe*; this task makes hand edits *unnecessary* by moving policy into
config. They compose — the regenerated PR task-47 opens must come out byte-identical to
what the consumer would have hand-edited. The issue's own scoping applies when slicing:
the `pixi_version`/`setup_pixi_cache` pair is the correctness fix; `permissions`,
`concurrency` and `timeout-minutes` are independent of it and of `push_paths`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The consumer config accepts an optional `[workflow]` table within schema 1 (no schema bump; absent renders byte-identical to the current template), unknown keys are rejected naming the field, and config readers keep accepting schema-1 files per task-47 AC#5's deprecation policy
- [ ] #2 Each configured block renders into the generated publisher — `push_paths` as a `paths:` allowlist, `permissions` top-level with `contents: write` scoped to the publish job, `concurrency` with group and cancel-in-progress, `timeout-minutes` on both jobs — and the output is actionlint-clean across the combination table in fixture tests
- [ ] #3 `push_paths` defaults to a derivation from the plan's known transport inputs (`pixi-sandbox.toml`, `pixi.toml`, `pixi.lock`, `package.json`, `bun.lock`, `Cargo.toml`, `Cargo.lock`, as applicable to the bundle), `init` emits that allowlist, `plan --json` surfaces the derivation so consumers can diff it, and an explicit `push_paths` overrides the derivation
- [ ] #4 The config can pin `pixi_version` for `setup-pixi` and set `setup_pixi_cache = false`; the default for the generated consumer publisher — and the reasoning for cache-off as the correctness-safe default versus current-behaviour compatibility — is recorded as a decision entry in `.knowledge/decisions.md`
- [ ] #5 With policy carried in config, `init` on an existing project reproduces the consumer's workflow byte-identically with no hand edits (fixture test: init, render, init again — identical output), and the generated header's "local policy edits follow" phrasing no longer applies to config-carried policy
- [ ] #6 Docs cover every `[workflow]` key with its default (configuration reference and the ci-publishing guide), and the guide's hand-edit-plus-`--force` ritual is retired in favour of the config
- [ ] #7 Fixture tests per D10 (fixture project, never this repository); gates green (fmt, lint — including `lint-generated-workflow` — and test); ships in a minor release, with the connected proof (a real consumer regenerates without hand edits) recorded or the task left In Progress naming that gap
<!-- AC:END -->

## Implementation Plan
<!-- SECTION:PLAN:BEGIN -->
Config first, renderer second, defaults last. Extend the schema-1 config model parsed by
`cli.rs`/`commands/plan.rs` with an optional `[workflow]` table (serde `deny_unknown_fields`
within the table, default-absent), thread it into `commands/init.rs`, and teach
`generated/github_workflow.rs` to emit each block only when configured. Build `push_paths`
derivation from the same input set `plan` already validates, expose it in `plan --json`,
and add the override path. Keep every emission conditional so the absent-table render stays
byte-identical — that property is the migration path for existing consumers and deserves an
explicit golden test.

The one judgement call to decide explicitly (AC#4): whether the generated publisher's
`setup-pixi` cache default flips to `false` for new configs. The issue argues correctness
(pixi-sandbox owns the native `pixi install` during pack; an action cache keyed on the
consumer manifest can restore a solve the pack will not reuse). Record the choice in
`.knowledge/decisions.md`. Slicing follows the issue's split: the pixi-version/cache pair
first (correctness), then `push_paths` with derivation, then the three independent blocks
(permissions/concurrency/timeouts). No dependency on task-47 — but read its AC#4 shape
before choosing the marker wording, so `init --check` later recognises config-carried
policy as owned, not foreign.
<!-- SECTION:PLAN:END -->
