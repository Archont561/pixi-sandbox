---
id: TASK-47
title: Give consumers a self-updating binary and reviewed generated-file upgrade path
status: In Progress
assignee: []
created_date: '2026-10-02 20:05'
updated_date: '2026-10-03 14:33'
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
  - crates/pixi-sandbox/src/commands/self_update.rs
  - crates/pixi-sandbox/src/commands/tools/update.rs
  - crates/pixi-sandbox/src/commands/doctor.rs
  - .github/workflows/publish-sandbox.yml
  - docs/src/content/docs/guides/ci-publishing.mdx
  - crates/pixi-sandbox/tests/self_update_replace.rs
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

Add a standalone-binary `pixi-sandbox self-update` command. With no version it resolves the
latest published release; with `--version X.Y.Z` it fetches that exact release. In both cases it
selects the native standalone asset, verifies it against the release's `SHA256SUMS`, and replaces
the managed binary safely. Package-manager-owned binaries and global-install trampolines are not
silently overwritten: CI bootstraps an exact, checksum-verified standalone binary into its own
temporary path and self-updates that binary.

The generated consumer CI calls the self-updater only in its upgrade lane, then uses the updated
binary to run the drift check and regenerate all owned files into a pull request. The regenerated
`publish-sandbox.yml` contains the new exact version; after review and merge, its ordinary publish
job uses that exact binary as driver and `--self-bin`, so the transport manifest and embedded
bootstrap both move to the reviewed version. `latest` is therefore discovery for a proposed PR,
never a floating production pin. This repository's owner-level publisher remains the source-built
exception and does not install or self-update a released CLI.

Give consumers the same drift policy this repository already enforces on itself and the same bot
precedent the relock workflow set (task-39), under the four-domain split recorded as decision-4 /
D16: the binary **self-updates in the upgrade lane**, generated files **regenerate**, config
**migrates explicitly behind review**, and the transport **repacks** when the regenerated workflow
lands on main.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `pixi-sandbox self-update` supports latest-by-default and an exact `--version X.Y.Z`; it maps every supported host to the canonical standalone release asset, downloads that asset plus `SHA256SUMS`, refuses a missing checksum entry or digest mismatch, and never executes or installs unverified bytes — resolver, platform-map, checksum, HTTP-failure, and pinned-version paths are fixture-tested without live network access
- [x] #2 Self-update stages beside the destination and replaces atomically on Unix and Windows without leaving a partial executable; `--check` reports the current and resolved versions without writing; package-managed binaries/global-install trampolines are refused with a remedy, while an explicit CI-managed standalone path is supported and covered by tests
- [x] #3 Every file `init` writes carries a machine-readable version stamp beside the ownership marker (a `pixi-sandbox-version: X.Y.Z` line, within the first three lines the marker check already reads) for the publisher workflow, the relock workflow, and the launcher; fixture tests cover presence and parsing on every render
- [x] #4 `pixi-sandbox init --check` renders fresh, writes nothing, and exits non-zero naming each drifted file with its remedy (`pixi-sandbox init`) and each foreign-owned file separately (`--force`); a clean tree exits 0 — fixture-tested in both directions (D10)
- [x] #5 `init` and `--check` never rewrite an existing config; a config whose `schema` is older than the CLI's is reported as a finding naming the explicit migration path (no `config migrate` command is built while config schema is 1 — it exists only once a schema bump gives it something to do), and config readers keep accepting older schemas for a deprecation window, mirroring the manifest reader policy
- [x] #6 The generated `publish-sandbox.yml` gains a scheduled and manually dispatchable upgrade job that bootstraps its currently pinned standalone binary with checksum verification, runs `self-update` (latest by default, exact version for manual dispatch), then uses that updated binary for `init --check` and regeneration; drift opens a pull request and never pushes to main or touches config, and the workflow remains actionlint-clean
- [ ] #7 The regenerated pull request contains a new exact `PIXI_SANDBOX_VERSION` and the templates emitted by that same binary; a human merge triggers the normal publisher, which uses the exact matching released binary for plan, pack, doctor, publish, and `--self-bin`, and the resulting manifest names that version — no production step resolves `latest`
- [x] #8 Because a `github.token` push starts no `on: push` workflows (task-44's lesson), any automated-merge path ends in an explicit `gh workflow run publish-sandbox.yml` dispatch; the workflow and docs distinguish this from a human merge and tests hold the dispatch behavior
- [x] #9 The repository owner workflow remains source-built and has no released-CLI/self-update dependency; the airlock side is unchanged (`restore.sh`/`restore.ps1` stay version-agnostic and doctor reads older manifests), and docs cover self-update, exact pinning, the reviewed upgrade PR, and rollback by dispatching an exact older version
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Build the updater as a transport-independent slice first: a release resolver (latest or exact
version), reviewed host-to-asset map, checksum parser/verifier, and injected downloader, followed
by platform-specific stage-and-replace. Keep it scoped to standalone binaries: detect and refuse
package-manager/global-trampoline ownership rather than corrupting a Pixi installation. Add
`--check` before the mutating path so CI and users can inspect the resolved transition.

