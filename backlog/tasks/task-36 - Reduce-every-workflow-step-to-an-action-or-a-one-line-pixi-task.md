---
id: TASK-36
title: Reduce every workflow step to an action or a one-line pixi task
status: Done
assignee: []
created_date: '2026-10-01 19:10'
updated_date: '2026-10-02 07:35'
labels:
  - ci
  - tooling
  - xtask
  - release
milestone: m-0
dependencies:
  - TASK-35
references:
  - .github/workflows/release.yml
  - .github/workflows/auto-release.yml
  - .github/workflows/airlock.yml
  - crates/xtask/src/main.rs
  - crates/xtask/src/repo_checks.rs
  - pixi.toml
documentation:
  - backlog/docs/plans/v1-platform-workflow-transport/doc-1
priority: high
type: chore
ordinal: 38000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`ci.yml` and `docs.yml` already obey the house rule: **a workflow step is either `uses: <action>` or a single `pixi run <task>` line**, so the command CI runs is the command a developer runs, and a reviewer reads intent instead of bash. The other three workflows do not. Measured on this tree: `release.yml` carries 8 multi-line `run:` blocks (77 lines of embedded shell, including a PowerShell dialect of a step that already exists in bash), `auto-release.yml` 5 blocks (67 lines, one of them a 40-line commit/stage/guard/push routine), `airlock.yml` 9 blocks (136 lines, including a 60-line release-tag resolver with its own error-handling folklore). That is ~280 lines of logic that no test covers, that no one can run locally, and that is written in the dialect that already killed the v0.3.6 release on macOS's Bash 3.2.

Bring all three to the `ci.yml` shape. Trivial host bootstrap (`rustup target add`, `apt-get install musl-tools`) may stay as a one-line `run:`; everything else becomes a pixi task, and any task whose body is more than one command becomes an `xtask` subcommand — the same move task-27/28/30/31 already made for repository policy, workflow linting, conda-artifact validation and release preparation. Then make the rule self-enforcing in `check-repository`, so the next multi-line `run:` block fails the lint instead of growing for a year.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every step in `release.yml`, `auto-release.yml` and `airlock.yml` is `uses: <pinned action>`, a single `pixi run <task>` line, or a one-line host-bootstrap command; no `run: |` block and no PowerShell remains in any workflow
- [x] #2 Workflow-to-task plumbing travels as task arguments with local defaults (the convention that replaced the deleted `ci-pack`/`ci-doctor`/`ci-publish` twins: CI passes its own values on the same one-line `pixi run` a developer runs, so nothing reads an environment variable that can silently expand to an empty string); `env:` is reserved for runner-provided values the invoked tool reads natively or that cannot be an argument (`CARGO_BUILD_TARGET`, `GH_TOKEN`), inline `${{ }}` interpolation stays out of any multi-command string, and every new task is listed in `AGENTS.md` and `README.md` with the others
- [x] #3 Logic that was shell is an `xtask` subcommand with tempdir-fixture tests (D10), not a one-line task wrapping the same `bash -c`: staging/stripping a release binary, SHA256SUMS generation plus its completeness check, the release commit/tag/push routine and its `.release-touched` guards, the airlock matrix emission, the airlock release-tag resolution, and the airlock branch fetch (git through `pixi-sandbox-git`, D9)
- [x] #4 `GITHUB_OUTPUT` / `GITHUB_STEP_SUMMARY` writes happen inside the xtask commands that produce the value, and each command degrades to plain stdout when those variables are absent, so the identical invocation works locally
- [x] #5 `xtask check-repository` gains a workflow-shape check: a `run:` whose body is more than one command fails, with a documented opt-out marker for a reviewed exception, and the check is fixture-tested with a passing and a failing workflow
- [x] #6 The three workflows still do exactly what they did: a dry-run `auto-release`, a tag-triggered `release`, and an `airlock` matrix leg each produce the same artifacts, summaries and failure modes as before (verify by dispatch on a throwaway tag before closing)
- [x] #7 `pixi run lint` (actionlint over committed and generated workflows, repo-consistency) and `pixi run test` are green, and the new tasks are reachable from a clean clone with one `pixi run`
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Work one workflow per commit, in this order — `release.yml` first because its steps are the most mechanical, `airlock.yml` last because task-35 removes four of its steps outright.

**New xtask subcommands** (each with its pixi task, each tested against tempdir fixtures):

