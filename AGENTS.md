# AGENTS.md

Instructions for coding agents working **in this repository**. For the agent-facing guide that
ships *inside a sandbox branch*, see the generated `AGENTS.md` under `.pixi-sandbox/` in that
branch — different audience, different file.

**Start of session?** Follow `.agents/skills/session/SKILL.md` — it sequences the whole loop:
the environment bootstrap on an airlocked machine, the backlog survey, the session proposal,
and, after the pull request merges, the session report that closes the task files and writes
the next session's opening prompt. Its four templates are in
`.agents/skills/session/standup-template.md`.

**Adding a feature or fixing a bug?** Two skills are mandatory, in order, not merged into one
pass:

1. `.agents/skills/tdd/SKILL.md` for the implementation loop — agree the seams up front, red
   before green, one slice at a time. New behavior in `pixi-sandbox-core`/`pixi-sandbox-git`
   lands as a library test; a `commands/*` change is black-box from `tests/` (see the repo map
   below for which modules are promoted through `lib.rs` for that reason).
2. `.agents/skills/refactor/SKILL.md` once the tests are green — cleanup is its own step, never
   mixed into the red→green cycle (the TDD skill's own rule). Keep it to the task at hand: fix
   the code you just touched, don't open an unrelated refactor mid-task. For anything larger —
   a repo-wide pass, a smell that spans files the current task doesn't touch — write it up as
   its own backlog task or plan doc (see `backlog/docs/plans/`) instead of folding it in.

Either way, the loop still ends at the house rules below: one focused conventional commit,
`pixi run --frozen test` not dropping below its last known count, and a task file closed in its
own format.

## What this project is

`pixi-sandbox` packs pixi environments into a git orphan branch and restores them on machines
with no network. The design is settled and measured: **read `.knowledge/design.md` before
proposing a different one**, and `.knowledge/decisions.md` for the short form. The recorded
decisions are load-bearing; if you think one is wrong, bring a measurement, not an opinion.

## Repo map

| path | notes |
| --- | --- |
| `crates/pixi-sandbox-core/src/manifest.rs` | **wire format** — `manifest.json` shape and `SCHEMA_VERSION`. Changing it needs a schema bump + fixture update in `tests/manifest.rs` |
| `crates/pixi-sandbox-core/src/shard.rs` | the only two file operations: `record_file` (split if oversized) and `materialise`/`join_parts` (verify, then write) |
| `crates/pixi-sandbox-core/src/tools_lock.rs` | compiles the canonical helper-tool pins into the binary; `--tools-lock` is an explicit override |
| `crates/pixi-sandbox-core/src/verify.rs` | collects *all* failures instead of stopping at the first — an airlock operator wants the full list; `verify_restored` is the restored-tree half (D13) |
| `crates/pixi-sandbox-core/src/files_manifest.rs` | the per-file oracle (D13): `scan_prefix` records the unpacked prefix at pack time, canonicalising either side's path spellings to a sentinel so relocation cannot defeat the digests |
| `crates/pixi-sandbox/src/commands/*` | `init`, `pack`, `publish`, `restore`, `unpack`, `doctor`, `plan`, `tools list`, and `self-update` — pure Rust, no Python. Thin wiring: these resolve the ambient inputs (HOME, host triple, the running executable) and call into the library, so the logic stays testable from `tests/` |
| `crates/pixi-sandbox/src/lib.rs` | what `tests/` and `xtask` can reach: `generated`, `release`, `self_update`, `user_tools`. A module that needs tests is promoted here — the binary target's own modules (`cli.rs`, `commands/`) are only coverable black-box |
| `crates/pixi-sandbox/src/release.rs` | the `ReleaseSource` trait every network-touching command injects (D9's shape, applied to HTTP): `GitHubReleaseSource` in production, an in-memory fake in tests. Shared by `tools update` and `self-update` so one fake serves both |
| `crates/pixi-sandbox/src/self_update/*` | the updater's trust boundary (decision-4), in enforcement order: `resolver` (latest vs exact), `assets` (the canonical host→asset map, mirroring `xtask/src/release_assets.rs`), `checksums` (`SHA256SUMS`), `ownership` (the refusal ladder — a Pixi-managed binary is never overwritten), `replace` (stage beside the destination, then swap; see task-37 AC#3 for why Unix renames over a running binary) |
| `crates/pixi-sandbox-git` | **all** git access: `GitProtocol` + `ShellGit` (real git, with a swappable `Runner`) + `FakeGit` (in-memory mock). Never run `git` from anywhere else |
| `.github/workflows/relock.yml` | **generated, not written**: the committed render of `generated/relock_workflow.rs`, the lockfile bot `pixi-sandbox init` also hands consumers. `check-repository` (check 10) fails when it drifts from a fresh render, so it is edited through the template and `pixi run --frozen xtask render-relock`, never by hand. Unlike the publisher template it is house-shaped — one-line steps throughout — which is what makes committing it possible |
| `crates/pixi-sandbox/tests/fixtures/` | the fixture project and the synthetic transport. **Tests must not point at this repository** — see `.knowledge/decisions.md` D10 |
| `crates/pixi-sandbox-core/assets/tools.lock.json` | canonical embedded helper-tool pins; edit as reviewed data and keep the embedded-lock tests green |
| `.pixi-sandbox.toml` | explicit project publish bundles / native runners consumed by `pixi-sandbox plan` and the reusable release workflow |
| `feature.utils.actionlint` + `feature.utils.shellcheck` | runs `actionlint` from conda directly when validating workflows in the `default` environment; ShellCheck sits beside it because actionlint silently skips the Bash inside `run:` blocks when no `shellcheck` executable is on PATH |
| `crates/xtask` | typed repository automation invoked by Pixi tasks — every repo-targeting job that used to be a shell script: `check-repository` (the D10-exempt repo lints; opt out with `stale-ref-allowed` on or above the line), `prepare-release` + `check-release-refs` + `version`, `smoke-conda-package` (the `package-smoke` task on every `release.yml` build leg), `check-conda-platforms` (all five platforms contributed exactly one package; reads `dist/conda/conda-<platform>/`, which is why those artifacts are downloaded *unmerged*), and `lint-generated-workflow`, plus `stage-release-binary` and `release-checksums` (the release matrix's asset staging and SHA256SUMS — the `*unknown-*` glob this replaced had silently checksummed only the two musl binaries of five), and `commit-release` (auto-release's commit/tag/push half: stage exactly what `.release-touched` reports, refuse unaccounted modifications, refuse an existing remote tag, then commit, tag and push — git through `pixi-sandbox-git`, D9). The airlock workflow's machinery lives there too: `airlock-matrix` (plan JSON validated, `matrix=` written to `GITHUB_OUTPUT`), `resolve-release-tag` (override → declared-but-published → newest-published, each branch an incident), `airlock-pack` (frozen env installs + the released-binary pack), `airlock-fetch` (the developer-shaped fetch, D9), and `deny-egress` (`sudo unshare -n` / `sudo sandbox-exec`, loud on unknown OS). One Rust runtime on every runner — the v0.3.6 release died on macOS's Bash 3.2 over a single bashism — and every policy is tested against tempdir fixtures, never this checkout |
| `scripts/restore.sh` | one-liner offline reconstruction from orphan branch; derives the branch from `.pixi-sandbox.toml` for the host platform. Also selects the task-33 user-tool policy explicitly (`PIXI_SANDBOX_USER_TOOLS`, exported before the restore): a verified restore registers `pixi` + `pixi-sandbox` launchers in the user's home by default, `skip` opts out for CI. The policy travels as an environment variable, never a flag — the binary this script drives comes from the packed branch and may predate `--user-tools` (the airlock workflow proved that skew), while an unknown variable is simply ignored. It never sources `.pixi/sandbox-env.sh`; restored/project commands go through pixi (`pixi run ...`) or the manifest-owned pixi path when registration is skipped. This is the only shell left under `scripts/`, and `check-repository` holds it to the Bash 3.2 surface |
| `package.json` + `bun.lock` | the root bun workspace — `docs` is a member, so `docs-install` runs at the root and package scripts target it with `bun --filter=pixi-sandbox-docs` — plus the repo-wide `backlog.md` / `skills` / `@biomejs/biome` devDependencies (`pixi run --frozen bunx backlog`, `pixi run --frozen bunx skills`, `lint-docs`) |

## Invariants (do not break these)

1. **Verify before write.** No file reaches the user's working tree before its sha256 matches the
   manifest. `join_parts` removes a partially written file on mismatch.
2. **Never write into the fetched branch checkout.** Treat it as read-only; stage elsewhere.
3. **Never use `/tmp` as a work dir.** `pixi-unpack` stages into `$TMPDIR`; a small tmpfs fails
   mid-restore. Default to `<project>/.pixi/.restore-work` and redirect `TMPDIR` for children.
4. **Shards are whole files.** Splitting is a size workaround, not a chunking strategy — chunking
   would destroy git dedup (measured: +0.07 MiB vs +3.96 MiB per crate bump).
5. **Nothing is downloaded at restore time.** The bundle is self-contained; CI fetches tools, the
   airlock never does.
6. **Do not commit generated weight**: `.pixi/`, `target/`, `vendor/`, `node_modules/`, `.pixi-sandbox/`, `docs/dist/`.
7. **A dynamically linked tool is a bug**, not a warning to ignore (`verify.rs` flags it).
8. **Git only through `GitProtocol`** — never a bare `Command::new("git")` in production code
   outside `pixi-sandbox-git` (D9). Tests use `FakeGit` to exercise the consumers of the
   protocol, while `#[cfg(test)]` fixture builders (`tests/support`, commit-release,
   airlock, prepare-release) may run real `git` to construct the world under test — an
   oracle must not share code with the thing it judges; `--dry-run` uses a runner, not a
   second code path.
9. **Tests target the fixtures, never this repository** (D10): `tests/fixtures/demo-project`
   for project-level work, `tests/fixtures/transport` for payload-level work. Two tests enforce it.
10. **Anything you did not implement goes in `CONTEXT.md` § Session scratchpad** — suggestions,
    drafts, alternatives considered, "we should probably…", measurements worth keeping. Under a
    dated heading, newest last. Never into `AGENTS.md`, `README.md`, `.knowledge/`, a
    `backlog/docs/` spec or a code comment: those state what *is*, and a proposal mixed into
    them reads as a decision nobody made. A scratchpad entry graduates only by becoming a
    backlog task, a decision in `.knowledge/decisions.md`, or a deletion.
11. **Pixi is the sole environment entrypoint.** Do not source `.pixi/sandbox-env.sh` (restore no
    longer generates it) and do not run bare `cargo`, `rustc`, `bun`, `turbo`, `taplo`, `convco`,
    etc. Repository verbs go through `pixi run --frozen <task>`; `test`, `lint`, `fmt`, `coverage`
    and `docs` enter the Bun/Turbo graph from there. Focused Rust work uses
    `pixi run --frozen bunx turbo run test --filter=@repo/<package>` (same graph) or
    `pixi run --frozen -- cargo <cmd> -p <crate>` for a Cargo operation that is not a repository
    gate; repository automation stays `pixi run --frozen xtask <subcommand>`.

## Commands

**Workflow step rule.** A step under `.github/workflows/` is one of three things: `uses:` a
pinned action, a single `pixi run <task>` line, or a one-line host bootstrap that pixi cannot
provide (`rustup target add …`). Nothing else. A workflow command that needs more than one
line is a task in `pixi.toml`; a task whose body needs more than one command is an `xtask`
subcommand with tempdir-fixture tests — one Rust runtime instead of five shell dialects, and a
reviewer reads intent instead of bash. Paths and flags reach the task as arguments with local defaults, so CI passes its own
values on the same one-line `pixi run` a developer runs (`env:` is reserved for runner-provided
values the invoked tool reads natively — `GH_TOKEN`, `CARGO_BUILD_TARGET`), and an xtask that
produces a value writes it to `GITHUB_OUTPUT`/`GITHUB_STEP_SUMMARY` itself, falling back to
stdout when those are unset so the same invocation works locally.

`ci.yml`, `docs.yml`, `release.yml`, `auto-release.yml` and `airlock.yml` are the reference
shape — no multi-line `run:` block remains in any workflow, and `xtask check-repository`
(check 9) fails any new one: a step must be `uses:`, a single `pixi run <task>` line, or a
one-line host bootstrap. The restored-project airlock gate is a Cargo nextest archive of the
e2e suite's `ci` feature, replayed in both tiers by one-line pixi tasks; `scripts/restore.sh`
is the only sanctioned shell bootstrap. A reviewed exception (a generated consumer artifact,
say) carries `multiline-run-allowed` on the step.

**Workflow permission rule.** Every `permissions:` scope must be one GitHub accepts; check 11
fails the rest. An invented scope is not a weaker permission, it is a parse error that takes
the whole file down — GitHub records a 0s run with no jobs and no diagnostic outside the
Actions tab, which is how a one-line `workflows: write` left `auto-release.yml` unrunnable.
In particular there is no `workflows` scope: pushing a commit that touches
`.github/workflows/**` is a property of the *token* (a PAT's `workflow` scope, a fine-grained
token's `Workflows: read and write`, or a GitHub App permission), and `permissions:` only
tunes GITHUB_TOKEN, which GitHub bars from workflow files outright. `actions:` is a different
right — workflow *runs*, not workflow *files*. The scope list tracks the pinned actionlint, so
the two gates in `pixi run lint` cannot contradict each other; a scope newer than both carries
the house `stale-ref-allowed` marker until the pin catches up.

```bash
# One development environment. Pixi supplies it; Turbo owns the cross-language task graph.
pixi run --frozen lint           # per-crate clippy + fmt/deny + workflow/TOML/Biome/repo checks
pixi run --frozen test           # per-crate nextest + doctests; full merge gate
pixi run --frozen coverage       # workspace llvm-cov → crates/lcov.info
pixi run --frozen fmt            # rewrite Rust; `pixi run --frozen fmt --check` is the gate form
pixi run --frozen docs dev       # Turbo-filtered Astro dev server
pixi run --frozen docs build     # Turbo-filtered Astro production build
pixi run --frozen bun-install    # frozen root Bun workspace install
pixi run --frozen bunx backlog   # repo backlog
pixi run --frozen bunx skills    # agent skills CLI
pixi run --frozen bunx turbo run test --affected  # focused changed-package loop, not the CI gate
pixi run --frozen -- cargo check -p pixi-sandbox  # focused Cargo operation, not a second gate

# every repository-automation subcommand goes through ONE task (crates/xtask); extra arguments
# follow the subcommand, and `--` separates them when there is more than one.
pixi run --frozen xtask check-repository            # the D10-exempt repo lints
pixi run --frozen xtask lint-generated-workflow     # actionlint over both workflows `init` generates
pixi run --frozen xtask render-relock               # rewrite the committed render of .github/workflows/relock.yml
pixi run --frozen xtask prepare-release auto        # stamp the next version everywhere (no git actions)
pixi run --frozen xtask commit-release v0.3.8       # commit + tag + push a prepared release (--dry-run shows the diff)
pixi run --frozen xtask check-release-refs          # documented release references vs the declared version
pixi run --frozen xtask check-conda-platforms dist/conda
pixi run --frozen xtask version

# the transport pipeline. Every path is an argument with the local default baked in, so CI runs
# the same task with its own values instead of a second `ci-*` twin reading the environment.
pixi run --frozen sandbox-plan              # validate .pixi-sandbox.toml and show native publish jobs
pixi run --frozen sandbox-plan --json       # CI form: machine-readable, also what `lint` runs
pixi run --frozen sandbox-pack              # build the self-bin, then pack (defaults: . / default / .sandbox-out / linux-64)
pixi run --frozen sandbox-doctor            # verify a transport: every sha256, nothing written
pixi run --frozen sandbox-publish           # force-push it as an orphan branch
pixi run --frozen sandbox-restore           # scripts/restore.sh

# packaging: the only tasks that name an environment. `package` is empty on purpose, so these
# run on any native runner — including win-64, where `default` cannot resolve at all.
pixi run --frozen -e package package                        # build the .conda for the current platform
pixi run --frozen -e package xtask smoke-conda-package      # install it and run the packaged binary here

# the release pipeline: release.yml drives these on every native runner through `-e package`
# (the empty environment resolves on win-64 too); locally the same tasks run in `default`
pixi run --frozen build-release-binary                      # cargo build -p pixi-sandbox --release (-- --target <triple> in CI)
pixi run --frozen xtask stage-release-binary                # strip + stage pixi-sandbox-<target>[.exe], host triple by default
pixi run --frozen xtask release-checksums                   # SHA256SUMS over the standalone binaries + completeness check
pixi run --frozen dispatch-release v0.3.8                   # manual recovery only: re-run release.yml for a tag whose push trigger did not fire

# the airlock proof (.github/workflows/airlock.yml): these drive the RELEASED binary, so the
# proof is about published assets; airlock-install-released puts it on PATH first
pixi run --frozen xtask airlock-matrix                       # plan JSON → matrix= (validated), stdout locally
pixi run --frozen xtask resolve-release-tag -- --repo <owner/name>   # which release to prove, and why
pixi run --frozen airlock-doctor <transport>                 # verify a transport with the installed binary
pixi run --frozen xtask deny-egress -- <cmd>                 # re-exec a command with egress denied
```

Git hooks (`lefthook.yml`, installed with `pixi run --frozen -- lefthook install`) are split by cost:
`pre-commit` runs only checks that never invoke cargo, over staged files (rustfmt, taplo,
biome, actionlint), and `commit-msg` runs convco; every gate that needs a Rust build — clippy,
the `check-repository` and `lint-generated-workflow` xtasks, and the whole-workspace `test` —
runs once at `pre-push`, in order, stopping at the first failure. Pre-push jobs carry no `glob`
on purpose: lefthook skips a pre-push job whose push-file list comes back empty, which would
make the heavy gates silently optional on exactly the unusual pushes.

Tests that must exist for any change to sharding or the manifest: a round-trip property test
(`tests/shard.rs`), a schema freeze (`tests/manifest.rs`), and a corruption case
(`tests/verify.rs`). A change to the embedded tool catalogue must keep
`tests/manifest.rs::the_embedded_tool_pins_are_valid_and_complete` green.

### Test conventions

**Tests live under `tests/`, never in the file they judge.** No `#[cfg(test)] mod tests` beside
production code: an oracle must not share a file with the thing it judges (the same reasoning
invariant 8 applies to git), because one refactor then adjusts both at once, and a unit test
reaching privates hides what the module's real callable surface is. So:

- A module worth testing is **promoted to `lib.rs` as `pub mod`** and tested through its public
  API from `tests/` — the shape `generated`, `release`, `self_update` and `user_tools` already
  have. One test file per module, named after it (`tests/self_update_replace.rs`).
- A handful of internals a test legitimately needs (a private helper whose failure branch has no
  public route, a marker constant) are exported `#[doc(hidden)] pub` — visible to `tests/`,
  marked as a test boundary rather than API.
- What is genuinely private to the **binary** target — `cli.rs`, `commands/` — cannot be reached
  from `tests/` at all, because only `lib.rs` is importable. Cover it **black-box** by driving
  the binary from `tests/cli.rs` with `assert_cmd`. That is the stronger test anyway: it is the
  surface a user meets. (This is how the clap command tree is audited.)

`production_sources_carry_no_inline_test_modules` in `tests/fixtures.rs` enforces this for
`crates/pixi-sandbox`. Its `LEGACY` list is the remaining pre-rule debt — it **may shrink, never
grow**, and the test also fails on a stale entry so the list cannot rot. `pixi-sandbox-core` and
`xtask` still carry inline tests and are not yet enforced; splitting them is backlog work, and
new modules there should follow this rule regardless.

Integration setup belongs in `crates/pixi-sandbox/tests/support/mod.rs`: request its zero-argument
`#[fixture]` values from `#[rstest]` tests, and use the paired `*_fixture` function pointer for a
helper that needs runtime paths. Never copy a support helper into a test. Turn finite input tables
into named `#[case]`s; express universal contracts as bounded `proptest!` properties and commit
the corresponding `.proptest-regressions` seed file. The `coverage_guard` fixture test requires
each non-wiring production module to have a named integration-test route under `tests/`.

## Style

- Rust: `pixi run --frozen fmt`, clippy clean with `-D warnings`, no `unwrap()` in library code paths that
  handle user input (the tests may unwrap freely).
- Comments explain *why* (usually pointing at a decision ID or a measurement), not *what*.
- GitHub `uses:` dependencies are full commit SHAs with a trailing human release-label comment;
  do not downgrade them to mutable tags. See `.knowledge/publish-automation.md`.
- Docs pages in `docs/src/content/docs/` are user-facing; keep the airlock page (`restore.mdx`)
  actionable for someone with no network and no context.

<!-- BEGIN:turborepo-agent-rules -->

# This is NOT the Turborepo you know

Turborepo configuration, task behavior, and CLI commands can vary between installed versions and may differ from your training data. Resolve the `turbo` package from this file's directory or relevant workspace; in monorepos, it may not be visible from the repository root. For example, run `node -p "require.resolve('turbo/package.json')"` from a workspace that depends on `turbo`.

Read `docs/README.md` inside that installed package first, then read the relevant pages from its `docs/` directory before changing Turborepo configuration or commands. Heed deprecation notices. These bundled docs match the installed package version and are available without network access.

This block is written and re-added by `turbo` before repository-scoped commands when an AI agent is detected. In the Turborepo source repository, its template is defined in `crates/turborepo-cli/src/cli/agent_guidance.rs`. Removing the managed block while updates are enabled means a later qualifying invocation will add it again. Set `"agentGuidance": false` in the root `turbo.json` or `turbo.jsonc` to opt out; this does not remove an existing block. Keep the block committed with your work to avoid an uncommitted change on the next agent invocation.
<!-- END:turborepo-agent-rules -->