Next stamp the three renderers plus launcher templates with the version line (the marker stays
within the first three lines `ensure_replaceable` reads), then add `init --check` as a
render-and-compare over the same four paths `init` writes. It reports all findings, writes
nothing, and separates owned drift from foreign files.

Finally change the generated publisher's upgrade lane, not its production pinning: download and
verify the workflow's exact current standalone binary into `$RUNNER_TEMP`, ask that binary to
`self-update` to latest (or the manually supplied exact version), and invoke the updated path for
`init --check` and regeneration. Open a PR containing the regenerated files; do not edit config,
push main, or publish from the update job. The reviewed files must name the resolved exact version.
After merge, the ordinary publisher downloads that exact release once and uses the same verified
binary for plan, pack, doctor, publish, and `--self-bin`, proving the transport contains the new
version rather than merely updating workflow text. Preserve the explicit publish dispatch for an
automated merge and keep this repository's owner publisher source-built. Close with actionlint,
fixture/e2e coverage, docs, and regeneration of committed consumer fixtures.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-03 owner direction: replace the proposed "install latest package CLI" upgrade lane with
a binary-native self-update command. CI must update a checksum-verified standalone binary, use
that exact updated binary to regenerate the owned files, and let the reviewed regenerated
publisher carry the resolved exact version into the next transport. The owner repository remains
source-built; latest resolution is never part of production publishing.

2026-10-03 slice 1 — the self-update core (v0.5.0 minor feature; the earlier hand-off called
this v1.0.0, the owner re-scoped it to a minor). Landed: `crates/pixi-sandbox/src/release.rs`
(the `ReleaseSource` trait, `Asset` and `GitHubReleaseSource` lifted out of
`commands/tools/update.rs` so the updater and the pin refresher share one injection point and
one fake, rather than two that drift), and `crates/pixi-sandbox/src/self_update/` with
`resolver` (latest-by-default / exact `--version`, strict `X.Y.Z`), `assets` (the five-row
host-to-asset map), `checksums` (`SHA256SUMS` parse + verify), `ownership` (the refusal
ladder) and `replace` (stage-before-replace). CLI: `self-update [--version X.Y.Z] [--check]
[--dest PATH] [--repo OWNER/NAME]`.

Order of enforcement is the trust boundary and is tested as such: resolve, classify the
destination, *then* download. A refused destination costs zero downloads
(`a_pixi_managed_destination_is_refused_without_downloading_a_single_byte`), and `SHA256SUMS`
is fetched before the binary so no unjudgeable bytes are ever held.

Ownership is decided on path provenance, never file contents: managed launcher (the
`user_tools` marker, size-capped so a binary that merely contains the marker string is not
misread), `pixi global` trampoline (sibling `trampoline_configuration/<name>.json`, the signal
`xtask airlock` already follows), conda/Pixi prefix (an ancestor `conda-meta/`), restored
transport tool (`.pixi/tools/<platform>/`), else standalone. First match wins and each refusal
carries its own remedy. No `--force`: an escape hatch would reintroduce the corruption the
ladder exists to prevent.

Replacement follows task-37 AC#3. Unix renames straight over the destination (atomic, and
ETXTBSY-proof because it leaves the busy inode to the running process — asserted by holding an
open handle across the swap). Windows renames the running image aside to a
`.pixi-sandbox-old-<version>` sibling, moves the new file in, and sweeps the corpse at the
*start* of the following update, because the unlink is expected to fail while the old image is
still mapped; a failed second rename rolls the first one back. `ReplaceStrategy` is a parameter,
not a `cfg!` read, so both paths run on every host (the `LauncherKind` precedent).

