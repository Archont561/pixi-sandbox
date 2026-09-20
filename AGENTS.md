# AGENTS.md

Instructions for coding agents working **in this repository**. For the agent-facing guide that
ships *inside a sandbox branch*, see the generated `AGENTS.md` under `.pixi-sandbox/` in that
branch — different audience, different file.

## What this project is

`pixi-sandbox` packs pixi environments into a git orphan branch and restores them on machines
with no network. The design is settled and measured: **read `.knowledge/design.md` before
proposing a different one**, and `.knowledge/decisions.md` for the short form. The eight
decisions (D1–D11) are load-bearing; if you think one is wrong, bring a measurement, not an
opinion.

## Repo map

| path | notes |
| --- | --- |
| `crates/pixi-sandbox-core/src/manifest.rs` | **wire format** — `manifest.json` shape and `SCHEMA_VERSION`. Changing it needs a schema bump + fixture update in `tests/manifest.rs` |
| `crates/pixi-sandbox-core/src/shard.rs` | the only two file operations: `record_file` (split if oversized) and `materialise`/`join_parts` (verify, then write) |
| `crates/pixi-sandbox-core/src/tools_lock.rs` | compiles the canonical helper-tool pins into the binary; `--tools-lock` is an explicit override |
| `crates/pixi-sandbox-core/src/verify.rs` | collects *all* failures instead of stopping at the first — an airlock operator wants the full list |
| `crates/pixi-sandbox/src/commands/*` | `pack`, `publish`, `restore`, `unpack`, `doctor`, `plan`, and `tools list` — pure Rust, no Python |
| `crates/pixi-sandbox-git` | **all** git access: `GitProtocol` + `ShellGit` (real git, with a swappable `Runner`) + `FakeGit` (in-memory mock). Never run `git` from anywhere else |
| `crates/pixi-sandbox/tests/fixtures/` | the fixture project and the synthetic transport. **Tests must not point at this repository** — see `.knowledge/decisions.md` D10 |
| `crates/pixi-sandbox-core/assets/tools.lock.json` | canonical embedded helper-tool pins; edit as reviewed data and keep the embedded-lock tests green |
| `.pixi-sandbox.toml` | explicit project publish bundles / native runners consumed by `pixi-sandbox plan` and the reusable release workflow |
| `feature.utils.actionlint` | runs `actionlint` from conda directly when validating workflows in the `dev` environment |
| `scripts/restore.sh` | one-liner offline reconstruction from orphan branch with PATH aliases |

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
6. **Do not commit generated weight**: `.pixi/`, `target/`, `vendor/`, `.pixi-sandbox/`, `docs/dist/`.
7. **A dynamically linked tool is a bug**, not a warning to ignore (`verify.rs` flags it).
8. **Git only through `GitProtocol`** — never a bare `Command::new("git")` outside
   `pixi-sandbox-git` (D9). Tests use `FakeGit`; `--dry-run` uses a runner, not a second code path.
9. **Tests target the fixtures, never this repository** (D10): `tests/fixtures/demo-project`
   for project-level work, `tests/fixtures/transport` for payload-level work. Two tests enforce it.

## Commands

Source-driven GitHub Actions commands are one of these lines (design.md §6): **if a
source-driven workflow command needs more than one line, add a task to `pixi.toml` instead.**
The checksum-verified release publisher is implemented as reviewed composite-action shell
(POSIX + PowerShell), not Python.

```bash
pixi run -e dev lint           # fmt-check + clippy -D warnings + deny + actionlint + taplo + biome
pixi run -e dev test           # cargo nextest run --workspace
pixi run -e dev test-actions   # hermetic release Action checksum / PATH-shadow tests
pixi run -e dev coverage       # cargo llvm-cov → lcov.info (CI uploads to codecov)
pixi run -e dev docs-dev       # Astro dev server for docs/
pixi run -e dev sandbox-plan   # validate .pixi-sandbox.toml and show native publish jobs
pixi run -e dev lint-sandbox-plan # CI form: validate root plan plus embedded tool coverage
pixi run -e dev sandbox-pack   # pack the transport (Rust CLI; embeds the chosen self-bootstrap binary)
pixi run -e dev sandbox-doctor # verify a transport: every sha256, nothing written
pixi run -e dev sandbox-publish# force-push it as an orphan branch
pixi run -e dev sandbox-proof  # the full cold-start proof

# the same steps as CI runs them (paths come from the SANDBOX_* environment variables)
pixi run -e ci ci-pack && pixi run -e ci ci-doctor && pixi run -e ci ci-publish
```

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
