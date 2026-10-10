---
id: TASK-76
title: Make pixi-sandbox the single entrypoint in generated workflows
status: In Progress
assignee: []
created_date: '2026-10-06 20:11'
updated_date: '2026-10-10'
labels:
  - ci
  - github-actions
  - generated-workflow
  - refactor
  - cli
dependencies:
  - TASK-36
  - TASK-71
references:
  - crates/pixi-sandbox/src/generated/github_workflow.rs
  - crates/pixi-sandbox/src/generated/relock_workflow.rs
  - crates/pixi-sandbox/tests/fixtures/generated/publish-sandbox.yml
  - crates/xtask/src/repo_checks/workflow_shape.rs
  - .github/workflows/ci.yml
priority: high
type: chore
ordinal: 76000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The workflows that pixi-sandbox init writes into consumer repositories still carry their logic as embedded shell. The golden render at crates/pixi-sandbox/tests/fixtures/generated/publish-sandbox.yml is 420 lines, and 218 of its non-comment lines sit inside 9 multi-line `run:` blocks written in two dialects (10 `shell: bash`, 2 `shell: pwsh`), the largest a single 62-line block. None of that shell is type-checked, formatted, linted, or unit-tested, and the per-platform duplication is the same failure mode that killed the v0.3.6 release on macOS.

This repository already proves the shape we want. `.github/workflows/ci.yml` contains 6 `run:` lines and every one of them is a single `pixi run <task>`: the workflow decides when, the tool decides how. The pattern is proven inside the generated surface too, because the generated `relock.yml` renders 0 multi-line `run:` blocks and keeps its logic in `actions/github-script` steps.

Refactor crates/pixi-sandbox/src/generated/github_workflow.rs so pixi-sandbox itself is the entrypoint of the generated publisher, exactly as `pixi run <task>` is the entrypoint in ci.yml. Every step that carries logic becomes one `pixi-sandbox <verb>` invocation, and the absorbed behaviour moves into Rust where the existing test suite reviews it, both platforms share it, and it is versioned with the binary. The crisp end state is that check 9 no longer needs to exempt generated artifacts.

Sequenced after TASK-71 so the upgrade-delivery shell is not rewritten before its outstanding acceptance criterion has been proven on a real consumer run.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every logic-bearing step of the generated publisher is a single command: one `pixi-sandbox <verb>` invocation (`plan`, `fetch-release`, `pipeline`, `self-update`, `upgrade`), replacing all 9 multi-line `run:` blocks in the current render.
- [x] #2 The behaviour absorbed from those blocks lives in Rust behind named subcommands covered by the existing test suite instead of in YAML, including the pack-verify-publish wrapper with its per-step outcome recording (`pipeline`, `src/pipeline.rs`, `tests/pipeline.rs`), the pinned-download plus self-update plus regenerate plus check-owned-files upgrade sequence (`fetch-release`, `upgrade`, `tests/self_update_fetch.rs`, `tests/upgrade.rs`), and the upgrade-patch delivery with its step-summary and artifact fallback (`upgrade`'s deliver phase, via `GitProtocol::commit_files`/`push_branch` and a `PullRequestSource` seam).
- [x] #3 The generated publisher no longer forks on runner OS to express logic: each bash and pwsh twin collapses into one cross-platform step, and `shell: pwsh` is gone entirely (`the_render_carries_no_multiline_run_blocks_and_no_pwsh`).
- [x] #4 The steps that download the pixi-sandbox binary are the only exception, because they cannot use that binary as their own entrypoint; each is one checksum-verified command for every platform (`pixi-sandbox fetch-release --dest … --version "$PIXI_SANDBOX_VERSION"`, verified against the release's SHA256SUMS inside the verb) and is annotated in the render as the bootstrap exception.
- [x] #5 Every subcommand and flag the generator emits exists in the binary that the same init run pins as PIXI_SANDBOX_VERSION, enforced by `tests/generated_cli.rs`: the render is produced with the running binary's version, the workflow pins exactly that version, every emitted invocation is checked against the pinned binary's own `--help`, and the emitted verb set is asserted exactly.
- [x] #6 Check 9 in crates/xtask/src/repo_checks/workflow_shape.rs stops exempting generated artifacts through GENERATED_MARKER (cfb7c958), so the one-command-per-step rule applies to rendered consumer workflows; the surviving exception mechanism is the named `multiline-run-allowed` marker with its recorded reason, which this repository's own deliberately source-built publisher uses on its four multi-line blocks.
- [x] #7 Golden fixtures under crates/pixi-sandbox/tests/fixtures/generated are regenerated through the generator (`zz_regenerate_fixture` scratch test, since deleted; the golden test holds render == fixture), init output stays byte-identical to the committed renders (`init_writes_the_exact_render_the_generator_produces`), `xtask lint-generated-workflow` reports actionlint clean over the full combination table, and non-comment embedded shell in publish-sandbox.yml falls from 218 lines to 35 single-command lines (`non_comment_shell_lines_fall_to_the_single_commands_alone`).
- [ ] #8 A real consumer repository runs the refactored publisher end to end on a released binary and reaches the same transport result as the pre-refactor lane, so the change is proven outside fixtures.
- [x] #9 scripts/restore.sh is untouched (`git diff 0a036ba2..HEAD -- scripts/restore.sh` is empty) and stays shell-only and version-agnostic per TASK-47 AC #9, because the airlock bootstrap has to work before any binary exists.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
<!-- DOD:END -->

