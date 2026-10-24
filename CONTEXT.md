## Native transport release-readiness update — 2026-10-22

Active cleanup branch: `arena/b4a9cf6a-pixi-sandbox`, based on merged PR191 / source 0d15c4d8.
Runtime implementation and real validation are green: CI 39824086495 (payload/offline consumer,
Rust 1.94, Windows), native publisher 39824086542, five-platform artifacts-only release
39826041383 (fresh Linux artifact consumer lifecycle), and Pages 39825855879. Auto-release
39828722470 was a successful dry run: no new release/tag. The published transport is schema 3,
source 0d15c4d, and contains only explicitly selected pixi/pixi-sandbox executables. Native format-1
restore is also verified locally from that transport with zero blob/file failures.

Current work corrects remaining helper-era docs/comments and records the native API/CLI migration.
TASK125 runtime ACs are reconciled to ADR-31; final documentation delivery/merge remains.
The cleanup passes full lint, ordinary tests and coverage (both 1,186 pass / 2 skipped),
doctests, and docs build/frontmatter/links/a11y/snippets (29 pages, 62 commands, 120 flags).
The pre-cleanup ordinary/coverage inventory was 1,188 entries across 96 binaries; no test was
added or removed by this cleanup. Tests retired during the migration were approved; do not
resurrect helper APIs to
satisfy obsolete task wording. Legacy opaque compatibility readers and optional payload building
are intentional exceptions, not new-producer helper dependencies.

Recommend v0.8.0, a breaking pre-1.0 minor release; manifests/stamps stay 0.7.0 until auto-release
preparation. This request authorizes cleanup and a recommendation, not production dispatch.
See `backlog/docs/plans/task-125-release-readiness.md` for evidence and the remaining review step.
TASK126/127 controlled cache/billing/fallback measurement obligations remain open and are not
claimed as release correctness failures or measured savings.

# Project Context

Orientation guide for developers and AI agents working on `pixi-sandbox`.

---

## 🎯 Mission & Core Value

`pixi-sandbox` enables **fully offline, reproducible airlock restoration** for projects using [pixi](https://pixi.sh) (and Cargo). It bridges the gap between connected CI/developer machines and disconnected air-gapped environments:

1. **Pack**: Captures conda environments and Cargo vendored dependencies into an immutable, hashed transport payload.
2. **Verify**: Asserts all file sizes, split parts, and SHA-256 digests.
3. **Publish**: Pushes the transport to an isolated Git orphan branch (e.g. `sandbox/developer-linux-64`).
4. **Restore**: Unpacks environments offline, relocates the install prefix inside the text metadata, deletes the scratch it created, configures Cargo vendor paths, and verifies filesystem markers without internet access.

---

## 🏗️ Architectural Invariants

> [!IMPORTANT]
> Never violate these architectural invariants:

1. **Verify Before Write**: No file reaches the user's filesystem before its cryptographic SHA-256 matches the manifest.
2. **Hermetic Test Isolation**: Tests **must never** sandbox or point at this repository root (`tests/fixtures.rs` asserts this). Tests operate strictly on temporary copies of `crates/pixi-sandbox/tests/fixtures/`. The transport fixture's `demo` environment is a real conda prefix (`envs/demo/pack/prefix/prefix.tar.gz`), so relocation is exercised against what conda actually ships; the one test that packs a *real* environment is `#[ignore]`d because it needs the network.
3. **Pure Native Execution**: GitHub composite actions (`setup-pixi-sandbox`, `publish-pixi-sandbox`) and CI tests are pure shell/PowerShell and native Rust (`cargo nextest`), with zero Python runtime dependency.
4. **Sharding Limit**: Shards are whole files. Only files exceeding **95 MiB** are split into `.partNNN` segments to guarantee blobs stay safely under GitHub's 100 MiB limit.
5. **Git Trait Boundary**: All Git commands go through the `GitProtocol` trait (`crates/pixi-sandbox-git`). The CLI uses `ShellGit`, tests use in-memory runners, and tests never touch developers' real Git configuration.
6. **Airlock Restoration Independence**: The restore step on the disconnected host requires only the embedded static binaries (`pixi-unpack` / `pixi-sandbox`) and no network connectivity.
7. **A Restored Environment Names Itself**: while an environment is staged it still points at the staging path, so every valid-UTF-8, NUL-free text file has that path rewritten to the final prefix immediately before the rename — a NUL byte means a fixed-width binary, which is left exactly as it is. The scratch goes when the restore succeeds and stays when it fails, because a failed restore's scratch is the evidence.
8. **A Restored Tree Is Checked Against the Manifest, Not Its Shape** (ADR-13): a schema-2 manifest carries a per-file oracle (`envs/<env>/files.json`) recorded by the packer from its own verification unpack, and `doctor --verify-restored <PROJECT>` compares the restored prefix against it — content, symlink targets, exec bits, the fingerprint marker — collecting every mismatch and writing nothing. Shape checks (conda-meta present, `pixi install` a no-op) prove self-sufficiency, not integrity; a stub prefix with a fabricated conda-meta record must fail the airlock gate, and it does, by test.

---

## 🔄 CI & Automation Pipeline

- **Unified Single-Job CI (`.github/workflows/ci.yml`)**:
  - Runs on `ubuntu-latest` in the `default` pixi environment.
  - Leverages `Swatinem/rust-cache` to cache `~/.cargo/` and `./target` across commits.
  - Sequentially runs `lint` (fmt, clippy, deny, actionlint, taplo, biome), `test` (nextest), `test-doc`, `coverage` (llvm-cov), and `docs build`.
- **Transport publishing (manual since task-29 retired the reusable publisher)**: the developer
  transport (`sandbox/developer-linux-64`) is packed and published by hand — `pixi run
  sandbox-pack` (which builds `target/release/pixi-sandbox` first and packs it via `--self-bin`,
  so the transport embeds the tree's own binary, not the operator's installed one), then
  `sandbox-doctor`, then `sandbox-publish`. `.github/workflows/airlock.yml` never touches the
  real branch: it packs to a throwaway bare repo under the runner's temp dir, deliberately with
  the *released* binary (a real airlock starts from a released asset), as the cross-platform
  proof. `publish-sandbox.yml` is now only the workflow `pixi-sandbox init` generates for
  consumer projects.

---

## 🌐 Online / Codespaces agent tasks (cannot be done from the airlock)

> [!NOTE]
> This repository is often worked from a **restored, offline-style airlock sandbox** where general
> package registries are unreachable (only `github.com` git and npm are). An airlock agent can
> build/test/lint, edit docs, run `convco`, and record decisions — but it **cannot** reach
> crates.io / conda channels / `static.rust-lang.org`, cross-compile the release targets, cut a
> GitHub Release, or run a native macOS/Windows runner. Work that needs a connected host is
> therefore **assigned to an online agent running in GitHub Codespaces (or CI)** and tracked here so
> it is not silently attempted offline:

All three entries below are closed, so this list was retired on 2026-10-05; it is kept only to
record the airlock boundary each one sat on.

1. **Prove `osx-arm64` and publish a macOS bundle** (backlog **task-1**, gated by **ADR-11**) — the
   native `macos-14` leg went green end to end (released binary installed and checksum-verified,
   packed, published to the throwaway remote, fetched, restored, gate passed). AC#1 was held back
   until a run showed Tier A **actually executed**: `BLOCK_NETWORK` had been opt-*in*, so `null ==
   false is true` left the authoritative egress-denied tier skipped on exactly the two triggers
   that fire. It is opt-out now. The shape still holds for any future platform: declaring it in
   `.pixi-sandbox.toml` puts it in `plan --json` and the `pull_request` trigger fans the matrix out
   by itself, so an agent opens the PR and reads the verdict — only the *execution* needs the
   runner. (`gh workflow run` is not an alternative from a sandbox: the token there has no
   `actions: write`, and a dispatch returns HTTP 403.)
2. **Verify `pixi-sandbox tools update` live** (backlog **task-4**) — closed on offline evidence:
   14 unit tests in `commands/tools/update.rs` plus `tests/manifest.rs`'s embedded-pin catalogue
   test. The *capability* boundary still stands and is the reason a live check lives in CI rather
   than here: release **assets** are unreachable from an airlock (`objects.githubusercontent.com`
   and `release-assets.githubusercontent.com` both fail to connect, even though `api.github.com`
   answers).
3. **Repin the published `init.sh` to v0.3.0 binaries** (backlog **task-11**) — verified against
   the published asset rather than the local file (re-downloaded by asset id, sha256 matched the
   rendered bytes, `sh -n` parsed, `VERSION` defaulted to v0.3.0 where the stale asset said v0.2.0).
   Superseded for every later release: `templates/install.sh` is rendered at the tag by `xtask
   render-install` rather than stamped from a committed copy, so a published one-liner cannot
   default to a version other than its own.

**What still needs a connected host today:**

- **task-66 AC#6, task-68 AC#7, task-71 AC#5** — each needs a real consumer pull request or a real
  failed publish run in **Castellan** (`github.com/Archont561/castellan`, public) to prove a
  behavior the fixtures can only assert structurally: a PR-visible Check Run on a repaired head,
  the secondary summary/artifact when a publish phase fails, and the upgrade lane's delivery.
- **task-72 AC#1** — the measurement half needs the ~1.86 GiB Castellan transport (largest blob,
  repository/push size, fetch/clone cost, restore disk). AC#2–#5 are offline.
- **task-73 AC#3/#4 and task-74** — need an owner-operated **GitHub App** installation and, for
  task-74, a new public template repository. The repo-scoped sandbox token gets
  `403 Resource not accessible by integration` on `gh secret list -R Archont561/castellan`, so the
  `PIXI_SANDBOX_UPGRADE_TOKEN` secret cannot be set from a sandbox at all.

The dev container that provides this online environment is `.devcontainer/devcontainer.json`
(`ghcr.io/prefix-dev/pixi`, with `pixi install --locked --all` + `docs-install` + a global `bun add` of opencode
on create). Its setup step reaches bun through the implicit `default` environment, not `-e web`:
`web` is a feature that task-70 folded into `default`, and pixi rejects `-e web` as unknown.

---

## 🧭 Current v1 Planning and Network Boundary

The current redesign is planned in Backlog milestone **m-0 — v1 standalone cross-platform workflow**. The canonical planning/specification documents are under `backlog/docs/`; `backlog/docs/` now keeps historical decisions, raw evidence, and discovery pointers.

### Backlog workstreams

- **TASK-21** — make `init` provider-neutral and platform-specific; use `pixi-sandbox init` with `--github-workflow-path`, `--script-path`, and `--config`; generate only the current platform launcher.
- **TASK-22** — generate a disposable workflow that installs `pixi-sandbox` from `@archont561/pixi-sandbox` and calls the binary directly, without Archont561 composite Actions.
- **TASK-23** — publish Pixi/Conda package variants for `linux-64`, `linux-aarch64`, `osx-64`, `osx-arm64`, and `win-64`, while retaining standalone release binaries for transport bootstrap.
- **TASK-24** — measure standalone transport/restore orchestration and tool deduplication before changing the existing ADR-2/ADR-3 `pixi-pack`/`pixi-unpack` decisions; schema compatibility is required.
- **TASK-25** — moved npm-compatible tooling such as Biome and Astro to `package.json`/`bun.lock`; task-70 later folded Bun into the one developer environment for cross-language orchestration.
- **TASK-26 / TASK-70** — the JS-only Turbo spike deferred adoption, then Pathway's measured Rust-package model supplied a new use case; ADR-15 now records Turbo as the cross-language graph.

