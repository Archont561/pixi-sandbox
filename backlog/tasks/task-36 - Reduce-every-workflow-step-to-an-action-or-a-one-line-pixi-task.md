---
id: TASK-36
title: Reduce every workflow step to an action or a one-line pixi task
status: To Do
assignee: []
created_date: '2026-10-01 19:10'
updated_date: '2026-10-01 18:20'
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
- [ ] #1 Every step in `release.yml`, `auto-release.yml` and `airlock.yml` is `uses: <pinned action>`, a single `pixi run <task>` line, or a one-line host-bootstrap command; no `run: |` block and no PowerShell remains in any workflow
- [ ] #2 Workflow-to-task plumbing travels as task arguments with local defaults (the convention that replaced the deleted `ci-pack`/`ci-doctor`/`ci-publish` twins: CI passes its own values on the same one-line `pixi run` a developer runs, so nothing reads an environment variable that can silently expand to an empty string); `env:` is reserved for runner-provided values the invoked tool reads natively or that cannot be an argument (`CARGO_BUILD_TARGET`, `GH_TOKEN`), inline `${{ }}` interpolation stays out of any multi-command string, and every new task is listed in `AGENTS.md` and `README.md` with the others
- [ ] #3 Logic that was shell is an `xtask` subcommand with tempdir-fixture tests (D10), not a one-line task wrapping the same `bash -c`: staging/stripping a release binary, SHA256SUMS generation plus its completeness check, the release commit/tag/push routine and its `.release-touched` guards, the airlock matrix emission, the airlock release-tag resolution, and the airlock branch fetch (git through `pixi-sandbox-git`, D9)
- [ ] #4 `GITHUB_OUTPUT` / `GITHUB_STEP_SUMMARY` writes happen inside the xtask commands that produce the value, and each command degrades to plain stdout when those variables are absent, so the identical invocation works locally
- [ ] #5 `xtask check-repository` gains a workflow-shape check: a `run:` whose body is more than one command fails, with a documented opt-out marker for a reviewed exception, and the check is fixture-tested with a passing and a failing workflow
- [ ] #6 The three workflows still do exactly what they did: a dry-run `auto-release`, a tag-triggered `release`, and an `airlock` matrix leg each produce the same artifacts, summaries and failure modes as before (verify by dispatch on a throwaway tag before closing)
- [ ] #7 `pixi run lint` (actionlint over committed and generated workflows, repo-consistency) and `pixi run test` are green, and the new tasks are reachable from a clean clone with one `pixi run`
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

**New pixi tasks** (plain one-liners, no xtask needed): `build-release-binary` (cargo build driven by `CARGO_BUILD_TARGET`), `lint-conda-platforms` (the `check-conda-platforms` invocation the workflow currently spells as a raw `cargo run`), `publish-conda` (the `pixi upload prefix …` line, `-e package`), `dispatch-docs`, `dispatch-release`, `airlock-install-released`, `airlock-pack` / `airlock-doctor` / `airlock-publish` / `airlock-restore` (the released-binary twins of `ci-pack`/`ci-doctor`/`ci-publish`, reading `SANDBOX_*`).

**Steps that simply disappear**: `release.yml` "Determine release tag" (`${{ inputs.version || github.ref_name }}` is the tag on a tag push — no step needed), airlock "Report the platform contract" and "Summarise" (the matrix leg name and the job log already carry it; keep a summary line only if an xtask writes it), and the two `airlock-gate.sh` tiers (task-35).

**Then the guard**: add `workflow_shape` to `repo_checks.rs` — parse each `.github/workflows/*.yml`, flag any `run:` block whose body holds more than one command line, allow `multiline-run-allowed` on the step, and fixture-test both verdicts. Finally update `AGENTS.md`, `README.md` and `docs/` so the rule is stated once and the task list is complete.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
<!-- SECTION:SUMMARY:END -->
