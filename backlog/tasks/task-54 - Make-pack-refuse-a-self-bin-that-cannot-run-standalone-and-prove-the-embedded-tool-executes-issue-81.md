---
id: TASK-54
title: >-
  Make pack refuse a self-bin that cannot run standalone and prove the embedded
  tool executes (issue 81)
status: In Progress
assignee: []
created_date: '2026-10-03 08:04'
updated_date: '2026-10-03 10:12'
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
- [x] #2 `pack --self-bin` runs a standalone-execution probe before embedding — executes the candidate's `--version` with an isolated environment (empty tempdir HOME, no pixi/conda prefixes) — and refuses a failing candidate with a remedy naming the standalone release asset; the probe sits behind an injected runner (the `ReleaseSource` precedent) so fixture tests exercise a trampoline-shaped fake that demands `trampoline_configuration` and a healthy static fake, with no network and no real HOME (D10)
- [x] #3 `doctor` flags an embedded tool that fails the same standalone probe (or its structural trampoline signature), so a fetched branch in the consumer's exact state — faithful copy, unrestorable tool — is diagnosed *before* a restore is attempted, while a healthy transport passes unchanged
- [x] #4 The e2e lifecycle (`tests/e2e.rs`, pack → publish → restore) gains an isolated-environment assertion that the packed branch's `tools/<platform>/pixi-sandbox --version` executes without any pixi global prefix, so a green publisher run proves the embedded bootstrap is executable — closing the "successful publish ≠ restorable" gap the issue names
- [x] #5 The ci-publishing guide (and any `--self-bin` reference) states the source must be a standalone binary, names the `command -v pixi-sandbox`-from-`pixi global install` trap explicitly, and gives the correct interim recipe (checksum-verified standalone asset download) until task-52 ships
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-03 — implemented (AC#2–#5). The design landed as planned: a two-layer guard in the
new library module `crates/pixi-sandbox/src/standalone.rs` (`pub mod` in `lib.rs`, tested
only from `tests/standalone.rs` — no inline test module, per convention).

**Structural layer** — `pack_refusal_for_ownership` reuses the task-47 ladder
(`self_update::ownership::classify`) on the *candidate path*: a `pixi global` trampoline
(its sibling `trampoline_configuration/<name>.json` exists in the global prefix — exactly
the context a transport can never ship) and a managed launcher script are refused *before*
any embedding, each with its own remedy. A conda-prefix binary and a restored transport
tool are deliberately *not* refused: they are real binaries, and a transport tool is a good
self-bin for a repack; execution judges them.

**Behavioural oracle** — after `embed_tool`, `pack` probes the *embedded copy* (the exact
bytes that will ship) via `ProbeRunner::run_version`: `--version` under `env_clear`,
HOME/USERPROFILE/TMP anchored at a scratch dir, minimal PATH, null stdin, pipes drained on
reader threads capped at 16 KiB each, killed at a 30 s budget. The fake-runner injection
(ReleaseSource precedent) scripts every verdict offline; the real `CommandRunner` is
covered by in-process script tests. Cross-platform packs skip with a printed reason —
probing foreign bytes would be a lie, and doctor on the target host is the same guard.

**Doctor** — the probe runs on the embedded `pixi-sandbox` tool under `--verify`, *after
and only after* the hash report is green (unverified bytes are never executed — the trust
order, tested by the tamper test), exits non-zero with the rendered refusal on failure, and
prints a `probe skipped — <reason>` line for platform mismatch or red hashes. Human label
is `probe` (short, matching `verify`/`restored`); JSON key `standalone`. A requirement the
issue's wording did not spell out and verify.rs confirmed: a self-bin's content is *not
pinned into the manifest at all* (`ToolEntry.pinned_sha256` is `None` for self-bins — only
size + linkage are checked), so hashes were never even theoretically able to see issue #81;
that fact is now in the description and in the refusal text.

**Evidence**: suite 500 → **516 passing / 1 skipped** (+16: 11 in `tests/standalone.rs`, 4
new in `tests/cli.rs` — both pack refusals, the recreated consumer state, the
never-execute-unverified test — and 1 e2e: the AC#4 isolated execution plus the doctor OK
line). Gates: `fmt`, `lint` (clippy `-D warnings`, deny on cached index, actionlint, taplo,
biome, check-repository), `test` — all green. Binary smoke (built debug binary, not the
suite): healthy fixture via a *relative* `--branch-location` reports `verify OK` +
`probe tool pixi-sandbox v0.1.0: runs standalone` and exits 0; the recreated consumer
state (trampoline script + manifest size fixed to stay faithful) reports `verify OK` then
`probe FAILED — …` with the trampoline's own error text as evidence and the remedy, exit 1.

**Three defects found by smoke-testing the binary, not the suite** (same lesson task-47
recorded): (1) my refusal strings used the `\n\\` continuation idiom wrongly, so every
rendered message carried literal backslashes — the suite's `contains` assertions were blind
to it; fixed to the ownership.rs `\n\` idiom. (2) A relative `--branch-location` produced
`ENOENT` in the probe: the runner chdir'd to the anchor and the relative tool path was
re-resolved against it; the runner now absolutizes both paths, with a regression test
(`the_runner_resolves_relative_paths_against_the_calling_cwd`). (3) The `standalone` label
is 10 chars and glued to its text (`standaloneFAILED`); renamed the human label to `probe`.
A fourth was caught by the first full run: a killed candidate that had forked left an
orphaned grandchild holding the probe pipes, so joining the reader threads waited out the
orphan's whole `sleep` (the kill reaches one pid, not the tree). The refusal path now
detaches the readers instead of joining — verdict already decided, the refusal is prompt;
the regression test asserts a 150 ms timeout returns in well under 10 s.

**AC#1 — partial, named gap.** Root cause pinned and recorded in the description:
consumer's issue-#80 workaround packed `$(command -v pixi-sandbox)` (a pixi global
trampoline). The release-asset theory is answered at the *pipeline* level: release.yml
stages standalone assets with `xtask stage-release-binary` straight from cargo's
`target/<triple>/release/` output, so the v0.5.0 asset cannot be a trampoline. The
byte-level proof (downloading the asset and executing it in a bare HOME) was impossible
from this sandbox: `release-assets.githubusercontent.com` is egress-blocked (`gh release
download` → EOF), so that sentence of AC#1 stays unproven and the AC unchecked — one
command on a connected host closes it.

AC#6 is untouched by definition (merge → auto-release patch → consumer repack → fresh
consumer restore). It stays open with the release flow.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
The hole issue #81 exposed is closed at both ends of the pipeline, as a behavioural oracle
rather than another declarative check. `pack --self-bin` refuses the known-broken shapes on
provenance (`pixi global` trampoline, managed launcher — before embedding, each with its
remedy) and proves every other candidate by executing exactly the shipped bytes —
`--version` under an empty environment — refusing with the child's own failure text, the
reason hashes were green anyway (a self-bin's content is not pinned into the manifest), and
the one remedy that works (checksum-verified standalone asset, never
`$(command -v pixi-sandbox)`). `doctor --verify` applies the same probe to a fetched
transport after, and only after, every declared byte matches; the recreated consumer state
(a faithful but unrestorable branch) now fails loudly with exit 1 instead of blessing the
push. The e2e lifecycle asserts the packed branch's embedded tool executes with no pixi
prefix, and the docs name the trap in both the guide and the CLI reference.

AC#2–#5 are met and checked. AC#1's root-cause half is recorded; its asset-execution half
and AC#6 (patch release + consumer repack + fresh restore) remain open — both need a
connected host or a release, so the task stays In Progress naming exactly those proofs.
<!-- SECTION:SUMMARY:END -->