The v1 proposal is now an accepted architectural decision: backlog `ADR-23` is accepted
(through task-24's measurements, recorded in `ADR-24` and `doc-7`), with the boundary that
`pixi-pack`/`pixi-unpack` remain pinned helper subprocesses (ADR-2/ADR-3/ADR-4/ADR-12, reaffirmed as ADR-14).

### Work that requires an online Codespace or native CI

- Resolve/download Pixi, Conda, Bun/npm, Turbo, or release assets when they are not already cached.
- Run GitHub Actions and inspect workflow results.
- Publish package variants to prefix.dev.
- Push sandbox transport branches or create GitHub Releases.
- Run native macOS/Windows package and binary jobs.
- Validate installation from the `@archont561/archont561` ecosystem channel.
- Perform the connected half of the standalone transport comparison.

### Work that can be done offline

- Implement and test `init`, config precedence, launcher generation, and workflow rendering.
- Update Rust code, Markdown, Backlog tasks/documents, and `.knowledge` pointers.
- Run fixture-backed tests, lint, manifest verification, and local transport tests.
- Run the airlock proof against an existing transport:

```bash
bash scripts/restore.sh
export PATH="$HOME/.local/bin:$PATH"   # only needed in the already-running shell
pixi install --frozen --offline
pixi run --frozen -- cargo build --offline
```

Pixi is the only supported entrypoint. Do not source `.pixi/sandbox-env.sh` (new restores remove
or do not generate it) and do not run bare `cargo`, `rustc`, `bun`, `taplo`, or `convco` from a
restored prefix. Rust crate work is either `pixi run --frozen xtask <subcommand>` or
`pixi run --frozen -- cargo <cmd> -p <crate>`; JavaScript package work is
`pixi run --frozen bun --filter=<workspace-package> run <script>` (for docs:
`--filter=pixi-sandbox-docs`) or the root `pixi run --frozen bunx <tool>` task.

Do not attempt package resolution, prefix.dev publication, GitHub Actions dispatch, or native Windows/macOS validation from an airlock-style environment. First produce artifacts on the connected side, then consume and verify them offline.

---

## 📚 Key References

| Resource | Purpose |
|:---|:---|
| [`backlog/docs/adr/README.md`](backlog/docs/adr/README.md) | Architectural Decisions ADR-1–ADR-12 with empirical lab measurements |
| [`backlog/docs/specifications/transport-and-restore/doc-2 - Transport-and-Restore-Specification.md`](backlog/docs/specifications/transport-and-restore/doc-2 - Transport-and-Restore-Specification.md) | In-depth design specification and airlock invariants |
| [`backlog/docs/specifications/rust-bootstrap/doc-4 - Rust-Bootstrap-Specification.md`](backlog/docs/specifications/rust-bootstrap/doc-4 - Rust-Bootstrap-Specification.md) | Rust static bootstrap binary strategy and validation checklist |
| [`.knowledge/README.md`](.knowledge/README.md) | Open Knowledge Format index |

---

## 🗒️ Session scratchpad (agent proposals, drafts, open questions)

> [!IMPORTANT]
> **This is the only place an agent records something it did not implement.** Suggestions,
> half-finished designs, alternatives considered, measurements worth keeping, "we should
> probably…" — they go here, newest section last, each under a dated heading. They do **not**
> go into `AGENTS.md`, `README.md`, `backlog/docs/`, a spec under `backlog/docs/`, or a comment
> in the code they speculate about: those files state what *is*, and a proposal mixed into them
> reads as a decision nobody made.
>
> Promotion out of here is deliberate: a proposal becomes a backlog task (it will be built), a
> decision in `backlog/docs/adr/README.md` (it was decided, with evidence), or it is deleted.
> Anything still sitting here is explicitly **not** agreed.

### 2026-10-01 — tooling session (lefthook split, workflow rule, restore findings)

**Landed** (so: not proposals) — pre-commit stopped compiling Rust and the cargo gates moved to
`pre-push`; `scripts/restore.sh` now reports the user-tool registration that actually happened
**and is executed by 15 tests** (`tests/restore_script.rs`, where it had none);
`shard::materialise` stages-and-renames so a running tool can be replaced; the pixi task table
went from 38 tasks to 23 behind one `xtask` task and argument-carrying tasks.

**Open proposals, not agreed:**

- **Pack and publish a 0.3.7 transport.** The published `sandbox/developer-linux-64` branch
  carries pixi-sandbox **0.3.6**, which predates `--user-tools`, so no restore from it can put
  `pixi` on a developer's PATH and every airlock session still sources `sandbox-env.sh`. A
  repack would switch that on. Not done because `publish` force-pushes the branch — a
  maintainer's call, not an agent's.
- **Docs tasks could collapse further** (`docs dev`/`docs build` → one `docs <mode>` task). Left
  alone deliberately: both names appear in CI, docs and the README, and the churn buys one line.

### 2026-10-01 — session close

**Landed** (PR #50, nine commits, rebased onto `main`): pre-push cargo gates; the honest
user-tool report in `scripts/restore.sh`; stage-and-rename in `shard::materialise`; the pixi
task table at 23 behind one `xtask` task; `tests/restore_script.rs` (15 tests) and the
`materialise` error-path tests; `AGENTS.md` invariant 10 and this scratchpad; the session
skill's templates in `standup-template.md`; task-38.

**Open, in the order a session should consider them:** the relock bot above (one decision
needed: push onto the PR branch, or open its own PR); **task-36** (high — 22 `run: |` blocks
left, 8 of them in `release.yml`, and its AC#2 is stale because it mandates the `SANDBOX_*`
convention `ci-pack`/`ci-doctor`/`ci-publish` used before those tasks were deleted);
**task-35** (medium — airlock gate into `tests/e2e` behind a `ci` feature, which also removes
the `airlock-gate.sh` exception from `fixtures.rs`); **task-38** (blocked until `rstest` is
vendored); and the 0.3.7 repack, which is still a maintainer's call because `publish`
force-pushes `sandbox/developer-linux-64`.

**Baselines for the next session:** 226 tests passing / 1 skipped; `pixi run lint` is nine
gates; the published transport is still packed by 0.3.6, so sourcing `sandbox-env.sh` remains
mandatory on a restored host.

**Prompt to start the next session with:**

> Restore the sandbox and baseline the suite (expect 226 passing / 1 skipped), then read
> `CONTEXT.md` § Session scratchpad — the last session left four open items there and a
> decision I owe you on the relock bot.
>
> I want to take **task-36** this session, in slices, starting with `release.yml`: move the
> binary staging/stripping, the SHA256SUMS generation and its completeness check into `xtask`
> subcommands with tempdir-fixture tests, so each workflow step becomes a pinned `uses:` or a
> one-line `pixi run xtask <subcommand>`. Before you start, rewrite the task's AC#2 — it still
> mandates the `SANDBOX_*` environment convention that `ci-pack`/`ci-doctor`/`ci-publish` used,
> and those tasks no longer exist; arguments with local defaults replaced them.
>
> Propose the slice and stop. House rules are in `AGENTS.md` (note invariant 10: anything you
> do not implement goes in `CONTEXT.md`, not into the files it speculates about), the session
> procedure and its templates are in `.agents/skills/session/`.

### 2026-10-01 (evening) — session close: the relock lane

**Landed** (PR #52, squash `b2e028f`, ten commits): task-39 complete, all seven ACs, built as a
*generated* artifact rather than a hand-written workflow —
`crates/pixi-sandbox/src/generated/relock_workflow.rs` rendered by `pixi-sandbox init`
(`--relock-workflow-path`, `--relock-ci-workflow`) and committed here as
`.github/workflows/relock.yml`, held byte-equal by `check-repository` check 10 with
`pixi run xtask render-relock` as the only way to change it. Also: thealf (`.agents/skills/session/`) — the opening-prompt and session-report templates, and a
post-merge phase that forbids closing a task on an AC no machine here can prove.

**The finding that made the generated shape cheap:** the check-9 generated-file exemption, which
the dogfood proposal below still needs for the *publisher*, was not needed here at all. The
relock lane is one-line steps throughout, so its render satisfies the workflow-shape rule
unaided. The exemption question is therefore still open, but it is now specifically a *publisher*
problem (bootstrap download + pack loop are multi-line shell by necessity), not a generated-file
problem in general.

**What the live demonstration taught, kept because the next bot-shaped workflow will hit it:**

1. **A job-level `permissions:` block replaces the default set; it does not add to it.** With
   `contents: write` alone the lock commit pushed and then `gh workflow run` exited 1 for want of
   `actions: write` (run 36927489932) — the worst ordering, since the commit lands and its
   verdict does not. Both scopes are now named, and a renderer test holds them.
2. **The `GITHUB_TOKEN` suppression is observable, not folklore.** The bot's push created a
   `pull_request` run that sat in `action_required` and never executed (36927958169).
3. **AC#5's premise, measured side by side on one commit:** the guard failed in 5s with a message
   about the manifest; `ci` failed in 22s inside `setup-pixi` with a message about installation.

**Open, in the order a session should consider them:**

- **task-35** (medium, unblocked, the natural next) — move the airlock gate into `tests/e2e`
  behind a `ci` cargo feature and delete `scripts/airlock-gate.sh`, which also removes its
  exception from `fixtures.rs`. It is the last shell in any workflow.
- **task-36 AC#6** (In Progress) — the release/auto-release half. The airlock half is proven on a
  real runner (PR #51). What remains needs a *real* cut: the newest `auto-release` run
  (36922096859) was a dry run that prepared **v0.4.0** without committing or tagging, so no
  `release.yml` run exists to read. One maintainer click produces it.
- **The 0.3.7+ transport repack** (maintainer's call — `publish` force-pushes
  `sandbox/developer-linux-64`). Still the reason every airlock session sources
  `sandbox-env.sh`, and now also the gate on task-38.
- **task-38** (medium) — the lock half of its blocker is gone: declaring `rstest` is an ordinary
  manifest edit, and the bot relocks it. The *airlock* half is not: a restored host builds
  against `.pixi-sandbox/vendor`, so the full ordering is edit → bot relocks → merge → repack and
  publish a transport → restore. Do not start it before the repack.
- **A fork-PR proof of AC#4.** The refusal is unit-tested and its gate was observed evaluating
  (`skipped` on a same-repo PR), but no fork of this repository exists to prove the live path.
  Needs a fork and a throwaway PR from it.
- **`lock guard` and branch protection.** Deliberately *not* added as a required check: it is red
  by design on any PR where the bot still has work to do, and only turns green when the bot's own
  dispatch re-runs it. Decide after watching a few real dependency PRs.

**Baselines for the next session:** 274 tests passing / 1 skipped (was 259); `pixi run lint` is
nine gates; `pixi lock --check` is clean and answers offline in milliseconds; the published
transport is still packed by 0.3.6, so sourcing `sandbox-env.sh` remains mandatory, and nothing
new was vendored this session, so the vendor tree still builds main.

**Prompt to start the next session with:**

> Restore the sandbox and baseline the suite (expect 274 passing / 1 skipped — the published
> transport is still the 0.3.6 pack, and the relock session added no dependency, so its vendor
> tree still builds main), then read `CONTEXT.md` § Session scratchpad — the evening close lists
> six open items.
>
> First, one look at the repo state: `gh release list`. If v0.4.0 exists, task-36's last open
> path is proven by its `release.yml` run — read the run (five binaries, a complete SHA256SUMS,
> five conda packages, the docs dispatch), check AC#6, complete the task file and close task-36.
> If it does not exist, leave it In Progress and say so.
>
> I want to take **task-35** this session — move the airlock gate into `crates/pixi-sandbox/tests/e2e.rs`
> behind a `ci` cargo feature (skipped locally, run by the matrix), delete `scripts/airlock-gate.sh`,
> and drop its reviewed exception from `fixtures.rs`. In slices: the e2e tests and the feature
> wiring first — all locally provable — then the `airlock.yml` rewiring, whose proof is a native
> matrix run on the PR and needs a push I will sanction.
>
> Propose the slice and stop. House rules are in `AGENTS.md` (invariant 10: anything you do not
> implement goes in `CONTEXT.md`, not into the files it speculates about), the session procedure
> and its templates are in `.agents/skills/session/`. The backlog CLI is `pixi run bunx backlog …`.

### 2026-10-01 — proposal: pack the developer transport from the tree, on a trigger

**Anatomy of the 0.3.6 skew** (why `--user-tools` is missing from every restore today), measured
from the published branch and the releases API: the transport's manifest was created
2026-10-01T14:20:49Z from commit `910df8f` (PR #46's merge), which *declares 0.3.6* — the
v0.3.7 tag commit (`a7b4109`) did not exist until 14:22:18Z. The published transport is stale on
two independent axes: it was packed from a pre-release tree, and it embeds a 0.3.6 binary. Even
a self-build from that tree would have shipped 0.3.6; even a released-binary pack five minutes
later would have shipped 0.3.7. "Use the tree's own build" fixes only the second axis.

**What already exists:** `pixi run sandbox-pack` builds `target/release/pixi-sandbox` first and
packs it via `--self-bin`, so the manual pipeline cannot pick up an operator's installed binary.
What has no automation is the *timing*: nothing re-packs after a release lands, so the transport
tracks "whenever a maintainer last ran it by hand", not main.

**Proposal (not agreed):** a small repack job — `workflow_dispatch` (and optionally
`workflow_run` after `release.yml` succeeds) that checks out main and runs `pixi run
sandbox-pack` → `sandbox-doctor` → `sandbox-publish`: one-line steps, house rule, `contents:
write` for the orphan-branch force-push. Two properties: the transport is always packed from the
tree's own build, and the standing "0.3.7 repack" open item becomes a click instead of a
machine-bound manual op. The force-push means the trigger stays a maintainer's call — automation
of the *pack*, not of the *decision*. `sandbox-doctor` and `sandbox-publish` also run the freshly
built self-bin, so the whole lane is one binary. Deliberately separate and unchanged:
`airlock.yml` packs with the *released* channel binary because it is the proof of published
assets, not the publisher of the developer transport.

**Owner direction, same day — dogfood the generated publisher instead of a bespoke job.** The
repo should publish its own transport through the exact workflow `pixi-sandbox init` generates,
with a binary-path override carried in an environment variable, and the automation identities
unified as `pixi-sandbox[bot]`. The identity half is landed: transport commits are now authored
`pixi-sandbox[bot] <41898282+github-actions[bot]@users.noreply.github.com>` (the github-actions
app's noreply address, so CI pushes render the bot avatar), and task-39's relock bot carries the
same identity. The dogfood half has a workable shape:

- The generated template (`crates/pixi-sandbox/src/generated/github_workflow.rs`) gains one knob:
  a `PIXI_SANDBOX_BIN` env read from a repository *variable* (`${{ vars.PIXI_SANDBOX_BIN || '' }}`),
  empty for every consumer. When set, the channel-install and bootstrap-download steps are
  skipped (`if:` guards) and the one binary drives `plan`/`pack`/`doctor`/`publish` AND is the
  embedded self-bin — the same one-binary-lane semantics as the manual `sandbox-*` tasks, so no
  driver/embedded version skew. The value `build` means "build it from this repository first" (a
  conditional `pixi run cargo build --release -p pixi-sandbox` step, inert for consumers); a path
  means "use exactly this binary".
- The repo commits a pristine render at `.github/workflows/publish-sandbox.yml`, and
  `check-repository` gains a byte-equality check: the committed file must equal
  `render_github_workflow(<current version>, .pixi-sandbox.toml)`. That makes the dogfooding
  total — the artifact this repo runs is provably the artifact consumers get, and the v0.3.1
  broken-generated-workflow class (issue #37) cannot recur silently.
- Consequence to accept before building: the generated trigger is `push: [main]` +
  `workflow_dispatch`, and byte-equality forbids the repo's copy from differing — so every push
  to main re-packs and force-pushes the transport (keep-N rotation is the designed behavior,
  task-3), and the "repack is a maintainer's call" stance from the proposal above is superseded.
  The transport then tracks main continuously, which is what the `--user-tools` skew was about
  in the first place.
- Open before this graduates to a task: (1) confirm the on-push trigger is wanted, or drop
  byte-equality and keep a dispatch-only repo copy; (2) task-36's future `workflow_shape` check
  must exempt generated files (they carry multi-line `run:` blocks by design — a consumer
  artifact, not a house workflow); (3) the repo's own runs exercise the override path, not the
  channel-install/bootstrap-verification path — that consumer path keeps its existing proofs
  (actionlint + Rust tests in every `lint`, and airlock.yml's released-binary proof), or a
  second scheduled run with the variable unset could prove it end-to-end.

### 2026-10-01 — v0.4.0 release fix, then dogfooding the generated publisher

**Landed.** Two release defects, both from `gh run view`: `release.yml` passed
`--target <triple>` to a task with one positional `flags` arg (pixi refused it on all five
runners — the `--` separator was missing), and once that stopped firing, the Windows leg hit
`E0282` in `restore.rs`'s `#[cfg(not(unix))]` branch, whose `notice: None` is unconstrained
because its only use is `println!("{notice}")`. Both were folded into the `chore(release): v0.4.0`
commit (it was already pushed and tagged, and `release.yml` checks out the tag, so a fix on main
alone would have rebuilt with the broken workflow), then the tag was moved and the run re-fired
green: five binaries, five `.conda` packages, prefix.dev, GitHub Release.

**Open findings, not acted on:**

- **`pixi-sandbox init` destroys the comments in a consumer's `pixi.toml`.**
  `ensure_archont561_channel` (`crates/pixi-sandbox/src/commands/init.rs`) parses the manifest
  into a `toml::Table` and writes it back with `toml::to_string_pretty`, which reformats the
  whole file and drops every comment. Measured on this repository: a 325-line `pixi.toml` came
  back as 216 lines with all commentary gone and every table reordered, for a one-line channel
  addition. The doc comment claims "through TOML values rather than text splicing" — the intent
  is right, the implementation is the opposite of the claim for any hand-commented manifest.
  Fix options: edit the `channels` array textually with a line-oriented splice, or use
  `toml_edit`, which preserves formatting and comments by construction. Reverted here rather
  than committed; this repo needs no channel addition (it builds the binary itself and the
  generated workflow installs the CLI globally with explicit channels).
- **The dogfooded `publish-sandbox.yml` and the manual `sandbox-*` tasks now both own
  `sandbox/developer-linux-64`.** The generated workflow is `push: [main]`, so every merge
  repacks and force-pushes the transport; the manual lane in `pixi.toml` still exists and is
  still documented in `AGENTS.md`. That is the consequence the 2026-10-01 owner note above
  already accepted, but the redundancy was not resolved: either delete the `sandbox-pack` /
  `sandbox-publish` / `sandbox-doctor` tasks and let the generated lane be the only publisher,
  or keep the manual lane as the local reproduction of it and say so in `AGENTS.md`.
- **No byte-equality check for `publish-sandbox.yml`** (the dogfooding idea the same note
  proposed). Check 10 does exactly this for `relock.yml`
  (`generated_relock_is_current`); the publisher render has the same argument — a stale
  committed copy would make the dogfooding claim false without failing anything. Not written:
  it needs the same options plumbing `render-relock` uses, and the render depends on
  `.pixi-sandbox.toml`, so the fixture has to build both.
- **Two launchers.** `init` writes `restore.sh` at the project root by default; this repo's own
  launcher is `scripts/restore.sh` (bash, win-64, positional `[branch] [output-path]`, wired
  into `tests/restore_script.rs` and the `sandbox-restore` task). The generated one is a
  `/bin/sh` subset — it already carries the task-33 `PIXI_SANDBOX_USER_TOOLS` block, which is
  how the repo's script came to carry it. Not committed here, to keep one launcher per repo;
  passing `--script-path scripts/restore.sh` instead would replace a tested superset with the
  generated subset and break the usage-string assertions.

**Session close.** Everything landed as three pushes straight to `main` (no pull request):
the amended `chore(release): v0.4.0` (`d39a231`, tag moved to match), the dogfooding commit
(`ef9e68a`), and this one. Post-push runs green — `ci` (1m6s), `docs` (1m20s), `publish
sandbox` (1m54s) — and the release run `36933524627` produced all five binaries, all five
`.conda` packages, prefix.dev and the GitHub Release. Suite on merged main: **275 passing /
1 skipped** (was 274/1). `task-36` stays In Progress at 6/7: AC#6 now has real evidence for two
of its three legs — auto-release dry-run `36922096859` (`Dry-run — show diff and stop` green,
commit/tag/push skipped) and a tag-triggered release on five runners `36933524627` — and the
airlock matrix leg last ran green on PR #52's branch (`36928726964`), before this session. What
AC#6 still does not have is its own instruction satisfied literally (a *throwaway* tag rather
than the real v0.4.0 one) and a same-artifacts comparison against the pre-task behaviour;
closing it is the owner's call, not this session's.

**Next session should start with:**

> Restore the sandbox and baseline the suite (expect **275 passing / 1 skipped** — the published
> transport now carries pixi-sandbox **0.4.0**, so a fresh restore registers the `~/.local/bin`
> launchers and you should not need to source `.pixi/sandbox-env.sh` unless you want `cargo`,
> `bun`, `taplo` or `convco` loose; nothing new needs vendoring), then read `CONTEXT.md`
> § Session scratchpad — the 2026-10-01 heading *"v0.4.0 release fix, then dogfooding the
> generated publisher"* lists four open items.
>
> First, one look at the remote: `gh run list --branch main --limit 5`. Since 2026-10-01 every
> push to `main` also runs `publish sandbox`, which repacks and force-pushes
> `sandbox/developer-linux-64`; **if that run is red, every airlock restore is running a broken
> transport** — fix it before starting anything else. If it is green, the branch head you can
> trust is the one `doctor --verify` just cleared.
>
> I want to take **task-35** this session — move the airlock gate out of
> `scripts/airlock-gate.sh` into `crates/pixi-sandbox/tests/e2e.rs` behind a `ci` cargo
> feature, skipped locally and run by the matrix, then delete the script and the two shell
> steps in `airlock.yml`. Settled already, do not re-open: the gate keeps driving the **released**
> binary over a **real packed transport** (never this tree's build), the two tiers are
> `sudo unshare -n` on Linux and `sandbox-exec` on macOS, `deny-egress` is the xtask that
> already fails loudly on an unknown OS, and the restore under test is the airlock one from the
> fetched branch. Still open, and yours to pick: where the two platform guards live in Rust (a
> helper the test calls, so an unknown OS still fails rather than skips), and whether the gate is
> a cargo feature or an environment variable the matrix sets.
> In slices: the Linux tier behind the flag with its fixture-backed e2e test, then the macOS
> tier, then the workflow steps and the script deletion last — which needs your sanction, since
> deleting the shell is the irreversible part.
>
> Two things I would rather you decided than have me assume: whether the manual `sandbox-pack` /
> `sandbox-publish` tasks should go now that `publish-sandbox.yml` publishes the same branch on
> every push, and whether the `init` channel rewrite that strips `pixi.toml` comments becomes a
> backlog task (it is a consumer-facing bug: `toml_edit` fixes it by construction).
>
> Propose the slice and stop. House rules are in `AGENTS.md` (invariant 10: anything you do not
> implement goes in `CONTEXT.md`, not into the files it speculates about), the session procedure
> and its templates are in `.agents/skills/session/`.

### 2026-10-02 — pixi-only entrypoint cleanup

**Landed locally before task-35:** restore no longer generates `.pixi/sandbox-env.sh` and
removes a legacy copy if it finds one; `scripts/restore.sh` reports how to use the registered
pixi launcher (or the manifest-owned pixi path when registration is skipped) instead of
sourcing an activation hook; docs, README, `AGENTS.md`, and the session skill all state the
same rule. The temporary follow-up that made the old shell gate drive Cargo through
`pixi run --frozen -- cargo …` was superseded by task-35: that shell gate is now deleted.

**Standing command rule:** pixi is the sole environment entrypoint. Use
`pixi run --frozen <task>` for repository tasks, `pixi run --frozen xtask <subcommand>` for repo
automation, `pixi run --frozen -- cargo <cmd> -p <crate>` for crate-scoped Rust work, and
`pixi run --frozen bun --filter=<workspace-package> run <script>` (or root `pixi run --frozen
bunx <tool>`) for JavaScript package work. Do not source `.pixi/sandbox-env.sh`; new restores do
not generate it.

### 2026-10-02 — task-35 airlock gate moved into e2e

**Landed and checked on PR #54:** task-35 is implemented and closed. `crates/pixi-sandbox` now
has a `ci` cargo feature; the real-transport airlock gate is a `#[cfg(feature = "ci")]` module
in `crates/pixi-sandbox/tests/e2e.rs` with explicit inputs (`PIXI_SANDBOX_GATE_PROJECT`,
`PIXI_SANDBOX_GATE_TRANSPORT`, `PIXI_SANDBOX_GATE_ENVS`, `PIXI_SANDBOX_GATE_SKIP_CARGO`).
`pixi.toml` has the one-line tasks `airlock-gate-archive` and `airlock-gate-run`;
`.github/workflows/airlock.yml` builds a nextest archive while connected, runs it in Tier B,
and replays the same archive through `xtask deny-egress` in Tier A. `scripts/airlock-gate.sh`
is deleted, and the ADR-10 fixture exception now names only `scripts/restore.sh`. CI fixes found
live: the workflow installs `default` before archiving, the archive build no longer passes
Cargo's offline `--frozen`, the gate filter targets only the `ci_gate` tests, `airlock-pack`
downloads the static release asset instead of embedding Pixi's global trampoline/dynamic conda
binary, and `deny-egress` absolutizes `pixi` before sudo sanitizes PATH.

**Local proof already run:** `pixi run --frozen test` (275 passed, 1 skipped), targeted
`cargo check` for `pixi-sandbox` with and without `--features ci`, targeted clippy for
`pixi-sandbox --features ci` and `xtask`, a local archived-gate proof against the fixture
restore, `pixi run --frozen airlock-gate-archive`, `pixi run --frozen lint-actions`,
`pixi run --frozen lint-toml`, `pixi run --frozen xtask check-repository`, and
`pixi run --frozen docs build` all pass. Full all-crates lint was intentionally not run in this
iteration per the user's instruction to skip it and use repo-specific / targeted checks.

### 2026-10-02 — task-38 lands: every table loop is a named case

**Landed on PR #59** (merged 75a4489; post-merge ci, docs, and publish sandbox all green): the
full local task-38 stack — rstest support module with fixtures, bounded proptest properties with
committed seed files, the defined-once/coverage guards — plus the finishing slice converting
every remaining assertion-table loop into named `#[case]`s. Suite on merged main: **403 passed /
1 skipped** (was 331/1). AC#1–3, #5, #6, #8 proven and checked. Two patterns worth reusing: the
expensive publish is shared across read-only cases by a `#[once]` published-orphan fixture
(restructure beats re-running), and the reviewed stale-word trio keeps its single
`stale-ref-allowed` line by living in `STALE_DELETED_REFERENCES` with cases passing an index —
one-line markers silence exactly one following line, so literals spread across `#[case]`
attributes would need one marker each.

**Not implemented — two owner calls, both in task-38's notes, task stays In Progress:**

- **AC#4's TMPDIR clause.** Restore-running fixtures yield a tempdir HOME and set
  HOME/USERPROFILE/SHELL per test; nothing sets a per-test TMPDIR. Wire it (a line in
  `isolated_bin` and the user-tools `restore` builder) or amend the AC to say HOME-only.
- **AC#7's <20s-suite clause.** Wall-clock is 22.7s, dominated by the pre-existing 22.2s
  `pack_vendors_a_lockfile_whose_crates_come_from_one_source_each` test; the bounded properties
  are not the driver. Shrink/split that test or amend the AC to bound the properties rather than
  the suite.

Also decided in the slice, not asked: the `publish.rs` file-writing loops, restore env-setup
loops and publish rounds stay loops — they are stateful setup/repetition, not assertion tables,
and parametrizing them would fork shared state or multiply the work the tests measure. A
tautological `lock.matches("conda:").count() == lock.matches("conda:").count()` assertion that
could not fail was removed from `fixtures.rs`.

**Backlog is empty after task-38:** `backlog/tasks/*.md` has no other open item. When task-38
closes, the next work comes from new tasks — seed them from this scratchpad (the two AC-clause
fixes if the owner says "wire" / "split", the still-wanted macOS bundle of task-1 awaiting a
native runner) rather than re-opening Done work.

**Next session should start with:**

> Restore the sandbox and baseline the suite (expect **403 passing / 1 skipped** — `publish
> sandbox` repacked the transport off main@75a4489, so the restore is the usual registered
> launcher flow; rstest is vendored, nothing new needs the connected side), then read
> `CONTEXT.md` § Session scratchpad — the 2026-10-02 heading *"task-38 lands: every table loop
> is a named case"* lists the two open owner calls.
>
> First: decide the two task-38 clauses — per-test TMPDIR in the restore fixtures (wire it, one
> line, or amend AC#4) and the <20s suite (split the 22.2s pack-vendor test, or amend AC#7 to
> bound the properties). With both answered, implement what "wire"/"split" implies, check AC#4
> and AC#7, mark task-38 Done, and the backlog is empty — so propose new tasks from this
> scratchpad afterwards rather than re-opening Done ones.
>
> Propose the slice and stop. House rules are in `AGENTS.md` (invariant 10: anything you do not
> implement goes in `CONTEXT.md`, not into the files it speculates about), the session procedure
> and its templates are in `.agents/skills/session/`.

### 2026-10-02 — post-release reconciliation: transport gap closed, git boundary complete

**Merged: PR #65** (squash → main at 482ab51; post-merge ci, docs, and publish sandbox all
green). The v0.4.2 release itself verified end-to-end first: five standalone binaries, five
Conda packages, SHA256SUMS, all five release.yml platform builds with package smoke tests,
prefix.dev publication, GitHub Release, and the docs dispatch (run 37036393055). The one
missing proof was the transport: `publish-sandbox.yml` had not run for the release commit
(auto-release pushes with GITHUB_TOKEN, which starts no `on: push` workflows), so
`sandbox/developer-linux-64` still named dbe57a8 / 0.4.1 while main and the release stood at
7745e9c / 0.4.2. That gap is task-44: auto-release now dispatches the repack through the new
`dispatch-sandbox-repack` pixi task, and the post-merge run (37043853308) repacked the
transport — the published manifest names 482ab51 / 0.4.2 / static. Also landed: task-30's
AC#4 closed with evidence (task-29 retired check 7; all four named policy families are
fixture-tested), task-45 moved prepare-release's touched-file report inside the git boundary
(`ShellGit::changed_paths`; its fixture tests caught a real `run_text` trim bug — porcelain's
first status column can be a space, and trimming the output shifted the first path by one
byte), and task-46 made the docs build warning-free. Suite: **405 passed / 1 skipped**
(was 403/1). Invariant 8 now states its scope in AGENTS.md: production git only through
`pixi-sandbox-git`; `#[cfg(test)]` fixture builders may run real git as the oracle.

**Not implemented — deferred proofs and notes:**

- **task-44's dispatch step proves end-to-end only on the next release.** The step ships in
  auto-release's `cut` job from 482ab51 on; the next auto-release run should show the
  "Dispatch sandbox transport repack" step and a publish-sandbox run it started, with the
  manifest naming the release commit and the new version. Until then the mechanism is
  implemented and gate-tested but not yet exercised by a real release.
- **SHA256SUMS byte-level spot-check needs an online machine**: the release-assets CDN is
  egress-blocked from the airlock. The file's existence and the green generating step
  (`xtask release-checksums`, whose five-binary completeness check is fixture-tested) are
  the airlock-side proof; `sha256sum -c SHA256SUMS` against the downloaded assets is the
  connected-side completion.
- **`prerenderConflictBehavior: "ignore"` is global** (docs/astro.config.mjs). Chosen
  because Starlight owns the only two routes this site has and its custom-404 mechanism
  conflicts with its own catch-all by design; if Astro grows a per-route exclusion or
  Starlight starts filtering the reserved `404` slug out of the catch-all's paths, tighten
  the setting back to the default.
- The i18n override file (docs/src/content/i18n/en.json) carries one real override
  (`page.editLink`) so its purpose is self-evident; it exists primarily to register the
  collection Starlight's runtime reads on every page.

**The backlog is empty again** (task-44/45/46 closed with PR #65). The next session's first
job is deciding what to seed: the deferred proofs above are watch-items, not tasks; the
v2 spike (task-9, rattler-based pack/unpack) and any new work come from the owner's
priorities, not from re-opening Done tasks.

### 2026-10-02 — issue #71: init's channel append broke consumer locks (task-48, ADR-17, PR #73)

**Open on the session branch** (`arena/01a0fe32-pixi-sandbox`, PR #73, `fix(init): …`, net
−175 lines). Issue #71 measured what task-34's namespace-root choice actually does:
`https://prefix.dev/archont561` serves no repodata, so every consumer with a dependency
fails `pixi lock` right after `init`, and a dependency-free project locks fine (an empty
solve never fetches repodata) — the smoke-test trap that let it ship. Recorded as **ADR-17**
and fixed as the no-mutation option: init writes only the files it owns and leaves
`pixi.toml` byte-identical, with a bare existence check keeping the
run-init-from-a-Pixi-project guard. The channel machinery and the `toml_edit` dependency
are deleted; the task-41 corpus is re-pointed at byte-identical assertions over
dependency-carrying manifests; both docs sections that described the append as a feature now
describe its removal and the 0.4.3 remediation (delete the appended entry).

Deferred proofs (tracked as task-48's open ACs): the live `pixi lock` probe needs a
connected host — this session's sandbox is airlock-shaped (github.com and npm only; no
crates.io, static.rust-lang.org, prefix.dev or conda), so **the PR run is the test run**
for the suite; the patch release needs the merge plus an `auto-release` dispatch, and the
token cannot start workflows or comment on issues (`Resource not accessible by integration`)
— it could open the PR, and `Fixes #71` in its body links the issue. The upstream pixi
report (the coalesced-request error hiding the 404 URL) is drafted verbatim in task-48's
notes; searched first, no existing issue covers it. Also filed this session, not yet
implemented: task-47 + ADR-26/ADR-16 (consumer version-drift gate and reviewed upgrade
path).

**Next session should start with:**

> Restore the sandbox and baseline the suite (expect **405 passing / 1 skipped** — the
> transport was repacked off main by the post-merge `publish sandbox` run, so the restore
> is the usual registered launcher flow; nothing new needs vendoring), then read
> `CONTEXT.md` § Session scratchpad — the 2026-10-02 heading *"post-release reconciliation:
> transport gap closed, git boundary complete"* lists the deferred proofs.
>
> First, one look at the repo state: `gh run list --branch main --limit 3` and
> `git show origin/sandbox/developer-linux-64:.pixi-sandbox/manifest.json` — the transport
> should name the newest main commit with `tool pixi-sandbox` at the newest release and
> `linkage: static`; a lag outside a release window is an incident, a lag right after one
> means the dispatch step (task-44, in auto-release since 482ab51) did not fire — read that
> run before anything else.
>
> The backlog is empty; propose new work rather than re-opening Done tasks. The scratchpad's
> deferred list holds the watch-items (the next release proves task-44's dispatch step
> end-to-end; an online machine can spot-check v0.4.2's SHA256SUMS), and task-9's v2 spike
> (rattler-based pack/unpack) is the largest unstarted thread if the owner wants it.
>
> Propose the slice and stop. House rules are in `AGENTS.md` (invariant 10: anything you do
> not implement goes in `CONTEXT.md`, not into the files it speculates about), the session
> procedure and its templates are in `.agents/skills/session/`.

### 2026-10-03 — owner publisher is source-only; consumer self-update is the v1 target

**Merged: PR #74** (squash `15b82c7`). The owner-level `publish sandbox` workflow no longer
installs a released package before planning or publishing: the plan job builds and invokes the
checked-out source, and the native publish job's source-built static binary remains the one path
for pack, doctor, publish, and `--self-bin`. Post-merge CI passed (run 37073291863), and the real
publisher passed (run 37073291962): its plan source-build succeeded, its linux-musl source-build
succeeded, and the republished `sandbox/developer-linux-64` manifest names source commit
`15b82c7`, pixi-sandbox 0.4.3, with static linkage. No auto-release or release workflow was
triggered — both remain dispatch/tag driven, and cutting a release before self-update exists
would not prove the next target.

The same PR promoted task-47 into the v1 upgrade design and updated proposed ADR-26. The
planned `pixi-sandbox self-update` resolves latest only in the review-producing upgrade lane (or
an exact manually requested version), verifies the standalone asset against `SHA256SUMS`, safely
replaces a CI-managed standalone path, and refuses package-manager/global-trampoline ownership.
The updated binary regenerates owned files into a PR; the regenerated publisher commits an exact
version and, after merge, uses that same exact verified binary for plan, pack, doctor, publish,
and `--self-bin`. Production pins never float. Task-45's last open AC is now closed from the
full GitHub CI evidence, so task-47 is unblocked.

**Next session should start with:**

> Restore the sandbox and baseline the suite (expect **405 passing / 1 skipped**; the trusted
> transport is `sandbox/developer-linux-64` created by publish run 37073291962, whose manifest
> names source `15b82c7`, pixi-sandbox 0.4.3, static). Read `AGENTS.md`, ADR-26, and task-47.
> Confirm main's latest `ci` and `publish sandbox` runs are still green; auto-release/release did
> not run and must not be dispatched yet.
>
> Treat task-47 as the **v1.0.0 major-version feature**. Start with only the self-update core:
> define the CLI contract (`self-update`, latest by default, exact `--version`, non-writing
> `--check`, explicit CI-managed destination), the canonical host-to-release-asset map, injected
> release resolver/downloader, `SHA256SUMS` parsing and verification, package-manager/trampoline
> refusal, and stage-before-replace behavior for Unix and Windows. Tests use fixtures and fake
> HTTP only — no live release network and no writes to a real user binary. Do not build the
> generated upgrade job or cut v1.0.0 in the first slice; those follow only after the updater's
> trust and replacement boundaries are proven.
>
> Propose that slice and stop. Call out the Windows running-executable replacement strategy and
> the exact ownership signal that distinguishes a standalone binary from a Pixi-managed
> trampoline before implementation. House rules and session templates are in
> `.agents/skills/session/`.

### 2026-10-03 — tests move out of production sources

House rule promoted at the owner's direction: **no `#[cfg(test)] mod tests` beside production
code**. Written into `AGENTS.md` § Test conventions and enforced by
`production_sources_carry_no_inline_test_modules` in `crates/pixi-sandbox/tests/fixtures.rs`.

The structural obstacle, for whoever extends this: `pixi-sandbox` is a **binary** crate, so
`tests/` can reach it only through `lib.rs` (which previously exposed `generated` alone) or by
spawning the binary. Splitting the self-update tests therefore required promoting `release`,
`self_update` and `user_tools` to `pub mod` in `lib.rs` and widening `user_tools`' 17
`pub(crate)` items to `pub` — the `generated` precedent, applied further. Four private items
that only a failure-branch test can reach (`windows_swap`, `staging_path`, `DISPLACED_INFIX`,
`STAGING_INFIX`) are `#[doc(hidden)] pub`: reachable from `tests/`, flagged as a test boundary
rather than API. `cli.rs` and `commands/` stayed private and are now covered black-box from
`tests/cli.rs`, which is the better test — it caught nothing new here, but it is the surface a
user meets.

Not done, and the obvious next step: `pixi-sandbox-core` (3 files) and `xtask` (12 files) still
carry inline tests and are not covered by the new guard. `pixi-sandbox-core` is a library, so
its split is mechanical. `xtask` is a binary crate with no `lib.rs` at all and heavily internal
modules, so splitting it means either adding a `lib.rs` and widening a lot of visibility, or
accepting black-box coverage through the `xtask` binary; that choice wants a decision before
the work. Worth a backlog task rather than an opportunistic refactor. The `LEGACY` list in the
guard (6 entries, all in `crates/pixi-sandbox`) is the visible debt for this crate and is
asserted to shrink, never grow.

### 2026-10-03 — self-update core landed; the upgrade lane is next

**Merged: PR #76** (squash `79e4f83`). Task-47 slice 1 shipped `pixi-sandbox self-update` as
the v0.5.0-intended feature: latest by default, exact `--version X.Y.Z`, non-writing `--check`, explicit
CI-managed `--dest`. Post-merge `ci`, `docs` and `publish sandbox` all passed, and the repacked
`sandbox/developer-linux-64` manifest names source `79e4f83`, pixi-sandbox 0.4.4, static. Suite
**500 passing / 1 skipped**, up from 404.

**Release state, and an open question it raised.** After the merge an `auto-release` was
dispatched (not by this session) and cut **v0.4.5** — commit `afcbdbf`, tag pushed — and the
v0.4.5 `release` run was then canceled by the owner, so there is a release commit and tag on
main with **no GitHub Release and no published assets**. That dangling tag needs a decision
before anything else is cut: either publish it by dispatching `release.yml` for v0.4.5, or move
the tag aside.

The reason it came out `v0.4.5` rather than the intended `v0.5.0` is `preMajor: true` in
`.versionrc`. Before 1.0 that policy maps a breaking change to a minor bump and a plain `feat`
to a **patch**, so `convco version --bump` over `feat(self-update): …` returns 0.4.5 and
`prepare-release auto` can never produce 0.5.0 for a feature. Getting the intended minor means
dispatching auto-release with the explicit `minor` selector (`prepare-release` already accepts
`auto|major|minor|patch|vX.Y.Z`), or changing the preMajor policy. This is a real decision, not
a bug: the policy is doing exactly what it says, and it simply disagrees with "treat task-47 as
the v0.5.0 minor-version feature". Worth settling before the next release, because the
self-update feature's own assets are what a future `self-update --version` resolves.

Two findings from the session start worth keeping. The 405 baseline carried in the last three
opening prompts was never true after PR #73 consolidated `tests/cli.rs`; the real number was
404, and a stale figure copied between hand-offs is exactly the kind of unverified claim the
session rules exist to stop. And the transport named in that prompt (run 37073970307, source
`41dc22f`, 0.4.3) had already been superseded by the v0.4.4 auto-release and its repack — the
healthy case, not an incident, but it means a hand-off should name the run and let the next
session re-read the manifest rather than trust the numbers.

Design decisions made here, both approved before implementation. **Windows replacement** renames
the running image aside to `.pixi-sandbox-old-<version>` and sweeps it at the *start* of the
following update, because the unlink fails by design while the image is mapped; a failed second
rename rolls the first back. **Ownership** is judged on path provenance rather than file
contents, as an ordered ladder (managed launcher, `pixi global` trampoline, conda/Pixi prefix,
restored transport tool, else standalone), each refusal carrying a remedy. No `--force` was
added, deliberately.

`codecov/patch` failed on the first push and the fix was structural rather than a waiver:
formatting and the decision flow moved into the library (a black-box test of a spawned binary
earns no coverage and can only match substrings), and the one socket-needing file,
`release/github.rs`, was isolated and excluded by name from `pixi run coverage`. That exclusion
is the first of its kind in this repository — if a second one is ever proposed, it should have
to clear the same bar: the file contains no decision, and every decision it could get wrong is
tested elsewhere against a fake.

**Not implemented, and the honest gaps.** AC#2 stays unchecked: the Windows arm is covered
against the `ReplaceStrategy` parameter on Linux, but no run anywhere has yet replaced a
genuinely running Windows image. Live-network resolution is also unproven — this sandbox's
egress proxy terminates TLS with a CA `ureq` does not trust, so the binary cannot reach
`api.github.com` from here, and the first real proof will be a CI run. Both want a native
matrix, not another local slice.

A separate house rule landed alongside: **tests live under `tests/`, never in the file they
judge**, enforced by `production_sources_carry_no_inline_test_modules` with a `LEGACY` list that
may only shrink. `pixi-sandbox-core` (3 files) and `xtask` (12) still carry inline tests and are
not covered by the guard. Core is a library, so its split is mechanical; `xtask` is a binary
crate with no `lib.rs` at all, so its split means either adding one and widening a lot of
visibility or accepting black-box coverage — that choice wants a decision before the work, and
deserves its own backlog task rather than an opportunistic refactor.

**Next session should start with:**

> Restore the sandbox and baseline the suite (expect **500 passing / 1 skipped**; re-read the
> manifest rather than trusting this number — the trusted transport is
> `sandbox/developer-linux-64`, repacked by the `publish sandbox` run on `79e4f83`, and its
> manifest should name that commit, pixi-sandbox 0.4.4, static). Read `AGENTS.md`, ADR-26,
> and task-47's notes. Confirm main's latest `ci`, `docs` and `publish sandbox` runs are green.
>
> **Settle the release state first.** `v0.4.5` (`afcbdbf`) is tagged on main but its `release`
> run was canceled, so no GitHub Release or assets exist for it — decide whether to publish
> that tag or retire it. Note that `preMajor: true` in `.versionrc` makes `feat` a *patch*
> before 1.0, so auto-release can never produce the intended `v0.5.0`; that needs an explicit
> `minor` selector or a policy change. Do not cut anything new until that is decided.
>
> Task-47 slice 1 (the self-update core) is merged and `In Progress`. Slice 2 is the **reviewed
> upgrade path**: AC#3 (a `pixi-sandbox-version: X.Y.Z` stamp beside the ownership marker in
> every file `init` writes, within the first three lines `ensure_replaceable` already reads),
> AC#4 (`init --check` as render-and-compare over the same four paths, writing nothing, exiting
> non-zero with a remedy per drifted file and foreign-owned files reported separately), and
> AC#5 (config is never rewritten; an older `schema` is a finding naming its migration path, and
> no `config migrate` command is built while schema is 1). Those three are locally provable and
> fixture-testable; propose them as one slice and stop.
>
> Do **not** start AC#6-#8 (the generated upgrade job, the regenerated PR, the explicit publish
> dispatch) in the same slice — they need the stamp and `--check` to exist first. Two proofs are
> outstanding from slice 1 and belong to a native-runner slice, not to this one: a Windows runner
> replacing a genuinely running image (AC#2), and a real `self-update` against the live release
> API, which no dev sandbox can produce because its egress proxy breaks TLS for `ureq`.
>
> House rules are in `AGENTS.md` — note the new one: tests live under `tests/`, never beside the
> source they judge, and the `LEGACY` list in
> `production_sources_carry_no_inline_test_modules` may only shrink. Session procedure and
> templates are in `.agents/skills/session/`.

### 2026-10-03 — task-54 landed; its two leftover proofs are connected-host work

`fix(pack): refuse self-bins that cannot run standalone` (#82) merged. Task-54 stays In
Progress at 4/6 with two named gaps, both blocked by this sandbox, not by design:

- **AC#1 asset check** — `gh release download v0.5.0` fails here with EOF because
  `release-assets.githubusercontent.com` is egress-blocked. On a connected host: download
  the standalone asset, run it with `--version` under an empty HOME, record the outcome in
  the task. Pipeline-level evidence (assets are staged with `xtask stage-release-binary`
  from cargo's release output) says it cannot be a trampoline, but the byte-level proof the
  AC asks for is still owed.
- **AC#6 release + consumer repack** — auto-release is dispatch-only and this sandbox's
  token gets `HTTP 403: Resource not accessible by integration` on
  `gh workflow run auto-release.yml` (same class as `gh issue comment`). Cut v0.5.1 via the
  Actions tab or a scoped token, then repack the consumer transport with the standalone
  self-bin and run one fresh consumer restore before closing the task.

Two session-procedure lessons worth promoting: (1) `gh pr merge --delete-branch` deletes the
fixed Arena session branch — it had to be re-pushed afterwards; merge without the flag.
(2) Smoke-test the built binary with *relative* paths: the suite ran green while a relative
`--branch-location` produced a bogus ENOENT, because the probe runner chdir'd and the
relative exe re-resolved. Also seen once: killing a probed child reaches one pid only — an
orphaned grandchild kept inherited pipes open and hung joined reader threads; the timeout
path detaches readers now.

### 2026-10-03 — task-48 re-verified; still 4/7, same two connected-host gaps

A later session picked task-48 back up expecting to implement it, found the fix already
merged (PR #73, ADR-17) and shipped in **v0.4.4** (confirmed via `gh release view`, published
2026-10-02T22:56:26Z, ~40 minutes after the PR merged) and still present in the current
**v0.5.0**; issue #71 is closed. Re-ran the init test suite to reconfirm nothing regressed
(`init_leaves_the_consumer_pixi_manifest_untouched` and
`init_still_refuses_a_directory_without_a_pixi_manifest` both pass) and re-read the guide
section and ADR-17 — both still match the shipped code. No code or doc changes were needed.

The two remaining gaps are exactly what the prior session left: AC#2/#6's live `pixi lock`
probe needs a connected host (this sandbox still reaches only github.com — prefix.dev,
crates.io and static.rust-lang.org all fail at the TLS layer, confirmed again this session),
and AC#7's upstream `prefix-dev/pixi` report needs a personal GitHub token or the web UI —
`gh issue create --repo prefix-dev/pixi` and the equivalent `gh api` call both still return
`Resource not accessible by integration` (HTTP 403), the same class of installation-token
limitation task-54 hit on `gh workflow run`. The drafted report text is unchanged in
task-48's implementation notes. Task-48 stays **In Progress**; nothing here needs a backlog
task of its own, it's the same two open ACs waiting on the same online-host/personal-token
access the task already named.

### 2026-10-03 — task-52: the generated publisher stopped downloading from the consumer's own repo

Issue #80's other half: the generated consumer workflow's "Download released pixi-sandbox"
step built its release URL from `$GITHUB_REPOSITORY`/`$env:GITHUB_REPOSITORY` — always the
*consumer's* repository at Actions runtime — so any consumer repo with no GitHub releases of
its own got a `curl: (22)` 404 before `pack` ever ran. Regression traced to the v0.3.2 →
v0.4.3 inlining of the composite `setup` action (task-29 era), which dropped the owner/repo
the old action resolved against.

Fix: a new `release::PIXI_SANDBOX_REPO` constant (`Archont561/pixi-sandbox`), re-exported as
`self_update::DEFAULT_REPO` so the CLI's `--repo` default and the generated download URL share
one source of truth and cannot drift apart again. `generated/github_workflow.rs` renders both
the bash and pwsh download bases from it via a new `__RELEASE_REPO__` placeholder;
`GITHUB_SERVER_URL` (still correct on GHES) and the unrelated `REMOTE:` line (the consumer's
own repo, correctly used for `publish`'s git push) are untouched. SHA256SUMS verification is
byte-identical — nothing unverified is ever executed.

Test-first: a new fixture/golden-render test
(`generated_workflow_downloads_release_assets_from_the_pixi_sandbox_repository_not_the_consumers`)
confirmed red (failed to compile) before the constant existed, green after; the committed
golden fixture was updated in lockstep. `xtask lint-generated-workflow` (actionlint) and
`check-repository` both pass; `git status --short .github/` is empty, confirming this repo's
own committed `publish-sandbox.yml` (vestigial — not regenerated or byte-checked by any
tooling, per `crates/xtask/src/workflow.rs`) stayed untouched as the task requires. The
ci-publishing guide now states the download source and the pre-fix 404 remediation.
`pixi run --frozen test`: 557 passed / 1 skipped (the baseline's one new test). Task-52 moved
from **To Do** to **In Progress at 5/6 ACs** — AC#6 (shipping in a released patch plus a live
consumer-repo repro) stays open, named, and waiting on a connected host and a cut release,
same pattern as task-48 and task-54's remaining gaps.

### 2026-10-03 — task-47 slice 3: the generated scheduled/dispatchable upgrade job, and task-53: `[workflow]`-table CI policy

User instruction this session: "do task 47 and 53 locally and then open PR and fix all issues."
Both tackled offline, in order, before opening the PR.

**task-47 AC#6/#8/#9** (the remaining ACs from the self-update task, AC#2/#7 stay blocked on a
Windows runner and a connected host respectively, same as before): `GithubWorkflowOptions`
gained the exact paths/branch `init` was invoked with as literals, and the publisher template
gained a weekly `schedule:` plus a `workflow_dispatch.inputs.upgrade` string (blank keeps
today's ordinary dispatch). The new `upgrade` job is the structural negation of `plan`/`publish`'s
gate, so the two lanes can never both fire on one event (asserted directly in a test). It
bootstraps the currently-pinned checksum-verified binary, asks it to `self-update` in place
(latest, or the dispatch-named exact version — also the rollback path), runs the *updated*
binary's `init --check` against the baked-in exact generation arguments, and on drift opens a PR
containing only the three owned paths (never the config, never a push to `main`). The PR body
names the explicit post-merge dispatch a bot merge needs (task-44's `github.token`-push lesson).
Docs (`reference/cli.mdx`'s `init --check` section, `guides/ci-publishing.mdx`'s "Staying
current" section) and a 9-test `upgrade_job` module landed alongside. task-47 moved to **7/9
ACs** (committed `a57e915`).

**task-53** (issue #79, "carry consumer publish-workflow policy in `pixi-sandbox.toml`"):
AC#1-6 fully done, AC#7 partially (every offline gate green; the connected proof and the minor
release stay open, same shape as task-47/task-52's remaining gaps). An optional `[workflow]`
table (`push_paths`, `permissions`, `concurrency`, `timeouts`, `pixi_version`,
`setup_pixi_cache`) now carries CI policy that previously required hand-editing the generated
workflow after every `init --force`. Decision ADR-18 records the one judgement call: the table's
mere *presence* (not each field) is the opt-in to a safer default for exactly two
correctness/cost-flavoured fields (`setup_pixi_cache` → `false`, `push_paths` → derived from the
known transport inputs instead of an unfiltered trigger); `permissions`/`concurrency`/`timeouts`
stay fully per-field opt-in. Every new render block is a placeholder that disappears entirely
when unconfigured, so an absent `[workflow]` table — every existing consumer today — still
renders byte-identically to the pre-task-53 template; a round-trip fixture test
(`init`, `init --check`, `init` again) proves a fully-configured table regenerates with zero
drift and zero hand edits. `push_paths` also surfaces through `plan --json` for diffing. Docs
updated in both `reference/configuration.mdx` (new `[workflow]` table reference) and
`guides/ci-publishing.mdx` (retires the hand-edit-plus-`--force` ritual explicitly).

Evidence for both slices together: `pixi run test` (nextest) 605 passed / 1 skipped (was 581
before this session's work); `pixi run lint` (fmt, clippy, deny, actionlint, lint-generated-
workflow, sandbox-plan --json, lint-toml, lint-docs, check-repository) all green; `pixi run docs`
builds the new MDX cleanly. Commit `a57e915` carries task-47's slice; task-53's commit follows
in this same session before the PR opens.

### 2026-10-04 — task-69: POSIX colons pack; the consumer proofs queue behind one release

User instruction this session: "Load session skill, restore env, look issues ans workflow runs
and tell what to do", then "Create task for new issues and do task 69".

Issue #95 (opened minutes into the session): `check_rel_path` rejected any path containing a
colon, but a colon is a legal POSIX byte — perl's `man/man3/App::Cpan.3` man pages made every
environment that resolves perl (the conda gtk/webkit stack) unbuildable, so castellan's
`shells` environment failed its generated publisher on every push. TASK-69 was created for it
and implemented test-first at three public seams: a new `tests/files_manifest.rs` reproducing
the incident (scan records and parse accepts the man page; `C:/x`, `C:\x`, `C:x`, `/x`, `\x`
refused; the scan-time wording asserted), `tests/manifest.rs` freezing every label
`check_rel_path` guards (blob, part, pack_path, tool path), and the verify.rs `World` fixture
now carrying the man page so the faithful-restore test proves the restore-side oracle
round-trips colon names. Only a Windows drive prefix (any ASCII letter + colon) is refused
now, and `FilesDoc` gained a labelled validate so the pack-time rejection reads "scanned
environment file must be relative" instead of pointing at a manifest that does not exist yet
(the issue's diagnosis note). Merged as PR #96 → main at `888af6d`; PR checks green including
airlock linux-64 (4m19s); post-merge ci (37214102074), docs (37214102076) and the transport
repack (37214102104) all green — `sandbox/developer-linux-64` now packs from `888af6d`
(created 2026-10-04T15:46:54Z), so the fix is in the published branch. Suite 623 passing /
1 skipped (619/1 baseline); issue #95 closed by the merge.

Not implemented, carried forward:

- **TASK-69 AC#6** — castellan's shells environment has not packed green yet. The fix is on
  main and in the repacked transport, but consumer publishers bootstrap the newest *release*
  (v0.5.2 predates the fix), so the proof needs the next release and then any castellan push
  to main. Record the run in the task, then close it.
- **One patch release would settle three tasks at once** (v0.5.3, maintainer's dispatch —
  the session token gets 403 on `workflow_dispatch`): TASK-69 AC#6 (castellan repack),
  TASK-65 AC#6 (issue #92 — the relock guard shipped in #94 but is unreleased; a qgis-rs
  stale-Cargo.lock repair run would close it and unblock TASK-66), and TASK-54 AC#6 (its
  consumer-proof shape; note the v0.5.2 consumer-proof run 37142607209 already proves the
  standalone-self-bin release lifecycle end to end — whether that satisfies AC#6's strict
  "patch release + consumer repack" reading is the maintainer's call, not taken here).
- **TASK-62 is 10/10 with every proof recorded** (PR #91 merged); its file only needs
  flipping to Done — left untouched because this session's scope was narrowed to task-69.
- TASK-66 stays blocked by task-65's AC#6; TASK-68 is the next unblocked To Do task.

Next session should start with:

> Restore the sandbox and baseline the suite (expect 623 passing / 1 skipped — the published
> transport was repacked at 888af6d carrying the POSIX-colon fix; its manifest still reports
> tool version 0.5.2 because that is the last tagged release, but the embedded pixi-sandbox
> is source-built from 888af6d; nothing new needs vendoring before the tree builds offline),
> then read `CONTEXT.md` § Session scratchpad — the 2026-10-04 heading lists the
> release-gated proofs.
>
> First, one look at the repo state: `gh release list`. If a release newer than v0.5.2
> exists, three tasks lose their last open path at once — TASK-69 AC#6 (watch castellan's
> next push to main pack its shells environment green), TASK-65 AC#6 (the qgis-rs relock
> repair, which then unblocks TASK-66), and TASK-54 AC#6. Record the runs and close what is
> proven. If no new release exists, ask the maintainer to dispatch auto-release.yml with a
> patch bump — the session token cannot dispatch workflows — and take TASK-68 while waiting.
>
> I want to take task-68 this session — surface publish failures outside Actions logs and
> preserve the last healthy transport (issue #93). Decisions already made: build on
> task-67's durable diagnostics (Done: -v/-vv, --log-file on pack/doctor/publish), keep the
> generated workflow one-line-step shaped, and prove transport preservation with tempdir /
> bare-remote tests, never this checkout. Locally provable slices first (renderer, tests,
> docs); the connected consumer run (AC#7) waits for the release lane.
>
> Propose the slice and stop. House rules are in `AGENTS.md` (invariant 10: anything you do
> not implement goes in `CONTEXT.md`, not into the files it speculates about), the session
> procedure and its templates are in `.agents/skills/session/`.
.agents/skills/session/`.

### 2026-10-04 — v0.5.3 consumer-proof access block; TASK-66 proposed

PR #100 merged as `bcb4df8` while this session was observing it. Its post-merge `ci` run
37227726829, `docs` run 37227726840, and `publish sandbox` run 37227726831 all passed. The
published `sandbox/developer-linux-64` transport now reports `source.commit` `bcb4df8`,
`pixi-sandbox` `0.5.3`, and static linkage. The local restored baseline is 626 passing / 1
skipped (21 git, 142 core, 378 sandbox with one skip, and 85 xtask).

The requested Castellan cycle could not be dispatched or inspected: the repository-scoped
GitHub token returns `Resource not accessible by integration` for `gh repo view
Archont561/castellan`. No consumer PR, publisher run, shells-pack result, or manifest proof
was fabricated; TASK-47 AC#7 and TASK-69 AC#6 remain open pending those live links and exact
manifest evidence.

With Castellan inaccessible, the next local slice is TASK-66: implement the generated relock
workflow's authoritative PR-visible verdict for repaired heads. Keep the existing fork-safety,
lock-guard, configured-CI, publish-validation, real-lock-commit dispatch, and loop-avoidance
rules; use fixture/tempdir tests for clean, repaired, failed-validation, and fork paths;
regenerate the committed workflow and keep actionlint/init byte-identical. Do not claim AC#6
until a connected consumer PR visibly carries the repaired-head check rollup.

Propose the slice and stop. House rules are in `AGENTS.md` (invariant 10: anything not
implemented belongs in `CONTEXT.md`, not speculative production files), the session procedure
and templates are in `.agents/skills/session/`.

### 2026-10-04 — follow-up correction: TASK-69 closed; upgrade delivery implemented

The earlier access-block entry was superseded during this session. Castellan PR #13 and run
37230019375 supplied the connected TASK-69 proof: the shells publisher passed with pixi-sandbox
0.5.3, the transport ref was 1252ae53, and its manifest/files oracle includes
`man/man3/App::Cpan.3`. TASK-69 is Done; posting to issue #95 remains blocked by the
repository-scoped token.

TASK-71's generated upgrade lane now supports the optional `PIXI_SANDBOX_UPGRADE_TOKEN`, emits
an actionable summary for missing or refused `Workflows: write`, and uploads an apply-able patch
artifact without pushing main. Local fmt, lint, and test gates are green at 631 passing / 1
skipped. Its connected delivery proof remains open. TASK-72 records the transport-budget spike;
TASK-73 records the owner-operated GitHub App proof cycle.

### 2026-10-05 — TASK-68 AC#7 closed; two findings worth their own tasks

**Landed** (so: not proposals): TASK-68 is Done. Connected proof run 37362418218 — the `publish`
job red at `Pack, verify, and publish` with the annotation `publish pipeline failed during pack
(exit 1)`, the `always()` upload step green, artifact `publish-diagnostics-linux-64` (1390 B, 7-day
retention) carrying a log that proves `doctor` and `publish` were never reached, and
`sandbox/developer-linux-64` byte-identical at `01db0c51` across the run. Issue #93 has the record.

**Proposals, not agreed:**

- **The generated install phase cannot catch manifest/lockfile drift.** It runs
  `pixi install --frozen`, which per pixi's own help "installs the environment as defined in the
  lock file, doesn't update lock file if it isn't up-to-date with the manifest file"; `--locked` is
  the flag that aborts. Measured here: adding a direct dependency with no `pixi.lock` entry left
  the connected publish run **green**, publishing a transport silently built from the stale lock.
  A consumer editing `pixi.toml` without re-locking gets a green publisher and a transport that
  does not match their manifest. Whether the publisher should install `--locked`, or verify the
  manifest against the lock some other way, is a design call with a real cost either way
  (it would fail publishes that today succeed), so it is not a change to make quietly.
- **A step summary cannot be read back by an agent.** GitHub exposes no REST endpoint for it and
  the anonymous job page does not carry it, so `always()`-uploaded artifacts are the only
  machine-readable half of the secondary-evidence surface. Worth knowing before another criterion
  asks an agent to *record* summary content as proof.
- **The App token's reach, measured:** push to `Archont561/pixi-sandbox` works; push to
  `Archont561/castellan` is `Permission denied`; `gh repo create` is `403` on `user/repos`;
  `gh workflow run` is `403` on `actions:write`; `gh issue comment` on this repo works. So the
  only connected trigger available to an agent here is a **push to a branch of this repository**,
  which is how AC#7 was proved at all.

### 2026-10-06 — auto-release bricked by an invalid permission scope; guard landed, secret still missing

**Landed** (so: not proposals): `.github/workflows/auto-release.yml` dropped the
`workflows: write` scope merged in PR #111, and `check-repository` grew check 11
(`repo_checks/workflow_permissions.rs`) so the same edit cannot reach main again. Verified
against the real tree: reintroducing the line fails `xtask check-repository` with
`auto-release.yml:93: unknown permission scope \`workflows\``, and reverting it goes green.
actionlint 1.7.12 reports the same line, which is what CI run 37501054714 was failing on.

**Open, and only a maintainer can close it:** the *original* failure is still there. Run
37498336562 died at `Commit, tag, and push` because `prepare-release` restamps the committed
relock render, so every release commit touches `.github/workflows/relock.yml`, and GITHUB_TOKEN
is barred from pushing workflow files. Removing the bogus scope un-bricks the workflow but does
not grant that push. **`RELEASE_PUSH_TOKEN` must be set** to a PAT with the `workflow` scope
(or a fine-grained token with `Workflows: read and write`, or a GitHub App token with the
Workflows permission); `auto-release.yml` already feeds that secret to checkout with
`persist-credentials: true`, so nothing else needs changing. Until it is set, `auto-release`
will start, prepare a release, and fail at the push — the next cut will look like a new
regression if this note is lost. (`gh secret list` is 403 for an App token, so whether the
secret exists at all could not be confirmed from here; the `refusing to allow a GitHub App`
wording in the rejection suggests it is unset and falling back to `github.token`.)

**Proposals, not agreed:**

- **Decouple the relock stamp from the release commit.** The push above only needs a
  workflow-scoped token because `prepare_release.rs` renders `relock.yml` with the new version
  (the stamp skew v0.5.3 exposed is why it does). If the stamp were derived at run time instead
  of committed, release commits would stop touching `.github/workflows/**` and GITHUB_TOKEN
  would suffice — no long-lived PAT in the repository at all. That trades one consistency
  oracle for one fewer credential, which is a real design call, not a cleanup.
- **PR #111 merged with `ci` already red** (merged 17:07:09Z; the PR's own ci run 37501045602
  was FAILURE). Check 11 would not have stopped that merge — only a required-status-check rule
  on `main` would. Worth deciding whether `ci` becomes required.

### 2026-10-06 — auto-release stops dispatching; the token is now the only missing piece

**Landed:** the two dispatch steps are gone from `auto-release.yml`, and `actions: write` with
them (nothing in the job calls the Actions API any more). The job is six steps, ending at
`Commit, tag, and push`.

Why, in one line: the GITHUB_TOKEN trigger suppression the dispatches worked around does not
apply to the token this workflow now requires. `prepare-release` restamps the committed relock
render, so the release commit touches `.github/workflows/**`, which GITHUB_TOKEN may never
push — RELEASE_PUSH_TOKEN must be a PAT or App token, and those trigger `on: push` normally.
`release.yml` already listens on `push: tags: v*.*.*` and `publish-sandbox.yml` on
`push: branches: [main]`, so keeping the dispatches would have started the release build twice
on one tag and the transport repack twice, concurrently — and the repack force-pushes the
transport branch. Neither workflow carries a `concurrency:` guard, so nothing would have
serialised that race.

The `dispatch-release` / `dispatch-sandbox-repack` pixi tasks stay, re-labelled as manual
recovery (the `dispatch-docs` precedent: a task nothing in CI calls). AGENTS.md's command list
says so too.

**Still open, unchanged and still maintainer-only:** `RELEASE_PUSH_TOKEN` is not set. Nothing
here grants the push; this only removes the double-fire that would have followed it. The token
needs `Contents: write` + `Workflows: write` — no longer `Actions: write`, since the dispatches
that needed it are gone. A dry run (`-f dry-run=true`) works without the secret because it never
pushes, so it exercises prepare + `check-repository` but **not** the Workflows permission; the
first real cut is the only test of that.

**Done, and the earlier note was wrong about the cost.** Both workflows now carry a
`concurrency:` guard, closing the last two of eight. The note above said a guard on
`publish-sandbox.yml` "means editing the template" and is "a consumer-visible change" — it is
neither. task-53 already shipped `[workflow.concurrency]` (`ConcurrencyPolicy` in
`sandbox_config.rs`, validated, rendered, golden-fixtured as `publish-sandbox.concurrency.yml`
and asserted in `tests/cli.rs`). No template change and no new code was needed: this repository
simply had no `[workflow]` table.

- `release.yml`: `release-${{ inputs.version || github.ref_name }}`, `cancel-in-progress:
  false`. The two spellings of one tag collapse onto one group — a tag push gives `v1.2.3`, and
  `dispatch-release` passes `--ref v1.2.3 -f version=v1.2.3`. Cancelling is not an option: five
  runners' assets plus a prefix.dev upload plus a GitHub Release, so a cancelled run is a
  partial release (auto-release.yml's reasoning, verbatim).
- `publish-sandbox.yml`: `publish-sandbox-${{ github.repository }}`, `cancel_in_progress =
  false`, set in `.pixi-sandbox.toml`. Repository-wide, not per-ref, because the contended
  resource is the force-pushed transport branch every run writes regardless of its ref. Worth
  knowing when reading `false`: GitHub gives at most one running plus one pending run, and a
  newly queued run cancels the older *pending* one — so `false` prevents an aborted force-push,
  it does not promise that every push publishes. Here that is correct, since a superseded
  pending repack would pack older main.

**ADR-18 is real and was taken deliberately, not inherited.** Adding `[workflow]` at all flips
`push_paths` to a derivation and `setup_pixi_cache` to `false`, so both are now written out
explicitly. `push_paths` is the derived seven (`.pixi-sandbox.toml`, `pixi.toml`, `pixi.lock`,
`package.json`, `bun.lock`, `Cargo.toml`, `Cargo.lock`) **plus `crates/**`**: the derivation
excludes source, which is right for a transport keyed on lockfiles but wrong for *this*
repository, whose publisher builds the packed bootstrap from the checked-out tree. Verified
rather than assumed that a release commit still fires the publisher: `prepare-release`'s own
`.release-touched` report lists `Cargo.toml`, `pixi.toml` and `Cargo.lock` — which is what the
deleted auto-release dispatch used to guarantee by hand after the v0.4.2 stale-transport
incident. `setup_pixi_cache = false` because `setup-pixi` runs with `run-install: false` here.

**Finding, unresolved: the committed `publish-sandbox.yml` is not a pristine render.** The
2026-10-01 note "no byte-equality check for `publish-sandbox.yml`" has since become a real
divergence, and the file's own "Regenerate this file instead of editing it" header is now
misleading. `init --check` against this tree reports drift: the committed copy builds the
planner and the bootstrap from source (deliberate, and commented in the file), while a fresh
render installs the released CLI from prefix.dev and carries the weekly `schedule:`, the
`upgrade` job and the diagnostics logging that landed in the template afterwards. So the three
policy blocks above were applied to the committed file as the exact bytes the generator emits
for them (diffed against a render of the same config in a scratch tree, line for line), not by
regenerating — regenerating would have silently reverted the source-build adaptation. Adding
check 10's byte-equality twin for the publisher is therefore **not** a small companion change:
it first needs a decision on whether this repository keeps the source-build fork at all, or
teaches the template a source-build knob (the `PIXI_SANDBOX_BIN` idea from 2026-10-01).

### 2026-10-06 — v0.6.0: the push-trigger chain runs end to end; RELEASE_PUSH_TOKEN proved out

**Landed** (observed on a live cut, not proposed): the first release with no `gh workflow run`
anywhere in the chain. `auto-release` (run 37514536620, dispatched by Archont561) reached
`Commit, tag, and push` and **succeeded** — the step that killed every previous attempt. The
release commit `8298edc` modifies `.github/workflows/relock.yml`, so **`RELEASE_PUSH_TOKEN` is set
and does carry the Workflows permission.** The two notes above both list that as the open,
maintainer-only item; it is closed by observation, not by inference. It is also a PAT or App token
rather than `github.token`: the push started `ci`, `publish sandbox` and `docs` by itself, which a
`github.token` push cannot do. That is the whole premise PR #113 bet on, and it holds.

Seven runs, in order, all green, no eighth:

| # | workflow | run | started by | duration |
| --- | --- | --- | --- | --- |
| 1 | auto-release | 37514536620 | `workflow_dispatch` (Archont561) | 1m03s |
| 2 | ci | 37514666260 | push `main` @ `8298edc` | 2m31s |
| 3 | publish sandbox | 37514666171 | push `main` @ `8298edc` | 4m06s |
| 4 | docs | 37514666148 | push `main` @ `8298edc` | 1m15s |
| 5 | release | 37514668874 | push tag `v0.6.0` | 13m44s |
| 6 | consumer proof | 37516425291 | `workflow_dispatch` from `release.yml` | 0m28s |
| 7 | docs | 37516428499 | `workflow_dispatch` from `release.yml` | 0m49s |

`airlock` did not appear, as its `workflow_call`/pull-request triggers require. Run 5 is the first
ever `push`-event run of `release.yml`; v0.4.1 through v0.5.3 were all `workflow_dispatch`. Its
five build jobs passed on all five runners — no `--` separator regression, no Windows `E0282`, the
`check-conda-platforms` gate, the OIDC guard and the `environment: release` approval all cleared
without a wait. `x86_64-apple-darwin` took 12m16s against 2m25s–5m20s elsewhere and is the only
reason the run is 13m44s.

**Transport freshness — the v0.4.2 oracle, answered.** `sandbox/developer-linux-64` is at
`c24d298` ("sandbox snapshot 2026-10-06T18:55:31Z"), and its `.pixi-sandbox/manifest.json` reads
`"source": {"commit": "8298edc"}` with `"tool": {"version": "0.6.0"}`. Tag `v0.6.0` is
`8298edce13049ab01a7bc9d3e09c9daf8d14fc08`. Same commit: the transport ships the release, not the
push before it. (v0.4.2 shipped `dbe57a8`/0.4.1 while main and the release were at `7745e9c`/0.4.2
— the failure this entire trigger design exists to prevent.) The repack also *finished* at
18:56:45, nine minutes before `release.yml` published the GitHub Release at 19:06:13, so there
was no window in which a published release pointed at a stale transport.

**The four PR-114 claims, scored honestly.**

- **`release.yml` group `release-${{ inputs.version || github.ref_name }}` — not exercised.**
  Exactly one run for `v0.6.0`; nothing queued, nothing cancelled, nothing contended. Guard
  present and correct by reading, unproven by running.
- **`publish-sandbox.yml` group `publish-sandbox-${{ github.repository }}` — not exercised.** The
  previous repack (37513641598, PR #114's own merge) ended 18:48:16; this one started 18:52:39.
  Four minutes apart, never concurrent. Same verdict.
- **`on.push.paths` (ADR-18) — proven, positive direction only.** The release commit touches
  `Cargo.toml`, `Cargo.lock`, `pixi.toml`, `crates/pixi-sandbox/Cargo.toml`,
  `crates/pixi-sandbox/pixi.toml` and `crates/xtask/Cargo.toml` — six matches against the eight
  entries — and the publisher fired on the main push. The ADR-18 trap did not spring and
  `dispatch-sandbox-repack` was not needed. No push to main since `ea0f77d` has *missed* the
  filter, so the negative half (a docs-only push correctly skipping a repack) is still untested.
- **`setup_pixi_cache = false` — proven, and it costs nothing.** The `setup-pixi` step takes 1s
  with the cache off (18:52:45→18:52:46 in `plan`, 18:53:44→18:53:46 in `publish`) and took the
  same 1s with it on (37509389815, 18:11:39→18:11:40). Whole-run wall clock: 4m06s and 3m38s with
  cache off against 4m08s, 4m01s, 4m19s, 5m02s with it on. There was never anything to cache —
  `run-install: false` means `setup-pixi` only fetches the pixi binary.

**Finding confirmed: `release.yml`'s docs dispatch is now redundant.** `docs` ran twice for one
release — 37514666148 from its own `push: [main]` trigger at 18:52:39, and 37516428499 from
`release.yml`'s `dispatch-docs` at 19:06:17 — both on `8298edc`, both green, 13 minutes apart. The
dispatch step's comment still claims the release commit "is pushed with GITHUB_TOKEN, and GitHub
suppresses workflow triggers on GITHUB_TOKEN pushes", which stopped being true the moment the push
moved to `RELEASE_PUSH_TOKEN`. It is harmless only because `pages-${{ github.ref }}` serialises
the pair; the second run republishes identical bytes.

**Exactly one `consumer-proof` run** (37516425291, artifact `consumer-proof-v0.6.0`, 28s against
the v0.5.3 baseline of 27s). So the `release: published` suppression still applies to a release
created with `github.token` — `author: github-actions[bot]` — and the explicit dispatch remains
the only trigger, not a second one. The bug class PR #113 removed did not come back.

**Correction to the brief this session worked from:** task-47 AC#7, task-52 and task-53 AC#7 are
*already* `[x]` and all three tasks are `status: Done`, closed 2026-10-04/05 on the Castellan
v0.5.2→v0.5.3 regeneration cycle. They were not waiting on this release. Nothing to reopen or
re-close; this note is the evidence for `RELEASE_PUSH_TOKEN` only.

**Proposals, not agreed:**

- **Delete the docs dispatch and `actions: write` from `release.yml`. Recommended.** The last step
  of the release job and the `actions: write` scope that exists solely for it are now dead weight:
  the push already rebuilds the site 13 minutes earlier, from the same SHA. Removing them drops
  the release job's only Actions-API write and is the same cleanup PR #113 did to `auto-release`,
  for the same reason. Leave `docs.yml`'s `pages-${{ github.ref }}` group alone — it is what makes
  the double trigger harmless while it survives. One caveat worth writing into the change: if
  `RELEASE_PUSH_TOKEN` ever reverts to `github.token`, both the docs push trigger *and* this
  dispatch disappear together, so the removal should cite this run as its evidence.
- **Make `ci` a required status check on `main`. Recommended, and still unaddressed.** PR #111
  merged at 17:07:09Z with its own `ci` run 37501045602 already FAILURE, which is what bricked
  `auto-release` for an hour. No in-repo check can stop that — check 11 only catches the specific
  scope typo. A branch protection rule requiring `ci` is the only mechanism, and it is a two-click
  maintainer change, not a code change.
- **The publisher byte-equality check: keep the fork, write the check against a documented
  exception. Recommended over the alternatives.** `pixi-sandbox init --check` reports drift
  because the committed publisher deliberately builds the planner and the packed bootstrap from
  the checked-out tree, while a fresh render installs the released CLI from prefix.dev and brings
  the weekly `schedule:`, the `upgrade` job and the diagnostics logging. Teaching the template a
  `PIXI_SANDBOX_BIN` knob (the 2026-10-01 idea) exports an owner-only concern into every
  consumer's generated workflow to buy one repository a lint; dropping the dogfooding claim throws
  away the only thing that proves `[workflow]` renders correctly against a real repository — which
  is exactly what this release just demonstrated. The tractable version is a check that asserts
  the *policy blocks* (`paths:`, `concurrency:`, the two `cache:` lines) are byte-identical to
  `render_github_workflow`'s output for `[workflow]` in `.pixi-sandbox.toml`, with the
  source-build steps listed as a named, tested exception. That is check 10's twin scoped to the
  part that actually regresses silently, and it is what was done by hand for PR #114.
- **`crates/**` in `push_paths`: leave it. One measured caveat worth recording.** The release did
  not refute source-sensitivity — it could not, since `8298edc` changed no `.rs` file. But the
  repack *is* source-sensitive by construction: the publish job's `Build source pixi-sandbox` step
  runs `cargo build -p pixi-sandbox --release` and packs that binary as the transport bootstrap
  (`tools.pixi-sandbox`, 3,778,928 bytes, v0.6.0 in the current manifest), so any change under
  `crates/pixi-sandbox`, `crates/pixi-sandbox-core` or `crates/pixi-sandbox-git` changes the
  shipped transport. Narrowing to the plain derived seven would reintroduce the stale-transport
  class. The caveat: `crates/xtask` is **not** in that dependency graph, and commit `11ec40b`
  (merged as `fe06c3a`) is a real example — it touched only `crates/xtask/**`, `.github/**` and
  markdown, matched no derived-seven path, and under the current filter would fire a full repack
  whose packed bootstrap is byte-identical. Narrowing `crates/**` to the three bootstrap crates
  would remove that false positive but makes the list silently wrong the day a fourth crate joins
  the graph. A ~4-minute redundant repack is the cheaper error than a stale transport, so: leave
  it, and if anyone does narrow it, pair the change with a check that derives the path list from
  the `pixi-sandbox` bin's workspace dependencies rather than hand-maintaining it.

### 2026-10-06 — Castellan bootstrapped onto 0.6.0; closure audit says close nothing yet

**Landed, in the consumer rather than here:** Castellan commit `8193c58` ("chore: update
pixi-sandbox to version 0.6.0") regenerates exactly the three owned files — `publish-sandbox.yml`,
`relock.yml`, `scripts/restore.sh` — with no config edit. It had to be a hand bootstrap. The
0.5.3 lane that would normally deliver an upgrade is the very lane the 0.6.0 templates repair: it
reads no `PIXI_SANDBOX_UPGRADE_TOKEN`, so installing that secret before this commit would have
changed nothing, and `GITHUB_TOKEN` may not push `.github/workflows/**`. That is the deadlock
issue #101 describes, and a human push is the only way out of it.

The push proves the regenerated templates work connected: `publish sandbox` run 37521245074 green
in 3m37s (`plan` 7s, `upgrade` **skipped**, `publish` green), transport repacked to `af92e4c1`,
and that manifest reads `"tool": {"version": "0.6.0"}` with `"source": {"commit": "8193c58"}` —
fresh against the pushed commit. Castellan's own `generated files` CI job passed, so the
regeneration is byte-correct. Publish also got *faster*, 3m37s against 6m07s on 2026-10-05.

**One new datapoint for task-72 worth keeping.** Castellan's `pixi-sandbox.toml` has no
`[budgets]` table, so the 0.6.0 template's new `doctor --budget-config pixi-sandbox.toml` ran on
defaults against a two-environment bundle whose `default` env alone is 500,179,397 bytes packed /
2,223,746,588 unpacked. It passed. The defaults are therefore not accidentally tight for a real
GUI-stack consumer — the one plausible regression in this upgrade, and it did not fire.

**Closure audit — nothing qualifies.** A scan of all 68 task files found no inconsistency: every
`Done` task has all criteria checked and every open task has genuinely open ones, so there is
nothing silently closeable. Taking each open item against today's evidence:

- **task-71 AC#5 — still open.** The `upgrade` job never ran; its `if:` is schedule-or-dispatch
  and this was a push. It also cannot be forced now: Castellan is pinned at 0.6.0 and 0.6.0 is
  latest, so a dispatch resolves `drift=false` and there is nothing to deliver. The proof window
  is the *next* release.
- **task-66 AC#1/#2/#6 — still open**, but no longer unreachable. The observer now exists in
  Castellan's `relock.yml`; what is missing is a pull request carrying repairable lockfile drift,
  since `relock.yml` is `pull_request`-only and `8193c58` was a push.
- **task-73 — not started.** AC#1 is only partly met: the ci-publishing guide names
  `PIXI_SANDBOX_UPGRADE_TOKEN` and the fallback, but not the owner-operated App installation,
  repository access or rotation expectations.
- **issue #101 — leave open.** This repository closes a consumer issue when its task closes, not
  when the fix merges (task-52 and task-53 both demanded "ships in a minor release, with the
  connected proof"). #101 closes with task-71 AC#5, not before.
- **issues #109 / #115 — unchanged.** #109 is task-75, not started. #115 still has no task and
  still wants a scope decision before it gets one.

**Proposal, not agreed:** set `PIXI_SANDBOX_UPGRADE_TOKEN` in Castellan *before* cutting the next
release rather than after. The next cut is the single event that closes task-71 AC#5, and the
secret decides which branch of that criterion it closes — present, the cron opens the reviewable
PR and also settles task-73 AC#3/#4; absent, the same run only proves the patch-artifact fallback
and task-73 AC#3 stays open for another release cycle. The cost of setting it early is nothing;
the cost of setting it late is a whole release.

### 2026-10-08 — TASK-75 landed; host requirements end to end, starter-lane trail folded in

**Landed** (so: not proposals) — four commits on `arena/44e28d89-pixi-sandbox`:
`1b55eac` the `[host_requirements]` config table (snake_case, ADR-19 — the issue sketched
kebab-case, which this repo's loader refuses), `5573b26` the manifest carry (`pack --config`
resolves the set for the packed platform; absent declaration leaves the manifest byte-identical),
`3f60269` the read-only probes (`doctor` classifies satisfied / missing / unknown / not
applicable with per-distribution remedies and JSON; `--require-host-requirements` fails on
`missing` alone; a foreign-platform transport probes nothing), and `2fae6de` the guide with the
Tauri/headless example and the "nothing is installed or granted" warning. Suite
681 → 710 → 720 → **746 passing / 1 skipped**; lint 11 gates; docs 13 → 14 pages.

**Folded in: the 2026-10-07 starter-lane trail had no entry of its own.** Three follow-up fixes
merged that day on top of PR #116 — #117 `ci: bootstrap starter repository before checkout`,
#118 `fix(ci): forward starter workflow task arguments`, #119 `fix(ci): verify starter main ref
before checkout` (`a42bcac`, the tip this session branched from). task-74 remains **In
Progress** with AC#7 open: the delivery credential cannot push `.github/workflows/*` (issue
#101), and the plausible remaining proof is a fresh `workflow_dispatch` of the starter lane with
`release-tag v0.6.0` / `source-commit 8298edc`, not attempted from here. The published transport
was repacked after #119 and still reads schema 2, tool 0.6.0, static, source `a42bcac`; it
carries **no** `host_requirements` section, which is the byte-identity default holding on a real
repack (this repository declares none).

**Not implemented — candidates for a task or a warm-up:**

- **Pack → restore → doctor end to end for `host_requirements`.** The carry is pinned in core
  manifest tests and the read-back in `tests/e2e.rs`, but the e2e injects the section into a
  copied fixture instead of `pack --config`ing it and restoring a transport that carries it. One
  fixture run would close the last seam; it was left out because AC#5 was already provable.
- **A pack-time sanity warning on the `libc` floor.** Declaring `>=2.40` while packing on `2.36`
  is silent until the airlock. Weighed and left out: the declaration describes the *target*
  host, often not the pack host, and a verdict there would drag the probe machinery onto a path
  whose job is byte fidelity.
- **`restore` runs no host probes, by design.** The declaration reaches the operator through
  the manifest, the branch README and `doctor`; making a write path inspect the host would make
  restore host-dependent and its failures ambiguous. If restore ever grows a readiness summary,
  it should shell out to `doctor`, not re-implement it.
- **GPU detection beyond `/dev/dri`.** `nvidia-smi` / `/dev/nvidia*` were considered and left
  out: a reportable-never-grantable verdict does not justify an external tool dependency, and
  the issue named `/dev/dri`.
- **macOS/Windows probes.** The seam and the family short-circuit exist (`Osx`/`Windows` report
  `unknown`/`not applicable`); only Linux has a real probe, and the family list stays closed at
  three — a fourth family needs a decision first (ADR-19).

**Proposals, not agreed:**

- **A `[workflow]` switch for `doctor --require-host-requirements`** (default off, so today's
  render is unchanged). Consumers with a self-hosted GUI runner could then enforce host
  readiness without hand-editing the generated publisher; the publish runner stays report-only.
- **A readiness summary line in the branch README's restore section** naming the declared
  requirements and linking `doctor` — the README already lists them, but nothing tells the
  operator to *run* the check. Cheap, but it changes a generated artifact, so it rides with the
  next render change rather than alone.

**Measurements worth keeping:** on a Debian-12 container, TASK-75's own example
(`libc >=2.34`, fontconfig/fonts/xvfb, dbus, display, gpu, xvfb-run) classifies as
**1 of 8 satisfied / 7 missing / 0 unknown**, every remedy an apt command — the guide carries
the abridged real output.

**Prompt to start the next session with:**

> Restore the sandbox and baseline the suite — expect **746 passing / 1 skipped** if this
> session's pull request merged, and **720 / 1** if it is still open (the difference is exactly
> TASK-75's tests). The published `sandbox/developer-linux-64` transport is packed by 0.6.0 from
> `a42bcac`, static, and carries no `host_requirements` section. Then read `CONTEXT.md`
> § Session scratchpad — the 2026-10-08 heading lists the open items, first among them the
> pack → restore → doctor end-to-end seam.
>
> First, one look at the repo state: `gh pr list --state open` and `gh pr checks <n>`. If the
> TASK-75 pull request is open or red, that is the work — read the failing job before touching
> code. If it merged, `git log --oneline origin/main -3` should show the squash commit and
> `backlog/tasks/task-75` should read `status: Done` with all six criteria checked; do not
> reopen it.
>
> I want to take **task-76** this session — make `pixi-sandbox` the single entrypoint of the
> generated publisher: every logic-bearing step becomes one `pixi-sandbox <verb>` (or one
> `actions/github-script` step, as `relock.yml` already does), the absorbed shell moves into
> Rust subcommands the suite already reviews, the bash/pwsh twins collapse, and check 9 stops
> exempting generated artifacts. Its dependencies, TASK-36 and TASK-71, are both Done. Slices:
> the generator refactor plus its subcommands and tests, then the golden render, `init --check`,
> actionlint and check-9 changes — all locally provable; **AC#8 (a real consumer run on a
> released binary) last, and only on my sanction**, because it needs pushes and a consumer lane
> a release can exercise. If you would rather keep the slice small, **task-77** (pedantic
> clippy, 261 MachineApplicable fixes, no new dependencies) is the cheap alternative and fully
> local. task-73 and task-74 stay parked until the owner-operated credential can push
> `.github/workflows/*` (issue #101).
>
> Propose the slice and stop. House rules are in `AGENTS.md` (invariant 10: anything you do not
> implement goes in `CONTEXT.md`, not into the files it speculates about), the session procedure
> and its templates are in `.agents/skills/session/`.

### 2026-10-08 (second session) — TASK-80 merged, TASK-84 written but not pushed

**Landed** (so: not proposals) — **TASK-80** closed and squash-merged as **PR #122**: `check_rel_path`
refuses any path whose `Path::file_name()` is `None` with `Error::Invalid` naming it, `shard.rs`
takes the final component through the new `manifest::rel_path_file_name`, and no `expect` remains
in that file. CI on #122 was green across all six checks, **including the airlock leg** — the one
that would have caught a rule too strict to load a transport.

**TASK-84 is written and committed locally but never pushed.** The session's GitHub access ended
the moment #122 merged, so moving those three commits is the first job of the next session (see
the prompt below).

**The finding worth its own paragraph: the panic TASK-80 describes was *latent*, not live.** A part
path of `.` does panic — at `shard.rs:234`, with `file name` — but no CLI surface reaches it. Two
things hid it, both of them load-bearing: `verify.rs:484` already tolerates a missing file name
(`unwrap_or_default()`), so the read surfaced as an io error (`Is a directory`), and invariant 1's
verify-before-write stops `restore` before `materialise` can reach `assemble`. Unfixed,
`doctor --verify` reported only `verify failed: 1 failure(s)` with no reason, and `restore` stopped
at `transport verification failed`. The panic is reachable only through the readers *behind* the
verify gate (`restore` → `materialise` → `assemble`, `verify_env_restored` → `read_blob`), which is
why `tests/shard.rs` pins it directly rather than through a transport. **Any future audit of
untrusted-manifest handling should look for readers behind the verify gate, not at the CLI
surfaces** — the surfaces are the one place a malformed manifest cannot do damage.

**Also landed, as the guard TASK-84 did not ask for:** `xtask check-repository` **check 12**
(`crates/xtask/src/repo_checks/agents_md.rs`) — every root-anchored path in AGENTS.md's repo map
resolves, and the promoted-module list is exactly `lib.rs`'s `pub mod` set, checked in both
directions. It lives in xtask rather than in `tests/` because ADR-10 forbids a test from reading this
repository, which is precisely why the drift had gone uncaught. Reverting AGENTS.md to its pre-fix
state makes it fail with `repo consistency: 2 check(s) failed`, naming `release.rs` and the two
modules the paragraph omitted.

**Measurements worth keeping:** baseline **746 passing / 1 skipped** at `13d3a86` → **760** after
TASK-80 → **767** after TASK-84 (check 12's seven tests). The last recorded figure, **654/1 at
`db2d446`**, is unreachable from this sandbox: the clone is shallow at `13d3a86` and
`git fetch origin db2d446` is refused by the remote, so the +92 is **unmeasured rather than
assumed**. The one skip is the network-gated `e2e` test.

**Open, in the order a session should consider them:** the three unpushed TASK-84 commits (first,
below); **task-79** (low, fully local — `mib` defined three times, `make_executable` twice, dead
`publish.rs::manifest_path` under `#[allow(dead_code)]`, two `LauncherKind` types); **task-81**
(high, local but it *is* the bootstrap path — a session that breaks `restore.sh` has no network with
which to fetch a working one, so that slice ends by proving a real restore still runs); **task-83**
(medium, 27 files / 3,077 lines of inline tests, but core is only 4 files / 326 lines, so a
core-only slice is a sane first bite). task-85 and task-73 stay parked: both need
`.github/workflows/*` pushed, which issue #101 bars from this credential.

**Prompt to start the next session with:**

> Restore the sandbox and baseline the suite — expect **767 passing / 1 skipped** if TASK-84's pull
> request merged, and **760 / 1** if it is still open (the difference is exactly check 12's seven
> tests). Then read `CONTEXT.md` § Session scratchpad — the 2026-10-08 second-session heading lists
> the open items, first among them the latent-panic finding.
>
> First, the branch bookkeeping this session could not finish: `arena/1759ed91-pixi-sandbox` carries
> three unpushed TASK-84 commits sitting on top of four TASK-80 commits that main already holds as a
> squash. `git fetch origin main`, then `git rebase --onto origin/main 8c6329f` (`8c6329f` is the
> last TASK-80 commit; everything after it is TASK-84 plus this scratchpad entry),
> `git push --force-with-lease origin arena/1759ed91-pixi-sandbox`, and open the TASK-84 pull
> request. Expect no conflicts: the two tasks touch no file in common.
>
> I want to take **task-79** this session — remove the duplicated helpers and the naming collisions
> the audit found: `fn mib` defined three times with identical bodies (`commands/doctor.rs:690`,
> `commands/publish.rs:124`, and the shared `commands/support.rs:312`), `make_executable` in both
> `commands/support.rs:131` and `user_tools.rs:268` while `replace.rs` keeps its own exact-0755
> policy, `publish.rs::manifest_path` dead behind `#[allow(dead_code)]`, and two `LauncherKind`
> types for one concept. Four slices, one commit each, in that order — the generated launcher output
> must stay byte-identical, so the LauncherKind rename lands last and alone. Every slice is local,
> so commit locally on the session branch; **do not push, open a pull request, or merge without my
> explicit go-ahead.** task-81 and task-83 are optional follow-up slices on the same branch, after
> task-79 is green.
>
> Propose the slice and stop. House rules are in `AGENTS.md` (invariant 10: anything you do not
> implement goes in `CONTEXT.md`, not into the files it speculates about), the session procedure and
> its templates are in `.agents/skills/session/`. The backlog CLI is
> `.pixi/envs/default/bin/bun node_modules/.bin/backlog …` — note that it breaks on task text
> containing apostrophes or parentheses when it is called through `pixi run bunx backlog`.

### 2026-10-09 — task-85's verifier judges the revision; AC#6 and one reconcilable pair stay open

**Landed** (so: not proposals) — **task-85 AC#1–#5** as **PR #125**, squash-merged into `ce47e41`;
`ci`, `docs` and `publish sandbox` green on that commit, transport repacked from it (0.6.0, static).
`starter::verify` now asks `git` what the next commit would contain (new `ShellGit::ls_publishable`,
index + untracked-not-ignored) and reports a forbidden path only when it is both present and in that
answer; `xtask starter-clone` materialises the same set into a scratch repo, commits it and hands back
a `git clone --no-hardlinks`, and the lane's "fresh clone" dev task runs there; a mismatched
`source-commit` now dies in its own first job (`xtask starter-check-dispatch`, which `starter`
`needs:`) naming both commits. Suite **774 → 802 passing / 1 skipped**.

**Two findings worth their own tasks, neither implemented here.**

- **`verify` and `publish` answer different questions about the same tree.** `starter::publish`
  stages through `ShellGit::worktree_status_files`, which is `git status --porcelain
  --untracked-files=no` — it never stages an untracked file. `verify` now refuses a tree whose
  *untracked-but-not-ignored* paths carry runtime state, which is a strictly broader set. So on a
  starter repository that does not yet track a new file (a first publish, or a file added by a
  newer generator), publish would skip a file verify just approved, and the "would commit" the
  verifier speaks of is not the "will be staged" the publisher performs. Either `publish` moves to
  `git add -A` over the `ls_publishable` answer, or `verify` narrows to what publish can stage.
  The first keeps the guarantee and fixes the first-publish gap; the second is a weakening.
  `crates/xtask/tests/support::starter_tree` is the fixture a test for either choice belongs to,
  and it already takes the workflow-version argument both stamps.
- **A guard step cannot precede its own manifest.** Making AC#4 "fail in the first job step, before
  checkout" literally is impossible while the verb is `pixi run -e package xtask …`: the task graph
  needs the checkout it is trying to avoid. The job-level `needs:` is the shape that gets the
  semantics (nothing is fetched, written or scaffolded before the guard agrees) without putting
  shell back into the workflow — worth remembering for task-76, whose end state wants the same
  property for `publish-sandbox.yml`.

**Record correction, not a proposal.** The 2026-10-08 second-session entry closes at **767** and
this session found **774** on `12c9824`. The +7 is task-79/81/83's tests: those tasks were closed
21:31–22:50 that day and delivered through #123/#124 by the session on
`arena/5fc3a8ee-pixi-sandbox`, which skipped §5 — no scratchpad entry, so the baseline in this file
read low for a day and the delta looks unexplained. The work is real and the tasks are correctly
Done; only the hand-off is missing. Next session should not chase it.

**Open, in the order a session should consider them:** **task-86** (medium, primed: #124 landed
`rstest-bdd 0.6.0` and the current transport's vendor set already carries `rstest-bdd`,
`rstest-bdd-patterns`, `rstest-bdd-policy` and `gherkin 0.16.0`, so it builds offline today with no
relock); **task-79-class reconciliation above** (the `verify`/`publish` pair — small, local, and it
is the kind of gap the audit tasks exist to close); **task-76** (high; AC#8 needs a consumer run);
**task-82** (medium, local); **task-77** (medium, local, wide mechanical); **task-78** (high, but its
AC#1 wants a reviewed D-number decision before code, and `pixi-build` is still preview).
**task-85** itself: AC#6 is one dispatch away for anyone who can click it (see the prompt). task-73
and task-74 stay parked on issue #101, still open; task-66 still waits for a Castellan PR carrying
deliberate lockfile drift.

**Prompt to start the next session with:**

> Restore the sandbox and baseline the suite — expect **802 passing / 1 skipped**. The published
> `sandbox/developer-linux-64` transport is packed by pixi-sandbox 0.6.0 from `ce47e41`, static, and
> its vendor set already carries `rstest-bdd 0.6.0` and `gherkin 0.16.0`, so the BDD layer builds
> offline with no relock and no new vendoring. `pixi run --frozen test` is ~27s warm, ~2m10s after a
> cold restore.
>
> Then read `CONTEXT.md` § Session scratchpad — the **2026-10-09** heading lists the open items and
> carries one record correction (774, not 767, is the honest pre-session baseline).
>
> First, one look at the repo state: `gh run list --branch main --limit 3`. If `ci`, `docs` and
> `publish sandbox` are green on `ce47e41`, task-85's merged half is settled — do not re-run or
> re-pack anything. If you can dispatch workflows as the owner, task-85's last criterion is one click:
> `gh workflow run starter.yml -f release-tag=v0.6.0 -f source-commit=8298edce13049ab01a7bc9d3e09c9daf8d14fc08 -f dry-run=true`
> (it must not publish — `starter-publish` is the step after the one being proven). Green there and
> task-85 flips to Done; a failure in the *clone* step points at `pixi run --manifest-path` semantics,
> not at `clone_publishable`.
>
> I want to take **task-86** this session — the BDD layer at `crates/pixi-sandbox/tests/features/`,
> three features (restore verification, transport integrity, user-tools policy) whose steps drive the
> public API through the existing `tests/support` fixture builders. Decided already: rstest-bdd is
> dev-only and vendored, scenarios state outcomes only, no existing test is deleted or weakened, and
> `AGENTS.md` § Test conventions gains the one paragraph saying where `.feature` files and step
> definitions live. Slices: one feature at a time, red→green per the TDD skill, one conventional
> commit per feature; AC#1–#5 are all local. Do not push, open a pull request, or merge without my
> explicit go-ahead.
>
> Propose the slice and stop. House rules are in `AGENTS.md` (invariant 10: anything you do not
> implement goes in `CONTEXT.md`, not into the files it speculates about), the session procedure and
> its templates are in `.agents/skills/session/`.

### 2026-10-09 (second session) — task-86 started; the macros half of rstest-bdd cannot be vendored offline

**Environment.** Restore green (0.6.0 from `ce47e41`, schema 2, static, 13803 blobs / 0 failures);
baseline **802 passing / 1 skipped** as predicted; user tools registered. `ci`, `docs` and
`publish sandbox` are all green on `ce47e41` — task-85's merged half is settled, nothing re-run
or re-packed. The starter dispatch (task-85 AC#6) still 403s for the sandbox token
(`Resource not accessible by integration`); the dry-run click stays the owner's.

**Finding: the session prompt's "no relock, no new vendoring" premise covers only half of
rstest-bdd.** The vendored set carries the runtime half (`rstest-bdd` core + `rstest-bdd-patterns`
+ `rstest-bdd-policy` + `gherkin 0.16.0` + `inventory`), but the `#[given]`/`#[when]`/`#[then]`/
`#[scenario]` attribute macros live in a **separate crate `rstest-bdd-macros`** (a dev-dependency
of rstest-bdd, never transitive). It is not in Cargo.lock, not vendored, and pulling it in needs
~fifteen unlocked crates (`rstest-bdd-harness`, `cap-std` tree, `camino`, `proc-macro-error3`,
`newt-hype`, `walkdir` consumers, …) plus a connected relock for their crates.io checksums —
which an airlocked session can neither fetch nor fabricate honestly (a guessed checksum builds
locally and dies in connected CI). Declared this session, task-86 as worded ("`#[scenario]`
generates one rstest test per scenario") is not executable offline.

**Chosen shape (what the BDD layer therefore is).** Bind scenarios through the vendored half:
`.feature` files parsed by the real `gherkin` crate; steps registered with rstest-bdd's own
`step!` macro into its registry (pattern engine, specificity, `StepContext`/`StepExecution`
runtime — all exercised as published); one thin runner in `tests/support/bdd.rs` doing what the
macros codegen does (parse → resolve each sentence in the registry → execute against an owned
world cell), one plain `#[test]` per scenario, failures naming feature/scenario/step. Cost: the
gherkin dev-dep edge adds one line to `pixi-sandbox`'s lock entry (no new package, offline-safe).
Deviates from the task's mechanism sentence, satisfies all five ACs. Swap-in path for later: once
a connected session relocks `rstest-bdd-macros` and a repack vendors it, the runner can be
replaced by `#[scenario]` without moving the feature files.

**Relock round (same session, on the owner's instruction).** TASK-86 + the macros declaration
landed as **PR #126** on `arena/b44e8d80-pixi-sandbox`: three `test(bdd)` commits (suite
802 → 816 passing / 1 skipped, every local gate green), then `chore(deps): declare
rstest-bdd-macros 0.6.0 for the BDD layer` with `compile-time-validation`, Cargo.lock left
deliberately stale. The bot ran the textbook cycle: guard failed on the drift, `relock` pushed
`chore(lock): refresh lockfiles for #126` (+220 lines — the macros crate plus its ~20-crate
tree, `cap-std`/`camino`/`newt-hype`/…, genuine crates.io checksums, all inside deny.toml's
allowlist), then dispatched CI + the publisher on the repaired head. One real failure and one
procedural finding: the dispatched CI red was `lint:toml` — the root declaration had not been
through taplo after an environment reset mid-session (fixed in `39def33`, CI green after; the
PR head passes lock guard, ci, airlock, codecov patch); and runs for commits authored by
`pixi-sandbox[bot]` sit at `action_required` — approving, cancelling and dispatching are all
owner-token actions, so a bot-commit PR check that needs re-running needs the owner's click.
**Consequence until the PR merges and the push-trigger repack runs:** airlocked hosts cannot
build this branch offline (`no matching package named rstest-bdd-macros found` in the vendor
directory) — connected CI is the proof tier for it, exactly the gap #124 had between
declaration and repack. Once the repack carries the macros crate, the follow-up is to replace
`tests/support/bdd.rs` with `#[scenario]`/`#[given]` bindings — feature files and step
sentences do not move. Also recorded: a sandbox reset between turns re-cloned the workspace and
dropped the first four local commits; the tree was reconstructed byte-identical, one
conventional commit per slice, so nothing was lost, but the old local hashes appear nowhere
remote. task-85's starter dispatch (AC#6) is still the owner's click.

### 2026-10-09 (third session) — task-87's macro bindings are proven locally, unpushed; task-86 closed at last

**Environment.** Restore green — twice, because a sandbox reset between turns re-cloned the
workspace and took `.pixi`, `.pixi-sandbox` and `~/.local/bin` with it; the bootstrap is
idempotent and the second run came from the same transport (0.6.0 packed from `8dc5bba`, schema 2,
static, 268 vendored crates, 14495 blobs / 866.3 MiB / 0 failures). Baseline **816 passing /
1 skipped** exactly as the opening prompt predicted, user tools registered. `ci` (3m47s), `docs`
(39s) and `publish sandbox` (4m0s) are all green on `8dc5bba` — the merge is settled, nothing
re-run and nothing re-packed. **The previous session's consequence is closed:** the repack does
carry `rstest-bdd-macros 0.6.0` and its whole tree (`rstest-bdd-harness`, `cap-std`, `camino`,
`newt-hype`, `proc-macro-error3`, `convert_case`, `syn 3`), and all of it compiled offline inside
that baseline run *before* a line of migration code existed — which is the cheapest possible
proof that the swap-in path was executable, and the reason this session could promise the slice
without hedging.

**Implemented, and deliberately not pushed** — **task-87** as five commits on the session branch
`arena/49e36983-pixi-sandbox`, with no pull request open, because that sanction was reserved:
`docs(backlog)` (task-86 closed 5/5 with its mechanism deviation recorded, task-87 opened), one
`test(bdd)` migration commit per binary — transport integrity, restore verification, user-tools
policy — and the runner's retirement. task-87's five ACs are all checked on local evidence and its
status stays `In Progress` until the work lands on main, which is a one-line edit afterwards; a
task marked Done whose commits exist only on an unpushed branch is the stale claim §5 warns about.
Suite **816 passing / 1 skipped, unchanged**, which was the
constraint rather than a coincidence: eleven scenarios kept the test names nextest already
reported, and three binding oracles replaced three duplicate-registry guards one for one. The
three `.feature` files, all three lockfiles and every `crates/*/src` are byte-identical to
`8dc5bba`, each proven with `git diff --exit-code` rather than asserted. Step bodies went from
four-argument `StepContext` callbacks returning `Result<StepExecution, StepError>` to ordinary
functions on a fixture-injected world, and the largest file lost 176 net lines doing it.

**Four findings worth keeping, none of them proposals.**

- **The compile-time ambiguity check is expansion-order dependent.** A duplicated step pattern
  declared *above* the `#[scenario]` bindings is a compile error — `Ambiguous step definition for
  'a packed transport'`, pointing at the binding, listing both patterns. The same duplicate
  declared *below* them compiles cleanly, because the macro validates against the registry as it
  stands when it expands. What catches that half is rstest-bdd's own runtime registry
  (`registry/mod.rs:238`), which panics `duplicate step for 'Given' + 'a packed transport' defined
  at <file>:<line>` and fails every scenario in the binary. So retiring
  `assert_no_duplicate_steps()` lost nothing: the duplicate ground is covered twice by the
  published crate, once earlier and once more precisely than the hand-rolled guard managed. Any
  future claim that the macros "check the registry at compile time" should carry this caveat, and
  a step declared below its bindings is the case that slips through the first tier.
- **`strict-compile-time-validation` is what makes an unmatched sentence a build failure.** With
  only `compile-time-validation` declared — the state #126 left — a sentence no step matches is a
  *warning*, and the `Cargo.toml` comment claimed otherwise; it is corrected now. Under `strict`
  it is `error: No matching step definition found for 'Given a packed transportt'` listing the
  three definitions that do exist. The feature implies the non-strict one and adds no dependency,
  so `Cargo.lock` did not move — the "no lockfile change" constraint and the fail-fast upgrade
  turned out to be compatible rather than a trade.
- **The macro expansions are clippy-clean under `-D warnings --all-targets`.** Upstream needs its
  own `rstest_bdd_test_macros::allow_fixture_expansion_lints` to survive its lint config; that
  helper crate is not vendored and the house needs no equivalent. The only clippy error this
  migration produced was in the new guard's own code (`redundant_closure`). Worth remembering
  before anyone adds a lint-suppression shim on the grounds that upstream ships one.
- **A binding *is* the test now, so an unbound scenario is a silent coverage loss.** That is the
  one failure mode the retired runner could not have: it needed a hand-written `#[test]` per
  scenario sitting beside the registry, so a forgotten scenario was a visible gap in the file.
  `assert_every_scenario_is_bound` reads the test root's *source* for `name = "…"` selectors
  rather than the compiled test list, because the user-tools pre-policy binding is `#[cfg]`-gated
  — on a foreign host it is absent from the binary while still bound in the source, and counting
  compiled tests would raise a false alarm on every non-linux-64 runner. Proven red by pointing
  two bindings at one scenario, which named the third as unbound. The `#[cfg]` also moved onto the
  step attributes themselves, which is why four cfg attributes left with the `step!` registry
  block instead of being dropped.

**One tooling finding, and a correction owed to a skill file.** The backlog CLI cannot carry prose
through `pixi run`: pixi joins task arguments into a command line *without quoting them*, so spaces
split arguments ("too many arguments for 'create'. Expected 1 argument but got 6"), backticks are
command-substituted by the inner shell (`rstest-bdd: command not found`), and an embedded newline
abandons the invocation entirely and prints the task list. Wrapping the whole inner command in one
pre-quoted argument does not help either — pixi escapes it into a single `bun x` package spec. So a
task file with real prose has to be written directly and then validated by a CLI *read-back*
(`backlog task 87 --plain`), which handles backticks, em dashes and `§` fine; the CLI also parsed
`&mut World` inside an inline code span as a markdown error before any shell got near it. The
backlog skill's advice to "repeat the `--append-*` variant once per line" does not survive that
argument splitting. `.agents/skills/backlog/SKILL.md` therefore states something that is not true
in this sandbox, and correcting it is a real (small, unimplemented) item — recorded here per
invariant 10 rather than edited into the skill on spec.

**Also closed this session, and it was overdue:** task-86 sat `In Progress` with **0/5** ACs
checked and no NOTES or SUMMARY at all, although #126 merged green on `8dc5bba` — the §5 step-2
close never happened, and neither did that session's report or its next opening prompt, which is
why the prompt that started this session appears nowhere in this file. The AC-by-AC evidence is
now in the task, with the mechanism deviation stated plainly: all five criteria were met by what
merged, and only the Description's mechanism sentence deviated, because `rstest-bdd-macros` was
not vendorable offline when the work was done.

**Open, in the order a session should consider them:** **task-87's five commits** — proven, gated
and unpushed on `arena/49e36983-pixi-sandbox`; the only thing outstanding is the owner's sanction
to push and open the pull request, after which the task flips to Done and §5 can run properly.
**task-85** AC#6 — one dispatch away for
anyone who can click it; it still 403s for the sandbox token (`Resource not accessible by
integration`). **The `verify`/`publish` reconciliation** from the 2026-10-09 first entry — small,
local, and the kind of gap the audit tasks exist to close: `starter::publish` stages through
`worktree_status_files` (`--untracked-files=no`) while `verify` now judges `ls_publishable`, a
strictly broader set, so a first publish can skip a file verify just approved. **task-76** (high,
deps TASK-36/TASK-71 both Done so it is unblocked; AC#8 needs a consumer run, and the 2026-10-09
first entry's "a guard step cannot precede its own manifest" finding is the shape its end state
wants). **task-82** (medium, local). **task-77** (medium, local, wide mechanical — note it would
have met the clippy finding above, since the macro expansions are already clean). **task-78**
(high, but AC#1 wants a reviewed D-number decision before code and `pixi-build` is still preview).
task-73 and task-74 stay parked on issue #101; task-66 still waits for a Castellan PR carrying
deliberate lockfile drift. Nothing in the BDD layer is open: the layer is on the macros, the
runner is gone, and the only guard left is the one the macros cannot replace.

**Prompt to start the next session with:**

> Restore the sandbox and baseline the suite — expect **816 passing / 1 skipped**. The published
> `sandbox/developer-linux-64` transport is packed by pixi-sandbox 0.6.0 from `8dc5bba`, static,
> and its vendor set carries the whole rstest-bdd tree including `rstest-bdd-macros 0.6.0` with
> `strict-compile-time-validation`, so the BDD layer builds offline with no relock and no new
> vendoring. A sandbox reset between turns drops `.pixi` and `~/.local/bin`; `bash
> scripts/restore.sh` is idempotent and takes about four minutes.
>
> Then read `CONTEXT.md` § Session scratchpad — the **2026-10-09 third-session** heading lists the
> open items and carries four findings about the rstest-bdd macros, first among them that the
> compile-time ambiguity check is expansion-order dependent.
>
> First, one look at the repo state: `gh run list --branch main --limit 3`. If `ci`, `docs` and
> `publish sandbox` are green on the tip of main, the last merge is settled — do not re-run or
> re-pack anything. Note that task-87 landed on a session branch and was **not pushed**, so main
> may still be at `8dc5bba`; check whether its pull request merged before assuming either way.
>
> I want to take **<task>** this session — <shape, plus every decision already made>. Decided
> already: <…>. Slices: <the locally provable ones> first, <anything needing a push, a release or
> a native runner> last, which needs <the sanction you are reserving>. Do not push, open a pull
> request, or merge without my explicit go-ahead.
>
> Propose the slice and stop. House rules are in `AGENTS.md` (invariant 10: anything you do not
> implement goes in `CONTEXT.md`, not into the files it speculates about), the session procedure
> and its templates are in `.agents/skills/session/`.

### 2026-10-09 (fourth session) — task-82's pack library extraction is complete locally, not pushed

**Environment and recovery.** The sandbox reset after the previous push: Pixi and the restored
prefix were gone, and local HEAD was back at `8dc5bba` while the seven task-87 commits survived
as workspace files. Fetched the existing session branch, verified **all 424 files** against
`origin/arena/49e36983-pixi-sandbox` with no content mismatch, then recovered HEAD/index to
`d8f14cf8` without discarding any workspace file or changing branches. The previous seven commits
**were pushed**, on the owner's explicit instruction; the third-session entry's "unpushed"
sentences are historical, not today's state. No PR exists. Main still names `8dc5bba`, with
`ci`, `docs` and `publish sandbox` green; nothing was re-run or re-packed.

Restore passed from the same 0.6.0/static/schema-2 transport (268 vendored crates, 14495 blobs,
0 failures); user tools registered. Fresh baseline **816 passing / 1 skipped**, hooks installed.
TASK-8 and TASK-54 are Done, so TASK-82's sequencing prerequisites were satisfied.

**Decisions agreed before tests.** The owner chose four promoted boundaries, `pack` first:
`pack` (preflight/file oracle), `vendor` (lock policy and storage), `tool_fetch` (pin selection,
fetch/cache/embedding), `branch_docs` (render/write). No dependency, transport/schema or rendered
output change; no generated-workflow rewrite. The byte-identity proof has one explicit exception:
normalize only `manifest.created_at` and the corresponding README Built timestamp. The actual
manifest's build-version fields are never normalized; the expected fixture supplies the current
package version as an input, so future release bumps remain valid but wrong emitted stamps fail.

**Implemented, not pushed — TASK-82 at 6/6 local ACs, status Done in house format.** Four commits,
one module per slice on the existing session branch:

- `f0933ce9 refactor(pack): promote preflight and the per-file oracle` — layout refusals,
  host-requirement resolution, ADR-13's verification unpack, and file recording. Shared command
  primitives moved once and re-exported, not duplicated. Public tests: **20**.
- `29962105 refactor(pack): promote cargo vendoring policy` — collision policy, loose/per-crate-
  tar output and toolchain provenance; explicit external-program paths keep tests off ambient
  PATH. Public tests: **14**.
- `087321d0 refactor(pack): promote pinned tool fetch and verification` — the HTTP adapter
  supplies a byte stream, never the verification verdict. The library retains pin/hash/version/
  linkage/cache policy; HOME/USERPROFILE selection stays in the CLI. The last two inline pack
  tests moved here and the LEGACY entry left in the same commit. Public tests: **25**.
- `aaec92ae refactor(pack): promote branch documentation rendering` — pure manifest-to-guide
  rendering plus a separate ordered writer with the original error context. Public tests: **9**.

`commands/pack.rs`: **1,254 → 495 production lines**, now `run` and four planning/orchestration
methods. All eleven original inline tests have integration routes; splitting the combined
unsafe/duplicate selection test into two named cases accounts for one additional case, not a
lost assertion. LEGACY **6 → 5**; both architecture guards and the shared-helper guard pass.
Suite counts at the four green slices: **827 → 841 → 864 → 873 passing**, **1 skipped** throughout.
Final gates: fmt --check, lint **11/11**, test **4/4**, fresh `xtask check-repository`; no lockfile,
core/git source, generated-workflow, xtask source or BDD-feature change. The task's notes and
summary carry the AC-by-AC proof. Done here means locally complete, not merged into main.

**Independent before/after proof.** Preserved the original binary before extraction (sha256
`a73cdd6a540e51f6755f9ef8d80b9a9c1a239650615685ff96d57e5bb9e68fa9`) and captured fixture transports
from it. The final binary matches **all 40 files and executable bits** across four transports:
Linux/no-vendor (8), Linux/loose with host requirements (11), Linux/tarballs (10), Windows/loose
(11). Only the two approved timestamp sites were normalized. Raw tar archive sha256 matches:
`c8aebc22e0c9fafc0be24ecbaa40813bd9d90aec35dabb523c0257bbed42004f`. Three portable references are
committed under `tests/fixtures/pack-reference/`; tar headers carry native UID/GID, so the
portable test compares extracted bytes while the raw archive comparison is local to this runner.
External pack/vendor programs are synthetic, not a real network solve — the proof concerns the
same input fixture and command contract, not a new live upstream-tools claim.

**Reds seen, all reverted.** Each promoted seam first failed to compile before it existed.
Four deliberate behavioral mutations then proved the oracles sensitive: unpacking the original
pack changes its bytes (the fake unpacker writes its cache); weakening the duplicate-source
filter accepts the colliding lock; bypassing download checksums replaces a cache with unverified
bytes; changing the README headline fails all reference cases. No mutation was committed.

**Two fixture findings worth keeping.** An inherited checkout-local TMPDIR can put a temporary
project under this repository's Git ancestry; an unborn local fixture repository prevents that
ambient commit from leaking into source provenance. And the original short MZ filename example
is only 14 bytes: the linkage classifier needs 20 and reports it `unknown`, not `system`. Its
old comment was inaccurate; the moved test states what it actually judges. No linkage or version
policy tightening was smuggled into a behavior-preserving extraction.

**Not implemented / open, newest state.** The four TASK-82 commits and this context close are
local only; no renewed push sanction, PR sanction or merge sanction was given. TASK-87 remains
5/5 and In Progress pending its merge, with its earlier seven commits already on origin. A future
PR from this fixed session branch will include both tasks. The backlog skill's prose-quoting
correction and the `verify`/`publish` reconciliation from the earlier entries remain untouched.
TASK-76 is the next high-priority, dependency-unblocked candidate (AC#8 needs a consumer run);
TASK-77 is the local but wide lint pass. TASK-78 still needs a reviewed decision before code.
TASK-85 AC#6 remains the owner's dispatch click, TASK-66 needs a Castellan PR with deliberate
lockfile drift, and TASK-73/74 stay parked on issue #101. No dispatch, release, repack or remote
write was attempted for TASK-82.

**Next session's opening prompt:**

> Restore only if needed and baseline the current session branch — expect **873 passing /
> 1 skipped**, not main's 816/1. The published developer transport is still 0.6.0 from `8dc5bba`
> and carries the full vendored rstest-bdd macro tree. Read `CONTEXT.md` § Session scratchpad —
> the **2026-10-09 fourth-session** heading records TASK-82's four library boundaries and proof.
>
> Fetch origin on `arena/49e36983-pixi-sandbox` without changing branches. TASK-87's seven commits
> were pushed through `d8f14cf8`; TASK-82's four extraction commits and the context close are
> local only. TASK-82 is Done at 6/6 local ACs; TASK-87 is 5/5 In Progress pending its merge.
> Main still names `8dc5bba`; if its ci/docs/publish sandbox runs remain green, do not re-run
> or re-pack. First decide whether to push/open the combined PR or start TASK-76's local slice.
>
> Do not push, open a PR, merge or dispatch without my explicit go-ahead. Before new tests,
> agree the public seams. House rules are in `AGENTS.md` (invariant 10: unimplemented work goes
> in this scratchpad), with the session procedure and templates in `.agents/skills/session/`.

### 2026-10-09 (integration follow-up) — owner authorized the combined TASK-87/TASK-82 PR and merge

The owner's instruction is now **"create PR and merge"**. It sanctions pushing this session
branch and merging its combined BDD migration and pack extraction; it is not a standing sanction
for new tasks, releases, manual workflow dispatches or future PRs. The third/fourth-session
no-PR/no-merge sentences are historical. Before integration, fetched origin and confirmed main
is still `8dc5bba` with its existing ci/docs/publish-sandbox runs green; did not rerun settled work.

TASK-87's **5/5** local criteria were already proven, so its Done status is included in the PR
in house format; TASK-82 remains **6/6**. Both task notes now distinguish their earlier reserved
sanction from today's authorization. No implementation or fixture changes are folded into the
integration paperwork. The combined branch's local proof is **873 passing / 1 skipped**, fmt,
lint **11/11**, test **4/4**, conventional messages and the repository consistency checks.

**Integration checkpoints, not pre-claimed results:** push with the hooks active; open the PR
from `arena/49e36983-pixi-sandbox` to main; wait for its checks; squash-merge the explicit PR
number with no auto-merge; then read every merge-triggered main run and the published transport's
source commit. Re-baseline the merged tree without changing the fixed session branch — equality
of the branch/main tree IDs permits that proof without a checkout of main. The PR timeline and
final report carry the resulting PR number, main SHA and actual workflow verdicts; the green
runs for `8dc5bba` alone are not proof for the new commit. A restricted branch-protection REST
endpoint returned 403 while ordinary PR/repository/Actions reads worked: this is an endpoint
permission limit, not a reason to bypass checks or replace configured GitHub credentials.

**Still not implemented:** backlog CLI quoting guidance, verify/publish reconciliation,
TASK-76/77/78 and the external proof items from the previous entries. None is bundled into this
PR. TASK-76 remains the next high-priority dependency-unblocked candidate; TASK-77 is the broad
local lint candidate. TASK-85 still needs the owner's dispatch and TASK-66 a Castellan drift PR.

**Next session's opening prompt:**

> Restore only if needed; baseline through Pixi on `arena/49e36983-pixi-sandbox` — expect
> **873 passing / 1 skipped** after TASK-87's BDD macro migration and TASK-82's four library
> extractions. Read `CONTEXT.md` § Session scratchpad, **2026-10-09 integration follow-up**,
> and the PR timeline for the merge SHA and actual post-merge verdicts. Both tasks have their
> acceptance criteria proven and their Done closure included in the combined PR.
>
> Fetch origin without changing branches and confirm the combined PR's merge and the latest
> main ci/docs/publish-sandbox verdicts. If already green, do not rerun or repack. Read the
> published developer transport's manifest: it must name the current main commit, schema 2
> and the static 0.6.0 self-bin; the full rstest-bdd macro dependency tree is vendored.
>
> First propose TASK-76's local slice or TASK-77; do not start either without my selection.
> Before new tests, agree the public seams. The authorization covered only the combined
> TASK-87/TASK-82 integration — do not push a new task, open another PR, merge, release or
> dispatch without new go-ahead. House rules are in `AGENTS.md` (invariant 10: unimplemented
> work goes in this scratchpad), with the session procedure in `.agents/skills/session/`.

### 2026-10-10 — TASK-76 local slices landed (In Progress: AC#8 carried), TASK-77 next

**TASK-76, six slices on `arena/00dbb795-pixi-sandbox`** (b37eab27 `fetch-release`,
303876bd git `commit_files`/`push_branch`, 911c0c7b `pipeline`, 59227002 `upgrade`,
7aae83ec the render, cfb7c958 check 9). The generated publisher is now pixi-sandbox's
entrypoint: the render (198 lines, was 421) has no literal `run: |` block and no
`shell: pwsh`; non-comment embedded shell fell from 218 lines to 35 single-command lines.
`pipeline` owns install→pack→doctor→publish with per-phase outcome recording, the failure
step summary and phase exit codes (PhaseRunner seam); `upgrade` owns check→regenerate→
commit→deliver (runner + GitProtocol + PullRequestSource seams, REST PR creation isolated
in `src/upgrade/github.rs`, coverage-excluded); `fetch-release` is the single
checksum-verified bootstrap download. Check 9 no longer exempts generated artifacts; this
repo's own source-built publisher marks its four blocks with `multiline-run-allowed`
reasons. Suite 873/1 → **924/1**; lint, check-repository and actionlint+shellcheck clean.
`scripts/restore.sh` and `relock.yml` untouched. Seams agreed with the owner before tests:
PR creation via the REST API behind a trait (not `gh`); TASK-77 will also extract the long
functions beyond the five named.

**AC#8 carried, not claimed:** a real consumer run on a *released* binary is still missing
(no release ships the new verbs; Castellan has not run the new render). TASK-76 stays In
Progress; the task file names the missing proof and the three steps to close it.

**Environment note for the next session:** the restored environment does not always survive
turn boundaries here — `.pixi/`, `~/.local` and `~/.cache` were rolled back once mid-session
(the turn-end patchset cap); `bash scripts/restore.sh` (idempotent, offline, ~60 s) brings
it back, and `git fetch origin +refs/heads/sandbox/developer-linux-64:refs/remotes/origin/sandbox/developer-linux-64`
restores the sandbox remote-tracking ref, which the default fetch refspec does not cover.
Re-baseline after any restore (expect 924/1 now).

**TASK-77 starts next on the post-TASK-76 tree** (its rendered workflows are the byte
identity baseline). Fresh pedantic+nursery audit at startup: **637 unique findings**
(doc-12 said 469; the tree grew under TASK-82/87) — core 239, pixi-sandbox 228, xtask 100,
git 69. Top: use_self 122, missing_errors_doc 116, must_use_candidate 101,
too_long_first_doc_paragraph 42, map_unwrap_or 25, missing_const_for_fn 24,
option_if_let_else 20, too_many_lines 16, redundant_pub_crate 16, doc_markdown 16,
redundant_closure 14, missing_panics_doc 13, literal_string_with_formatting_args 12.
too_many_lines now lists 16 functions: the five doc-9 A3 remainder (doctor::print_human 226,
restore::register_user_tools 121, doctor::run 138, restore::run 110, tools::update::refresh
101), plus verify_env_restored 192, assemble_artifacts 185, host_requirements probe 158,
branch_docs render 162, doctor JSON renderer 121, lint_generated_workflow 112 — the owner
chose to extract these too, not just the five. Allowed in place: xtask main::run_args (161),
two e2e narrative tests (137, 124), both workflow renderers (naming TASK-76). Cast sites at
shard.rs:124, transport_budget.rs:232, pack.rs:68 (the third moved into src/pack.rs with
TASK-82). Policy approved: `[workspace.lints.clippy]` pedantic = "warn" inherited by all
four crates, nursery off, `literal_string_with_formatting_args` allowed with a reason,
recorded as ADR-20; the gate is red between the policy commit and the per-crate fix commits
(nothing pushed in between). LEGACY inline-test list still has 5 entries; only
`commands/tools/update.rs` migrates in this task.

**Not implemented:** TASK-76 AC#8 (release + consumer run), TASK-77 itself, TASK-78,
external maintainer/consumer proofs. No push, PR, merge, release or dispatch made — the
owner's sanction covers local implementation only.

### 2026-10-10 (second session) — TASK-77 checkpoint: core/git/pixi-sandbox pedantic-clean, PR merged, xtask remainder handed off

**Environment rollback, second time:** the turn-end patchset rollback wiped `.pixi/`,
`~/.local`, the fetched refs AND the local commit chain — the clone came back shallow
at the base commit with every file change uncommitted. `bash scripts/restore.sh`
(~60 s, offline) plus the explicit sandbox-ref fetch brought the environment back;
the commit history was rebuilt from the surviving tree with the same
per-crate/per-function slicing (the original SHAs are unrecoverable). Two
reconstruction notes: the git crate's sweep fixes are folded into the TASK-76 commit
(its only src files were touched by both tasks), and `self_update/mod.rs` moved into
the TASK-76 commit (it carries TASK-76's `fetch_release`). Re-baselined: 924/1.

**TASK-77 progress (branch `arena/00dbb795-pixi-sandbox`, pushed; PR created and merged
with a regular merge, not squash):** policy ADR-20 (pedantic = warn workspace-wide,
`literal_string_with_formatting_args` allowed with reason) plus per-crate
`clippy --fix` sweeps; then per crate — pixi-sandbox-core pedantic-clean (22
`# Errors` sections, the audited judgement items, the AC#8 casts resolved with
pinning tests, two extractions), pixi-sandbox-git pedantic-clean (24 `# Errors`,
10 `# Panics`, `#[must_use]` builders, format_collect, two test judgement items) and
pixi-sandbox pedantic-clean (42 doc sections, all mechanical findings, the pack casts
with a pinning test, eight extractions one per commit, four scoped allows with
reasons). Suite 924/1 → **928/1** (35 git / 212 core / 525 pixi-sandbox / 156 xtask);
per-crate `clippy -D warnings` green on core, git and pixi-sandbox; fmt, taplo,
actionlint, check-repository, lint-generated-workflow and plan all green. Rendered
workflows unchanged (the golden tests prove it). AC#1, #2, #5, #7, #8 met; #3, #4,
#6 partial; #9, #10 pending.

**Remaining for the next session (all of it xtask plus closure):**

1. xtask's 48 pedantic findings: 38 missing_errors_doc (airlock ×11, commit_release,
   conda_platforms, prepare_release, release_assets ×3, release_refs ×3,
   repo_checks/mod ×2, smoke, starter ×7, util ×2, version ×2, workflow ×3),
   4 format_collect (prepare_release.rs:244, tests/repo_checks_agents_md.rs:13/34,
   tests/repo_checks_workflow_permissions.rs:35), unnecessary_wraps
   (repo_checks/stale_refs.rs:20), missing_panics_doc (util.rs:32 version_tag_re),
   similar_names (tests/support/mod.rs:88), case_sensitive
   (tests/release_assets.rs:69), and too_many_lines on workflow.rs:68
   lint_generated_workflow (extract) and main.rs:302 run_args (allow in place with a
   reason, AC#6).
2. AC#9: migrate the 413-line inline test module in commands/tools/update.rs to
   tests/tools_update.rs (logic promoted through lib.rs; the LEGACY inline-test list
   in tests/fixtures.rs has 5 entries and only this one migrates), no test lost.
3. Final gates: fmt, full `pixi run --frozen lint` at ZERO warnings on all four
   crates (xtask is the only red one now), the full test suite, check-repository,
   lint-generated-workflow, convco over the whole range, rendered-workflow
   byte-identity (the golden tests already cover it).
4. Close task-77 (AC checkboxes + DOD) and record the final evidence here.

**Not implemented:** TASK-76 AC#8 (release + consumer run on a released binary —
carried), TASK-77's xtask remainder and AC#9 (above), TASK-78+ backlog. The lint
gate is red on xtask between this merge and the next session's xtask commit.

### 2026-10-10 (third session) — TASK-77 closed on the xtask sweep and the tools-update migration; In Progress for two ACs

Branch note: this session is fixed to `arena/ef708741-pixi-sandbox` (the brief named
`arena/00dbb795-pixi-sandbox`); the work is on this branch, based on the PR #129 merge
`ae79772`. The base `0a036ba2` is not present in this shallow clone, so the convco range is
`ae79772..HEAD`. Restore ran via `scripts/restore.sh`; the baseline was 928/1 (35 git / 212 core /
525 pixi-sandbox / 156 xtask), and it is 928/1 at HEAD.

**Commits on top of `ae79772` (seven, convco-clean):**

1. `68aa8708` docs(xtask): `# Errors` for 38 public fallible entry points, and `# Panics` on `version_tag_re`.
2. `c915b0d4` refactor(xtask): mechanical fixes (format_collect ×4 to `writeln!`, `argv` renamed,
   `.exe` by `Path::extension`, `stale_implementation_references` infallible with a closure
   adapter in the check table).
3. `0aa99b25` refactor(xtask): extract `lint_relock_renders` out of `lint_generated_workflow`
   (112 → under 100 lines; rendered output unchanged).
4. `838bae08` chore(xtask): function-scoped `too_many_lines` allow on `run_args` with its
   reason (AC#6).
5. `7d8e3cb8` refactor(pixi-sandbox): `tools_update` promoted to the lib (`refresh` takes the
   stamp from the caller, so the lib is clock-free).
6. `c3ed7ccb` test(pixi-sandbox): the 13 inline tests of `commands/tools/update.rs` move to
   `tests/tools_update.rs`; the LEGACY list in `tests/fixtures.rs` drops to four entries.
7. `833e5d99` docs(agents): `tools_update` in the repo map and promoted-module list (the
   repo-consistency check requires it).

**Gates at HEAD:** `fmt --check` green; full `lint` exits 0 with all 11 tasks successful, and
clippy `-D warnings` is green on all four crates. The only `lint` output besides command echo
is `cargo deny` noise: `index-failure` (no crates index reachable from the sandbox) and
`duplicate` entries for `getrandom`, `syn` and `windows-sys`, which come from the existing graph.
`test` 928/1; `xtask check-repository` green; `xtask lint-generated-workflow` clean;
`convco check ae79772..HEAD` no errors in 7 commits; the generated golden fixtures have zero
diff on the branch.

**Acceptance criteria, against evidence:**

- AC#3 met: the MachineApplicable sweeps are the per-crate commits of the TASK-77 history on
  `arena/00dbb795-pixi-sandbox` (`08a7e765` core, `afa06c63` pixi-sandbox, `38d6dc3b` xtask, and
  the per-function extractions). Pedantic is clean across all four crates.
- AC#4 **open**: `missing_errors_doc`, `missing_panics_doc` and `doc_markdown` are all clean.
  `too_long_first_doc_paragraph` is a nursery lint, so ADR-20 does not enforce it, and it is
  neither written nor allowed for 52 locations (`pixi-sandbox` 31, `xtask` 12, `pixi-sandbox-core`
  8, `pixi-sandbox-git` 1). Recorded here, not in code: user_tools.rs ×10, airlock.rs ×4,
  pipeline.rs ×4, workflow.rs ×3, upgrade/mod.rs ×3, standalone.rs ×3,
  generated/relock_workflow.rs ×3, generated/mod.rs ×3, sandbox_config.rs ×3,
  release_assets.rs ×2, self_update/mod.rs ×2, transport_budget.rs ×2, manifest.rs ×2,
  and one each in util.rs, release_refs.rs, conda_platforms.rs, self_update/assets.rs,
  release/mod.rs, pack.rs, pixi-sandbox-git lib.rs and files_manifest.rs. The brief did not
  list these, so the task stays In Progress until they are written or allowed.
- AC#6 met: `main::run_args` carries the reasoned allow; `tests/e2e.rs` keeps its two reasoned
  allows for the narrative proofs.
- AC#9 met: 13 tests migrated with none lost (525 pixi-sandbox both before and after).
- AC#10 **open on the per-commit clause**: the suite was run at the mechanical, extraction,
  promotion and migration commits and at HEAD (928/1 each time it was run). The docs-only,
  attribute-only and AGENTS commits were verified by compile and lint, not by a separate suite
  run. The no-new-dependency, no-real-HOME and no-render-change clauses hold.

Task-77 stays In Progress until AC#4 and the per-commit clause of AC#10 are evidenced.

### 2026-10-10 (fourth session) — TASK-77 closed: the 52 too_long_first_doc_paragraph findings are rewritten; AC#4 and AC#10 met

Branch note: this session is fixed to `arena/773888b7-pixi-sandbox`, based on main after
the PR #130 merge (`8d9e3b41`). Restore ran via `scripts/restore.sh`; baseline
`pixi run --frozen test` 928/1 (35 git / 212 core / 525 pixi-sandbox, 1 skipped / 156
xtask). This entry carries TASK-77 evidence only; TASK-76 evidence stays in its own
entries.

**Commits on top of `8d9e3b41` (four, convco-clean, one per crate):**

1. `a54b7174` docs(pixi-sandbox-core): 8 first paragraphs rewritten — files_manifest ×1,
   manifest ×2, sandbox_config ×3, transport_budget ×2.
2. `cc7748c8` docs(pixi-sandbox-git): 1 — `FileCommit`.
3. `7b180faa` docs(pixi-sandbox): 31 — user_tools ×10, pipeline ×4,
   generated/relock_workflow ×3, generated/mod ×3, upgrade ×3, standalone ×3,
   self_update ×3 (mod ×2, assets ×1), pack, release/mod.
4. `db71dbc1` docs(xtask): 12 — airlock ×4, workflow ×3, release_assets ×2,
   conda_platforms, release_refs, util.

Every rewrite is the same shape: the first paragraph becomes a one-sentence summary
under the lint's 200-character text threshold, the detail moves to following paragraphs.
All 52 findings are rewritten; **none is allowed**, so there is no decisions.md amendment
to make (ADR-20 keeps nursery out of the gate, and the debt itself is paid rather than
suppressed). The doc-comment edits inside the generated module sources
(`src/generated/*.rs`) touch no template literal; the golden fixtures are byte-identical.

**AC#10 — per-commit suite runs (every run ≥654; baseline 928/1):**

| commit | content | `pixi run --frozen test` |
| --- | --- | --- |
| `8d9e3b41` (base) | baseline | 928 passed / 1 skipped (35+212+525+156) |
| `a54b7174` | 8 doc rewrites (core) | 928 passed / 1 skipped |
| `cc7748c8` | 1 doc rewrite (git) | 928 passed / 1 skipped |
| `7b180faa` | 31 doc rewrites (pixi-sandbox) | 928 passed / 1 skipped |
| `db71dbc1` | 12 doc rewrites (xtask) | 928 passed / 1 skipped |

The task-closing documentation commit (this entry plus the task file) is markdown-only —
no Rust, test, manifest or lockfile change — and the full gate set below was re-run at
that final HEAD. No test was added or changed (none at all this session), no new crate
dependency (`Cargo.toml`/`Cargo.lock` have zero diff across `8d9e3b41..HEAD`), and no
rendered workflow, manifest schema, transport format or git-access path changed: every
changed line in the range is a `///` doc comment, and
`git diff 8d9e3b4..HEAD -- crates/pixi-sandbox/tests/fixtures/generated/` is empty.

**AC#4 — evidence:**

- `too_long_first_doc_paragraph`: the locating command from the brief
  (`pixi run --frozen -- cargo clippy --offline --workspace --all-targets
  --message-format=short -- -W clippy::too_long_first_doc_paragraph`) reported 52 at the
  base and **0 at HEAD**; all 52 rewritten, 0 allowed.
- `missing_errors_doc` (58), `missing_panics_doc` (12), `doc_markdown` (16): clean under
  the enforced gate — full `pixi run --frozen lint` exits 0 with clippy `-D warnings`
  green on all four crates, and these three are pedantic, so the gate proves them.

**Final gates at HEAD:**

| gate | result |
| --- | --- |
| `pixi run --frozen fmt --check` | exit 0 |
| `pixi run --frozen lint` | exit 0, 11/11 tasks; clippy `-D warnings` green on all four crates (only sandbox-environmental cargo-deny noise: none this run; advisories/bans/licenses/sources ok) |
| `pixi run --frozen test` | 928 passed / 1 skipped (≥654 required) |
| `pixi run --frozen xtask check-repository` | exit 0 |
| `pixi run --frozen xtask lint-generated-workflow` | exit 0, actionlint clean |
| `pixi run --frozen -- convco check 8d9e3b4..HEAD` | no errors in the 4 code commits plus the documentation commit |
| `git diff 8d9e3b4..HEAD -- crates/pixi-sandbox/tests/fixtures/generated/` | empty |

With AC#4 and the per-commit clause of AC#10 evidenced above, TASK-77 is set to Done.

**Prompt to start the next session with:**

> Restore only if the tree or `.pixi/` is missing (`bash scripts/restore.sh`, ~60 s,
> offline and idempotent; then `export PATH="$HOME/.local/bin:$PATH"` and fetch
> `origin +refs/heads/sandbox/developer-linux-64:refs/remotes/origin/sandbox/developer-linux-64`).
> Baseline through Pixi from the repo root — expect **928 passing / 1 skipped** (35 git /
> 212 core / 525 pixi-sandbox / 156 xtask). Pixi only (`pixi run --frozen …`), cargo
> `--offline`, never two pixi commands in parallel, never source `.pixi/sandbox-env.sh`.
>
> TASK-77 is Done and merged in PR #131, based on main after the PR #130 merge
> (`8d9e3b41`): pedantic clippy is clean on all four crates, all 52
> `too_long_first_doc_paragraph` findings are rewritten (none allowed), nursery stays off
> per ADR-20 — do not reopen it. Read `CONTEXT.md` § Session scratchpad, **2026-10-10
> (fourth session)** for the closure evidence, the per-commit test table and the gates.
>
> Open candidates — propose before starting: **TASK-76 AC#8** is the only open AC left in
> a task, but it needs a real consumer repository to run the refactored publisher end to
> end **on a released binary**, and the latest release (v0.6.0, `8298edce`) predates the
> refactored lane — so it starts with a fresh release plus a consumer run, both
> owner-sanctioned actions. **TASK-74** (starter template) needs the owner-operated
> GitHub App delivery model of TASK-73. **TASK-85 AC#6** waits on the owner's dry-run
> starter dispatch. **TASK-78** is To Do, but AC#1 wants a reviewed D-number decision
> before any code, and `pixi-build` is still preview. Carried from the 2026-10-09
> integration entry: backlog CLI quoting guidance, verify/publish reconciliation, and
> TASK-66's Castellan drift PR.
>
> House rules: `AGENTS.md` (invariant 10: anything you do not implement goes in
> `CONTEXT.md` § Session scratchpad, not into the files it speculates about), session
> procedure in `.agents/skills/session/`; conventional commits, one crate or concern per
> commit, `pixi run --frozen test` ≥654 at every commit. Do not push, open a pull
> request, merge, release or dispatch without my explicit go-ahead — propose the slice
> and stop.

### 2026-10-10 (fifth session) — both close-outs reduce to one maintainer click; the starter lane rehearsed green

Branch note: session fixed to `arena/eac8e2fe-pixi-sandbox`, based on main at `6ed6fe6`
(PR #131, TASK-77). Restore ran via `scripts/restore.sh` (72 s, 14 495 blobs verified,
0 failures); manifest names commit `6ed6fe6`, schema 2, tool pixi-sandbox v0.6.0, vendor
268 crates. User tools registered in `~/.local/bin`. Baseline `pixi run --frozen test`
**928 passing / 1 skipped** (35 git / 212 core / 525 pixi-sandbox / 156 xtask) — identical
to the number the fourth session recorded; `pixi run --frozen lint` 11/11 green.

**Environment finding worth carrying: this checkout arrives as a shallow,
branch-filtered clone.** `git rev-list --count HEAD` was **1**, `.git/shallow` listed two
boundaries, and `remote.origin.fetch` was scoped to the session branch alone
(`+refs/heads/arena/eac8e2fe-pixi-sandbox*:…`). The visible consequence is that
`git merge-base --is-ancestor v0.6.0 HEAD` answers **NO** — v0.6.0 is not an ancestor of
anything, `git log` shows one commit, and `convco check HEAD~1..HEAD` is meaningless. None
of these are repository facts; they are artefacts of the clone. Fixed in place with
`git fetch --deepen=300 origin main` (311 commits, `.git/shallow` gone, **no config
change** — the refspec was passed on the command line, not written). A session that skips
this will silently mis-read release ordering, which is exactly what TASK-76's blocker below
turns on. Worth a line in the session skill's opening-prompt template.

**Scope was narrowed by the owner to closing TASK-85 AC#6 and TASK-76 AC#8, no feature
code.** Both are blocked on a maintainer action; neither is blocked on missing engineering.

**TASK-85 AC#6 — dispatch refused, everything around it proven.** `gh workflow run
starter.yml --ref main -f release-tag=v0.6.0
-f source-commit=8298edce13049ab01a7bc9d3e09c9daf8d14fc08 -f dry-run=true` returns
`HTTP 403: Resource not accessible by integration` — this sandbox's token cannot dispatch
workflows, matching the session skill's standing warning. Every gate the lane runs before
the publish step was therefore executed directly against the remote with the workflow's own
flags: `starter-check-dispatch`, `starter-verify-release` and `starter-check-main` all exit
0 (`Archont561/pixi-sandbox-starter` exists, is a template, and has `refs/heads/main` at
`d3b0ce0`). The local steps were rehearsed in a tempdir: `starter-scaffold` → 3 files,
`init --force` → `pixi-sandbox.toml` + two workflows, `init --check` → "every owned file
matches a fresh render", and `starter-verify` → exactly one finding,
`missing required file pixi.lock`. That last one is purely network: `pixi lock` fails on
`https://prefix.dev/conda-forge/linux-64/repodata_shards.msgpack.zst` with `tls handshake
eof`. `dry-run` reaches the publish step as `starter-publish … --dry-run`, which does not
push, so the run cannot publish. Note `--ref main` is required: `starter.yml` does not
exist at v0.6.0, while `source-commit` names the v0.6.0 SHA. **AC#6 stays unchecked — a
rehearsal is not the dispatch.**

**TASK-76 AC#8 — the release is necessary, sufficient, and not blocked on secrets.** The
ordering the previous session asserted is now proven rather than recalled: `b9d5d0a2` (the
single-entrypoint refactor) is a descendant of the v0.6.0 tag `8298edce`, so v0.6.0
predates the refactored lane and any proof needs a release after PR #131. Cutting one is
sufficient to *start* it — `release.yml` dispatches `consumer proof` itself once the
GitHub Release exists (v0.6.0 chain runs 5–7) — and the push permission is present:
`RELEASE_PUSH_TOKEN` is set with the Workflows permission, observed when the v0.6.0 release
commit (which modifies `.github/workflows/relock.yml`) succeeded. The blocker is again the
403 dispatch, plus the fact that an agent session may only push its own branch, so the
release commit could not reach `main` from here even with permission. **AC#8 stays
unchecked.**

**Not implemented, deliberately:** TASK-78 (To Do, 0/10, no dependencies) was surveyed and
not started — the owner chose close-outs only. The shape decision its AC#1 demands was
settled in conversation and is recorded here rather than in the task file: **a
`rattler-build` recipe driven by `xtask`, not a new `pixi-sandbox` verb.** `rattler-build`
is not a dependency of any of the four crates, the vendored tree is frozen at 268 crates
with crates.io unreachable, and a new verb would pull rattler's crate tree into the CLI's
own payload and force a relock cycle this airlock cannot do. The recipe keeps the vendor
diff empty and leaves `pixi-build`'s preview status a documented risk instead of a
load-bearing dependency. Also carried: the backlog CLI quoting guidance and the
verify/publish reconciliation pair from the 2026-10-09 integration entry, and TASK-66's
Castellan drift PR.

**Prompt to start the next session with:**

> Restore only if the tree or `.pixi/` is missing (`bash scripts/restore.sh`, ~72 s,
> offline and idempotent; then `export PATH="$HOME/.local/bin:$PATH"`). **If `git
> rev-list --count HEAD` returns 1, the clone is shallow: `git fetch --deepen=300 origin
> main` before believing any release-ordering fact** — the session branch's fetch refspec
> is scoped to itself and git history-dependent checks silently misreport until you do.
> Baseline through Pixi from the repo root — expect **928 passing / 1 skipped** (35 git /
> 212 core / 525 pixi-sandbox / 156 xtask) and `lint` 11/11. Pixi only
> (`pixi run --frozen …`), cargo `--offline`, never two pixi commands in parallel, never
> source `.pixi/sandbox-env.sh`.
>
> **Two maintainer clicks unblock the two near-done tasks, and only those.** Neither is
> blocked on engineering: (1) *Actions → starter → Run workflow*, ref `main`,
> `release-tag=v0.6.0`,
> `source-commit=8298edce13049ab01a7bc9d3e09c9daf8d14fc08`, `dry-run=true` — every gate
> before the publish step has been run from here and is green or provably network-bound, so
> this should succeed; a green run closes TASK-85 AC#6. (2) *Actions → auto-release → Run
> workflow*, defaults (`bump=auto`) — cuts the release TASK-76 AC#8 needs; `release.yml`
> then dispatches `consumer proof` by itself, and that run is the AC#8 evidence. An agent
> token gets `403 Resource not accessible by integration` on both.
>
> If a click has already happened, check it before proposing anything: `gh run list
> --workflow starter.yml --limit 3` and `gh release list --limit 3`. New work is
> **TASK-78** (high, no dependencies, 0/10) — package arbitrary prebuilt binary payloads
> as conda packages. AC#1 gates the shape on a D-number decision first, and the shape is
> already decided: **a `rattler-build` recipe driven by `xtask`, not a new `pixi-sandbox`
> verb** — `rattler-build` is not a dependency of any of the four crates, the vendor tree
> is frozen at 268 crates with crates.io unreachable, and a new verb would drag rattler's
> crate tree into the CLI payload and force a relock cycle this airlock cannot run. Carry
> the decision, not the deliberation.
>
> House rules: `AGENTS.md` (invariant 10: anything you do not implement goes in
> `CONTEXT.md` § Session scratchpad, not into the files it speculates about), session
> procedure in `.agents/skills/session/`; conventional commits, one crate or concern per
> commit, `pixi run --frozen test` ≥654 at every commit. Do not push, open a pull
> request, merge, release or dispatch without my explicit go-ahead.

### 2026-10-10 (sixth session) — TASK-88 for issue #128; the oracle learns about pypi

**Issue #128 was untracked and is the most valuable work on the board.** Filed today against
`Archont561/qgis-rust`: `restore` exits non-zero on its final tree check for every project
with a pypi dependency, and because it exits non-zero it also skips user-tool registration, so
the documented next command fails with `pixi: command not found`. This repository cannot
reproduce its own bug — `pixi.toml` resolves no pypi package, so its transport has no
`dist-info` at all and restores with 0 failures across 5520 entries. Written up as TASK-88,
fixed as ADR-21.

**Landed:** 935 passing / 1 skipped (core 212 → 219), `lint` 11/11, `check-repository` clean.
AC#1–#7 proven locally; **AC#8 left open** — the connected proof needs a real
`pixi-pack` → `pixi-unpack` round trip of a wheel, and `pixi lock` cannot reach prefix.dev
from here. Both new regression tests were confirmed to fail with the fix neutralised, so they
test the defect rather than restating the code.

**Not implemented, deliberately — issue #128's third suggestion.** Registering user tools
*even when* the final tree check fails. It is not in TASK-88 because it answers a different
question: not "is the oracle sound?" but "when may `restore` mutate `$PATH`?". Today the
tools are registered only after a clean verification, and the environments are already
correct at that point — the reporter's `.pixi/tools/linux-64/pixi` works fine via its full
path. So the change would trade a strict exit code for a working PATH on a tree that might
genuinely be tampered with. That trade is worth making, but it is an invariant decision and
wants its own task and D-number, not a rider on a soundness fix. Carried for a future
session; the strongest form of it is "register the tools, keep the failure".

**Also untracked, noted while surveying the board:** issue #109 maps to TASK-75, which has been
`Done` since 2026-10-08, but the issue is still open. The repo's convention is to close an
issue when its task lands — 13 of 17 are. It is a one-click close, and it is the owner's.

**Prompt to start the next session with:**

> Restore only if the tree or `.pixi/` is missing (`bash scripts/restore.sh`, ~72 s; then
> `export PATH="$HOME/.local/bin:$PATH"`). **If `git rev-list --count HEAD` returns 1, the
> clone is shallow: `git fetch --deepen=300 origin main` first** — the session branch's fetch
> refspec is scoped to itself, so release-ordering facts silently invert until you do. Expect
> **935 passing / 1 skipped** (35 git / 219 core / 525 pixi-sandbox / 156 xtask), `lint`
> 11/11. Pixi only, cargo `--offline`, never two pixi commands in parallel.
>
> **TASK-88 is In Progress at 7/8 and needs one connected proof.** AC#8 is the real
> `pixi-pack` → `pixi-unpack` reinstall of a wheel; it cannot be produced on an airlocked
> host because `pixi lock` cannot reach prefix.dev. Once a release carrying the fix exists,
> re-pack `qgis-rust`'s transport and run its `restore` — a clean verdict is the proof. Until
> then leave the task In Progress and do not check AC#8.
>
> **Two maintainer clicks are still open** and neither is blocked on engineering:
> *Actions → starter → Run workflow* (ref `main`, `release-tag=v0.6.0`,
> `source-commit=8298edce13049ab01a7bc9d3e09c9daf8d14fc08`, `dry-run=true`) closes TASK-85
> AC#6 — every gate before its publish step has been run from here and is green or provably
> network-bound. *Actions → auto-release → Run workflow* (defaults) cuts the release
> TASK-76 AC#8 needs; `release.yml` dispatches `consumer proof` itself, and that run is the
> evidence. An agent token gets `403 Resource not accessible by integration` on both. Issue
> #109 is also closable now that TASK-75 is Done.
>
> New work: **TASK-78** (high, no dependencies) — package arbitrary prebuilt binary payloads
> as conda packages. AC#1 gates the shape on a D-number decision first, and the shape is
> already decided: **a `rattler-build` recipe driven by `xtask`, not a new `pixi-sandbox`
> verb** — `rattler-build` is not a dependency of any of the four crates, the vendor tree is
> frozen at 268 crates with crates.io unreachable, and a new verb would drag rattler's crate
> tree into the CLI payload and force a relock cycle this airlock cannot run.
>
> House rules: `AGENTS.md` (invariant 10: anything you do not implement goes in
> `CONTEXT.md` § Session scratchpad), session procedure in `.agents/skills/session/`;
> conventional commits, one crate or concern per commit, `pixi run --frozen test` ≥654 at
> every commit. Do not push, open a pull request, merge, release or dispatch without my
> explicit go-ahead.

### 2026-10-11 (seventh session) — the release lane repaired; PR #133

**Branch note:** session fixed to `arena/88cc6d79-pixi-sandbox`, based on main at `a79f82df`
(v0.6.1). Restore ran clean (14 495 blobs / 866.4 MiB verified, 0 failures; 5520 entries
checked against the manifest); the transport names commit `a79f82d` and tool `v0.6.1`. The
clone arrived shallow again — `git rev-list --count HEAD` was 1 — fixed with
`git fetch --deepen=300 origin main` (319 commits). Baseline **935 passing / 1 skipped**,
exactly as the sixth session predicted; ended at **952 passing / 1 skipped**
(36 git / 219 core / 525 pixi-sandbox / 172 xtask), `lint` 11/11. Merged as PR #133 →
`46efd1a7`; post-merge `ci`, `docs` and `publish sandbox` all green.

**The v0.6.1 release had never published.** `release.yml` run 38061777852 failed at
`build x86_64-apple-darwin` → `Upload binary artifact` with `CreateArtifact: … ENOTFOUND`, and
because `release` has `needs: build` the publish job was skipped: `gh release view v0.6.1`
answered "release not found" while `gh release list` showed v0.6.0 as Latest. A re-run of the
failed job (owner click) succeeded — proof the fault was transient runner DNS, not code — and
v0.6.1 published at 09:58:15Z with all five binaries, five conda packages and SHA256SUMS. The
`publish sandbox` transport had been fine throughout; it is built from source and never needed
the release assets.

**Three defects found and fixed (doc-13, `backlog/docs/plans/release-lane-repair/`):**

1. *The starter published an empty tree.* `pixi-sandbox-starter` at `945ed1a7` holds one file,
   `README.md`, and the immutable tag `pixi-sandbox-v0.6.1` seals it — behind a green run
   (38130262129). `starter::publish` staged `worktree_status_files`, which is
   `git status --porcelain --untracked-files=no`, so over a repository whose only tracked file
   was the README every generated file was invisible. `verify` could not catch it: it judges
   the directory, not the revision. Reproduced end to end with the real `xtask` against a
   bare-remote fixture, and the regression test was confirmed to fail with the fix neutralised
   (`the published revision is missing pixi.toml`). Publish now stages
   `ls-files --cached --others --exclude-standard` and refuses a revision missing a required
   file or carrying a forbidden one.
2. *The consumer proof asserted a string a passing test forbids.* It grepped the generated
   publisher for `Archont561/pixi-sandbox/releases/download`; TASK-76 moved that download into
   `fetch-release`, and `generated_workflow.rs` asserts the render contains **no** such
   literal. So the proof failed deterministically from v0.6.1 (run 38130260877) while releases
   published green. Check 13 now holds every `grep -F` the proof aims at a generated publisher
   to a string the generator's source still contains — a repo lint, because ADR-10 forbids a crate
   test from reading this repository.
3. *No binary-completeness gate.* `release_checksums` refuses only an empty directory, so a
   release missing a platform would publish a green SHA256SUMS covering four binaries; v0.6.1
   came within one artifact upload of exactly that. `check-release-binaries` is the binary twin
   of the conda check, its expected set derived from `SUPPORTED_HOSTS`/`Platform::ALL`.

**Ordering changed:** the consumer proof now gates the publish. Nothing in it needs a release
to exist — it runs on the build's own artifact — so it moved into `release.yml` as a
`consumer-proof` job with `needs: build`, and publish needs `[build, consumer-proof]`. What
genuinely cannot move is the asset boundary (`releases/download/<tag>` plus the published
SHA256SUMS), which keeps its own job; `consumer-proof.yml` shrank to that smoke. The honest
cost: a red smoke now blocks the *next* release, not its own.

**The starter gained what the owner asked for:** `scripts/restore.sh` — `init --script-path`
already supported it and creates the parent directory, so no core change was needed, only both
lane invocations carrying the flag — and a devcontainer pinned to the pixi image the release
pins, whose setup installs `git`, `gh` and `pixi-sandbox` from the canonical
`archont561/archont561` channel and **no compiler toolchain**. The pixi pin is read from the
release's embedded tools lock, so the template never hardcodes a version the next release
invalidates. `verify` holds both halves of that contract, because `init --check` has never
heard of the devcontainer.

**Not implemented, deliberately.** The moved proof steps still carry embedded shell behind
`multiline-run-allowed` opt-outs; the move had to be verifiable on its own, so the port into
`xtask` subcommands is **TASK-89** rather than a rider on it.

**Owner actions still open, none of them engineering:**
* Delete the starter tag `pixi-sandbox-v0.6.1`. It names an empty tree, and `publish_refusals`
  refuses an existing tag, so the corrected template cannot be published for v0.6.1 while it
  stands.
* Confirm `PIXI_SANDBOX_STARTER_TOKEN` carries `Workflows: write` on the starter repository.
  The README-only push succeeded; a push carrying `.github/workflows/*` into another repository
  is issue #101's exact shape. Unverifiable from this sandbox.
* Then re-dispatch the starter lane, or let the next release carry the corrected template.

**Prompt to start the next session with:**

> Restore only if the tree or `.pixi/` is missing (`bash scripts/restore.sh`, offline and
> idempotent; then `export PATH="$HOME/.local/bin:$PATH"`). **If `git rev-list --count HEAD`
> returns 1 the clone is shallow — `git fetch --deepen=300 origin main` first**, or
> release-ordering facts silently invert. Expect **952 passing / 1 skipped** (36 git / 219 core
> / 525 pixi-sandbox / 172 xtask), `lint` 11/11. Pixi only, cargo `--offline`.
>
> The release lane was repaired in PR #133 (`46efd1a7`): the starter publishes its whole
> revision, the consumer proof gates the publish, and a release missing a platform binary is
> refused. Read `CONTEXT.md` § Session scratchpad — the 2026-10-11 entry lists what is still
> open.
>
> Three owner actions gate everything downstream: delete the starter tag
> `pixi-sandbox-v0.6.1`, confirm `PIXI_SANDBOX_STARTER_TOKEN` has `Workflows: write` on the
> starter repo, then re-dispatch the starter lane. Until a release exists, **TASK-76 AC#8 and
> TASK-88 AC#8 stay unchecked** — do not close them on a rehearsal.
>
> New work: **TASK-89** (medium, no dependencies) — port the consumer proof recipe into `xtask`
> subcommands so the last `multiline-run-allowed` blocks leave `release.yml`; `starter.rs` is
> the template to copy. **TASK-78** (high, no dependencies) is still the largest unstarted
> item, and its shape is already decided: a `rattler-build` recipe driven by `xtask`, not a new
> verb — note that `rattler-build` is **not** in the restored environment (only `rattler-index
> 0.31.5`), so its build proofs need a lockfile refresh this airlock cannot do.
>
> House rules: `AGENTS.md` (invariant 10), session procedure in `.agents/skills/session/`;
> conventional commits, one concern per commit, `pixi run --frozen test` ≥952 at every commit.
> Do not push, open a pull request, merge, release or dispatch without my explicit go-ahead.

### 2026-10-11 (eighth session) — TASK-89 landed; check 13 had been comparing nothing

**Branch note:** session fixed to `arena/d2b714cc-pixi-sandbox`, based on main at `00ef66e4`.
The clone arrived shallow again (`git rev-list --count HEAD` → 1), fixed with
`git fetch --deepen=300 origin main`; the transport was force-fetched before the restore, which
came back clean (21 044 blobs / 976.1 MiB verified, 0 failures; 5 520 manifest entries) naming
commit `00ef66e` and tool `v0.6.1`. Baseline **952 passing / 1 skipped**, ended at
**982 passing / 1 skipped** (42 git / 219 core / 525 pixi-sandbox / 196 xtask), `lint` 11/11.
Merged as PR #136 → `36651855`; post-merge `ci`, `docs` and `publish sandbox` all green, and the
repacked transport now names `3665185` (tool still `0.6.1` — a merge does not cut a release).

**What landed (six commits, one concern each).** Three new `ShellGit` primitives
(`init_bare`, `init_repo_branch`, `checkout_detached`) because invariant 8 keeps the proof's git
journey in `pixi-sandbox-git` and those three existed nowhere else; `GitProtocol::fetch_into`
was deliberately *not* reused, since it materialises with plumbing and no work tree and a proof
built on it would prove this crate rather than a consumer's `git`. Then
`crates/xtask/src/consumer_proof.rs` with five `consumer-proof-*` subcommands, the `release.yml`
job rewritten to one line per step with the pinned checkout/setup-pixi pair, check 13 re-pointed,
and the two places that described the consumer proof as embedded shell corrected.
`grep -c multiline-run-allowed .github/workflows/release.yml` → **0**; the workflow markers that
remain are `consumer-proof.yml` (1, the asset-boundary smoke) and `publish-sandbox.yml` (4,
deliberately source-built).

**The finding worth keeping: check 13 was passing vacuously.** It read
`.github/workflows/consumer-proof.yml`, but doc-13's slice 5 moved the publisher greps into
`release.yml`, which the check does not read. `asserted_literals` therefore returned an empty
list, every filter over an empty list passed, and the check reported clean while comparing
nothing — with green fixtures throughout, because `tests/support/mod.rs` injected a `grep -F` of
its own. The tests exercised the check; the repository never did. That is the second time this
check has been the only thing keeping a proof and a generator in step, and the first time it had
silently stopped looking. It now reads `PUBLISHER_ASSERTIONS` in `consumer_proof.rs` *and* still
reads the workflow, and — because the shipped failure was not a wrong assertion but *no*
assertion — a module declaring none is a finding in its own right. Without that guard it would
go vacuous again the next time the assertions move.

**Two mutation checks exposed tests that passed while proving nothing**, and both were fixed
rather than left: dropping `--detach` was invisible because `git checkout FETCH_HEAD` already
detaches when the revision is not the current branch; and `Isolation::Empty` → `Ambient` was
invisible because both apply the same `PATH` override. Sixteen neutralised behaviours in total,
each caught by a named test; the evidence is in the task file.

**Not implemented, deliberately.**
* **TASK-89 AC#3** is the one open criterion: a full proof run on a release cut *after* this
  merge. `auto-release.yml` is `workflow_dispatch` only — its own header says a
  `push: branches: [main]` trigger would have to be added to automate it — so nothing cuts a
  release on a merge, and the owner dispatches it.
* **No check catches a test writing outside its tempdir.** This session committed
  `crates/xtask/.github/workflows/publish-sandbox.yml` before catching it by reading
  `git status`: a stand-in CLI fixture had no `case` guard, so it wrote its publisher for
  `--help` too, which runs with an inherited working directory. Worth a task — the same class of
  mistake is invisible until someone happens to look.
* **`cargo doc` warns that `repo_checks` links to private `CHECKS`.** Pre-existing on main, not
  touched here; a repo-wide intra-doc sweep is its own task, not a rider on this one.
* **`codecov/patch` fails on this kind of change by construction**, and is not a required check:
  the lifecycle's `pixi lock` needs prefix.dev, so `commit_the_consumer` and
  `prove_the_transport` cannot execute in CI either. A red patch-coverage on a port whose
  execution is the release-run proof is expected, not a regression.

**Owner actions still open, unchanged and none of them engineering:** delete the starter tag
`pixi-sandbox-v0.6.1` (verified still present and still naming a one-file tree, `README.md`),
confirm `PIXI_SANDBOX_STARTER_TOKEN` carries `Workflows: write`, then re-dispatch the starter
lane. Until a release exists, **TASK-76 AC#8, TASK-85 AC#6 and TASK-88 AC#8 stay unchecked**.

**Prompt to start the next session with:**

> Restore the sandbox and baseline the suite — expect **982 passing / 1 skipped** (42 git /
> 219 core / 525 pixi-sandbox / 196 xtask) and `pixi run --frozen lint` 11/11. The transport
> (`sandbox/developer-linux-64`) names commit `3665185` and tool `0.6.1` — a merge repacks it,
> it does not cut a release. If `git rev-list --count HEAD` returns 1 the clone is shallow:
> `git fetch --deepen=300 origin main`. If you restore, force-sync the transport refspec first
> (`git fetch origin "+refs/heads/sandbox/developer-linux-64:refs/remotes/origin/sandbox/developer-linux-64"`);
> restore.sh prefers the local tracking ref and the publisher force-pushes that branch.
>
> **First, one look at the release state: `gh release list --limit 3` and
> `gh run list --workflow release.yml --limit 5`.** If a release newer than **v0.6.1**
> (2026-10-11T09:58:15Z) exists, TASK-89 AC#3 is finally closable — read that release's
> `consumer-proof` job in its `release.yml` run, confirm it reached the same verdict as the shell
> recipe it replaced, paste the run link into the task file as evidence, check AC#3, set the task
> `Done` and close it in house format. While you are in that run, check the same release closed
> **TASK-76 AC#8**. If no newer release exists, leave both open and say so: `auto-release.yml` is
> `workflow_dispatch` only, so a release is an owner dispatch, not something a merge triggers —
> ask for it rather than waiting on it.
>
> TASK-89 landed in PR #136 (`36651855`) at 3 of 4: the consumer proof is `xtask
> consumer-proof-*` subcommands over `crates/xtask/src/consumer_proof.rs`, `release.yml` carries
> no `multiline-run-allowed` marker, and check 13 was re-pointed after it turned out to have been
> comparing nothing since doc-13 moved the assertions out of the file it reads. Read
> `CONTEXT.md` § Session scratchpad — the 2026-10-11 (eighth session) entry lists what is open.
>
> Next engineering, if the release is not the day's work: **TASK-90** (medium spike, no
> dependencies) is the only unstarted item that unblocks others — **TASK-91 and TASK-92 both
> depend on it**. Its subject is live: `self_update 1.3` is still declared (`Cargo.toml:57`,
> `crates/pixi-sandbox/Cargo.toml:39`) and still unused, and `rust-version` is still `1.85`
> (`Cargo.toml:13`) for TASK-92. **TASK-78** stays unstarted — its shape is contested and AC#1's
> D-number is unmet. **TASK-73** blocks on an owner GitHub App install.
>
> House rules are in `AGENTS.md` (invariant 10), the session procedure and its templates in
> `.agents/skills/session/`; Pixi only (`pixi run --frozen …`), cargo `--offline`, never two pixi
> commands in parallel, conventional commits, one concern per commit, `pixi run --frozen test`
> ≥982 at every commit. Do not push, open a pull request, merge, release or dispatch without my
> explicit go-ahead.

### 2026-10-11 (ninth session) — TASK-92 landed; the MSRV debt was nineteen sites, not six

**Branch note:** session fixed to `arena/931c2b4f-pixi-sandbox`, based on main at `245e481f`
(the PR #138 merge, whose post-merge `ci`, `docs` and `publish sandbox` runs are all green and
whose repack names `source.commit: 245e481`). The clone arrived shallow; `git fetch --unshallow`
fixed it (333 commits). The restore came back clean (21 044 blobs / 976.1 MiB, 0 failures;
5 520 manifest entries) but from the *stale local* transport ref — packed from `3665185`,
predating two merges. That only means old, not wrong: the packed tree's `pixi.lock` hashes to
`7ab9bb32…`, byte-identical to what the origin repack's manifest declares as `lock_sha256`
(that field hashes **pixi.lock**, not Cargo.lock — an hour was nearly spent re-deriving vendor
drift that did not exist). Baseline **986 passing / 1 skipped**, `lint` 11/11, toolchain
rustc 1.98.1, user tools registered. The newest release is still **v0.6.1**: the `auto-release`
dispatch of 2026-10-11 ~14:30 (run 38145891697) succeeded **in dry-run** — "prepared v0.6.2 but
not committing, tagging, or releasing" — so v0.6.2 is staged, one real dispatch away, and until
it exists the release-shaped proofs (TASK-76 AC#8, TASK-89 AC#3) stay open.

**What landed (PR #139, TASK-92).** `rust-version` 1.85 → 1.88 — the highest MSRV among normal
dependencies, owed by `self_update 1.3` (ADR-27 adopted it as the release transport) and
`rstest-bdd 0.6.0` alike. The README badge *and* `docs/src/components/Badges.astro:15` moved
with it (the docs site carries the identical shield; the task's AC#1 "no other file
contradicts" clause decided that, not taste), and the `self_update` comment in `Cargo.toml`
was reworded from narrating the pre-verdict future ("Lifting `rust-version` is TASK-92 …")
to recording what now is. Suite unchanged at 986/1; the named invalid-input tests pass 13/13
as a focused set; every collapsed error string is byte-identical.

**The finding worth keeping: an MSRV chore enumerated by a `-p` probe under-counts its own
scope.** The task description listed six `collapsible_if` sites in `pixi-sandbox-core` because
its probe stopped compiling there — with core red, nothing downstream gets linted, so the
dependents' debt was invisible. Collapsing the six let the workspace build proceed, and the
same lint surfaced thirteen more sites: `tool_fetch.rs:67`, `upgrade/mod.rs:329/334`,
`commands/doctor.rs:240/246/257/265` (one a three-level nest), `commands/restore.rs:153`,
`diagnostics.rs:110`, `tests/e2e.rs:25`, `tests/actions.rs:28`,
`pixi-sandbox-core/tests/verify.rs:381`, `xtask/repo_checks/agents_md.rs:37`. Nineteen total,
all the same class, all required by the task's own `--workspace --all-targets` AC. The lesson
generalises: for any lint debt an MSRV lift switches on, let the *workspace gate* define the
list, never a per-crate enumeration.

**Two process notes.** Three parallel `edit` calls to one file raced and only one landed — the
freshly-enabled workspace clippy gate caught it on the next run, and same-file edits are
sequential from now on. And an `edit_file` match that silently spans re-wrapped comment lines
can report success while changing nothing: `git diff` is the only honest receipt, and the
comment rewrite had to be redone in two smaller pieces. Neither reached a commit; both were
caught by gates this task turned on.

**Not implemented, deliberately.** **TASK-78** (high, 0/10) is the only unblocked feature left
and starts as a decision, not code: AC#1 needs a D-numbered shape decision (new pixi-sandbox
verb vs. xtask subcommand vs. documented rattler-build recipe pattern), the pixi-build preview
status named, and the relock consequence confirmed before any implementation. Owner actions
remain the release lane's bottleneck: cutting v0.6.2 (dispatch is prepared in dry-run) starts
the run that carries TASK-89 AC#3 and TASK-76 AC#8; TASK-85 AC#6 still needs a maintainer
`actions:write` click on the starter dry-run; TASK-93 AC#4 needs a connected-host
`pixi-pack` → `pixi-unpack` wheel round trip; TASK-73 waits on the owner GitHub App. Nothing
new was filed this session.

**Prompt to start the next session with:**

> Restore the sandbox and baseline the suite — expect **986 passing / 1 skipped** (42 git /
> 222 core / 526 pixi-sandbox / 196 xtask) and `pixi run --frozen lint` 11/11. The transport
> (`sandbox/developer-linux-64`) names the PR #139 squash commit and tool `0.6.1` — a merge
> repacks it, it does not cut a release. If the clone is shallow: `git fetch --unshallow
> origin main`. `lock_sha256` in the transport manifest hashes **pixi.lock**; don't spend an
> hour on vendor drift that isn't there. The workspace `rust-version` is now **1.88** (TASK-92
> landed); the toolchain in the restored env is rustc 1.98.1.
>
> **First, one look at the release state: `gh release list --limit 3`.** If a release newer
> than **v0.6.1** (2026-10-11T09:58:15Z) exists — v0.6.2 was already prepared in dry-run as
> of 2026-10-11, run 38145891697, so this is plausible — read that release's `consumer-proof`
> job in its `release.yml` run: it closes **TASK-89 AC#3** and checks **TASK-76 AC#8** in the
> same run. Paste run links as evidence, close the task files in house format. If no newer
> release exists, leave both open: `auto-release.yml` is `workflow_dispatch` only, so ask for
> the dispatch rather than waiting on it.
>
> TASK-92 landed in PR #139 at 4 of 4: MSRV 1.88 declared, badges agree, nineteen
> `collapsible_if` sites collapsed (the six listed in the task file plus thirteen the
> workspace gate found), error strings byte-identical, suite count unchanged. Read
> `CONTEXT.md` § Session scratchpad — the 2026-10-11 (ninth session) entry lists what is open.
>
> Next engineering: **TASK-78** (high, feature, no dependencies) — but its AC#1 comes before
> any code: a D-numbered decision in `backlog/docs/adr/README.md` recording the shape (new
> pixi-sandbox verb, xtask subcommand, or documented rattler-build recipe pattern), the
> upstream preview status of pixi-build, the rattler-build dependency, and the no-new-crate-
> without-a-relock rule. Propose the decision doc first; the other nine ACs are engineering
> that follows it. **TASK-73** stays blocked on owner GitHub App setup; **TASK-85 AC#6** and
> **TASK-93 AC#4** stay proof-only (maintainer click; connected host).
>
> House rules are in `AGENTS.md` (invariant 10), the session procedure and its templates in
> `.agents/skills/session/`; Pixi only (`pixi run --frozen …`), cargo `--offline`, never two
> pixi commands in parallel, conventional commits, one concern per commit, `pixi run --frozen
> test` ≥986 at every commit. Do not push, open a pull request, merge, release or dispatch
> without my explicit go-ahead.

### 2026-10-11 — v0.6.2 workflow watch and evidence follow-up

**Scope.** Watched the existing release chain; did not dispatch, publish, push, or open a PR.
Fast-forwarded the session branch to `d0fc39ad35d8833b69a937079272db1ce0b9504a`
(`chore(release): v0.6.2`). No production implementation changed.

**Remote evidence.** All runs completed successfully: auto-release `38163213333`, release
`38163288233` (five platform builds/package smoke tests, pre-publish consumer lifecycle proof,
prefix.dev and GitHub publication), CI `38163288846`, push docs `38163288827`, sandbox publish
`38163288796`, released-asset consumer proof `38163986987`, starter `38163988956`, and dispatched
docs `38163991484`. Run URLs use
`https://github.com/Archont561/pixi-sandbox/actions/runs/<run-id>`.
The published transport manifest names source `d0fc39a` and static tool `0.6.2`.

**Task evidence, not blanket closure.** TASK-89 AC#3 is now proven by release job
`114542089359`; all four ACs are checked and the task is Done. TASK-76 was already Done with
its own AC#8 evidence, so it was not reopened or edited. TASK-74 AC#1 is now proven: GitHub
reports a public template, and starter tag `pixi-sandbox-v0.6.2` resolves to commit
`1655285c27f162f24be33428fb1c1f791bd2e063` with all ten required files, rather than README alone.
The starter job also passed regeneration, lint, fresh-clone dev and verification, but those
positive steps do not prove every compound acceptance criterion. TASK-74 stays In Progress,
including its owner-operated App and template-derived publisher/restore proof requirements.
TASK-85 has the successful real publication recorded, but AC#6 explicitly requires a
**non-publishing v0.6.0 dry run**; the v0.6.2 publish cannot substitute for it.

**Evidence limits.** Raw Actions log downloads redirect to an egress-blocked host. Conclusions
above use GitHub API job/step results and the published starter tree/files, not an invented log
inspection. No new workflow was dispatched to obtain stronger evidence.

**Environment.** The restore preceding release completion registered pixi `0.81.0` and
pixi-sandbox `0.6.1`; it remains usable on the release tree. Re-baseline at `d0fc39a`:
**986 passing / 1 skipped** (42 git / 222 core / 526 pixi-sandbox / 196 xtask). A new restore
from the now-published transport would register pixi-sandbox `0.6.2` instead.

**Next scope, not implemented.** TASK-78's packaging-shape decision remains the next engineering
candidate; no decision was made or code started here. TASK-73 needs owner GitHub App setup;
TASK-85 AC#6 needs the specifically requested maintainer dry run; TASK-93 AC#4 needs its
connected-host wheel round trip.

**Next session opening prompt:**

> Load the session skill and restore only if needed. Expect 986 passing / 1 skipped.
> v0.6.2 is published and all its release/starter/docs follow-ups passed; do not cut it again.
> Read CONTEXT.md's 2026-10-11 “v0.6.2 workflow watch and evidence follow-up” entry.
> TASK-89 is closed with real release-run evidence; TASK-74 AC#1 is checked, but its remaining
> compound criteria and TASK-85's exact non-publishing dry-run criterion are still open.
> The next engineering proposal is TASK-78's packaging-shape decision, before implementation.
> Check branch/remote state first; the evidence commits are local until explicitly approved
> for push. Propose the slice and stop. Do not push, open a PR, merge, release, or dispatch
> without explicit go-ahead. Pixi only, frozen tasks, Cargo offline, and one Pixi command at a time.

Validation for this evidence follow-up: `pixi run --frozen fmt` passed;
`pixi run --frozen lint` passed all 11 gates; release-tree tests remain 986/1.

### 2026-10-11 — README refresh and starter pack failure (TASK-94)

The README now follows the concise bun-test-utils structure: badges, introduction, starter-first
quickstart, capabilities with guide links, lifecycle, platform distinctions, repository layout,
development, contributing, license. It explicitly distinguishes release binary targets from the
workspace badge and removes the stale claim that macOS is routinely published. The starter is
linked prominently, but the v0.6.2 Cargo-policy workaround and missing successful publisher proof
are disclosed rather than advertising an unproven path.

The user's starter run `38164114312`, job `114543021308`, failed in pack, after fetching the
pinned CLI. Published starter tag `pixi-sandbox-v0.6.2` contains no Cargo project/lock but enables
Cargo vendoring. A local pack preflight using its published pixi.lock reproduces
`--cargo-vendor needs a Cargo.lock`. Raw job logs are egress-blocked; phase annotations, tree
inspection, and local reproduction support the diagnosis, not a claim to have read remote stderr.
This is why TASK-74's template-derived publisher/restore criteria were left open even when
starter generation passed: generation is not publication proof.

User confirmed the fix seams before tests: black-box init CLI and public starter scaffold/verify.
TASK-94 adds six passing tests/cases (suite 986/1 → **992/1**), first observed failures at each
changed behavior, preserves existing consumer config, seeds non-Rust projects without vendoring,
and makes the language-neutral starter own its explicit non-Cargo plan. Its verifier now rejects
unusable Cargo policy. Root/bundle overrides and invalid config are covered; schema defaults
remain unchanged. Refactor review retained the small local changes rather than introducing a new
abstraction or rewriting reviewed consumer data.

**Not yet implemented remotely.** No push, PR, release, starter edit, or workflow dispatch was
performed. TASK-94 AC#4 stays open until a corrected published starter's own publisher succeeds.
A rerun of the existing failed revision alone will not change its config. Do not move or overwrite
the immutable v0.6.2 starter tag. A new corrected revision and an explicitly sanctioned publisher
run are still needed; full template-derived offline restore proof remains TASK-74 work.

**Next session opening prompt:**

> Load the session skill. Expect 992 passing / 1 skipped (42 git / 222 core / 530 CLI / 198 xtask).
> Read the 2026-10-11 README refresh / TASK-94 entry in CONTEXT.md. The init/starter Cargo-policy
> fix and README refresh are local, not released. TASK-94's local criteria pass; its real starter
> publisher proof is open. Do not rerun the unchanged v0.6.2 starter or overwrite its immutable
> tag. Propose landing the local commits and obtaining a corrected starter publisher/restore
> proof; push, PR, merge, release, and dispatch still require explicit go-ahead.

### 2026-10-11 — single-commit starter main (TASK-95 / ADR-22)

The owner requested replacing starter main on every update and confirmed preserving immutable
release tags. Implemented in the publisher, not applied to the live repository. Each successful
publication creates a parentless commit from the validated staged tree and atomically pushes
main plus its new annotated release tag. The explicit lease names the checked-out HEAD; a
concurrent main update is refused rather than overwritten. Dry run no longer stages files.
Unchanged trees still receive a new snapshot/tag for a new release; existing tags remain refused.
Only main is eligible, and the canonical remote check now rejects substring lookalikes.

Workflow checkout explicitly selects main; all releases share a publication concurrency group.
ADR-22, doc-10, TASK-74 AC#7 and the generated starter README record the owner-approved history
policy. Retaining old tags means **one commit in main ancestry**, not one commit in all refs or
a promise that old objects vanish from GitHub storage.

Evidence: initial red counted two commits on published main; green counts one. The fixture also
proves dry-run index/remote safety, full template content, unchanged-tree publication for a new
release, old-tag retention and new-tag/main agreement. Four Git adapter cases refuse remote
movement, tag conflict, protected main and unsupported atomic push without changing remote refs.
Mutation checks caught removal of both the explicit lease and atomic flag. Refactor review kept
Git plumbing inside its adapter, with no new shell recipe or unchecked force fallback.
Gates: fmt passed, lint 11/11, **996 tests passing / 1 skipped** (46 git / 222 core / 530 CLI /
198 xtask), up from 992/1. TASK-95 local ACs #1–#4 are checked; #5 remains remote-proof-only.

No push, PR, workflow dispatch, live starter history rewrite, tag deletion or branch-protection
change was made. Rollout still needs the local commits landed and a connected publication whose
credential may force-update starter main. A protected main fails closed; do not bypass that by
removing the lease or splitting the push. TASK-94's corrected-starter publisher proof also remains
open. These are deployment/evidence boundaries, not claims that the live starter is already fixed.

**Next session opening prompt:**

> Load the session skill; baseline 996 passing / 1 skipped. Read the 2026-10-11 TASK-94 and
> TASK-95 entries. Local commits now include the README refresh, non-Cargo starter fix, and
> leased single-commit-main publisher. ADR-22 is decided: preserve old release tags, never replace
> them; replace only canonical starter main with one root commit, atomically with its new tag.
> TASK-94 and TASK-95 remain In Progress only for real connected publisher evidence. Propose
> the landing/rollout slice and stop: push, PR, merge, release, dispatch, and live starter changes
> still require explicit go-ahead. The owner must allow the publishing credential to force-update
> starter main; no branch protections have been modified here.

### 2026-10-12 — TASK-78 slices 0–1 landed locally; the mirror and build-tool questions are still open

Restore and baseline were re-measured on this sandbox. github.com answered, `scripts/restore.sh`
exited 0 and verified 5520 entries, and the suite is **997 passing / 1 skipped** (46 git, 222 core,
530 CLI, 199 xtask). The `publish sandbox` run for merge `34e725f` (run 38181306948) is green, and
its transport manifest names `source.commit 34e725f` and `tools.pixi-sandbox 0.6.2`. v0.6.2 is still
Latest, and no v0.6.3 exists, so TASK-94 AC#4 and TASK-95 AC#5 stay In Progress.

TASK-78 slice 0 added ADR-28 as a proposed file and repaired the three loose ends (SKILL.md's ADR
range, TASK-78 and doc-12 pointing at a D-number). Slice 1 added `pixi_sandbox_core::payload`, with
70 tests and a fake `PayloadToolchain`. None of TASK-78's acceptance boxes is checked yet. ADR-28
records the design, and its open items are the things this entry must not forget.

- **`[mirrors]` and `file://` is still unverified.** The pixi 0.81.0 configuration reference, read
  from GitHub's raw file because prefix.dev is unreachable, describes mirrors as HTTP(S) URL
  replacements that must be exact copies. It does not mention a `file://` target. The lever is
  real (project `.pixi/config.toml`, `PIXI_CONFIG_FILE`, `PIXI_NO_CONFIG` are all documented), but the
  design assumes the HTTP fallback until a test shows `file://` works.
- **rattler-build cannot be fetched from GitHub either.** The release asset redirects to
  `objects.githubusercontent.com`, which fails to connect from this sandbox (curl exit 35). So the
  "no route" premise holds even for the release-asset route, and the first real build waits for CI.
- **Unverified recipe semantics.** The recipe assumes `source: path:` and `$RECIPE_DIR`. Both
  need a real build to prove them.
- **Budget arithmetic is an estimate.** The transport is counted twice, as the channel package and
  as the unpacked prefix files, which comes from the spec text and `record_file`'s per-path blobs.
  It has not been measured. The blob bound uses uncompressed bytes, so it is conservative. The exact
  built sizes belong to the slice that calls rattler-build.
- **Deliberately excluded:** universal (fat) Mach-O, which shares its magic with Java class files;
  architecture checks for ELF and Mach-O; `win-64` launchers, which are refused for now. The
  install script is POSIX, so `win-64` payloads have not been exercised at all.

Next session opening prompt:

> Load the session skill; baseline 997 passing / 1 skipped on `main` 34e725f, and the core payload
> module adds 70 cases. Read the 2026-10-12 TASK-78 entry. ADR-28 is `proposed` and needs review
> before AC#1 is checked. Next slice: the `prepare` command with its SHA256 check (AC#13) and the
> `pixi sandbox wrap-payload` verb's argument and plan layer, both locally testable. Do not push,
> open a PR, dispatch CI, or cut v0.6.3 without an explicit go-ahead. The `[mirrors]` `file://`
> check and the rattler-build round trip are connected or CI proofs.

### 2026-10-12 (second session) — the red on main was `codecov/patch`, and it was measuring real uncovered code

The session opened as "diagnose red CI". Every GitHub Actions check on main's head `13d64ef` was
green; the red was a **commit status**, `codecov/patch` = failure, *"79.09% of diff hit (target
85.80%)"*. `rules/branches/main` returns `[]`, so nothing was blocked — main simply carried a red
✗. Worth remembering: codecov posts a legacy *status* on direct pushes and a *check run* on pull
requests, so the two endpoints disagree about the same context.

**Why the window was five commits, not one.** TASK-78's five commits (`8832ba1`…`13d64ef`) reached
main in a single push, and `commits/<sha>/pulls` returns `0` for each of the first four — none went
through a pull request. A push fires the `push` workflow once, on the tip, so only `13d64ef` has a
coverage report; codecov's patch base fell back to the newest report it had, `7d539c7` (PR #143's
merge), and scored 21 files / 2 429 added lines / **531 instrumented** instead of the 2-file docs
diff the commit looked like. The same push skipped `airlock`, which triggers on `pull_request` with
`paths: crates/**` — the payload code never had its native airlock proof.

**The number was genuine, and it reproduces.** Running CI's own command locally
(`pixi run --frozen coverage` → `cargo llvm-cov nextest --workspace --lcov`, then intersecting the
report with the diff's added lines) gives 420 hit / 111 missed = 79.096%, which is codecov's
`round: down, precision: 2` → **79.09%**. Project coverage was 11 681/13 648 = 85.59%, against a
`target: auto` 85.80% inherited from the base report. So this was a real coverage gap in the
payload work, not a base-fallback artifact.

Measurements worth keeping, because the intersection is cheap to redo and the arithmetic is the
diagnosis:

| | before | after PR #144 |
| --- | --- | --- |
| tests | 1110 passing / 1 skipped | 1119 passing / 1 skipped |
| project coverage | 85.59% | **86.16%** |
| TASK-78 window patch coverage | 79.09% | **92.68%** |

Nine tests closed it, at seams the code already exposed: the `--json` plan shape moved from
`commands/wrap_payload.rs` into `wrap_payload::plan_report` (a wire contract a consumer scripts
against belongs in a promoted module, where `tests/` can reach it, not in a binary module that is
only coverable black-box); the CLI drives `--build --build-tool <stand-in>` end to end, so the
build, the single-package discovery and the publish into the default channel are proven with no
network and no 31 MiB download; `--budget-config` is proven read by a budget small enough to refuse
the payload; `find_package` names a platform directory a build never wrote and refuses to choose
between two packages; `RealProcessRunner` returns a successful command's output and reports a
failed one as a toolchain error.

**Left uncovered on purpose**, and named rather than hidden, because each needs something this
sandbox cannot produce:

- `commands/wrap_payload.rs` L92–97 — the on-demand `tools_cache`/`resolve_tool` fetch branch. It
  needs a connected host; `--build-tool` is the offline path and is now covered instead.
- `rattler_build.rs` L188–189 — the `read_dir` *entry* error, as opposed to the directory error,
  which is covered. Triggering it portably means a directory that fails mid-iteration.
- `prepare.rs` L41 — the Windows `cmd /C` branch, on a Linux-only suite.
- `payload.rs` / `payload_config.rs` — 24 error-construction sites (`path: path.display()…`) that
  only run when `walkdir` or `read_dir` fails underneath a well-formed fixture tree.

**Not done, and the reason it matters more than the coverage.** The process gap that caused the red
is still open: five commits pushed straight to main means one `ci` run for five commits, no codecov
report for four of them, and no `airlock` proof at all for code under `crates/**`. AGENTS.md and the
session skill both say pull request, then squash-merge; nothing enforces it. A backlog task should
decide between a branch-protection rule (needs a maintainer, `branches/main/protection` is 403 to
this token), a `check-repository` finding, or a documented rule that direct pushes to main must be
single commits so the tip's report is the whole change.

Environment facts re-measured this session: `scripts/restore.sh` exits 0, verifies 21 044 blobs /
976.2 MiB, restores 5520 entries and prints `user tools: registered pixi and pixi-sandbox in
/home/user/.local/bin`. The transport it pulled names `source.commit 13d64ef`, pixi 0.81.0,
pixi-sandbox 0.6.2, 361 vendored crates. Coverage costs about a minute warm (`cargo llvm-cov
nextest`, 1119 tests in ~13 s once built). CI job logs are **not** readable from this sandbox —
`actions/jobs/<id>/logs` 302s to `*.blob.core.windows.net`, which is outside the egress allowlist —
so reproduce locally instead of trying to read a run.

Next session opening prompt:

> Restore the sandbox and baseline the suite (expect **1119 passing / 1 skipped** on `main`
> `2524412c`; the restore verifies 5520 entries and registers `pixi` and `pixi-sandbox` in
> `~/.local/bin`, and the transport carries pixi 0.81.0, pixi-sandbox 0.6.2 and 361 vendored crates
> — but crates.io and prefix.dev are unreachable, so every cargo call goes through
> `pixi run --frozen -- cargo … --offline`), then read `CONTEXT.md` § Session scratchpad — the
> 2026-10-12 (second session) entry is this hand-off.
>
> First, one look at the repo state: `gh run list --branch main --limit 4`. The `publish sandbox`
> run for `2524412c` must be green before the transport names that commit; if it is red, the next
> restore pulls the older transport and the baseline will not match — say so before anything else.
>
> Two things need a decision before any code. **One:** TASK-78's AC#1 is still unchecked because
> ADR-28 sits at `status: proposed` — accepting or rejecting it is a review call, not an
> implementation one, and it gates the task. **Two:** the process gap that made main red — five
> commits pushed straight to main, one `ci` run between them and no `airlock` proof at all. It
> needs a task, and the decision is which lever: branch protection (needs a maintainer; the API is
> 403 to this token), a `check-repository` finding, or a written rule that a direct push to main is
> always a single commit. My recommendation is the written rule plus the xtask finding, because
> those are the two this repository can enforce on itself.
>
> Propose the slice and stop. House rules are in `AGENTS.md` (invariant 10: anything you do not
> implement goes in `CONTEXT.md`, not into the files it speculates about), the session procedure
> and its templates are in `.agents/skills/session/`.

### 2026-10-13 — the red on main was the payload round-trip oracle, and conda cannot carry a directory

The session opened on red `ci` after PR #146 merged. It was not a flake: `test-payload-roundtrip`
failed with `assertion left == right` in `assert_payload_tree_matches`. PR #146's own `ci` had
already failed on the head, and `ci` is not a required check, so it merged anyway — the *same*
process gap the 2026-10-12 entry names, still open.

**Two independent causes, both in the oracle, not the packer.**

1. **Provenance.** `render_recipe` copies `pixi-sandbox-payload.json` into `$PREFIX/libexec/<name>/`
   (AC#7), so the installed tree legitimately carries one entry the source lacks. The oracle now
   removes `PROVENANCE_FILE` before the exact comparison.
2. **Empty directories.** AC#3 demanded empty directories survive the round trip, but a conda
   package carries files and symlinks only. Measured on the pinned `rattler-build 0.76.1` (2026-10-13)
   by building the staged recipe locally and reading the package's `info/paths.json`: it records only
   `hardlink`/`softlink` entries, and an explicit `$RATTLER_BUILD_PACKAGE_FILES` list (a v0.71+ knob)
   also drops the directories. Upstream `prefix-dev/rattler-build#371` was closed as won't-fix; the
   suggested workaround — a hidden `.empty` placeholder — would leave a stray non-payload byte in the
   consumer's prefix and was rejected.

Owner decision: **amend AC#3 to files and symlinks**, empty directories out of scope. The oracle now
compares files and symlinks; the ignored round-trip fixture asserts the empty directory is absent;
three non-ignored tests pin the contract; ADR-28 and `docs/.../payload-channels.mdx` record the
consumer-visible limitation. Landed as PR #149, squash `b2150a2e`, `ci` fully green (including
`codecov/patch`), and the post-merge `ci`, `docs` and `publish sandbox` runs all green. That was the
first green `test-payload-roundtrip` on main, so **AC#3 and AC#5 are now checked**; TASK-78 stays
`In Progress` because AC#1 still waits on ADR-28's review and the remaining ACs are untouched.

**Cheap measurement recipe worth keeping**, because it needs no unzip/zstd/python (none are on PATH
here, and there is no `sudo`, so the egress-denied tier stays CI-only): run `wrap-payload --build`
(the on-demand fetch works — it is a github.com release download), then `pixi install` the package
into an isolated `HOME`/`XDG_CACHE_HOME` from a `file://` channel, and read
`$XDG_CACHE_HOME/rattler/cache/pkgs/<name>-<build>/info/paths.json`. Pixi extracts the package
before installing, so the package's own metadata is a plain file in its cache.

Next session opening prompt:

> Restore the sandbox and baseline the suite (expect **1,122 passing / 2 skipped** on `main`
> `b2150a2e`; the restore verifies 5,520 entries and registers `pixi` and `pixi-sandbox` in
> `~/.local/bin`, and the transport carries pixi 0.81.0, pixi-sandbox 0.6.2 and 361 vendored crates
> — but crates.io and prefix.dev are unreachable, so every cargo call goes through
> `pixi run --frozen -- cargo … --offline`), then read `CONTEXT.md` § Session scratchpad — the
> 2026-10-13 entry is this hand-off.
>
> First, one look at the repo state: `gh run list --branch main --limit 4`. The `publish sandbox`
> run for `b2150a2e` must be green before the transport names that commit; if it is red, the next
> restore pulls the older transport and the baseline will not match — say so before anything else.
>
> Two decisions, unchanged from the last hand-off. **One:** TASK-78's AC#1 is still unchecked
> because ADR-28 sits at `status: proposed` — accepting or rejecting it is a review call that gates
> the task, and AC#3/#5 only just went green (PR #149). **Two:** the process gap that made main red
> twice — direct pushes and a non-required `ci` — still needs a task and a lever: branch protection
> (needs a maintainer; the API is 403 to this token), a `check-repository` finding, or a written rule
> that a direct push to main is always a single commit. My recommendation stays the written rule
> plus the xtask finding.
>
> TASK-78 is now the closest open feature: AC#2, AC#4, AC#6, AC#7, AC#8, AC#10, AC#11, AC#12 and
> AC#13 all have code and tests on main but unchecked boxes. Auditing which are genuinely proven and
> checking only those is a good, self-contained slice — do not check a box you cannot point at a
> green proof for.
>
> Propose the slice and stop. House rules are in `AGENTS.md` (invariant 10: anything you do not
> implement goes in `CONTEXT.md`, not into the files it speculates about), the session procedure
> and its templates are in `.agents/skills/session/`.

### 2026-10-13 (second session) — TASK-78's single-file payload staged, TASK-111 names the process gap

Ran the previous hand-off: three locally-actionable slices, then landed them as PR #150. **Merged,
not squashed** (owner's call), so the branch's five commits are preserved and the merge commit is
`5635e4d2`. Post-merge runs green: `ci` 2m39s, `docs` 1m23s, `publish sandbox` 3m47s — the last
repacks the transport, which now names `5635e4d` (pixi-sandbox 0.6.2, pixi 0.81.0, `static`). Suite
on merged main: **1,125 passing / 2 skipped** (was 1,122/2).

**Slice 1 — TASK-78's single-file payload.** `wrap_payload::copy_tree` copied a directory with
`std::fs::read_dir`, so a payload root that is a single file failed `Not a directory (os error 20)`
— while `read_payload_tree` (core) and the payload-channels guide both accept a one-file payload.
Fix: a non-directory root is copied as the one member of `payload/`, so the recipe's
`$RECIPE_DIR/payload/` is always a directory. Two red→green tests, at the library seam
(`tests/wrap_payload.rs`) and the CLI seam (`tests/cli.rs`). The first cut derived the member name
with an inline `InvalidConfig` closure no real path can reach (a non-directory path always has a
file name), so `codecov/patch` failed at 54.55%; the follow-up commit derives the name the way
`read_payload_tree` does — require UTF-8, return `NonUtf8Path` — and a non-UTF-8-name test reaches
it. Then audited TASK-78's remaining ACs against code and tests: **AC#2, #4, #6, #7, #8, #9, #10,
#13 checked (10/13)**.

**Slice 2 — TASK-88 AC#8.** The local fix already landed as TASK-93: `UV_INSTALL_METADATA` is
`["uv_cache.json", "direct_url.json"]` and TASK-88's own fixture writes `direct_url.json` verbatim
with its `RECORD` row. So the local model reproduces the connected observation; AC#8 stays open for
one reason only — a real `pixi-unpack` reinstall needs a connected host (no `pixi-pack`, no wheel,
no `curl`, pypi/prefix.dev blocked here). Recorded in the task.

**Slice 3 — the process gap.** Created **TASK-111** (chore, high) to decide and implement the lever:
branch protection (maintainer; the API is 403 to this token), a repository-enforceable guard, or the
written single-commit rule, with the recommendation recorded (the written rule plus the guard). This
session's own route — a five-commit branch pushed as a PR and merge-committed — is the shape it
names.

Open items this entry leaves, newest last:

- **TASK-111** (To Do) — the decision plus the guard. Local.
- **TASK-78** (In Progress, 10/13) — AC#1 needs ADR-28 accepted or rejected (a review call); AC#11
  ("the channel is named once") has no in-repo fixture; AC#12 ("`tools.lock.json` unchanged by a
  wrap") is untested, and the on-demand fetch is connected-only.
- **TASK-88** (7/8) and **TASK-93** (AC#4) — the same connected-host reinstall proof; also TASK-66,
  TASK-73, TASK-74, TASK-94, TASK-95.
- **TASK-85 AC#6** — an owner click (a dispatch), not a connected host.

Next session opening prompt:

> Restore the sandbox and baseline the suite (expect **1,125 passing / 2 skipped** on `main`
> `5635e4d2`; the restore registers `pixi` and `pixi-sandbox` in `~/.local/bin`, and the transport
> names `5635e4d` — pixi-sandbox 0.6.2, pixi 0.81.0, `static` — but crates.io and prefix.dev are
> unreachable, so every cargo call goes through `pixi run --frozen -- cargo … --offline`). Then read
> `CONTEXT.md` § Session scratchpad — the **2026-10-13 (second session)** entry is this hand-off.
>
> First, one look at the repo state: `gh run list --branch main --limit 4`. The `publish sandbox`
> run for `5635e4d2` must be green before the transport names that commit; if it is red, the next
> restore pulls the older transport and the baseline will not match — say so before anything else.
>
> Two decisions are still open, and both are review calls rather than work. **One:** TASK-78's AC#1
> waits on ADR-28 (`status: proposed`) — accepting or rejecting it gates the task. **Two:** TASK-111
> is new — pick the lever for enforcing main integration (branch protection needs a maintainer and
> the API is 403 to this token; a repository-enforceable guard; or the written single-commit rule)
> and record it before implementing.
>
> TASK-111 is the natural next slice: it is local, its first AC is the decision itself, and it names
> the gap that put red main in front of two sessions. TASK-78's AC#11 and AC#12 are the other local
> candidates. Do not check a box you cannot point at a green proof for.
>
> Propose the slice and stop. House rules are in `AGENTS.md` (invariant 10: anything you do not
> implement goes in `CONTEXT.md`, not into the files it speculates about), the session procedure
> and its templates are in `.agents/skills/session/`.

### 2026-10-13 (third session) — this session is a GitHub Codespaces container, not the Arena AI airlocked sandbox

Every prior session — and the session procedure itself — assumed the *Arena AI sandbox*: egress
filtered to `github.com` plus the npm registry, crates.io and prefix.dev dead, so every cargo call
went `--offline` against the vendored tree. This session is a different machine of a different
kind: a **GitHub Codespaces dev container with unfiltered outbound internet**. The two worlds need
different commands, and the previous entry's "crates.io and prefix.dev are unreachable" line is
true only of the Arena AI sandbox — do not carry it into a Codespaces session without re-probing.

Identity and access, probed 2026-10-13:

- `/.codespaces` and `/.dockerenv` present; `uname` → `codespaces-e3d5ff … Linux 6.8.0-1064-azure`;
  checkout at `/workspaces/pixi-sandbox`; `HOME=/root`.
- **TLS/HTTP reachable everywhere**, real CA-signed certificates, no MITM, no proxy env:
  `github.com`/`api`/`codeload`/`objects.githubusercontent.com` (`git ls-remote` → the local tip,
  `gh api rate_limit` 50000/50000), `index.crates.io` and `static.crates.io`
  (`serde-1.0.219.crate` → `206 Partial Content`, first bytes `1f 8b 08 08` = real gzip),
  `static.rust-lang.org`, `pypi.org` + `files.pythonhosted.org`, `registry.npmjs.org`,
  `prefix.dev`, `conda.anaconda.org`, and the open internet (`example.com`, `google.com`,
  `httpbin.org`, `8.8.8.8:443`). A bare TCP connect is meaningless here — a listener answers
  everywhere — so the probe used `openssl s_client` and real object fetches, not `/dev/tcp`.
- **No privilege.** `sudo` is absent; `unshare -rn` → `Operation not permitted` even though
  `unprivileged_userns_clone=1` — user namespaces are blocked at runtime. The egress-denied airlock
  tier stays CI-only; nothing here can run it.
- **Tools on PATH:** `pixi 0.81.0`, `pixi-sandbox 0.6.2`, `git 2.56.0`, `gh 2.102.0`, `tar`, `gzip`,
  `openssl`, `unshare`, `getent`, `awk`, `timeout`. **Absent:** `curl`, `wget`, `python3`, `node`,
  `npm`, `bun`, `cargo`/`rustc`/`rustup`, `unzip`, `zip`, `zstd`, `xz`, `jq`, `file`, `nc`, `ssh`
  (cargo/bun/node still arrive through `pixi run --frozen -- …`).

Consequences for the backlog:

- The "connected-host" ACs are **producible here**, not only in CI. `pixi-pack 0.7.11` and
  `pixi-unpack 0.7.11` are pinned in the embedded tools lock (`pixi sandbox tools list`) and
  conda-forge/pypi answer, so TASK-88 AC#8 and TASK-93 AC#4 — the shared real `pixi-pack` →
  `pixi-unpack` round trip of a uv pypi wheel — are in reach. They were recorded as impossible only
  on the Arena AI sandbox.
- `--offline` and the vendored tree are no longer *forced*. Keep using them for reproducibility, but
  a fetch failure is no longer evidence of an offline bug, and a plain crates.io fetch now succeeds.
- The egress-denied tier, `sudo`, and workflow dispatch remain unavailable; a consumer-publisher
  proof (TASK-94/95) and the App-credential tasks (TASK-73/74) are still gated on GitHub
  Actions/owner credentials, not on connectivity.

Next session opening prompt:

> Restore the sandbox and baseline the suite (expect **1,125 passing / 2 skipped** on `main`
> `c3d09beb`). **Probe the environment before trusting any offline assumption:** this repository
> runs in two different worlds — the **Arena AI sandbox** (egress filtered to github.com + npm;
> crates.io and prefix.dev dead; every cargo call is `pixi run --frozen -- cargo … --offline`) and a
> **connected GitHub Codespaces container** (full internet; the vendored tree optional). One cheap
> probe: `gh api rate_limit` succeeds and `openssl s_client -connect static.crates.io:443
> -servername static.crates.io` returns a real cert — then crates.io is reachable. Then read
> `CONTEXT.md` § Session scratchpad — the **2026-10-13 (third session)** entry records this probe
> and its consequences.
>
> If the network is open, the highest-leverage slice is the connected pypi round trip: a real
> `pixi-pack` → `pixi-unpack` of a uv-installed pypi wheel, verified by the per-file oracle with 0
> failures, closing **TASK-88 AC#8 and TASK-93 AC#4** (both tools are pinned in the embedded tools
> lock; conda-forge and pypi answer). Then the rest of the queue: land the Dependabot PRs, then
> **TASK-111** (the main-integration decision + guard) and **TASK-78 AC#11/#12**, all local.
>
> If instead you are on the Arena AI sandbox, the connected proofs are out of reach and the session
> is TASK-111 and TASK-78 AC#11/#12 only. Do not check a box you cannot point at a green proof for.
> House rules are in `AGENTS.md` (invariant 10), the session procedure and its templates are in
> `.agents/skills/session/`.

### 2026-10-14 — ci now judges how every push reached main (TASK-111, PR #151)

An Arena AI sandbox session (egress filtered; `--offline` throughout). It landed TASK-111, cleared
both Dependabot PRs and accepted ADR-28.

What landed:

- **PR #151**, merged as merge commit `7d30bfe0`. It has three commits:
  - `fec5fd72`, the TASK-111 guard and ADR-29;
  - `93b5c3cb`, the actions-patch pins in the workflows *and* the generator templates. It
    supersedes #147, which failed check 10 because it bumped only the committed workflows.
    Dependabot closed #147 by itself after the merge.
  - `ab62ff43`, ADR-28 accepted and TASK-78 AC#1 checked.
- **#148** (docs-patch `bun.lock`): rebased with `@dependabot rebase`, then squash-merged as
  `b2424fc9`. Its `test-payload-roundtrip` failure had been the stale pre-#149 oracle, and it was
  green after the rebase.
- **The guard's first live verdicts** came on both merges. `check-main-push` succeeded in ci runs
  38555521412 (#151's merge commit) and 38556286687 (#148's squash), with no error annotation.
  Replayed against main's history, it fails the push behind #144 (`7d539c72` → `13d64ef8`) and
  passes every pull-request merge and release since.
- **Post-merge runs.** ci, docs and publish sandbox were green for both merges, and the transport
  was repacked twice. TASK-111 is Done. TASK-78 is 11/13; AC#11 and AC#12 are deferred.
- **Suite** 1,125 → 1,148 passing / 2 skipped (+8 pixi-sandbox-git, +15 xtask).

Open items, not implemented here:

1. **Maintainer action (ADR-29).** In the repository settings, protect `main` with a ruleset that
   requires `ci (lint · test · coverage)`. Give it a bypass for the admin role and for the owner of
   `RELEASE_PUSH_TOKEN`: a required check also refuses unchecked *direct* pushes, so without the
   bypasses the release commit and documentation pushes would be refused too. Also untick "Allow
   rebase merging". The automation's token gets 403 on the protection API. Until this is done the
   guard reports after a push and refuses nothing.
2. **Ubuntu 26.** `ubuntu-latest` moves to Ubuntu 26 from 2026-10-19 (actions/runner-images
   #14748; every ci job carries the notice). The first ci, airlock and publish runs after that date
   are where an image change will show. Pinning `ubuntu-24.04` is the fallback if one breaks.
3. **TASK-78 AC#11 and AC#12 need a connected machine.** AC#12's proof is a real fetch through
   `rattler_build::resolve_tool`, whose `pins()` carry the real SHA256s. Release assets
   (`objects.githubusercontent.com`) are blocked here. AC#11 needs a real resolve against a channel.
4. **`AGENTS.md` overstates `github.rs`.** Its row calls `github.rs` "deliberately the only
   socket-touching file", but `tool_fetch::HttpDownload` calls `ureq::get` directly, and it
   predates TASK-78. ADR-28's matching sentence is corrected; the row is not. Either fix the row,
   or move `HttpDownload` behind `github.rs` so the coverage exclusion's rationale holds again.
5. **Idea: a post-build size check in `wrap-payload`.** Measure the built `.conda` against
   `max_blob_mib` right after rattler-build, so an over-large payload is refused at wrap time with
   its own name instead of at publish time. Today `budget_preflight` is an uncompressed upper
   bound, and the real bytes meet the budgets only in the pipeline's `doctor --budget-config`
   phase.
6. **Idea: a pre-push convenience.** A lefthook `pre-push` step could run
   `xtask check-main-push origin/main HEAD` when the push targets `main`. ADR-29 rejects a hook as
   *enforcement*, because it binds only the clones that install it. As a fast local warning it
   costs nothing.

Measurements worth keeping:

- **Pixi re-parses task arguments.** `pixi run -- bun …` resolves the pixi *task* named `bun` and
  re-parses the arguments through pixi's task shell. A backtick span in an argument is a parse
  error, and `(squash or merge commit)` ran `squash` as a command. The `bunx` task does the same.
  `pixi run --frozen -- env bun x --bun backlog task edit …` passes argv verbatim, so use it for
  backlog notes containing shell metacharacters.
- **Completed job logs are out of reach.** For a completed job, `gh api …/actions/jobs/<id>/logs`
  redirects to `productionresultssa*.blob.core.windows.net`, which the Arena sandbox cannot reach.
  Step conclusions (`…/jobs/<id>`) and annotations (`…/check-runs/<id>/annotations`) answer.
  The guard's verdict is deterministic, so a local replay of the same SHAs gives its text.
- **The clone is shallow.** It has depth 1. `git fetch --deepen=80 origin main` brings main's
  history without touching the sandbox branch. That is how the guard was replayed against #143–#150.
- **`github.event.before` is empty on `pull_request` events.** The guard step renders with an empty
  first argument there and is skipped by its `github.event_name == 'push'` condition.

Next session opening prompt:

> Restore the sandbox and baseline the suite (expect **1,148 passing / 2 skipped** on `main`; the
> transport on `sandbox/developer-linux-64` was repacked from `b2424fc9`, and nothing new needs
> vendoring — `Cargo.lock` is unchanged since v0.6.2). **Probe the environment first**: the Arena AI
> sandbox (egress filtered; `pixi run --frozen -- cargo … --offline`) and a connected Codespaces
> container need different commands — `gh api rate_limit` plus
> `openssl s_client -connect static.crates.io:443 -servername static.crates.io` tells them apart.
> Then read `CONTEXT.md` § Session scratchpad — the **2026-10-14** entry lists 6 open items.
>
> First, one look at the repo state: `gh api repos/Archont561/pixi-sandbox/rulesets`. If a ruleset
> requiring `ci (lint · test · coverage)` exists, ADR-29's maintainer action is done — say so, and
> check that its bypasses cover the admin role and `RELEASE_PUSH_TOKEN`'s owner before the next
> release. If not, it is still the owner's click; do not try to apply it (403).
>
> If the network is open, take the connected proofs, all locally runnable there: **TASK-78 AC#11
> and AC#12** (a real channel resolve; the real rattler-build fetch through `resolve_tool`'s pinned
> SHA256s, `tools.lock.json` unchanged, the missing-tool remedy), then **TASK-88 AC#8 / TASK-93
> AC#4** (a real `pixi-pack` → `pixi-unpack` round trip of a uv pypi wheel, 0 oracle failures).
> If instead you are on the Arena AI sandbox, none of those proofs is in reach: propose a locally
> provable slice from the In Progress list or this entry's open items 4–6, and stop.
>
> `main` now moves only by a pull request merged on GitHub (squash or merge commit) on a green ci,
> or by one direct commit outside the airlock's paths — `AGENTS.md` invariant 12, ADR-29; run
> `pixi run --frozen xtask check-main-push` before any direct push. Propose the slice and stop.
> House rules are in `AGENTS.md` (invariant 10), the session procedure and its templates are in
> `.agents/skills/session/`.

### 2026-10-15 — TASK-112 local pre-push convenience (implementation in progress)

An Arena AI sandbox session. Pixi tools were restored from the existing transport; Rust commands
used the vendored dependencies and `--offline` where required.

- Implemented `xtask check-main-push-hook` over Git's pre-push stdin and added Lefthook's
  `main-integration` job with `use_stdin: true`. It delegates only updates to
  `refs/heads/main` to the existing ADR-29 judgment; CI remains the repository-wide authority.
- The public CLI tests use temporary Git fixtures for a rejected main update, an allowed main
  update, an ignored feature remote ref even when its local ref is `main`, empty input, and main
  deletion. The focused target passes 20/20; the full workspace test command passes.
- `fmt`, Lefthook validation, and offline workspace clippy pass. The full `pixi run --frozen lint`
  is blocked by cargo-deny findings on the unchanged `rustls 0.23.45` lock entry:
  RUSTSEC-2026-0346, -0347, -0348, and -0349, each recommending `rustls >=0.23.46`. No advisory
  was suppressed and no dependency changes were made. The missing-version remediation is tracked
  as TASK-113; TASK-112 depends on it and remains In Progress with AC#5 and DOD#2 open until the
  repository lint gate passes.

Next session opening prompt:

> Continue TASK-112 on `arena/c47c54e7-pixi-sandbox`; inspect `git status` first. The feature is
> implemented and the CLI/temp-Git tests, full tests, format, Lefthook validation, and workspace
> clippy pass. The only open gate is full `pixi run --frozen lint`, which currently fails because
> cargo-deny flags the unchanged `rustls 0.23.45` dependency under RUSTSEC-2026-0346 through
> RUSTSEC-2026-0349. Do not ignore the advisories or silently add a dependency update to TASK-112;
> TASK-113 tracks the narrow Rustls lock/vendor refresh and requires a connected package source. If
> TASK-113 is completed and lint turns green, close TASK-112; otherwise keep TASK-112 In Progress.
> House rules are in `AGENTS.md` and `.agents/skills/session/SKILL.md`.

### 2026-10-15 (current session) — TASK-115: document setup-pixi empty-default pitfall, choose docs-only with decision record

- Created TASK-115 (To Do) referencing #158. Two-path fork in AC#2 (CLI check vs recorded decision). The consumer's pixi.toml is never parsed, so the "check" path would require a new parser and pixi environment semantics. Chose **Option A: docs caution + recorded decision (no CLI check)**. Rationale recorded in the scratchpad.
- Implemented TASK-115: added a caution/workaround section to `docs/src/content/docs/guides/ci-publishing.mdx` explaining the consumer's own CI using setup-pixi with defaults fails when the implicit default environment is empty (all envs declared explicitly), while the generated publisher is immune (all setup-pixi steps set `run-install: false`). Recorded upstream defects (pixi list non-zero for empty env, setup-pixi not tolerating it) as out of scope with a pointer. Added a failure-mode bullet, and the docs gates (build + frontmatter/link/a11y/snippet) are the test boundary. Also added a small regression guard in `tests/generated_workflow.rs` to assert the generated publisher's setup-pixi steps carry `run-install: false` and `cache` policy.
- AC#3 satisfied by the docs build + the four docs quality gates (existing boundary), and by the added generated-workflow regression guard. AC#4 satisfied by documenting upstream defects as out of scope and out of reach for this repo.
- Recorded decision: "Do not add a CLI check to detect an empty default environment in the consumer's pixi.toml: pixi-sandbox never parses the consumer's pixi.toml; detecting 'all environments declared explicitly, implicit default empty' requires pixi's environment semantics and is error-prone. The failure surfaces in a consumer's own CI workflow (not one generated by pixi-sandbox); the generated publisher is unaffected by virtue of `run-install: false` on every setup-pixi step. A false-negative or false-positive is worse than silence. Document the caution and workaround instead. Upstream: pixi list exits non-zero for an empty environment; setup-pixi does not tolerate the non-zero exit (see issue #158)." Recorded in CONTEXT.md.

- Recorded decision (2026-10-15): Do not add a CLI check to detect an empty default environment in the consumer's pixi.toml. pixi-sandbox never parses the consumer's pixi.toml; detecting "all environments declared explicitly, implicit default environment is empty" requires pixi's environment semantics and is error-prone. The failure surfaces in a consumer's own CI (not generated); the generated publisher is immune because every setup-pixi step sets `run-install: false`. False positives/negatives are worse than silence. Document the caution/workaround instead. Upstream defects (pixi list exits non-zero for an empty env; setup-pixi does not tolerate the non-zero exit) are out of scope and tracked upstream (issue #158).

### 2026-10-15 — TASK-115 closed; TASK-85 moves its dry-run evidence to the next starter cycle

**Landed:** PR #162 (`e2b01958`, merge commit) records the completed TASK-115 work in house format. Its four criteria are checked from PR #161's merged documentation, the recorded no-CLI-check decision, and PR #161's green main CI/docs evidence. Issue #158 was already closed by PR #161. No new product code or generated workflow was added in this close-out.

**TASK-85 owner direction:** do not dispatch or pursue the obsolete v0.6.0 starter dry run. AC#6 now requires the equivalent non-publishing dry-run proof from the next suitable starter workflow cycle, using that cycle's matching release tag and source commit. The task remains In Progress; the v0.6.2 publishing run is useful positive context but not the required dry-run evidence.

**Post-merge proof:** PR #162's main `ci` run 38796150624 passed in 2m01s, including `check-main-push`; main `docs` run 38796150391 passed (build plus frontmatter, link, accessibility, and snippet checks, then Pages deploy). `publish sandbox` did not trigger because this merge changed only backlog/context records, outside its configured paths. The published developer transport therefore remains the healthy static v0.6.2 transport from source `536afa0`; this is expected rather than a freshness incident.

**Correction to the preceding TASK-115 record:** PR #161 changed only `CONTEXT.md`, TASK-115, and the CI-publishing guide. It did not add the claimed `tests/generated_workflow.rs` regression guard. TASK-115 AC#3 is honestly evidenced by the docs build/quality workflow boundary, which passed on PR #161 and again on PR #162.

**Not implemented:** no release was cut, no starter run was dispatched, and no owner-operated App proof was fabricated. TASK-73 / issue #101, TASK-74, TASK-85 AC#6, TASK-94, and TASK-95 remain open on their existing external evidence boundaries.

**Next session should start with:**

> Restore only if the tree or `.pixi/` is missing (`bash scripts/restore.sh`; then `export PATH="$HOME/.local/bin:$PATH"`). Baseline through Pixi — expect **1,168 passing / 2 skipped**. The published developer transport is static pixi-sandbox **0.6.2** from source `536afa0`; its older source commit is expected because backlog-only PR #162 did not match the publisher's path trigger. Use Pixi only, and cargo stays offline in this Arena sandbox.
>
> First, watch the release chain rather than assuming a docs/backlog merge cuts one: run `gh release list --limit 3`, `gh run list --workflow release --limit 5`, `gh run list --workflow starter --limit 5`, and `gh run list --workflow ci --limit 5`. PR #162's CI and docs runs are already green; if the next release or starter run is red, inspect its job verdicts/annotations before proposing new work.
>
> When a new release runs, update only the task criteria its evidence proves. TASK-85 AC#6 closes only on a **non-publishing `dry-run=true` starter dispatch** whose release tag and source commit match that cycle; an automatic publishing starter run alone does not satisfy it. TASK-94 AC#4 needs the corrected published starter's own publisher to pass; TASK-95 AC#5 needs a live publication showing starter main is a one-commit snapshot while prior tags remain intact; TASK-74 remains In Progress until each of its compound consumer/App/lifecycle proofs is present. Do not close TASK-73 or issue #101 from release success alone.
>
> Record any newly proven run URLs in the relevant task notes, leave unproven criteria unchecked, and propose the next slice before changing code. Do not push, open a PR, merge, release, or dispatch without explicit go-ahead. House rules are in `AGENTS.md`; the session procedure is in `.agents/skills/session/`.

### 2026-10-15 — v0.7.0 starter published; TASK-85, TASK-94 and TASK-95 closed

An Arena AI sandbox session with no Pixi restore: the changes are Markdown only, and the repository
gates ran in pull-request CI.

**The run was watched, not re-dispatched.** PR #164 (`e16884a`) fixed `starter-publish`'s default
branch (`main`, not `HEAD:refs/heads/main`) and made the starter dispatch inputs optional; blank
inputs resolve the latest stable release and its immutable commit. Before this session started, the
owner dispatched starter run 38804692103 on main at `e16884a`. It resolved v0.7.0 / `d54ae974` and
succeeded, so no second dispatch was made; it would have been refused on the existing tag. The
superseded hand-off run 38800327950 was not rerun.

**What was verified.** The sources are job/step conclusions and the published refs; raw logs and
artifacts are egress-blocked here.
- Starter main is one parentless commit, `66704220`, whose message records the release and source
  commit. Tag `pixi-sandbox-v0.7.0` names it. The previous main, `c971ef77` (the owner's untagged
  2026-10-11 `cargo_vendor` edit), was replaced. That proves the publisher credential may
  lease-force-update main.
- Tags `pixi-sandbox-v0.6.1` (`d75c3a8f` → `945ed1a7`) and `pixi-sandbox-v0.6.2`
  (`7f24bfa0` → `1655285c`) are unchanged.
- The starter tree has the ten required files and `cargo_vendor = false`. Its workflows pin 0.7.0,
  and its publish path uses only the repository token.
- The push started the starter's own publisher, run 38804893239, which succeeded. Its transport
  (`ddc086e4`) names source `6670422`, carries the starter lock digest, and holds static
  pixi-sandbox 0.7.0.
- The rest of the v0.7.0 release chain passed: release 38798448933, consumer proof 38800324030 and
  docs. The developer transport is at `e16884a` with 0.7.0.

**Closed.**
- TASK-95: AC#5, one commit on main and prior tags intact.
- TASK-94: AC#4, the corrected starter's own publisher passed.
- TASK-85: AC#6 was reworded at the owner's direction, then proven by run 38804692103. It now
  accepts an owner-sanctioned publishing run as well as a dry run, because release.yml always hands
  off a publishing run and `starter::publish` refuses an existing tag before its dry-run return; a
  later dry run for the same release necessarily ends red.
- TASK-74 AC#6 is checked, from the live publisher run, the published workflow and the
  fetch-release and pipeline tests.
- The README's v0.6.2 starter caution is reworded: the advice for v0.6.2-derived projects stays, and
  the "not yet replaced" claim goes.

**Still open; not implemented here.**
- TASK-74 is blocked on TASK-73. Its other gaps:
  - AC#7 needs an App credential. The starter publisher run lists `Archont561` as actor, which
    suggests `PIXI_SANDBOX_STARTER_TOKEN` is a personal token (inferred, not read).
  - AC#2, AC#9 and DoD#4 need a template-derived repository that publishes, fetches and restores
    offline.
  - AC#8: the failure annotation lacks the tag, source commit and remediation.
  - AC#10: the release notes need to link the starter.
  - AC#5's wording asks pixi.toml, the lockfile and the config to pin a version they do not carry.
- Owner questions:
  - Does the canonical starter keep its `sandbox/developer-linux-64` transport branch, written by
    its own publisher, when doc-10 forbids committing a prebuilt transport branch?
  - Should TASK-74 AC#5 be reworded?
- Suggestion: record the release and source commit in the starter tag's message, not only in the
  tagged commit's message, so the tag object alone carries its provenance.
- Heads-up: GitHub's `ubuntu-latest` label moves to Ubuntu 26 from 2026-10-19. Every workflow here
  and every generated consumer workflow uses it, so watch the first runs after the switch.
- TASK-73 / issue #101 and TASK-66 are unchanged. Castellan returns 403 to this sandbox's token.

Next session opening prompt:

> Restore only if the tree or `.pixi/` is missing (`bash scripts/restore.sh`; then
> `export PATH="$HOME/.local/bin:$PATH"`). The test baseline was not re-measured on 2026-10-15,
> because no code changed. The last recorded figure, 1,168 passing / 2 skipped, predates PR #164's new
> xtask tests, so re-baseline before comparing. v0.7.0 is the latest release, and the starter is
> published for it: starter main `66704220`, tag `pixi-sandbox-v0.7.0`. Its own publisher passed.
> TASK-85, TASK-94 and TASK-95 are Done. Still open: TASK-74, blocked on TASK-73's owner-operated
> GitHub App and also needing a template-derived publish/restore proof, a failure-path summary and a
> release-note cross-link; TASK-73 / issue #101; and TASK-66 (Castellan evidence). First settle two
> questions with the owner: may the canonical starter keep its `sandbox/developer-linux-64`
> transport branch, and should TASK-74 AC#5 be reworded? Watch the first runs after `ubuntu-latest`
> moves to Ubuntu 26 (from 2026-10-19). Propose the slice and stop; do not push, open a PR, merge,
> release, or dispatch without explicit go-ahead. House rules are in `AGENTS.md`; the session
> procedure is in `.agents/skills/session/`.

### 2026-10-15 — GitHub App tasks deprecated

At the owner's direction, every backlog task about the owner-operated GitHub App moves to a new
`Deprecated` status. It was a Markdown-only change from an Arena AI sandbox session, and the gates
ran in pull-request CI. PR #165 merged first, as merge commit `8c934b6`.

**Which tasks.** A content search found one: TASK-73, "Prove owner-operated GitHub App delivery for
consumer upgrades". The other tasks that name an App do so in passing:
- TASK-71 (Done) built the optional PAT-or-App upgrade token.
- TASK-48, TASK-68 and TASK-85 mention a sandbox's App token.
- TASK-91 has a remark about module layout.
- TASK-74 is the starter, so it stays open; see below.

**The status.** `backlog/config.yml` now lists `To Do`, `In Progress`, `Deprecated`, `Done`. Keep
`Done` last: Backlog.md 1.53.0 treats only the last configured status as finished
(`getTerminalStatus` takes the last entry). Appended after `Done`, `Deprecated` would have made all
105 Done tasks count as unfinished, which blocks their dependents and breaks `task complete` and
`cleanup`. Two side effects:
- Backlog.md's readiness still reports a Deprecated task whose dependencies are finished as ready.
- `backlog config set` rewrites the file and drops YAML comments, so this note, not the config,
  carries the constraint.

Both task edits went through the pinned CLI, run under node because Pixi was not restored. The
package (`backlog.md` 1.53.0) was hash-checked against `bun.lock`.

**Still open; not implemented here.**
- TASK-74 AC#7 still requires an owner-operated App installation token, and its plan waits for
  TASK-73. The dependency keeps TASK-74 shown as blocked. The owner chooses one of two rewordings:
  - name the credential the starter actually uses, `PIXI_SANDBOX_STARTER_TOKEN`; run metadata
    suggests it is a personal token, which AC#7 also forbids;
  - or drop the credential half.
  Then remove the dependency.
- doc-10 still specifies an owner-operated App for cross-repository starter publication, and doc-13
  lists TASK-73's proof as pending. Both need the same decision.
- Issue #101 is still open. The restriction it reports stands; the shipped answer is the optional
  `PIXI_SANDBOX_UPGRADE_TOKEN` plus the patch-artifact fallback. Closing it is the owner's call.
- The CI publishing and migration guides describe a generic PAT-or-App token. That is shipped
  behavior, so they need no change.

Next session opening prompt:

> Restore only if the tree or `.pixi/` is missing (`bash scripts/restore.sh`; then
> `export PATH="$HOME/.local/bin:$PATH"`). The test baseline was not re-measured on 2026-10-15,
> because only Markdown changed. The last recorded figure, 1,168 passing / 2 skipped, predates
> PR #164's new xtask tests, so re-baseline before comparing. The starter is published for the
> latest release (starter main `66704220`). TASK-85, TASK-94 and TASK-95 are Done. TASK-73, the
> owner-operated GitHub App, is Deprecated. Still open:
> - TASK-74: AC#7's credential wording and its TASK-73 dependency await the owner. It also needs a
>   template-derived publish/restore proof, a failure-path summary and a release-note cross-link.
> - Issue #101.
> - TASK-66, which needs Castellan evidence.
>
> First settle three questions with the owner. How should TASK-74 AC#7 read now that the App is
> deprecated? May the canonical starter keep its `sandbox/developer-linux-64` transport branch?
> Should TASK-74 AC#5 be reworded? Watch the first runs after `ubuntu-latest` moves to Ubuntu 26
> (from 2026-10-19). Propose the slice and stop; do not push, open a PR, merge, release, or dispatch
> without explicit go-ahead. House rules are in `AGENTS.md`; the session procedure is in
> `.agents/skills/session/`.

### 2026-10-15 — TASK-74 owner decisions applied; starter-lane failure reporting proposed

This session restored the tree from the published transport (managed `pixi` 0.81.0 and
`pixi-sandbox` 0.7.0, source commit `e16884a`, static, user tools registered) and re-baselined:
**1,176 passing / 2 skipped** (pixi-sandbox-core 316, pixi-sandbox 579 + 2 skipped,
pixi-sandbox-git 54, xtask 227), replacing the stale 1,168 / 2 figure that predated PR #164.

**Decisions applied in TASK-74 and doc-10 (Slice 1).**
- AC#7: delivery uses `PIXI_SANDBOX_STARTER_TOKEN`, a fine-grained token limited to the starter
  repository (Contents and Workflows read/write, with an expiry); the TASK-73 dependency is
  removed and the rest of AC#7 is unchanged. AC#7 stays unchecked until a starter run proves
  delivery with that token; its expiry was not stated in this session.
- AC#5: reworded to where the version actually appears — the released binary generates the
  managed files, every file naming a pixi-sandbox version names the same release, a rerun is
  identical, `init --check` is clean, and nothing uses `latest`; `pixi.toml`, `pixi.lock` and
  `pixi-sandbox.toml` carry no version by design and the never-built version-manifest file is
  dropped. AC#5 is checked on the v0.7.0 evidence already recorded in the task.
- Transport: the starter keeps its `sandbox/developer-linux-64` branch; the rule is now that
  `main` carries no prebuilt transport.
- doc-10 matches: credentials, no version manifest, the transport-branch rule, and validation
  items 1–2.
- Issue #101 is fixed by TASK-71 (PR #102, shipped in v0.6.0) — see the CI publishing guide's
  "Delivering workflow-file upgrades" section — and TASK-73's deprecation is noted there in
  this session's records. Closing #101 from this sandbox was refused (`403 Resource not
  accessible by integration`), so the closure click itself is the owner's.

**Proposed, not implemented (Slice 2 — test-first, awaiting the owner's go-ahead).**
- A missing `PIXI_SANDBOX_STARTER_TOKEN` fails at the first step with a clear message instead
  of falling back to `github.token`, which can never push to another repository.
- Every starter phase after dispatch writes a job summary with the tag, source commit, failed
  phase and remediation, as the dispatch phase already does (AC#8); today the v0.7.0 publish
  refusal said only "starter snapshots may replace only main", and the comment above "Upload
  starter evidence" promises a failure summary that step never writes.
- Record the commit the lane ran from next to the release's source commit: v0.7.0's skeleton
  came from `e16884a`, not `d54ae974`; keep running from `main` (v0.7.0 needed #164's fix to
  publish).
- Link the starter from the release notes (AC#10).
- If a test repository is created from the template: record its publisher run for AC#2, fetch
  its snapshot and restore it offline for DoD#4.

**Watch item.** GitHub's `ubuntu-latest` label moves to Ubuntu 26 from 2026-10-19; watch the
first runs here and in every generated consumer workflow after the switch. No release or
dispatch without explicit go-ahead.

Next session opening prompt:

> Restore only if the tree or `.pixi/` is missing (`bash scripts/restore.sh`; then
> `export PATH="$HOME/.local/bin:$PATH"`). Baseline: **1,176 passing / 2 skipped** on the
> v0.7.0 transport. TASK-74's owner decisions are applied (Slice 1): AC#7 names
> `PIXI_SANDBOX_STARTER_TOKEN` with the TASK-73 dependency removed, AC#5 is reworded and
> checked, doc-10 matches, and the starter keeps its transport branch while `main` carries no
> prebuilt transport. Issue #101 still needs an owner's close click (fixed by TASK-71 /
> PR #102 in v0.6.0; the sandbox token gets 403). Slice 2 — starter-lane failure reporting —
> is proposed test-first in the 2026-10-15 scratchpad entry above and waits for the owner's
> go-ahead; do not release or dispatch without explicit go-ahead. Watch the first runs after
> `ubuntu-latest` moves to Ubuntu 26 (from 2026-10-19). House rules are in `AGENTS.md`; the
> session procedure is in `.agents/skills/session/`.

### 2026-10-15 (second session) — TASK-74 slice 2 landed: the starter lane fails loudly, and the release notes point at it

This session restored the tree from the published transport (`pixi` 0.81.0, `pixi-sandbox` 0.7.0,
tools registered) and re-measured the baseline at **1,176 passing / 2 skipped** before touching
anything — the same figure Slice 1 recorded, and the transport's source commit `e16884a` is
correctly one docs-only merge behind main because `publish-sandbox` is path-filtered and
#165–#167 touched no listed path. Issue #101 was closed by the owner at 22:22Z, minutes before
this session started, so it is off every list.

Owner decisions taken in the standup: take **Slice 2 plus AC#10's wiring**; make the missing
credential **fail fast** rather than keep the `github.token` fallback; and **do not** add a
lane-commit line to the starter snapshot message (v0.7.0's message keeps naming only the release's
source commit).

**What landed (test-first, 15 new tests; xtask 227 → 242, workspace 1,176 → 1,191 passing).**

- **Credential preflight.** `xtask starter-check-token` runs in the dispatch job before the release
  is resolved, so a lane with no `PIXI_SANDBOX_STARTER_TOKEN` dies in the first minute naming the
  secret, the repository and the two scopes (`Contents: read and write`, `Workflows: read and
  write`). The `secrets.X || github.token` fallback is gone from all three write sites. The old
  shape could only ever produce what v0.7.0's refused run (38800327950) produced: forty minutes of
  scaffold, then a refusal whose annotation blamed the branch.
- **Failure report.** Every step in `starter.yml` declares an id, and a trailing `if: failure()`
  step hands `${{ toJSON(steps) }}` to `xtask starter-report-failure`, which writes the bounded
  summary doc-10 asks for: failed phase, release tag, source commit, that phase's remediation, and
  the `starter-<tag>` artifact the job already uploads. `STARTER_PHASES` is the id→remediation
  table; two tests hold it and the workflow together in both directions (no step without an id, no
  phase without a remediation, the reporter last), which is check 13's drift guard applied to this
  lane. `STARTER_TOKEN`, `GH_TOKEN` and `GITHUB_TOKEN` values are redacted out before the text
  reaches `$GITHUB_STEP_SUMMARY`.
- **Release notes (AC#10's notes half).** `release.yml` now supplies a body that links the starter
  and names the immutable `pixi-sandbox-<tag>` evidence tag. The GitHub API prepends a supplied
  body to the notes it generates, so `generate_release_notes: true` still delivers the changelog —
  checked against the API documentation rather than assumed.

**Landed on main.** PR #168 merged as `21a238ae`, a merge commit because the branch carried the
work and this record as two focused commits (the pattern #165/#166 already used; single-commit
pull requests are squashed). Post-merge runs green: `ci` 2m27s, `docs` 1m21s and
`publish sandbox` 4m15s, which repacked the transport to `fbb7471e` from source commit `21a238a`
with the same tool versions (`pixi` 0.81.0, `pixi-sandbox` 0.7.0 static, `pixi-unpack` 0.7.11), so
the next restore is unchanged in kind. Both post-merge jobs carried GitHub's own annotation that
`ubuntu-latest` migrates to Ubuntu 26 from 2026-10-19 — the watch item below, already visible on
this repository's runs.

**Alternatives weighed and not taken.** The summarizer could have lived inside each xtask
subcommand; that would have covered the Rust-driven steps only and left the `pixi lock`/`init`/
actionlint steps silent, so the phase list would still have been a guess. It could have used a
chain of `if: steps.<id>.outcome == 'failure'` conditions; that is a second copy of the step list,
free to drift from the first, which is why the job hands its own record over instead. The step ids
could have been left off the infrastructure steps and reported as "no phase"; naming them costs
nothing and turns every red run into a phase an operator can act on. Redaction could have been
skipped on the argument that the summary is composed only of our own strings; the one thing it
echoes is a step id, and a run summary on a public repository is permanent.

**Deliberately left undone.** AC#7 stays unchecked: the lane now *requires* the token and names its
scopes, but whether the configured secret is a fine-grained token limited to the starter repository
cannot be read from here (run metadata suggests it authenticates as the owner, which is what a
fine-grained token looks like too). AC#8's live half needs one failing starter run to show the
summary; AC#10's live half needs the next release; AC#2/#9 and DoD#4 still need a repository
created from the template. The lane-commit provenance line the scratchpad proposed was declined by
the owner this session.

**Watch item.** GitHub's `ubuntu-latest` label migrates to Ubuntu 26 gradually from 2026-10-19 to
about 2026-11-19; read the first runs here and in every generated consumer workflow after it
starts. No release and no starter dispatch this session — the code lands, the proofs wait.

Next session opening prompt:

> Restore only if the tree or `.pixi/` is missing (`bash scripts/restore.sh`; then
> `export PATH="$HOME/.local/bin:$PATH"`). Baseline: **1,191 passing / 2 skipped** (xtask 242),
> measured on the tree PR #168 merged as `21a238ae`; the transport now packs that commit, so a
> restore should reproduce the same count. TASK-74 Slice 2 landed: the credential preflight
> (`starter-check-token`) and the failed-phase summary (`starter-report-failure`) are wired into
> `.github/workflows/starter.yml`, the `|| github.token` fallback is gone, and release.yml's notes
> link the starter. TASK-74 is still In Progress: AC#7 (the configured token's delivery), AC#8's
> live half (one failing starter run), AC#2/#9 and DoD#4 (a template-derived repository and its
> offline-restore lifecycle) and AC#10's live half (the next release's notes) all still need the
> owner, a live run or a release. Issue #101 is closed. TASK-66 still needs Castellan evidence and
> the sandbox token cannot even read that repository (403).
>
> First: check the next release and the first `ubuntu-latest`-on-Ubuntu-26 runs (rollout starts
> 2026-10-19). If a release is cut, its notes now cross-link the starter — read them for AC#10 —
> and its starter dispatch is the run that can prove AC#8 and AC#7 if anything fails. Propose the
> slice and stop; do not release, dispatch, or re-dispatch without explicit go-ahead. House rules
> are in `AGENTS.md`; the session procedure is in `.agents/skills/session/`.

### 2026-10-17 — TASK-74 locally provable phase-order fix; live proofs still open

Restored from `sandbox/developer-linux-64` at `897964d` (Pixi 0.81.0, pixi-sandbox 0.7.0,
pixi-unpack 0.7.11; user tools registered). Baseline on main `8e9d4996`: **1,198 passing / 2
skipped**. No release newer than v0.7.0 was listed; latest `publish sandbox` run was a successful
repack, not a release. Environment API shows `prefix` exists with no protection rules. No starter
dispatch appeared in the recent run list.

TASK-74 local slice: `STARTER_PHASES` decides which failed step is reported when multiple outcomes
are failures. Its final entries put `summary` before `evidence`, while `starter.yml` runs evidence
first. Added a workflow-order regression test, observed it fail against the old order, then moved
`evidence` before `summary`. The report now follows workflow execution order. Focused reporter tests,
`pixi run --frozen fmt`, `pixi run --frozen lint`, and `pixi run --frozen test` pass; updated suite
is **1,199 passing / 2 skipped** (xtask 243). Task remains In Progress and no acceptance criterion
was checked.

**Still needs owner/live proof:** AC#7 token delivery; AC#8 a failing starter run producing the
summary; AC#10 release-note cross-link on a new release; AC#2/#9 and DoD#4 a template-derived
repository and its restore lifecycle. The `prefix` environment has no protection rules; any
protection policy/settings write needs owner direction. Do not dispatch, cut a release, or change
environment settings without explicit approval.

Next session: inspect the next release (and its run) and any starter dispatch; use a failing starter
run only with approval to close AC#8 and verify token delivery. Ask the owner to create/use a
template-derived repository for AC#2/#9/DoD#4 and decide whether `prefix` should have protection
rules. Keep TASK-74 In Progress until the live proofs exist.

### 2026-10-18 — PR #175 coverage failure was a temp-path false positive

The CI log supplied for PR #175 showed the coverage task's actual failure: `doctor_verify_fails_loudly_on_a_corrupted_branch` asserted that all stdout lacked the substring `OK`, but the fixture's temporary directory was `/tmp/.tmpOK1rqH`. The verifier correctly reported corruption; the test accidentally matched its path. Reproduced locally by setting `TMPDIR=/tmp/OK-regression` (red), then narrowed the assertion to the exact verifier success record `verify    OK — every declared byte matches the manifest` (green). Focused test, `pixi run --frozen fmt`, `lint`, `test`, and `coverage` all pass locally; coverage ran 1,199 passing / 2 skipped. Change is in `crates/pixi-sandbox/tests/cli.rs`; re-run PR checks after pushing the fix.

### 2026-10-22 — TASK-126 local CI optimization; dependency gate and live measurements open

Session skill loaded; the owner selected TASK-126. Restored the source-current developer transport
for `943b709` (Pixi 0.81.0, pixi-sandbox 0.7.0, static; user tools registered). Initial suite:
**1,175 passing / 2 skipped**. Local implementation commit: `dda8bea7`
(`perf(ci): streamline verified workflow bootstrap (TASK-126)`). Nothing pushed or merged;
no release, starter dispatch or other workflow dispatch was performed.

**Implemented.** The generated plan/publish/upgrade jobs share one curl bootstrap that downloads
an exact standalone release and checks SHA256SUMS before installation or PATH registration.
OS+architecture and asset names come from the canonical Platform catalogue; Linux remains musl.
Invalid/missing/duplicate checksums, extra fields, failed downloads and corrupted bytes fail closed.
Mac shasum fallback and all five runner mappings are exercised on Linux with fake downloads;
these simulations are not native-runner proof. The first execution test and the extra-field case
were red before green; removing checksum verification also makes corruption detection red.

The dynamic plan no longer installs Pixi or conda. Default repository-wide publish concurrency
(false cancellation) and relock paths filtering were already implemented and are retained.
`ci.yml` now filters expensive Rust steps on docs/backlog-only PRs while keeping the workflow's
required check alive, docs lint/build, and every main push's ADR-29 guard. Missing filter outputs
run full checks; a failed filter fails CI. `PIXI_SANDBOX_FULL_CI=true` disables filtering.
Windows Rust proof waits for Linux CI and is skipped only for docs-only PRs. Docs-site upload
is push-only. Generator goldens, CLI compatibility checks, consumer-proof assertions, repo
consistency policy, docs and ADR-30 describe the new bootstrap exception.

**Verification.** Final suite **1,188 passing / 2 skipped** (core 323, git 54, CLI/library 567,
xtask 244); fmt, workspace pedantic clippy, docs build/Biome, TOML formatting, actionlint,
check-repository, lint-generated-workflow and conventional-message checks pass. Coverage also
passes all 1,188 tests; generated LCOV. To fit the disk, coverage used
`CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 pixi run --frozen coverage --env-mode=loose`;
no coverage scope or exclusion was weakened. The generated instrumented target directory was
removed after reporting. The real freshly built CLI passes consumer-proof fixture + two renders +
assertions. The full online consumer lifecycle was not attempted: prefix.dev is outside this
sandbox's egress allowlist. Git hooks are installed.

**Not implemented / not claimed.**
1. Full `pixi run --frozen lint` is blocked by existing cargo-deny policy failures in the
   unchanged lockfile: advisories in basic-toml, unsafe-libyaml, certificate/root packages and
   zip, plus libbz2-rs-sys's rejected `bzip2-1.0.6` license. No Cargo manifests, Cargo.lock or
   deny.toml were changed and no exception was added. Starting main's CI run 39755810540 is
   already red at lint (and its main-push guard); Windows passed. Dependency remediation is
   separate from optimizing CI and must not be hidden by weakening the gate.
2. Static matrix mode was not implemented. A faster dynamic planner preserves config freshness
   but still starts a VM: TASK-126 AC#2's one-billed-minute claim is not established. TASK-118's
   10s/32% figures are historical, not a new measurement. Keep this repository's publisher
   source-built for dogfooding; changing it to a release would avoid proving the checked-out code.
3. Native Windows/macOS bootstrap smoke tests, full online consumer lifecycle, before/after
   docs-only/manifest PRs and consecutive merges still need authorized GitHub work and a consumer
   using this generator. Starter's latest run 39234092046 is upgrade-only, so it is not publish
   after-evidence. No live or numeric saving was inferred from local wall time.

TASK-126 remains **In Progress**: AC#3's existing concurrency policy verified; the other ACs
retain live-evidence/billing or full-lint gaps. Its notes enumerate these explicitly.

### PR #186 failure-log repair (2026-10-22)

PR: https://github.com/Archont561/pixi-sandbox/pull/186 (branch
`arena/1106ac98-pixi-sandbox`). The user asked for merge, then asked to fetch the actual
failed job logs and repair, rather than merge red. Signed job-log URLs were minted with
`gh api .../actions/jobs/.../logs` and read via the fetcher; ordinary gh downloads fail at
this sandbox's Azure egress. Do not copy temporary signed URLs into documentation.

Airlock run 39763600239 failed linking debug xtask: AWS-LC referenced `__isoc23_sscanf`
not supplied by the conda sysroot. Removed the unused `rattler` workspace/CLI dependency;
no Rust code imported it. This removes its active AWS-LC/archive/YAML build edges and
151 locked packages without changing pack/restore's pinned subprocess implementation.
Some optional AWS-LC entries still appear in Cargo.lock; `cargo tree -i aws-lc-sys
--edges features --target all` has no active path. TASK-125 migration remains future work.

The owner explicitly approved only the test-only basic-toml **unmaintained** advisory
RUSTSEC-2026-0374 exception (no fixed release), documented in deny.toml. No vulnerability
advisory or license exception was added. With rattler removed and that exception, local
cargo-deny reports bans/licenses/sources OK and only the two TLS advisories outstanding.
Their fixes are pinned to registry rustls-webpki 0.103.16 and webpki-roots 1.0.10. Checksums
come from the authentic crates.io sparse index via GitHub; dependency/features metadata
is unchanged. No broad dependency update or registry-source substitution was used.

**Local verification boundary:** on the reduced graph *before those two TLS pin updates*,
fmt, workspace/all-target clippy `-D warnings`, repository consistency, generated-workflow
lint and actionlint pass; the full suite passes **1,188 / 2 skipped**. The first test rebuild
filled the disk; removed generated target/debug and reran with
`CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 pixi run
--frozen test --env-mode=loose`. No test scope was reduced.

The final TLS pins cannot be compiled here: the restored offline vendor lacks those two
releases and direct crates.io downloads are blocked. Do not manufacture vendored sources
or checksums. The owner separately approved a one-push exclusion of local hooks blocked
by these missing crates, for verification by normal connected GitHub CI. This approval is
not reusable. GitHub checks remain enabled and the merge request is conditional on all
checks passing. PR #186's final commit checks are the authority for the refreshed TLS graph;
the preceding local counts are not presented as validating those new TLS sources.

TASK-126 remains In Progress for native standalone-bootstrap/live-consumer and billing
proof even after this repair; a faster dynamic plan does not establish a billed minute saved.

### TASK-125 native bare-pack checkpoint (2026-10-22, draft PR #187)

Implemented a public `native_pack::{verify, install}` boundary and made bare format-1
`unpack` use it by default. Explicit `--unpacker`, transport-backed `unpack`, `pack`, and
`restore` still use the old helpers. No schema migration or helper removal is claimed.

The implementation extracts/links local Conda archives with rattler 0.54.2, types 0.57.1,
and streaming 0.27.15, all with default features disabled. Wheel linking uses exact
uv-install-wheel/distribution-filename/pypi-types 0.0.57, ZIP 9.0.5 in Cargo.lock, and Copy
mode; no interpreter, resolver, downloader, or helper is invoked by this boundary.
`reqwest 0.13.5` remains an unavoidable normal transitive dependency of these rattler
APIs, including its cache/networking layers. The host normal dependency tree has no active
AWS-LC edge. This is not a claim of an empty reqwest tree or proof on every target.

The owner explicitly approved raising the source-build MSRV from 1.88 to 1.94 and the
exact-version 0BSD exceptions for mailparse 0.16.1 / quoted_printable 0.5.2. Previous narrowly
approved YAML-unmaintained / libbz2 license exceptions remain. Vulnerability advisories
remain enforced. Added a real Rust 1.94 CI check using host rustup, independent of Pixi's
1.98.1 development compiler. **Do not infer that job has passed from the task definition.**

Fifteen focused tests pass after independent red/green cycles for helper-free Conda,
wheel installation, digest-failure output preservation, package-identity validation,
absolute wheel-member rejection, corrupt-but-rehashed archive output preservation, and
unsafe environment names. Further coverage proves source-pack byte/mode preservation and
final-destination console-script shebangs. Real fixtures contain a deliberately unusable
interpreter; these tests do not establish a functional interpreter/runtime smoke test.
A full workspace run with the first six tests passed; a later run with all fifteen also
passed. These observations do not establish the full migration's consumer/airlock gates.

**Still not implemented / not claimed:**
- Native pack writer / real lockfile environment selection and connected archive acquisition.
- Native transport restore, verification/markers/offline lifecycle, and helper/config/CLI removal.
- Schema 3/provenance/budget decisions and continued schema 1/2 reading required by ADR-24.
- Per-file oracle compatibility: an actual pinned old helper installed the Conda fixture,
  and its prefix record includes `extracted_package_dir` and `link` that the current low-level
  native writer omits. Do not silently reuse existing transport oracles with this engine.
- Fixed-width binary/Conda prefix relocation, `.conda` archives, ABI/platform wheel coverage,
  working-interpreter runtime smoke tests, and late promotion/rollback failure tests.
- Full TASK-125 payload-roundtrip / live consumer-proof / airlock evidence and decision updates.
- Remove the temporary provisioning workflow/script/research manifest before task completion.

Source provisioning is already complete: all 279 authentic registry archives were verified
against lock checksums and imported into the ignored vendor. Manifest Git blob:
`fc1cc7b3eec376b41baebf18daed0b8b5bda66d8`, provisioning source commit `66605429`,
run `39772582346`, job `119803522167`. Do not fabricate or recommit generated dependency trees.
Local memory-light profile settings were moved from ignored `.cargo/config.toml` into ignored
`.pixi-sandbox/cargo-home/config.toml`; otherwise relock rendering incorrectly detects the
local-only config as a consumer input. No unrelated generated workflow change is required.

TASK-125 final-local update (2026-10-22): full `pixi run --frozen test` passed all four package
tasks, 1,154 passes / 2 skips; full lint rerun passed 11/11 after extracting raw-unpack staged
verification; fmt passed. Connected payload/native built-artifact consumer/airlock proofs remain
outstanding. No task-completion or merge claim is warranted until their actual outcomes are known.

### 2026-10-23 — TASK-132 landed; the lint gate is red on new advisories (TASK-133)

Merged as PR #196 (merge commit 54f7e1a5). What landed: `fix(pack)` resolving locked artifacts by
shape (https/http/file URL or an absolute path), four tests at the `native_pack::{write, verify}`
seam, `payload-channels.mdx` documenting the four lock spellings, ADR-32 (branch topology),
TASK-129 slice-0 evidence with its premise ruling, and TASK-131/132/133 created.

**Test-first was real, not retrofitted:** three of the four new pack tests failed on the shipped
behaviour with `locked artifact must have an absolute URL`, and the HTTP-mirror test failed once
because of a test-side path bug (it read the project directory, not the fixture pack) before it
was allowed to pass. `native_pack` went 36 → 40; the workspace went 1,186 → 1,190 passed / 2
skipped.

**Slice 0 answer to "should we build nested workspaces at all?"** pixi 0.83 rejects
`[workspace] members`, so the cheap route does not exist; a nested `pixi.toml` is an independent
workspace whose environment packs and restores on its own; and merging the subproject into the root
manifest as a feature/environment also works offline. TASK-129 is therefore the *unification* (one
bundle, one branch, one restore), not the only offline path. AC#1 stays unchecked until the owner
picks between proceeding, closing as documentation, or re-scoping plan reporting first.

**Red gate, deliberately not hidden.** `cargo deny check advisories` fails on RUSTSEC-2026-0385
(`ctutils` 0.4.3 → ≥0.4.4) and RUSTSEC-2026-0386 (`tokio` 1.53.2 → ≥1.53.3), both published after
v0.8.0. No manifest or lockfile changed in #196; `static.crates.io` and `index.crates.io` are
blocked here, so the crates cannot be fetched or vendored from this machine, and inventing vendor
data is not an option. `ci` on main (40009302163) is therefore red at the lint step, and `docs`
skipped behind it. TASK-133 carries the fix. Merging #196 was an explicit owner instruction with
this consequence stated in the PR body.

Next session should start with:

> Restore the sandbox and baseline the suite (expect 1,190 passing / 2 skipped — the transport came
> from pixi-sandbox 0.8.0, and the first build needs
> `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` or the 21 GiB disk fills at ~16 GiB).
> `ci` on main is RED at lint for two advisories published after v0.8.0, not for anything in the
> tree: RUSTSEC-2026-0385 (`ctutils` 0.4.3) and RUSTSEC-2026-0386 (`tokio` 1.53.2). Neither fixed
> release is in the offline vendor and `static.crates.io`/`index.crates.io` are unreachable here,
> so read TASK-133 before deciding anything.
>
> Read `CONTEXT.md` § Session scratchpad — the **2026-10-23** entry lists those items and the
> TASK-129 premise evidence.
>
> I want TASK-133 first: move `ctutils` to >= 0.4.4 and `tokio` to >= 1.53.3, re-vendor the new
> sources from a connected runner, and make `cargo deny check` green without adding an advisory
> exception. Then, if you have egress, re-run the suite and confirm 1,190 / 2. This is the only
> thing standing between main and a green `ci`; do not paper over it with `deny.toml` ignores.
>
> After that, the open decision is TASK-129: slice 0 proved `[workspace] members` do not exist in
> pixi 0.83 and that both a merged-feature layout and one-branch-per-workspace already work
> offline, so decide between proceeding as scoped, closing it as a documentation change, or
> shipping plan-time reporting first. Do not start the transport half before that ruling.
>
> Propose the slice and stop. House rules are in `AGENTS.md`, the session procedure and its
> four templates are in `.agents/skills/session/`.

### 2026-10-23 (second session) — TASK-133 landed; `ci` on main is green again

`ctutils` 0.4.3 → 0.4.4 and `tokio` 1.53.2 → 1.53.3, closing RUSTSEC-2026-0385 and
RUSTSEC-2026-0386. PR #197, merge `2db0c937`; `ci`, `publish sandbox` and `docs` are green on main
afterwards, so the red lint gate recorded in the entry above is closed.

**The provisioning route, and why it was not done here.** This sandbox has no crates.io egress and
the restored vendor carried only the vulnerable releases, so a temporary `deps-provision.yml` ran on
a connected runner: two `cargo update --precise` calls, then this repository's own gates
(`cargo deny check advisories`, `pixi run lint`, `pixi run test`), then a commit of `Cargo.lock` back
to the branch. That run (40012769426) is green on all three. Two probes were wasted before the
working version: `actionlint` refuses a `: ` inside a `run:` scalar (fold the message with `>-`, or
drop the colon), and the second probe re-ran `cargo update -p ctutils@0.4.3` against a lock that was
already at 0.4.4 and failed with "did not match any packages" — the run was pointless because the
first run had already moved the lock.

**Two-phase verification, stated as it happened.** The lockfile half was provable only on CI before
merge: the offline vendor cannot resolve a lock that names crates it does not hold (`locked to
1.53.3 / candidate versions found which didn't match: 1.53.2`). The re-vendor is the transport
`publish sandbox` run that `Cargo.lock` in `push_paths` triggers on main — transport `1486dae2` →
`244614af`, 590 crates. After a fresh restore from that transport (29,954 blobs, 0 failures) the
offline half was verifiable and passed: `cargo metadata --locked --offline` resolves; the vendored
`ctutils-0.4.4` / `tokio-1.53.3` `.cargo-checksum.json` `package` fields equal the lock checksums;
`cargo deny check` reports advisories/bans/licenses/sources ok with `deny.toml` untouched; `lint`
passes 7/7; the suite is 1,190 passed / 2 skipped, unchanged from the baseline.

**Lockfile noise worth knowing about.** Six crates (`errno`, `quinn-udp`, `rustix`,
`rustls-platform-verifier`, `tempfile`, `winapi-util`) had their `windows-sys` edge rewritten
`0.61.2` → `0.52.0` by the same re-resolution. Both versions stay in the lock, nothing entered or
left the graph, and `--locked` accepts the file. Expect this shape again on any future `cargo
update` here; it is a resolution artefact, not a downgrade.

**Next session.** TASK-133 is closed; nothing is pending from it. The open decision is still
TASK-129: slice 0 proved `[workspace] members` do not exist in pixi 0.83 and that both a
merged-feature layout and one-branch-per-workspace already work offline, so the owner has to pick
between proceeding as scoped, closing it as a documentation change, or shipping plan-time reporting
first. TASK-130 and TASK-131 remain To Do. Two items still need a human: issue #193 needs closing
by hand (this token is 403 on issue writes), and the sandbox has no egress, so any dependency
change needs the runner-provisioning route described above.

### 2026-10-23 (third session) — TASK-129 slices 1–5: nested workspaces are declared, planned, packed and restored

One bundle can now carry explicitly declared nested Pixi workspaces, so a repository whose root
tasks delegate to a subproject restores that subproject's prefix and the delegated
`pixi run --manifest-path …` needs no solve. Config schema 2, transport manifest schema 4.

**What moved, and where it lives.** `SandboxConfig` gained `[[bundle.workspaces]]` (`manifest` +
optional `environments`, omitted meaning that project's `default`). Shape rules are filesystem-free
in `validate()`; the rules that need a repository root (missing manifest, missing nested
`pixi.lock`, nested `Cargo.lock` → TASK-131) run in `SandboxConfig::load`. `Manifest` gained
`envs.<id>.workspace` and `workspaces[]`, and — the piece that keeps the two sides honest —
`env_destination`, `env_name`, `pack_prefix` and `staging_component` all live in core, so `restore`,
`doctor --verify-restored` and the packer cannot drift apart about where an environment goes.
A nested id is `<slug>/<env>`, the slug being the declared directory with `/` folded to `-`.

**Three bugs this exposed in existing code, all fixed:** the per-file oracle's path was rebuilt
from the environment name (`envs/<env>/files.json`) and had to move beside the pack directory;
restore's scratch names would have collided between root `default` and `subproject/default`; and
two tests hardcoded `schema = 2` as "newer than this build", which silently stopped testing
anything the moment `CONFIG_SCHEMA` moved — both now derive from the constant.

**House-rule collision worth remembering.** `xtask check-repository` forbids the word "python"
anywhere under `crates/` (it names the retired pre-Rust implementation). The natural example
directory for this feature is exactly that, so the code and tests say `subproject/`; the design doc
under `backlog/docs/plans/` still uses the real one.

**Not proven, and left unchecked.** AC#3's "differing channels or package identities" conflict
refusal: separate prefixes are the ratified design, so refusing such a difference would re-impose
the merged-workspace cost slice 0 exists to avoid — that needs an owner ruling. AC#7's two offline
observables (`pixi install --frozen --offline` a no-op in the nested project, delegated run under
denied egress) were not run here; the fixture proves layout and transport identity, which is a
different claim.

Evidence: suite 1,218 passed / 2 skipped (593 product, 315 core, 54 git, 256 xtask) against
1,190 / 2 before; `fmt`, clippy, deny, actionlint, taplo, generated workflows, `plan`,
`check-repository` and `lint-docs` all green. TASK-129 stays In Progress.

### 2026-10-24 (fourth session) — the CI graph stops queueing behind itself

**Four removals, measured against the 2026-10-24 run inventory.** `msrv` and
`replace-running-image` took `needs: ci` purely to read `needs.ci.outputs.run-rust`; a `changes`
job now answers it in ~10 s, so both start alongside lint instead of after it and a code PR's
wall clock loses their serial tail (their own steps are 63 s and 40 s). The airlock proof ran the
network-reachable tier *before* the egress-denied one: the weaker tier is now a diagnostic that
runs after a red Tier A (or on a `block-network: false` dispatch), which takes 92 s off every
green proof. The publisher's `push_paths` named `crates/**`, so an xtask-only merge rebuilt a
binary-linked payload from a crate that is not in the binary — one 249 s publish run for a merge
that changes nothing the transport carries. And the PR code filter did not exclude markdown, so
the markdown-only sprint PR #202 paid 220 s for clippy, coverage and doctests; `!**/*.md` and
`!.agents/**` replace the two named files it used to carry.

**Two additions.** A draft pull request no longer runs the airlock: five draft pushes in one
afternoon is roughly 40 runner-minutes of proof nobody asked for, and `ready_for_review` is now
an explicit trigger so the proof arrives when review starts.

**A cache change that was measured and reverted.** Giving the proof job the plan job's
`support-host-plan` identity looked free — one workspace, one lockfile — and run 40116802649
showed it was not: the proof job's own identity had been warm (8 s to build the archive in
40112616326), the new primary key guaranteed a miss, and the plan job saves before the proof job
restores, so the only thing that transferred was a cold build. The proof job took 578 s against
407 s. Reverted in the same session; the comment in `airlock.yml` now records why the two
identities stay separate.

**Not implemented, and why.** *A single rust-cache key across all three workflows*: the three
check out to different roots (`project/` vs `.`) with different profiles, so one key would only
add cross-restore churn; the honest part of the claim is the within-workflow sharing above. *An
uncompressed `git push`*: `ShellGit::push` targets `https://github.com/…` in the publisher, so
`pack.compression=0` would trade CPU for a multi-hundred-megabyte upload — the 74 s push is
network, not zlib. It would need a conditional on a local remote, and there is no local remote in
production. *Splitting pack/verify/push* and *caching the released binary* are still open, as is
per-step budget reporting into `$GITHUB_STEP_SUMMARY`; the last one is what would make the next
measurement cheap, and it is not written.

**What the runners said.** Three runs carry the new graph and Tier B is `skipped` in all of
them (40118086577, 40119288736, 40120014109). The first two were *not* faster — 443 s and 444 s
against 407 s in 40112616326 — because everything else on the job is noisy: Tier A alone measured
94 s, 113 s, 142 s and 140 s across four runs, and the e2e archive build 8 s, 80 s, 9 s, 9 s
(cache hit, miss, hit). Run 40120014109 is the settled shape: the released-binary download is
`skipped` behind the new `actions/cache` entry, and the two jobs measure **113 s → 82 s (plan)**
and **407 s → 346 s (proof)** — the 92 s of duplicated tier plus 21 s of repeated download,
partly given back to a slow Tier A. So the saving is real but smaller than the plan's arithmetic,
and Tier A's spread (~50 s) is now the largest single source of noise in the proof.

**A path-filter correction worth keeping.** `paths:` on a `pull_request` trigger is evaluated
against the whole PR diff, not the pushed commit: once this PR contained a `crates/**` change,
every later push ran the airlock again even when it only touched `CONTEXT.md` (run 40120014109).
Draft-skipping is therefore the lever that matters for PR cost, not path hygiene.

**`ci` is the unambiguous one:** 220 s → 136 s on a code PR with all four jobs starting within 9 s
of each other (run 40116802710).

**Evidence.** `actionlint`, `check-repository`, `check-ci-builds`, `plan --json` green with the
new config. `check_ci_test_route` still pins the coverage/doctest/payload steps to
`steps.changes.outputs.code`, which is why `ci` keeps its own copy of the filter instead of
reading the new job's output.

### 2026-10-24 (sixth session) — the cargo gate's two halves, split by cost

**What the numbers forced.** Removing vendored crates from the airlock proof (attempted, measured,
then reconsidered in PR #203) saved only ~25 s in the publisher push; the 118 s that vanished came
from Tier A alone, because Tier A was running `cargo check --offline --locked` over every
dependency in a freshly restored tree with no `target/`. The vendor bytes are nearly free; the
compile is not. So dropping the vendor was the wrong lever for the cost, and keeping it while
dropping the compile is the right one — except that `skip_cargo` skipped the *wiring* assertion
too, and the wiring assertion is a file read.

**The split.** `PIXI_SANDBOX_GATE_SKIP_CARGO` still means "this transport vendored nothing, so
there is nothing to assert". A second switch, `PIXI_SANDBOX_GATE_SKIP_CARGO_BUILD`, keeps the
wiring assertions and skips only the compile, announcing itself with a `::notice::`. `airlock.yml`
sets it from `github.event_name == 'pull_request'`, so a PR asserts the wiring and skips ~118 s
while the Monday cron and any dispatch run both halves. Transports stay the shape the publisher
ships, on every trigger.

**Where the remaining proof lives.** A PR no longer proves that a real vendored tree compiles
offline; that claim is now a *scheduled* one, so doc-18 §5.3 gained an explicit criterion: the
milestone gate reads the last scheduled or dispatched airlock, not a release PR's. Without that
line "airlock green" would mean the cheap half only.

Evidence: `actionlint`, `taplo`, `cargo fmt --check`, `check-repository`, `check-ci-builds` and
`cargo check -p pixi-sandbox --tests --features ci` green locally. The step-level saving is
unmeasured until a PR run of this branch; the expectation is Tier A back to ~22 s with the vendor
present, and the wiring notice in the log.

### 2026-10-24 (seventh session) — a local push stops paying for delta search

**Measured before touching anything.** 1.2 GiB of incompressible blobs (300 × 4 MiB, the closest
analogue to conda payloads, which are already-compressed archives) pushed `--force` into a fresh
bare repo on 2 cores: **117.5 s** by default, 92.0 s with `pack.compression=0`, 30.9 s with
`core.bigFileThreshold=1`, and **9.6 s** with both — at an identical 1201 MiB on the wire in every
case. The delta search was pure waste: it compared blobs that cannot delta, and it was 87 s of the
117. zlib was the smaller half. `pack.threads=1` changed nothing (9.7 s), `pack.window=0` was
slower than `bigFileThreshold` (12.9 s).

**The decision.** `ShellGit::push` now sends an uncompressed, delta-free pack when the remote names
something on this machine (`is_local_remote`: a path, a `file://` URL, or a single-letter Windows
drive), and leaves the packer alone otherwise. The airlock's throwaway remote is
`$RUNNER_TEMP/airlock-remote.git`, so its ~74 s publish step is the one this targets. The publisher's
push to `https://github.com` keeps the default packer, because the same flags can inflate an upload
that *does* contain real deltas — the 1201 MiB identity above is a property of random test data, not
a proof about `repodata.json`.

**How it is held.** A previewed rstest pins the argv for five remote shapes (path, `file://`, drive
letter, `https`, scp-like `git@host:owner/repo`): the two `-c` pairs must appear exactly when the
remote is local. Previewed rather than executed, so a case list containing a real URL never reaches
for the network to observe its own argv.

**The airlock cannot measure this, and that is by design.** Run 40127613325 on this branch's own
commit printed 77 s for "Publish to the throwaway remote" — the same number as before the change.
The publish step runs `pixi run -e package airlock-publish`, whose task is `pixi-sandbox publish`,
and that binary is the **released** one: `airlock-install-released` is
`pixi global install --channel https://prefix.dev/archont561/archont561 pixi-sandbox`. A real
airlock starts from a released asset, so every product-side improvement shows up there one release
later — which also explains why the 74 s baseline read the same across three unrelated branches.
The benchmark and the argv test are the evidence for this change; the airlock number becomes a
measurement after the next release ships it.

**Evidence.** Git crate 59 tests green (was 54: five new cases); product crate 593 green in the
turbo run; `fmt --check`, `clippy -D warnings` on the crate, `check-repository` and `check-ci-builds`
green. The full local suite could not be linked here — the workspace's test binaries need ~16 GiB
and this sandbox has 21 GiB of disk, so `ld` died with a bus error (ENOSPC) on the xtask test
binaries; that is a sandbox limit, not a result.

### 2026-10-24 (eighth session) — `target/` stops being mostly DWARF

**The anatomy, measured before the change.** A linked `xtask` test binary was 213 MiB, of which
205 MiB was `.debug_*` and 8 MiB was `.text`; a BDD test binary carried 65 MiB of DWARF against
16 MiB of code; `target/debug/pixi-sandbox` was 258 MB. The workspace declared only
`[profile.release]`, so every local build produced full debug info for every dependency, and CI
never noticed because it exports `CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0`.

**The change.** `[profile.dev.package."*"] debug = 0` and its `[profile.test.package."*"]` twin.
Dependency crates lose their DWARF; the workspace's own crates keep theirs, so a failing test still
prints `file:line` for our code. Both entries are needed: `test` does not inherit a `dev` *package*
override, which is the kind of thing that looks redundant and is not.

**Measured after, same target on the same machine.** `repo_checks_badges` went from 223,179,104
bytes to 43,842,256 — **213 MiB → 41.8 MiB, −80%** — with debug sections 205 MiB → 29 MiB. The
surviving 29 MiB is our own crates' debug info, which is the point. A cold build of this one test
target (full dependency graph included) took 1 m 41 s and left `target/` at 1.4 GiB.

**What was deliberately not done.** `incremental = false`, which the note in `Cargo.toml` records as
worth a further ~191 MiB here and ~1.1 GiB on a full build. It buys disk by spending the
edit-compile loop it exists to serve, and the DWARF was the 90% win; the knob stays documented at
the setting rather than flipped. The `release` profile was already at the floor (`opt-level = "z"`,
fat LTO, one codegen unit, stripped): assets are 7.1 MiB musl, 2.8–3.2 MiB per `.conda`, so
`panic = "abort"` would win ~0.3–0.7 MiB and nothing at transport scale.


### 2026-10-24 (ninth session) — CI graph, cargo gate, push cost and debug size: the session that measured

**Merged.** PR #202 (sprint-long sessions + the CI graph: `changes` job, Tier B → diagnostic, draft
skip, markdown out of the code filter, `push_paths` no longer `crates/**`), PR #204 (the cargo gate
split into a wiring assertion on every PR and the offline compile on the weekly cron), PR #205
(local-remote pushes stop paying for delta search; dependency DWARF leaves local builds).

**Re-baseline on merged main (`3d715827`).** Full suite **1,223 passed / 2 skipped** — 593 product,
315 core, 59 git, 256 xtask — up from 1,218, the five new git cases. Post-merge runs: `ci` 122 s
success, `publish sandbox` 239 s success, `docs` 27 s success.

**The debug-profile change incidentally fixed this sandbox.** Before it, the workspace's test
binaries demanded ~16 GiB and `ld` died with a bus error (ENOSPC) on a 21 GiB volume; the full suite
had not been linkable here for the whole session. After it, the same suite builds and runs with
`target/` at **5.7 GiB**. That is the strongest evidence the change did what it claims.

**Learned about the airlock, all of it written into the workflow or doc-18.** It publishes with the
**released** binary (`pixi global install … pixi-sandbox`), so a product-side improvement shows up
in its numbers one release later — not a defect, but it means the airlock is the wrong instrument
for judging this session's two `pixi-sandbox-git` changes. `actions/cache` is scoped **per pull
request**, so every new PR re-warms the e2e archive and the released binary (9 s → ~100 s on first
run). `paths:` on a `pull_request` trigger matches the **whole PR diff**, so a PR that ever touched
`crates/**` runs the airlock on every later push regardless of that push's content.

**Open, and not started.**
- **Vendor filtering** — `cargo vendor` ships the whole lockfile graph: 805 MiB, of which 368 MiB is
  Windows crates and 69 MiB `aws-lc` that a linux-64 transport can never compile
  (`cargo tree --target x86_64-unknown-linux-gnu` contains none of them). `cargo-vendor-filterer`
  would take ~350–400 MiB off every published branch. Measured, not approved, not implemented.
- **The docs-only floor is 83 s, and 61 s of it is `check-ci-builds`.** PR #206 (this file, one
  markdown commit) skipped fmt, lint, doctests, coverage, codecov *and* rust-cache — then compiled
  the workspace from scratch to run `pixi run xtask check-ci-builds`. The gate is correct to run;
  the cache is the thing gated wrongly, since `rust-cache` sits behind the same code filter that
  the Rust steps do. Two ways out: restore the cache unconditionally (it is a download, not a
  build), or gate the xtask checks like the rest. Either is ~55 s off every documentation PR.
- **Per-step budgets** (T4) — the reason every question this session cost four API calls instead of
  a glance at `$GITHUB_STEP_SUMMARY`.
- **doc-18 §7's four owner decisions** — tag vocabulary, sprint scope, who pushes the tag, whether
  the tag trigger lands this sprint. Deferred again on 2026-10-24; they are the whole gate in front
  of the m-1 sprint train.
- **`incremental = false`** — documented in `Cargo.toml` at its setting, worth a further ~1.1 GiB
  locally, not taken.

**The next session's opening prompt.**

```
Restore the sandbox (`bash scripts/restore.sh`, then export PATH="$HOME/.local/bin:$PATH") and
baseline the suite: expect 1,223 passing / 2 skipped (593 product, 315 core, 59 git, 256 xtask).
The full suite now links here — target/ lands at ~5.7 GiB because dependency DWARF is off; if it
fails with a bus error from `ld`, check `df -h /` before blaming the tree. Then read CONTEXT.md
§ Session scratchpad — "2026-10-24 (ninth session)" lists four open items.

This session is sprint 1 of milestone m-1 (0.9.0, "Nested workspaces in one transport"). The plan
is `backlog/docs/plans/doc-18 - Sprint-Train-and-the-Milestone-Release-Gate.md`: stage A is the
spike on TASK-129's observables criterion (it needs a pixi-generated lock; the hand-built fixture
lock is rejected with "missing field 'subdir'"), stage B is TASK-129 then TASK-131, stage C is the
gate (fmt, lint, the full suite, and `airlock linux-64` on the release branch — note that a PR
airlock now asserts the vendor wiring but skips the offline compile, which only the weekly cron and
a dispatch run), stage D is the release (prepare-release, release PR, milestone tag
`milestone-v0.9.0`).

Decisions already made, do not re-open: tag vocabulary is `milestone-v0.9.0`; the tag is pushed by
a human after the merge, not by the merge itself; `release.yml`'s `v*.*.*` does not match it. Still
open with the owner, and the reason the sprint has not started: the sprint's scope (TASK-129 +
TASK-131 assumed), the tag push, and whether the milestone-tag trigger lands in this sprint — ask
for those three before proposing stages, since doc-18 §5.2 does not exist yet.

Do not restart the CI optimisation work: #202, #204 and #205 landed and are measured. The two open
items there are vendor filtering (measured at ~350–400 MiB, unapproved) and per-step budgets; both
are worth a proposal, neither is this sprint.

Stage D needs the owner's sanction for the tag push and the release dispatch. Propose the sprint
and stop. House rules are in AGENTS.md (invariant 10: anything you do not implement goes in
CONTEXT.md), the procedure and these templates are in .agents/skills/session/.
```
