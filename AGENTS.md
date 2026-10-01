# AGENTS.md

Instructions for coding agents working **in this repository**. For the agent-facing guide that
ships *inside a sandbox branch*, see the generated `AGENTS.md` under `.pixi-sandbox/` in that
branch — different audience, different file.

**Start of session?** Follow `.agents/skills/session/SKILL.md` — it sequences the environment
bootstrap on an airlocked machine, the backlog survey, and the session proposal.

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
| `crates/pixi-sandbox/src/commands/*` | `init`, `pack`, `publish`, `restore`, `unpack`, `doctor`, `plan`, and `tools list` — pure Rust, no Python |
| `crates/pixi-sandbox-git` | **all** git access: `GitProtocol` + `ShellGit` (real git, with a swappable `Runner`) + `FakeGit` (in-memory mock). Never run `git` from anywhere else |
| `crates/pixi-sandbox/tests/fixtures/` | the fixture project and the synthetic transport. **Tests must not point at this repository** — see `.knowledge/decisions.md` D10 |
| `crates/pixi-sandbox-core/assets/tools.lock.json` | canonical embedded helper-tool pins; edit as reviewed data and keep the embedded-lock tests green |
| `.pixi-sandbox.toml` | explicit project publish bundles / native runners consumed by `pixi-sandbox plan` and the reusable release workflow |
| `feature.utils.actionlint` | runs `actionlint` from conda directly when validating workflows in the `default` environment |
| `crates/xtask` | typed repository automation invoked by Pixi tasks — every repo-targeting job that used to be a shell script: `check-repository` (the D10-exempt repo lints; opt out with `stale-ref-allowed` on or above the line), `prepare-release` + `check-release-refs` + `version`, `smoke-conda-package` (the `package-smoke` task on every `release.yml` build leg), `check-conda-platforms` (all five platforms contributed exactly one package; reads `dist/conda/conda-<platform>/`, which is why those artifacts are downloaded *unmerged*), and `lint-generated-workflow`, plus `stage-release-binary` and `release-checksums` (the release matrix's asset staging and SHA256SUMS — the `*unknown-*` glob this replaced had silently checksummed only the two musl binaries of five), and `commit-release` (auto-release's commit/tag/push half: stage exactly what `.release-touched` reports, refuse unaccounted modifications, refuse an existing remote tag, then commit, tag and push — git through `pixi-sandbox-git`, D9). The airlock workflow's machinery lives there too: `airlock-matrix` (plan JSON validated, `matrix=` written to `GITHUB_OUTPUT`), `resolve-release-tag` (override → declared-but-published → newest-published, each branch an incident), `airlock-pack` (frozen env installs + the released-binary pack), `airlock-fetch` (the developer-shaped fetch, D9), and `deny-egress` (`sudo unshare -n` / `sudo sandbox-exec`, loud on unknown OS). One Rust runtime on every runner — the v0.3.6 release died on macOS's Bash 3.2 over a single bashism — and every policy is tested against tempdir fixtures, never this checkout |
| `scripts/restore.sh` | one-liner offline reconstruction from orphan branch with PATH aliases; derives the branch from `.pixi-sandbox.toml` for the host platform. Also selects the task-33 user-tool policy explicitly (`PIXI_SANDBOX_USER_TOOLS`, exported before the restore): a verified restore registers `pixi` + `pixi-sandbox` launchers in the user's home by default, `skip` opts out for CI. The policy travels as an environment variable, never a flag — the binary this script drives comes from the packed branch and may predate `--user-tools` (the airlock workflow proved that skew), while an unknown variable is simply ignored. With `scripts/airlock-gate.sh` (which targets a *restored* sandbox under blocked egress, not this repo), the only shell left — both run where no Rust toolchain can be assumed, and `check-repository` holds them to the Bash 3.2 surface |
| `package.json` + `bun.lock` | the root bun workspace — `docs` is a member, so `docs-install` runs at the root — plus the repo-wide `backlog.md` / `skills` / `@biomejs/biome` devDependencies (`pixi run backlog`, `skills`, `lint-docs`) |

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
8. **Git only through `GitProtocol`** — never a bare `Command::new("git")` outside
   `pixi-sandbox-git` (D9). Tests use `FakeGit`; `--dry-run` uses a runner, not a second code path.
9. **Tests target the fixtures, never this repository** (D10): `tests/fixtures/demo-project`
   for project-level work, `tests/fixtures/transport` for payload-level work. Two tests enforce it.
