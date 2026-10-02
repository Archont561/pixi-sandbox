---
id: TASK-48
title: Fix the init channel append that breaks consumer pixi lock (issue 71)
status: In Progress
assignee:
  - '@agent'
created_date: '2026-10-02 20:30'
updated_date: '2026-10-02 21:05'
labels:
  - bug
  - init
  - pixi
  - channels
dependencies: []
references:
  - crates/pixi-sandbox/src/commands/init.rs
  - crates/pixi-sandbox/tests/cli.rs
  - docs/src/content/docs/guides/using-in-your-project.mdx
  - backlog/tasks/task-34 - Add-the-Archont561-prefix-namespace-during-init.md
  - backlog/tasks/task-41 - Preserve-pixi.toml-comments-when-init-adds-the-canonical-channel.md
priority: high
type: bug
ordinal: 49000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Issue #71 (2026-10-02): `pixi-sandbox init` 0.4.3 appends `https://prefix.dev/archont561` —
the publisher **namespace root**, a web page — to the consumer's `[workspace] channels`
(task-34, comment-preserving per task-41). prefix.dev serves repodata only at
`/<owner>/<channel>`, so every consumer project with at least one dependency fails
`pixi lock` immediately after init, with an unhelpful `a coalesced request failed → other
error`; the 404 on `https://prefix.dev/archont561/noarch/repodata.json` surfaces only under
`--verbose`. An empty project locks fine — an empty solve never fetches repodata — which is
the smoke-test trap that let it ship. Confirmed in the wild on the wirewright monorepo:
all four environments failed to lock; removing the appended channel fixed every one.

This falsifies task-34's premise: its AC#2 chose the namespace root *over* the package
channel "so users can consume other Archont561 channels and packages" — but there is no
owner-level aggregation endpoint, so the appended URL is not a channel at all. The issue
offers three fix options: **(1)** append the real channel
`https://prefix.dev/archont561/pixi-sandbox` (minimal; couples consumers to one tool's
channel); **(2)** append an ecosystem channel such as `archont561/main` carrying every
published tool — the option the wirewright repo proposed in its own decision-7; requires
creating the channel and a second `pixi upload` in the release job; **(3)** stop touching
`channels` and print the `pixi global install` command instead — init currently rewrites a
manifest it does not own for an affordance no generated file uses (`pixi global install`
never reads project channels).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 One fix option is chosen and recorded as a decision entry in `.knowledge/decisions.md`, explicitly reversing or amending task-34's namespace-root intent, with the trade-off stated against the two options not taken
- [ ] #2 After `init`, a consumer pixi.toml with at least one dependency passes `pixi lock` on a connected host — the issue's reproduction probe re-run against the fix, output recorded in this task; the empty-project case is explicitly not sufficient evidence, because an empty solve is what hid this bug
- [x] #3 Any URL `init` still appends serves repodata at `<url>/noarch/repodata.json` (probe output or an `#[ignore]`d network test); if no URL is appended, `init` leaves the consumer's `pixi.toml` byte-identical
- [x] #4 Fixture tests cover the chosen behavior per D10 against a fixture project that has a dependency; the task-41 preservation tests are updated to the chosen append or replaced by the byte-identical assertion
- [x] #5 The "Channel configured by init" section of the project guide (and any configuration-reference or README mention) states the chosen behavior, and no documentation presents the namespace root as a consumable channel
- [ ] #6 The fix ships in a released patch package whose `init` passes the reproduction probe on a clean connected host — or the release is explicitly reported as awaiting the online release workflow, per the online-task constraints in CONTEXT.md
- [ ] #7 The pixi-side error surface that hid the cause (`a coalesced request failed → other error` naming neither URL nor status) is reported upstream in pixi/rattler and linked from this task, or the attempt and its outcome are recorded
<!-- AC:END -->

