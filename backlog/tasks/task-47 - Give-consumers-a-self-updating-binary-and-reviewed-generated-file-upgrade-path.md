---
id: TASK-47
title: Give consumers a self-updating binary and reviewed generated-file upgrade path
status: In Progress
assignee: []
created_date: '2026-10-02 20:05'
updated_date: '2026-10-03 23:55'
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
- [ ] #2 Self-update stages beside the destination and replaces atomically on Unix and Windows without leaving a partial executable; `--check` reports the current and resolved versions without writing; package-managed binaries/global-install trampolines are refused with a remedy, while an explicit CI-managed standalone path is supported and covered by tests
- [ ] #3 Every file `init` writes carries a machine-readable version stamp beside the ownership marker (a `pixi-sandbox-version: X.Y.Z` line, within the first three lines the marker check already reads) for the publisher workflow, the relock workflow, and the launcher; fixture tests cover presence and parsing on every render
- [ ] #4 `pixi-sandbox init --check` renders fresh, writes nothing, and exits non-zero naming each drifted file with its remedy (`pixi-sandbox init`) and each foreign-owned file separately (`--force`); a clean tree exits 0 — fixture-tested in both directions (D10)
- [ ] #5 `init` and `--check` never rewrite an existing config; a config whose `schema` is older than the CLI's is reported as a finding naming the explicit migration path (no `config migrate` command is built while config schema is 1 — it exists only once a schema bump gives it something to do), and config readers keep accepting older schemas for a deprecation window, mirroring the manifest reader policy
- [ ] #6 The generated `publish-sandbox.yml` gains a scheduled and manually dispatchable upgrade job that bootstraps its currently pinned standalone binary with checksum verification, runs `self-update` (latest by default, exact version for manual dispatch), then uses that updated binary for `init --check` and regeneration; drift opens a pull request and never pushes to main or touches config, and the workflow remains actionlint-clean
- [ ] #7 The regenerated pull request contains a new exact `PIXI_SANDBOX_VERSION` and the templates emitted by that same binary; a human merge triggers the normal publisher, which uses the exact matching released binary for plan, pack, doctor, publish, and `--self-bin`, and the resulting manifest names that version — no production step resolves `latest`
- [ ] #8 Because a `github.token` push starts no `on: push` workflows (task-44's lesson), any automated-merge path ends in an explicit `gh workflow run publish-sandbox.yml` dispatch; the workflow and docs distinguish this from a human merge and tests hold the dispatch behavior
- [ ] #9 The repository owner workflow remains source-built and has no released-CLI/self-update dependency; the airlock side is unchanged (`restore.sh`/`restore.ps1` stay version-agnostic and doctor reads older manifests), and docs cover self-update, exact pinning, the reviewed upgrade PR, and rollback by dispatching an exact older version
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
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
Slice 1 of task-47 delivered the self-update core as the v0.5.0 feature: `pixi-sandbox
self-update` with latest-by-default resolution, exact `--version X.Y.Z`, non-writing `--check`,
and an explicit CI-managed `--dest`. The trust boundary is the enforcement order and is tested
as one — resolve, classify the destination, then download; `SHA256SUMS` before the binary; a
missing entry and a digest mismatch as distinct refusals; nothing downloaded ever executed.
Ownership is judged on path provenance, so a conda/Pixi prefix, a `pixi global` trampoline, a
managed launcher and a restored transport tool are each refused with a remedy that works, and
there is no `--force`. Replacement stages beside the destination and swaps atomically, Unix by
renaming over the running binary's inode and Windows by renaming the image aside and sweeping
it on the next run.

AC#1 is met and checked. AC#2's behaviour is implemented and covered but stays unchecked
pending a Windows runner. AC#3-#9 were out of scope by instruction and are untouched, so the
task remains In Progress and decision-4 remains proposed.
<!-- SECTION:SUMMARY:END -->