<!-- SECTION:NOTES:BEGIN -->
## Implementation Notes

2026-10-10, six slices, six conventional commits on `arena/00dbb795-pixi-sandbox`
(b37eab27 fetch-release, 303876bd git ops, 911c0c7b pipeline, 59227002 upgrade,
7aae83ec the render, cfb7c958 check 9). Suite: 873/1 at start → 924/1 after (per-crate
35 git / 209 core / 524 pixi-sandbox / 156 xtask); `pixi run --frozen lint` clean including
`check-repository` and `lint-generated-workflow` (actionlint + shellcheck).

The generated publisher is now pixi-sandbox's entrypoint end to end. The render (198 lines,
was 421) carries no literal `run: |` block and no `shell: pwsh`; the only multi-line shell
left is single commands (the package install, the fetch-release bootstrap, and the
pipeline/self-update/upgrade invocations). The pack-verify-publish orchestration, its
per-phase outcome recording, the failure step summary and the phase exit codes moved into
`pixi_sandbox::pipeline` behind a `PhaseRunner` seam; the upgrade lane's check/regenerate/
commit/deliver flow moved into `pixi_sandbox::upgrade` behind runner + `GitProtocol` +
`PullRequestSource` seams; the pinned download is `self_update::fetch_release` (always
downloads — no up-to-date shortcut, because the destination is not the running binary).
Git access for the upgrade commit went through two additive `GitProtocol` operations
(`commit_files`, `push_branch`) with ShellGit and FakeGit implementations, so invariant 8
holds and the flow is tested against `FakeGit`. The pull request is created through the
GitHub REST API behind `PullRequestSource` (real HTTP isolated in `src/upgrade/github.rs`,
coverage-excluded like `release/github.rs`); no new crate dependencies, no lockfile change
(base64 for the git auth header is hand-rolled and pinned by RFC 4648 vectors).

Generator/CLI compatibility is enforced locally by `tests/generated_cli.rs`: the render is
produced with the running binary's version, the workflow pins exactly that version, and
every emitted invocation is validated against the pinned binary's own `--help`. The
structural argument that a newly built local CLI is not treated as proof for an older
released binary: the template and the binary release together (one crate, one version),
and the pin only advances when `init` runs a newer release.

**AC#8 is NOT met and is carried explicitly.** It requires a real consumer repository
running the refactored publisher end to end on a *released* binary and reaching the same
transport result as the pre-refactor lane. No release of these verbs exists yet, and no
consumer has run the new render; fixtures prove the render's shape and the verbs' policy,
not a live consumer run. The missing proof, in order: (1) a release shipping
`fetch-release`/`pipeline`/`upgrade` (the workflow pins `PIXI_SANDBOX_VERSION`, so the new
verbs exist only from that release on); (2) a consumer (Castellan is the standing proof
repository) running `init` with it, then its `publish sandbox` run packing/verifying/
publishing through `pipeline`; (3) the upgrade lane's delivery, which additionally needs
task-71 AC#5 / task-73's credentialed token. Until then this task stays In Progress, and
no push, PR, release or consumer-repository modification has been made (the owner's
sanction covers local implementation only).

**2026-10-10: the release AC#8 needs is confirmed necessary, sufficient, and blocked on
exactly one click.** `b9d5d0a2` (the single-entrypoint refactor) is a descendant of the
v0.6.0 tag `8298edce` — `git merge-base --is-ancestor v0.6.0 HEAD` is true while the
refactor itself merged later, in PR #129 — so v0.6.0 provably predates the refactored
lane and any AC#8 proof needs a release after PR #131. Cutting one is *sufficient to start*
the proof: `release.yml` already dispatches `consumer proof` itself once the GitHub Release
exists (the v0.6.0 chain, runs 5–7: release → consumer proof → docs), so no hand-off step
is missing. The push permission is also not the obstacle it looked like:
`RELEASE_PUSH_TOKEN` is set and carries the Workflows permission, proved by observation
rather than inference when the v0.6.0 release commit (`8298edc`, which modifies
`.github/workflows/relock.yml`) succeeded. The remaining blocker is dispatch:
`gh workflow run auto-release.yml` from an agent sandbox returns `HTTP 403: Resource not
accessible by integration`, and an agent session is confined to its own branch, so the
release commit could not be pushed to `main` from here even if the token allowed it. One
maintainer dispatch of `auto-release.yml` with its default `bump=auto` cuts the release;
the transport repack and the consumer proof then follow without further action.
<!-- SECTION:NOTES:END -->
