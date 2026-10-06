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
8. **A Restored Tree Is Checked Against the Manifest, Not Its Shape** (D13): a schema-2 manifest carries a per-file oracle (`envs/<env>/files.json`) recorded by the packer from its own verification unpack, and `doctor --verify-restored <PROJECT>` compares the restored prefix against it — content, symlink targets, exec bits, the fingerprint marker — collecting every mismatch and writing nothing. Shape checks (conda-meta present, `pixi install` a no-op) prove self-sufficiency, not integrity; a stub prefix with a fabricated conda-meta record must fail the airlock gate, and it does, by test.

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

1. **Prove `osx-arm64` and publish a macOS bundle** (backlog **task-1**, gated by **D11**) — the
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

The current redesign is planned in Backlog milestone **m-0 — v1 standalone cross-platform workflow**. The canonical planning/specification documents are under `backlog/docs/`; `.knowledge/` now keeps historical decisions, raw evidence, and discovery pointers.

### Backlog workstreams

- **TASK-21** — make `init` provider-neutral and platform-specific; use `pixi-sandbox init` with `--github-workflow-path`, `--script-path`, and `--config`; generate only the current platform launcher.
- **TASK-22** — generate a disposable workflow that installs `pixi-sandbox` from `@archont561/pixi-sandbox` and calls the binary directly, without Archont561 composite Actions.
- **TASK-23** — publish Pixi/Conda package variants for `linux-64`, `linux-aarch64`, `osx-64`, `osx-arm64`, and `win-64`, while retaining standalone release binaries for transport bootstrap.
- **TASK-24** — measure standalone transport/restore orchestration and tool deduplication before changing the existing D2/D3 `pixi-pack`/`pixi-unpack` decisions; schema compatibility is required.
- **TASK-25** — moved npm-compatible tooling such as Biome and Astro to `package.json`/`bun.lock`; task-70 later folded Bun into the one developer environment for cross-language orchestration.
- **TASK-26 / TASK-70** — the JS-only Turbo spike deferred adoption, then Pathway's measured Rust-package model supplied a new use case; D15 now records Turbo as the cross-language graph.

The v1 proposal is now an accepted architectural decision: backlog `decision-1` is accepted
(through task-24's measurements, recorded in `decision-2` and `doc-7`), with the boundary that
`pixi-pack`/`pixi-unpack` remain pinned helper subprocesses (D2/D3/D4/D12, reaffirmed as D14).

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
| [`.knowledge/decisions.md`](.knowledge/decisions.md) | Architectural Decisions D1–D12 with empirical lab measurements |
| [`.knowledge/design.md`](.knowledge/design.md) | In-depth design specification and airlock invariants |
| [`.knowledge/rust-bootstrap.md`](.knowledge/rust-bootstrap.md) | Rust static bootstrap binary strategy and validation checklist |
| [`.knowledge/README.md`](.knowledge/README.md) | Open Knowledge Format index |

---

## 🗒️ Session scratchpad (agent proposals, drafts, open questions)

> [!IMPORTANT]
> **This is the only place an agent records something it did not implement.** Suggestions,
> half-finished designs, alternatives considered, measurements worth keeping, "we should
> probably…" — they go here, newest section last, each under a dated heading. They do **not**
> go into `AGENTS.md`, `README.md`, `.knowledge/`, a spec under `backlog/docs/`, or a comment
> in the code they speculate about: those files state what *is*, and a proposal mixed into them
> reads as a decision nobody made.
>
> Promotion out of here is deliberate: a proposal becomes a backlog task (it will be built), a
> decision in `.knowledge/decisions.md` (it was decided, with evidence), or it is deleted.
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
is deleted, and the D10 fixture exception now names only `scripts/restore.sh`. CI fixes found
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

### 2026-10-02 — issue #71: init's channel append broke consumer locks (task-48, D17, PR #73)

**Open on the session branch** (`arena/01a0fe32-pixi-sandbox`, PR #73, `fix(init): …`, net
−175 lines). Issue #71 measured what task-34's namespace-root choice actually does:
`https://prefix.dev/archont561` serves no repodata, so every consumer with a dependency
fails `pixi lock` right after `init`, and a dependency-free project locks fine (an empty
solve never fetches repodata) — the smoke-test trap that let it ship. Recorded as **D17**
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
implemented: task-47 + decision-4/D16 (consumer version-drift gate and reviewed upgrade
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

The same PR promoted task-47 into the v1 upgrade design and updated proposed decision-4. The
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
> names source `15b82c7`, pixi-sandbox 0.4.3, static). Read `AGENTS.md`, decision-4, and task-47.
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
> manifest should name that commit, pixi-sandbox 0.4.4, static). Read `AGENTS.md`, decision-4,
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
merged (PR #73, D17) and shipped in **v0.4.4** (confirmed via `gh release view`, published
2026-10-02T22:56:26Z, ~40 minutes after the PR merged) and still present in the current
**v0.5.0**; issue #71 is closed. Re-ran the init test suite to reconfirm nothing regressed
(`init_leaves_the_consumer_pixi_manifest_untouched` and
`init_still_refuses_a_directory_without_a_pixi_manifest` both pass) and re-read the guide
section and D17 — both still match the shipped code. No code or doc changes were needed.

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
workflow after every `init --force`. Decision D18 records the one judgement call: the table's
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

**D18 is real and was taken deliberately, not inherited.** Adding `[workflow]` at all flips
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
- **`on.push.paths` (D18) — proven, positive direction only.** The release commit touches
  `Cargo.toml`, `Cargo.lock`, `pixi.toml`, `crates/pixi-sandbox/Cargo.toml`,
  `crates/pixi-sandbox/pixi.toml` and `crates/xtask/Cargo.toml` — six matches against the eight
  entries — and the publisher fired on the main push. The D18 trap did not spring and
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
