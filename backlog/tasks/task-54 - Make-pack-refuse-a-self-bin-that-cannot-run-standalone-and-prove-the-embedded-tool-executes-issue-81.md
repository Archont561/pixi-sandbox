---
id: TASK-54
title: >-
  Make pack refuse a self-bin that cannot run standalone and prove the embedded
  tool executes (issue 81)
status: To Do
assignee: []
created_date: '2026-10-03 08:04'
labels:
  - bug
  - pack
  - doctor
  - restore
dependencies: []
references:
  - crates/pixi-sandbox/src/commands/pack.rs
  - crates/pixi-sandbox-core/src/verify.rs
  - crates/pixi-sandbox/src/commands/doctor.rs
  - crates/pixi-sandbox/tests/e2e.rs
  - crates/pixi-sandbox/src/self_update/ownership.rs
  - crates/xtask/src/release_assets.rs
  - docs/src/content/docs/guides/ci-publishing.mdx
priority: high
type: bug
ordinal: 55000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Issue #81 (2026-10-03, <https://github.com/Archont561/pixi-sandbox/issues/81>): the
published consumer transport on `qgis-rs` (`sandbox/developer-linux-64`, tip `514c224`,
packed 2026-10-03T01:55Z by a *successful* publisher run) cannot be restored: the embedded
`tools/linux-64/pixi-sandbox` dies before even `--version` with

```
Couldn't open "…/tools/linux-64/trampoline_configuration/pixi-sandbox.json"
```

— the signature of a **pixi global launcher trampoline**, which resolves its configuration
relative to the executable's own directory, not of the standalone release binary the step
believed it had. Evidence assembled in this repo, 2026-10-03:

1. The standalone v0.5.0 *release asset* cannot be what the title suspects: release.yml
   stages it with `xtask stage-release-binary <target>`, which copies cargo's
   `target/<triple>/release/pixi-sandbox` (`crates/xtask/src/release_assets.rs`). The one
   remaining confirmation — executing the downloaded asset itself — needs a connected host:
   this sandbox's egress blocks `release-assets.githubusercontent.com` (EOF on download),
   so the asset was never fetched here.
2. A healthy embedded tool passes the issue's "Expected" list trivially: this repo's own
   restored transport tool (v0.5.0, 3.5 MiB, `readelf -d` shows zero `NEEDED` — static)
   runs `--version` copied into a bare directory under `env -i` with an isolated HOME.
3. The trampoline reached the consumer's transport through the issue-#80 workaround: with
   the generated download step 404ing (task-52's bug), the consumer pointed the pack at
   `SELF_BIN="$(command -v pixi-sandbox)"`, which — after the workflow's
   `pixi global install pixi-sandbox==${PIXI_SANDBOX_VERSION}` step — is the *global
   trampoline* at `~/.pixi/bin/pixi-sandbox`, not the standalone binary.

The repo-side defect that turned that workaround into a silently broken, force-pushed
orphan branch: **nothing ever executes the embedded tool standalone.** `pack --self-bin`
accepts any file; `doctor` verifies sha256s — and the trampoline was copied *faithfully*, so
hash verification passes; `verify.rs`'s dynamic-linkage flag (AGENTS.md invariant 7) cannot
catch a statically linked trampoline either; and a green publisher run therefore proves
nothing about restorability — the issue's own closing point. task-47's ownership ladder
already encodes the trampoline signature (sibling `trampoline_configuration/<name>.json`),
but only for classifying self-update *destinations*; nothing consumes that knowledge at pack
time.

Relationship to task-52: task-52 removes the consumer's trigger (the 404 that made the
workaround necessary); this task closes the tool-side hole that made the trigger fatal —
and it also guards every future self-bin source, workaround or not. Independent tasks,
either order works; this one is the higher-severity half because today it is a published,
unrestorable transport.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Root cause pinned and recorded in this task: the consumer transport's embedded tool is a pixi global trampoline introduced by the issue-#80 workaround, and the issue's release-asset theory is resolved one way or the other — a downloaded v0.5.0 standalone asset executed with `--version` in an isolated bare HOME on a connected host (impossible from this sandbox: `release-assets.githubusercontent.com` is egress-blocked), with the outcome recorded here
- [ ] #2 `pack --self-bin` runs a standalone-execution probe before embedding — executes the candidate's `--version` with an isolated environment (empty tempdir HOME, no pixi/conda prefixes) — and refuses a failing candidate with a remedy naming the standalone release asset; the probe sits behind an injected runner (the `ReleaseSource` precedent) so fixture tests exercise a trampoline-shaped fake that demands `trampoline_configuration` and a healthy static fake, with no network and no real HOME (D10)
- [ ] #3 `doctor` flags an embedded tool that fails the same standalone probe (or its structural trampoline signature), so a fetched branch in the consumer's exact state — faithful copy, unrestorable tool — is diagnosed *before* a restore is attempted, while a healthy transport passes unchanged
- [ ] #4 The e2e lifecycle (`tests/e2e.rs`, pack → publish → restore) gains an isolated-environment assertion that the packed branch's `tools/<platform>/pixi-sandbox --version` executes without any pixi global prefix, so a green publisher run proves the embedded bootstrap is executable — closing the "successful publish ≠ restorable" gap the issue names
- [ ] #5 The ci-publishing guide (and any `--self-bin` reference) states the source must be a standalone binary, names the `command -v pixi-sandbox`-from-`pixi global install` trap explicitly, and gives the correct interim recipe (checksum-verified standalone asset download) until task-52 ships
- [ ] #6 The fix ships in a patch release; the affected consumer transport is repacked with a standalone self-bin and a fresh consumer restore passes — a connected/CI proof recorded here, or the task stays In Progress naming exactly this gap
<!-- AC:END -->

## Implementation Plan
<!-- SECTION:PLAN:BEGIN -->
Behavioural probe first, structural hints second. Add a `StandaloneProbe` in the library
(promoted `pub mod` per the test conventions — one test file under `tests/`), modelled on
`release.rs`'s injection shape: the real runner executes `<candidate> --version` with
`env_clear`, a scratch tempdir as `HOME`, and a minimal `PATH`; tests inject a fake runner
scripted with the trampoline failure and the healthy exit-0. Wire it into
`commands/pack.rs` (ambient wiring only) so `--self-bin` is probed before anything is
embedded, and into `commands/doctor.rs` so the fetched transport's tool gets the same
verdict as a *finding that fails the verify* for the tools entry it owns (sha256 faithfulness
stays a separate line — a transport can be faithful and unrestorable, and the operator
needs both facts).

A structural pre-check (sibling `trampoline_configuration/<name>.json`, the task-47
ownership-ladder signature) may run first as a fast, execution-free hint with its own error
wording — but the behavioural probe is the oracle, because it catches every broken-tool
shape, not just today's trampoline. Do not extend `verify.rs`'s linkage check: a static
trampoline would pass it, and blurring "is static" with "runs standalone" muddies
invariant 7.

Then the e2e assertion (AC#4) in the existing fixture-backed lifecycle — on Linux the
restore half already re-runs under `unshare -rn`, so the isolated-exec assertion rides the
same shape with an emptied environment. Docs (AC#5), then the patch release rides
auto-release; the repack + fresh-restore proof on the real consumer (AC#6) is connected
work, sequenced last. Read task-52's constant when writing the AC#5 recipe so the guide's
download URL and the eventual template fix cannot drift apart.
<!-- SECTION:PLAN:END -->