## Implementation Plan
<!-- SECTION:PLAN:BEGIN -->
Decide first (AC#1), then implement the chosen branch. **Option 1** — change
`ARCHONT561_CHANNEL` to `https://prefix.dev/archont561/pixi-sandbox` (the same URL the
generated workflow's `PIXI_SANDBOX_CHANNEL` env already uses), keep the task-41
comment-preserving machinery, update its tests and the guide section. **Option 2** — same
code change against a new `archont561/main` channel, plus online work: create the channel
on prefix.dev, add the second `pixi upload` to release.yml's package job (OIDC is already
configured), and update the README badge/URLs. **Option 3** — delete
`ensure_archont561_channel` and its helpers (`append_channel`, `normalized_channel`,
`source_text`), replace the task-41 channel tests with a byte-identical `pixi.toml`
assertion, rewrite the guide section to print the install command, and decide whether
`init` still requires a `pixi.toml` at all (the channel step is currently the only thing
enforcing "run init from a Pixi project"). Recommended: option 3 now — the smallest correct
surface, fully offline-implementable, and aligned with the ownership boundary the
`GENERATED_MARKER` already draws (init would write only files it owns); record option 2 as
the revisit trigger when a second tool ships under the namespace. Whatever is chosen, add
the with-a-dependency fixture the smoke test lacked, re-run the issue's probe on a
connected host, and let the patch release carry it (auto-release from the merge).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-02: Implemented as the no-mutation option in **PR #73** (commit `fix(init): …`,
net −175 lines), recorded as **D17** (AC#1): `init` writes only the files it owns, leaves
`pixi.toml` byte-identical, and keeps a bare existence check (`ensure_pixi_project`) so a
missing manifest still fails with *run init from a Pixi project* — malformed TOML no longer
does, because the manifest was only ever parsed in order to be edited. Deleted:
`ensure_archont561_channel`, `append_channel`, `normalized_channel`, `source_text`,
`ARCHONT561_CHANNEL`, and the `toml_edit` dependency (both Cargo.tomls plus the single
Cargo.lock edge; `proc-macro-crate` keeps the package in the lock). Tests: the six
channel-append tests are replaced by two — `init_leaves_the_consumer_pixi_manifest_untouched`
(five dependency-carrying / comment-carrying manifests, each run through init twice and
asserted byte-identical, including the issue's probe shape and malformed TOML) and
`init_still_refuses_a_directory_without_a_pixi_manifest`. Docs: both sections rewritten, with
the 0.4.3 remediation (delete the appended entry) spelled out. The `0.4.3` literals in
docs/src/content are not "ours"-lines under `release_refs` (no `releases/download/v…`,
`version: v…`, `PIXI_SANDBOX_VERSION=v…` or `Archont561/pixi-sandbox` on the line), so the
derived-docs literal rule is not tripped.

Verification constraint, stated honestly: this session's sandbox is airlock-shaped (only
github.com and npm reachable — no crates.io, static.rust-lang.org, prefix.dev or conda), so
no local toolchain exists and the PR **is** the test run: the full gate (fmt, clippy
`-D warnings`, deny, actionlint, nextest, doc tests, docs build, check-repository) executes
on PR #73. AC#2's live probe therefore remains open: init can no longer touch the manifest
at all (the fixture suite proves the byte-identical property), so the probe reduces to *pixi
lock passes on an untouched project* — one run on any connected host or a post-merge CI
check closes it. CI outcome on PR #73: `ci (lint · test · coverage)` **success on both
commits** (runs 37064073342 and 37064515288 — fmt, clippy `-D warnings`, deny, actionlint,
nextest, doc tests, coverage, docs build), `validate airlock plan`, `lock guard` and
`codecov/patch` success, PR mergeable; `airlock linux-64` (the released-binary
offline-restore proof) in flight at close. AC#6 needs the merge plus an `auto-release`
dispatch this token cannot start. AC#7: searched `prefix-dev/pixi` for `coalesced request failed`, `repodata 404`,
`verbose 404`, `error does not show url` — no existing issue covers it; filing from the
session token failed with `Resource not accessible by integration` (repo-scoped). The report
is drafted below verbatim for a one-click filing from a personal token:

> **Title:** Solve failure on a 404 channel reports only 'a coalesced request failed →
> other error' — URL and status appear only under --verbose
>
> When a channel listed in the manifest serves no repodata (HTTP 404 on
> `<channel>/<subdir>/repodata.json`), `pixi lock` / `pixi install` fails with an error that
> names neither the URL nor the status (`a coalesced request failed → other error`); only
> `pixi lock --verbose` shows `HTTP status client error (404 Not Found) for url
> (https://prefix.dev/archont561/noarch/repodata.json)`. Reproduction (pixi 0.81.0,
> linux-64): a workspace with `channels = ["conda-forge", "https://prefix.dev/archont561"]`
> and one dependency (`bun = "=1.3.11"`) fails `pixi lock` with the coalesced error; with no
> dependencies the same manifest locks fine (an empty solve never fetches repodata, so a
> broken channel stays silent until the first real solve). Suggested improvement: in the
> coalesced-request error path, when an underlying failure is an HTTP status error for a
> repodata URL, propagate the URL and status into the rendered top-level error so `pixi
> lock` alone is enough to fix a bad channel entry. Real-world occurrence: a tool appended
> the namespace-root URL to a four-environment monorepo and every environment failed with
> the unhelpful message (Archont561/pixi-sandbox#71).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
In progress in PR #73: the channel append is gone (D17), the manifest is byte-identical by
construction and by test, the docs tell the truth and carry the 0.4.3 remediation. Open: the
live `pixi lock` probe on a connected host, the patch release (merge + auto-release
dispatch), and the upstream pixi report (drafted in the notes; the session token cannot file
cross-repo).
<!-- SECTION:SUMMARY:END -->