10. **Anything you did not implement goes in `CONTEXT.md` § Session scratchpad** — suggestions,
    drafts, alternatives considered, "we should probably…", measurements worth keeping. Under a
    dated heading, newest last. Never into `AGENTS.md`, `README.md`, `.knowledge/`, a
    `backlog/docs/` spec or a code comment: those state what *is*, and a proposal mixed into
    them reads as a decision nobody made. A scratchpad entry graduates only by becoming a
    backlog task, a decision in `.knowledge/decisions.md`, or a deletion.

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
one-line host bootstrap. The last shell steps are the two `scripts/airlock-gate.sh` tiers in
`airlock.yml` (one-line invocations; **task-35** moves the gate into the e2e suite and deletes
them), plus the sanctioned bootstrap scripts. A reviewed exception (a generated consumer
artifact, say) carries `multiline-run-allowed` on the step.

```bash
# one development environment, so every task is `pixi run <task>` with no -e flag. The bun tasks
# are declared under `[feature.web.tasks]`, and pixi runs them in the `web` environment anyway —
# including as a dependency of `lint` — so these lines do not change.
pixi run lint           # fmt --check + clippy -D warnings + deny + actionlint (committed + generated workflows) + taplo + biome + repo-consistency
pixi run test           # nextest workspace, including fixture-backed offline lifecycle tests
pixi run coverage       # cargo llvm-cov → lcov.info (CI uploads to codecov)
pixi run fmt            # rewrite; `pixi run fmt --check` is the gate form
pixi run docs-dev       # Astro dev server for docs/
pixi run docs-install   # bun install --frozen-lockfile at the root of the bun workspace
pixi run bunx backlog   # repo backlog (the bunx task runs docs-install itself)
pixi run bunx skills    # agent skills CLI, same workspace
pixi run bun <args>     # bun itself, in the web environment, args pass through

# every repository-automation subcommand goes through ONE task (crates/xtask); extra arguments
# follow the subcommand, and `--` separates them when there is more than one.
pixi run xtask check-repository            # the D10-exempt repo lints
pixi run xtask lint-generated-workflow     # actionlint over the workflow `init` generates
pixi run xtask prepare-release auto        # stamp the next version everywhere (no git actions)
pixi run xtask commit-release v0.3.8       # commit + tag + push a prepared release (--dry-run shows the diff)
pixi run xtask check-release-refs          # documented release references vs the declared version
pixi run xtask check-conda-platforms dist/conda
pixi run xtask version

# the transport pipeline. Every path is an argument with the local default baked in, so CI runs
# the same task with its own values instead of a second `ci-*` twin reading the environment.
pixi run sandbox-plan              # validate .pixi-sandbox.toml and show native publish jobs
pixi run sandbox-plan --json       # CI form: machine-readable, also what `lint` runs
pixi run sandbox-pack              # build the self-bin, then pack (defaults: . / default / .sandbox-out / linux-64)
pixi run sandbox-doctor            # verify a transport: every sha256, nothing written
pixi run sandbox-publish           # force-push it as an orphan branch
pixi run sandbox-restore           # scripts/restore.sh

# packaging: the only tasks that name an environment. `package` is empty on purpose, so these
# run on any native runner — including win-64, where `default` cannot resolve at all.
pixi run -e package package                        # build the .conda for the current platform
pixi run -e package xtask smoke-conda-package      # install it and run the packaged binary here

# the release pipeline: release.yml drives these on every native runner through `-e package`
# (the empty environment resolves on win-64 too); locally the same tasks run in `default`
pixi run build-release-binary                      # cargo build -p pixi-sandbox --release (--target <triple> in CI)
pixi run xtask stage-release-binary                # strip + stage pixi-sandbox-<target>[.exe], host triple by default
pixi run xtask release-checksums                   # SHA256SUMS over the standalone binaries + completeness check
pixi run dispatch-release v0.3.8                   # hand a fresh tag to release.yml (a GITHUB_TOKEN tag push starts nothing)

# the airlock proof (.github/workflows/airlock.yml): these drive the RELEASED binary, so the
# proof is about published assets; airlock-install-released puts it on PATH first
pixi run xtask airlock-matrix                       # plan JSON → matrix= (validated), stdout locally
pixi run xtask resolve-release-tag -- --repo <owner/name>   # which release to prove, and why
pixi run airlock-doctor <transport>                 # verify a transport with the installed binary
pixi run xtask deny-egress -- <cmd>                 # re-exec a command with egress denied
```

Git hooks (`lefthook.yml`, installed with `pixi run -- lefthook install`) are split by cost:
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

## Style

- Rust: `cargo fmt`, clippy clean with `-D warnings`, no `unwrap()` in library code paths that
  handle user input (the tests may unwrap freely).
- Comments explain *why* (usually pointing at a decision ID or a measurement), not *what*.
- GitHub `uses:` dependencies are full commit SHAs with a trailing human release-label comment;
  do not downgrade them to mutable tags. See `.knowledge/publish-automation.md`.
- Docs pages in `docs/src/content/docs/` are user-facing; keep the airlock page (`restore.mdx`)
  actionable for someone with no network and no context.