Evidence: suite 404 -> 476 passing / 1 skipped; `pixi run --frozen lint` green (clippy
`-D warnings`, deny, actionlint, taplo, biome, check-repository). Fixtures and an in-memory
fake release only — no socket is opened by any test, and no test writes to a real user binary
(D10). Two defects were found by smoke-testing the built binary rather than by the unit tests:
clap's propagated `--version` collided with the requested-release `--version` (fixed with
`disable_version_flag`, and `Cli::command().debug_assert()` is now a test so the whole command
tree is audited), and the first rollback test passed vacuously on Linux (rewritten to drive
`windows_swap` directly). Against the real machine, all four refusals fire correctly on the
restored transport tool, the registered `~/.local/bin` launcher and a synthetic conda prefix.
The host-to-asset map was diffed against the actual v0.4.3 release and covers exactly the five
published standalone assets.

AC#2 is deliberately left unchecked. Every behaviour it names is implemented and tested —
staging beside the destination, atomic replacement under both strategies, non-writing
`--check`, package-managed refusal with a remedy, an explicit CI-managed standalone path — but
the Windows half is proven only against the strategy parameter on Linux. The outstanding proof
is a Windows runner replacing a genuinely running image, which no local run can produce; it
belongs with the release matrix in a later slice. Live-network resolution is also unproven
here: this sandbox's egress proxy terminates TLS with a CA `ureq` does not trust, so
`api.github.com` and the release asset host are unreachable from the binary (the failure is
reported cleanly, with the URL and cause).

Not built in this slice, by instruction: AC#3-#8 (the version stamp, `init --check`, the
generated upgrade job and its dispatch) and the v1.0.0 cut. Decision-4 stays `proposed`.

2026-10-03 merged as PR #76 (squash `79e4f83`). Post-merge `ci`, `docs` and `publish sandbox`
all green; the repacked `sandbox/developer-linux-64` manifest names source `79e4f83`,
pixi-sandbox 0.4.4, static. Re-baselined on the merged tree: **500 passing / 1 skipped**
(404 at session start).

Review feedback folded in before merge. `codecov/patch` failed on the first push, for two
reasons that were design problems rather than gaps to waive. (1) `commands/` is private to the
binary target, so its only test route is spawning the binary — and a separate process earns no
coverage credit and can assert only on substrings; the report formatting and the whole decision
flow (`self_update::run`) therefore moved into the library, leaving `commands/self_update.rs` as
the ambient-input wiring that genuinely cannot be tested offline. (2) Lifting
`GitHubReleaseSource` into `release` turned long-uncovered HTTP code into *newly added* lines;
it now sits alone in `release/github.rs`, excluded by name from `pixi run coverage`, so the gap
is one documented file instead of a number smeared through testable logic. New-code coverage
finished at 93.67% against a project at 84.91%.

AC#2 remains unchecked, with the same missing proof as before: a Windows runner replacing a
genuinely running image. Everything it names is implemented and covered against the
`ReplaceStrategy` parameter on Linux, including the rollback and aside-failure branches, but the
OS behaviour the Windows arm assumes — that a running image can be renamed but not deleted — is
not exercised by any run that has happened. Live-network resolution is likewise unproven: the
dev sandbox's egress proxy terminates TLS with a CA `ureq` does not trust, so the binary cannot
reach `api.github.com` from it. Both belong to a later slice with a native matrix.