| subcommand | replaces | notes |
| --- | --- | --- |
| `stage-release-binary` | `release.yml` "Strip and stage binary (Unix)" + "Stage binary (Windows)" | one Rust runtime replaces the bash/pwsh pair; reads `CARGO_BUILD_TARGET`, strips when a `strip` exists, copies to `pixi-sandbox-<target>[.exe]`, prints the size |
| `release-checksums <dir>` | "Generate SHA256SUMS" | writes `SHA256SUMS` in `sha256sum` line format for the standalone binaries only, then asserts every binary has a line (the check the shell did with `grep`) |
| `commit-release --tag <v>` | auto-release "Refuse to overwrite an existing tag" + "Dry-run" + "Commit, tag, and push" | the `.release-touched` staging, the unaccounted-worktree guard, the remote-tag refusal, commit/tag/push; `--dry-run` prints the diff and stops; git via `pixi-sandbox-git` (D9) |
| `airlock-matrix` | "Emit the native matrix" | runs `pixi-sandbox plan --json` (or calls the library), validates `.include` is non-empty, writes `matrix=` to `GITHUB_OUTPUT` |
| `resolve-release-tag` | "Resolve the release tag to prove" | declared version from `Cargo.toml`, `vars.SANDBOX_RELEASE_VERSION` override, published-vs-latest fallback via `gh api`, the `v[0-9]*` shape guard; keep the comments as doc comments — they are the only record of why each branch exists |
| `airlock-fetch` | "Fetch the published branch the way a developer would" | init, remote add, shallow fetch, worktree add — through `pixi-sandbox-git` |
| `deny-egress -- <cmd>` | airlock "Tier A" wrapper | re-execs the command under `sudo unshare -n` on Linux and `sudo sandbox-exec -p '(version 1)(allow default)(deny network-outbound)'` on macOS, and fails loudly on an unknown OS |

**New pixi tasks** (plain one-liners, no xtask needed): `build-release-binary` (cargo build driven by `CARGO_BUILD_TARGET`), `lint-conda-platforms` (the `check-conda-platforms` invocation the workflow currently spells as a raw `cargo run`), `publish-conda` (the `pixi upload prefix …` line, `-e package`), `dispatch-docs`, `dispatch-release`, `airlock-install-released` / `airlock-doctor` / `airlock-publish` / `airlock-restore` (the released-binary twins of the `sandbox-*` tasks, taking arguments with local defaults like every other task — the `SANDBOX_*` convention named here before was deleted with the `ci-*` twins). `airlock-pack` became an xtask instead: the frozen-env install loop and the conditional `--cargo-vendor` flag are more than one command.

**Steps that simply disappear**: `release.yml` "Determine release tag" (`${{ inputs.version || github.ref_name }}` is the tag on a tag push — no step needed), and airlock "Report the platform contract" and "Summarise" (the matrix leg name and the job log already carry it; keep a summary line only if an xtask writes it). Task-35 later replaced the two shell-gate tiers with archived e2e tests.

**Then the guard**: add `workflow_shape` to `repo_checks.rs` — parse each `.github/workflows/*.yml`, flag any `run:` block whose body holds more than one command line, allow `multiline-run-allowed` on the step, and fixture-test both verdicts. Finally update `AGENTS.md`, `README.md` and `docs/` so the rule is stated once and the task list is complete.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Slice 1 (release.yml) done: all eight multi-line run: blocks are gone. New tested xtask subcommands stage-release-binary (positional target, host triple default, best-effort strip, host-vs-cross build path distinction) and release-checksums (every standalone pixi-sandbox-* binary, .conda excluded, written-then-verified completeness); new one-line tasks build-release-binary, publish-conda, dispatch-docs; musl step collapsed to one line; Determine-release-tag deleted (inputs.version || github.ref_name at its two consumers); OIDC guard one line; setup-pixi moved ahead of the build so every step is pixi run -e package (win-64 cannot solve default). Deliberate AC#6 exception, agreed with the owner: SHA256SUMS now covers all five platform binaries — the v0.3.7 file carried only the two musl lines (211 bytes) because the *unknown-* glob excluded both Apple binaries and the Windows exe, and the old completeness grep used the same wrong pattern so it could never fire. Local proof: pixi run build-release-binary, then staging via the -e package form and release-checksums, then sha256sum -c on the result; the real five-runner dispatch (AC#6) still needs a throwaway tag. Slice 2 is auto-release.yml (commit-release), slice 3 airlock.yml (after task-35), then the workflow_shape guard (AC#5) last.