2026-10-03 slice 2 — version stamps and the `init --check` drift gate (AC#3-#5), done offline
per owner direction to implement task-47 and task-53 locally in one pass and open a PR.

AC#3: `generated::MARKER_WINDOW = 3` replaces the hardcoded 3 `ensure_replaceable` and the new
drift classifier both read, so the ownership-marker window and the version-stamp window can
never drift apart. `version_stamp_line`/`parse_version_stamp` render and parse a bare
`pixi-sandbox-version: X.Y.Z` line (comment-syntax prefix supplied by each template, matching
how `GENERATED_MARKER` already works, so one convention covers YAML, POSIX `sh` and pwsh — a
first draft that had the helper own the `#` doubled it in the YAML template, caught immediately
by the golden-fixture test). The publisher template, the relock template (stamped with the new
`RelockWorkflowOptions::cli_version`, a field distinct from the existing `pixi_version` which
pins the sandboxed pixi binary, not the CLI release — tests assert the two are never confused),
and both launcher templates (`posix_restore`/`powershell_restore`, now taking a `version`
parameter) all carry it. Tests: `tests/generated_stamp.rs` (new — `generated/mod.rs` has no
inline tests because it is not in `tests/fixtures.rs`'s `LEGACY` grandfather list), a new
`generated_workflow_carries_a_parseable_version_stamp` in `tests/generated_workflow.rs`, an
inline relock test asserting the stamp names the CLI release and never the pinned pixi version,
and an inline `launchers_carry_a_parseable_version_stamp` in `commands/init.rs`.

AC#4: `init --check` shares a new `render_targets()`/`Targets` builder with the writing path, so
the comparison is always against the exact bytes `init` would write — never a second
implementation that could drift. Each owned file (workflow, relock, launcher) classifies as
Missing / Foreign (exists, no marker — remedy `--force`) / Stale (owned, content differs —
remedy `pixi-sandbox init`) / Current; config is audited separately (AC#5). All findings print,
nothing is written in either success or failure, and the command exits non-zero iff there is at
least one finding. New `--check` flag on `InitArgs`, `conflicts_with = "check"` on `--force`.
Fixture-tested in both directions under `commands::init::tests::check` (new inline tests — this
module is in `tests/fixtures.rs`'s `LEGACY` list already): a freshly-initialised tree passes and
provably writes nothing byte-for-byte; a hand-edited owned file is reported as drifted and left
untouched; a file with no marker is reported as foreign, separately, and classified correctly;
a missing owned file is its own finding; a config schema newer than this CLI is a named finding.

AC#5: mirrored `manifest.rs`'s exact reader policy into `sandbox_config.rs` — doc comment,
`schema_supported(schema) -> (1..=CONFIG_SCHEMA).contains(&schema)`, and `validate()` now calls
it instead of a hard `!=` bail, so an older schema stays accepted for a deprecation window and
only a newer one is refused. Added `peek_schema(path) -> Result<u32>` (a minimal
`#[derive(Deserialize)] struct SchemaOnly { schema: u32 }`) so `init --check` can name a schema
finding even when the rest of the file would fail `SandboxConfig::load`'s stricter
`deny_unknown_fields`. `init`'s `default_config()` already only wrote when the config was
absent, so "never rewrite" required no change — `--check` reuses the same existing-file guard.
No `config migrate` command is built, as the AC anticipates: schema has never bumped past 1, so
there is nothing yet to migrate. Tests: `schema_support_is_an_inclusive_range_from_one_to_current`,
`peek_schema_reads_the_field_even_when_the_rest_of_the_file_is_unrecognisable`,
`load_refuses_a_schema_newer_than_this_build_understands` (sandbox_config.rs), plus
`init_never_rewrites_an_existing_config_in_either_mode`, `a_newer_config_schema_is_a_named_finding`
and `the_current_schema_produces_no_finding` (init.rs).

The committed `.github/workflows/relock.yml` was re-rendered via `xtask render-relock` to pick
up its new version-stamp line (`xtask check-repository`'s check #10 catches exactly this drift
and did, correctly, before the re-render). The repo's own committed `publish-sandbox.yml` is
left untouched, per task-52's established precedent — it is documented vestigial, not
regenerated or byte-checked by any gate.

Evidence: `cargo clippy --workspace --all-targets -- -D warnings` clean; `xtask
lint-generated-workflow` and `xtask check-repository` both clean; `pixi run --frozen test`:
**573 passed / 1 skipped** (557 at session start after task-52, net +16 from this slice).

Still open after this slice: AC#2 (unchanged, blocked on a Windows runner), AC#6-#9 (the
generated upgrade job, exact-version pinning through to the transport, the explicit post-merge
dispatch, and docs) — targeted next in the same session.

2026-10-03 slice 3 — the generated upgrade job (AC#6, #8, #9), same session.

`GithubWorkflowOptions` gained `workflow_path`, `relock_workflow_path`, `relock_ci_workflow`,
`script_path`, and `branch` — the exact arguments `init` was invoked with, baked into the
rendered file as literals (the same pattern `config_path` already used) so the upgrade job's
own `init`/`init --check` calls reproduce the *exact* generation invocation rather than relying
on defaults that silently diverge the moment a project customises any path.

The publisher template gained a `schedule:` trigger (weekly) and a `workflow_dispatch.inputs.
upgrade` string (blank by default, so an existing manual dispatch keeps doing an ordinary
publish with zero behaviour change). A new `upgrade` job: `if: event_name == 'schedule' ||
(event_name == 'workflow_dispatch' && inputs.upgrade != '')`, exactly the negation of the
`plan`/`publish` jobs' new `if:` — the two lanes can structurally never both fire on one event,
asserted directly in `upgrade_job::the_publish_lane_and_the_upgrade_lane_can_never_both_fire`.

The job: downloads the currently-pinned standalone binary with the same checksum verification
`publish` already uses, asks that *exact* binary to `self-update --dest "$BIN"` (latest, or the
dispatch-supplied exact version) — so only a verified self-update, never a floating resolver,
ever decides what gets written — then runs the *updated* binary's own `init --check` against
the baked-in exact paths/branch/config. A clean result stops here, nothing written, nothing
proposed. A drifted result re-runs `init` for real, stages only the three owned paths
(explicitly never the config — D16; the test
`upgrade_job::the_regenerated_commit_never_stages_the_config` holds this), commits as
`pixi-sandbox[bot]` (the same identity `relock.yml` already uses), pushes a side branch, and
opens a PR with `gh pr create` — never a push to `main`. The PR body names the exact explicit
dispatch an automated merge still needs (task-44's `github.token`-push lesson, restated for this
lane): `gh workflow run .github/workflows/publish-sandbox.yml --ref main` (AC#8). The *same*
dispatch input doubles as the rollback path: naming an older released version proposes
downgrading the generated files to it, reusing `self-update`'s existing exact-version support —
no new code, just the existing input used in the other direction.

No new third-party action is pinned: every new step is a pinned `uses:` already present or a
single `run: |` block, consistent with the publisher template's existing exemption from the
repository's own single-command workflow-shape rule (it was already multi-line by necessity).

Tests: a new `upgrade_job` module in `tests/generated_workflow.rs` (9 tests) checks the mutual
exclusion of the two lanes, the bootstrap-then-self-update order, that drift-check and
regeneration use the exact baked-in generation arguments, that the config is never staged, that
no step ever pushes `main` directly, that the PR body names the exact dispatch command, the bot
identity, and the job-level `permissions:` block (which replaces rather than adds to the
workflow-level one, same trap `relock.yml` already documents, so `pull-requests: write` has to
be named explicitly or `gh pr create` 403s). The golden fixture and `xtask lint-generated-
workflow` (actionlint) both regenerated/re-verified clean. `xtask check-repository` clean — the
repository's own vestigial `publish-sandbox.yml` stays untouched, same as slice 2.

Docs: `reference/cli.mdx` documents `init --check`'s finding types and remedies and the new
version stamp; `guides/ci-publishing.mdx` gained a "Staying current: the upgrade job" section
covering the schedule, the manual-dispatch exact-version input, the review-then-merge flow, the
required explicit post-merge dispatch, and rollback by naming an older version. `pixi run docs`
(astro build) and `pixi run lint-docs` (biome) both pass on the new content.

AC#7 stays unchecked by necessity, the same shape as AC#2 and task-52's AC#6: everything it
names is implemented and structurally tested (the exact-version stamp flows automatically
because `init` always stamps its own `CARGO_PKG_VERSION`, and the unchanged `plan`/`publish`
jobs already install and bootstrap that exact pin), but "the regenerated pull request" and "the
resulting manifest names that version" describe a live run this sandbox cannot produce without
a connected GitHub host and a real cut release.

Evidence: `cargo clippy --workspace --all-targets -- -D warnings` clean; `xtask lint-generated-
workflow` and `xtask check-repository` clean; `pixi run docs` and `pixi run lint-docs` clean;
`pixi run --frozen test`: **581 passed / 1 skipped** (573 before this slice).

2026-10-03 slice 4 — the Windows running-image proof (AC#2), and the three defects the first CI run exposed.

AC#2 is checked. ci.yml gained a `replace-running-image` job on windows-latest running `test-self-update-replace` against the empty `package` environment: `pixi.lock` carries no win-64 entries, so nothing else resolves on that runner, and the job therefore uses the runner-provided Rust toolchain exactly as the release build leg already does. The test spawns a copy of itself as a child that holds the mapped image open, replaces it through `windows_swap`, and asserts the corpse survives this update at its original byte length, that no staged file is left, that the running process is undisturbed, and that the corpse is swept by the next update once the image is released.

It does not make win-64 a supported platform: `pixi.lock` still carries no win-64 entries, the README Windows gap (no conda-forge `bun`, D11) stands, and `.pixi-sandbox.toml` still publishes linux-64 only.

`self_update = 1.3.0` (crates.io) had been added to `crates/pixi-sandbox/Cargo.toml` and was never referenced: `mod self_update` in `commands/mod.rs` shadows the extern prelude, so every `self_update::` in the tree is the local module. It dragged in reqwest -> rustls -> aws-lc-rs -> aws-lc-sys, whose C build needs cmake and cannot link against conda glibc (`rust-lld: undefined symbol: __isoc23_sscanf`); that failed `ci` at `lint-generated-workflow` and the airlock matrix job before either reached a test. Dropped, and `Cargo.lock` sheds 1485 lines. The trust boundary stays hand-written, as recorded in slice 1.

The torn-write canary watched the destination from before the swap and flagged every read that was not the expected bytes, so the first reads of the untouched image counted as torn and the windows job failed on the size of the destination itself (`observed [3727360, 3727360]`) before reaching a single Windows assertion. A read of either endpoint is legitimate; only a third value is a torn write.

`feature = "ci"` on the permission-edge test compiled it out of every run that exists: `ci` is passed only by the airlock replay (`nextest archive --test e2e --features ci`), `pixi run test` enables no features, and the new job is windows-latest while the test is `cfg(unix)`. Replaced with a probe for the premise the test asserts — a host that enforces directory permissions — verified in both directions here: as uid 0 it reports the missing premise and stops, as uid 65534 it asserts and passes.

check-repository check 10 also rejected the relock dispatch as a hand-edit of a generated file (98 committed lines against 93 rendered). It now renders from the template behind a `publisher_workflow` option, taken from the file name of the publisher path `init` was handed — the `ci_workflow` precedent, because a literal would leave a consumer that renamed `--workflow-path` dispatching a workflow that does not exist.

Evidence: `pixi run --frozen lint` green; `pixi run --frozen test` 606 passed / 1 skipped; `pixi run --frozen test-doc` green. On the PR: `ci (lint . test . coverage)` pass, `validate airlock plan` pass, `airlock linux-64` pass, `codecov/patch` pass, and `replace a running image (windows)` pass with `a_genuinely_running_image_is_replaced_and_its_corpse_is_reaped_by_the_next_update ... ok`. The windows-only arms were type-checked here too, by compiling the file with its `cfg(windows)` gates enabled.

AC#7 stays unchecked by necessity, unchanged from slice 3: it needs a live cut release and a reviewed regeneration cycle.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Task-47 stands at AC#1-#6 and #8-#9 met, with AC#7 the only criterion left. Slice 1 delivered the self-update core as the v0.5.0 feature: `pixi-sandbox self-update` with latest-by-default resolution, exact `--version X.Y.Z`, non-writing `--check`, and an explicit CI-managed `--dest`. The trust boundary is the enforcement order and is tested as one — resolve, classify the destination, then download; `SHA256SUMS` before the binary; a missing entry and a digest mismatch as distinct refusals; nothing downloaded ever executed. Ownership is judged on path provenance, so a conda/Pixi prefix, a `pixi global` trampoline, a managed launcher and a restored transport tool are each refused with a remedy that works, and there is no `--force`. Replacement stages beside the destination and swaps atomically, Unix by renaming over the running binary inode and Windows by renaming the image aside and sweeping it on the next run.

Slices 2 and 3 added the version stamps, `init --check`, and the generated upgrade lane with its explicit dispatch. Slice 4 closed AC#2 with the one proof no local run can produce: a windows-latest job replacing a genuinely mapped image, asserting the corpse survives the update whole, the running process is undisturbed, and the corpse is reaped once the image is released.

AC#7 is blocked on a live cut release and a reviewed regeneration cycle rather than on code, so the task remains In Progress and decision-4 remains proposed.
<!-- SECTION:FINAL_SUMMARY:END -->