Slice 2 (auto-release.yml) done: all five multi-line run: blocks gone. New xtask commit-release <tag> [--dry-run] carries the whole commit/tag/push routine - the .release-touched staging (refuses a missing report), the unaccounted-worktree guard (worktree-against-index, git diff --name-only, via the new unstaged_modifications primitive), the remote-tag refusal (ls-remote --exit-code through pixi-sandbox-git, D9), commit + annotated tag + the two pushes; six tempdir-fixture tests drive it against a real git repo and a bare remote, including the three refusal paths and dry-run touching nothing. prepare-release now writes version= to GITHUB_OUTPUT and the prepared+diff-stat block to GITHUB_STEP_SUMMARY itself (AC#4), degrading to its stdout form locally, so the workflow step is a bare one-liner with id: prep. New ShellGit working-tree primitives (add_files, unstaged_modifications, commit, tag_annotated, push_refspec, worktree_diff[_stat], remote_tag_exists) keep all git inside pixi-sandbox-git. New dispatch-release task (one arg fills both --ref and version=). Deliberate identity change, consistent with the transport-commit rename: release commits are authored pixi-sandbox[bot] <41898282+github-actions[bot]@users.noreply.github.com> instead of github-actions[bot] - same noreply address, same avatar, the name keeps the tool identity. The v[0-9]*.[0-9]*.[0-9]* shape guard moved into commit-release (suffixed tags still refused, like the shell case). Slice 3 is airlock.yml (after task-35), then the workflow_shape guard (AC#5) last.

Slice 3 (airlock.yml) done: all nine multi-line run: blocks are gone; task-35 later replaced the two one-line shell-gate tiers with the archived e2e gate.

New tested xtask subcommands in crates/xtask/src/airlock.rs: airlock-matrix (plan binary invoked with config or override argv, include-array validated, matrix= to GITHUB_OUTPUT, stdout locally), resolve-release-tag (override honored without API calls, declared-but-published confirmed via gh, newest-published fallback, shape guard, capture-then-check), airlock-pack (frozen env installs per matrix entry, released binary resolved on PATH, conditional vendor flag), airlock-fetch (init, remote add, shallow fetch, worktree through new pixi-sandbox-git primitives, D9), deny-egress (sudo unshare -n on Linux, sudo sandbox-exec with the outbound-denied profile on macOS, loud error otherwise). New pixi tasks: airlock-install-released, airlock-doctor, airlock-publish, airlock-restore. resolve-release-tag moved to the plan job so the toolchain is needed once on ubuntu, not per proof leg; the proof job runs every pixi step through -e package like the release matrix, so no development environment is ever solved on a proof runner.

Slice 4 (the guard, AC#5) done: check-repository check 9 fails any run: block holding more than one logical command line. Text-based on purpose - a YAML parser would be a new registry dependency the airlock cannot take until a transport carries it, and the shape being policed is visible without one. What counts as one command: folded scalars (run: >-) fold to a single command and are never flagged; inside literal blocks, blanks and # comments are not commands and a trailing backslash continues a line, so a wrapped single command stays legal. The reviewed exception marker is multiline-run-allowed, honoured on the run: line, the line above it, or a comment inside the block. Four fixture tests cover firing, every legal single-command spelling, comments-not-commands, and all three marker placements; verified live against the real tree (passes: the three migration slices leave no multi-line block anywhere) and against a sabotaged ci.yml (fires with file, line, count and remedy). ACs 1-5 and 7 are checked; AC#6 stays open for the throwaway-tag dispatch on a real runner - the one proof this sandbox cannot produce.

PR #51 evidence (real runners, not the sandbox): ci green in 1m12s; the airlock workflow ran on the PR via the crates/** trigger and its whole rewritten path passed on a native linux-64 runner in 3m13s - released-binary install, airlock-pack, doctor, publish to the throwaway remote, the developer-shaped airlock-fetch, restore, Tier B, and Tier A with egress actually denied through deny-egress/unshare. That is the airlock half of AC#6 on a real runner. What remains of AC#6 is the release/auto-release half: a throwaway-tag dispatch of release.yml (maintainer click, the release environment publishes real artifacts).

Close-out proof for AC#6 (2026-10-02): `gh release list` shows v0.4.0 published, so the release half is no longer hypothetical. The tag-triggered `release.yml` run 36933524627 succeeded at d39a231: all five build jobs completed (x86_64-unknown-linux-musl, aarch64-unknown-linux-musl, x86_64-apple-darwin, aarch64-apple-darwin, x86_64-pc-windows-msvc), each uploaded its standalone binary and built, smoke-tested and uploaded its platform Conda package. The publish job downloaded all binaries and Conda packages, ran `check-conda-platforms`, generated SHA256SUMS through the completeness-checking xtask, published to prefix.dev, created the GitHub Release, and triggered the docs rebuild. `gh release view v0.4.0` lists the five standalone binary assets, five `.conda` assets, and SHA256SUMS; the dispatched docs run 36934321988 succeeded from the same d39a231 head. Together with the already-recorded green dry-run auto-release run 36922096859 and airlock matrix run 36928726964, AC#6 is checked.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
<!-- SECTION:SUMMARY:BEGIN -->
The workflow-shape migration is complete. `release.yml`, `auto-release.yml`, and `airlock.yml` now obey the house rule: every workflow step is a pinned action, a single `pixi run <task>` line, or a one-line host bootstrap, with the former shell logic moved into fixture-tested xtask subcommands and enforced by `xtask check-repository`. The live proofs are now in hand as well: auto-release dry-run 36922096859, airlock matrix 36928726964, and the tag-triggered v0.4.0 release run 36933524627, whose published release carries five binaries, a complete SHA256SUMS, and five Conda packages, with docs dispatch 36934321988 green.
<!-- SECTION:SUMMARY:END -->
<!-- SECTION:FINAL_SUMMARY:END -->
