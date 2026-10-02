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
  - Sequentially runs `lint` (fmt, clippy, deny, actionlint, taplo, biome), `test` (nextest), `test-doc`, `coverage` (llvm-cov), and `docs-build`.
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
> GitHub Release, or run a native macOS/Windows runner. The following work is therefore explicitly
> **assigned to an online agent running in GitHub Codespaces (or CI)** and is tracked here so it is
> not silently attempted offline:

1. **Prove `osx-arm64` and publish a macOS bundle** (backlog **task-1**, gated by **D11**). AC#1
   requires a **native macOS runner** doing an offline restore end-to-end; it cannot be validated
   from a Linux airlock. It does *not* require an online agent to drive it by hand: declaring the
   platform in `.pixi-sandbox.toml` puts it in `plan --json`, and the `pull_request` trigger on
   `.github/workflows/airlock.yml` fans that matrix onto `macos-14` on its own. An airlock agent
   can therefore open the PR and read the verdict; only the *execution* needs the runner.
   (`gh workflow run` is not an alternative from a sandbox: the token there has no `actions:
   write`, and a dispatch returns HTTP 403.)
2. **Verify `pixi-sandbox tools update` live** (backlog **task-4**, code Done). The command is
   implemented and unit-tested offline in `commands/tools/update.rs`; what still needs network is
   the live run — resolving/downloading/sha256-verifying the latest `pixi`, `pixi-pack`,
   `pixi-unpack`, `rattler-index` pins against real GitHub releases. Release *assets* are
   unreachable from the airlock (`objects.githubusercontent.com` and
   `release-assets.githubusercontent.com` both fail to connect, even though `api.github.com`
   answers), so `tools update --check` cannot be exercised here; it belongs in Codespaces/CI.
3. **Repin the published `init.sh` to v0.3.0 binaries** (backlog **task-11**). The v0.3.0 asset was
   cut before the repin landed, so the one-liner installs v0.2.0 binaries; re-uploading an asset
   needs `uploads.github.com`, which the airlock cannot reach. Superseded for future releases:
   `templates/install.sh` is now rendered at the tag by `xtask render-install` rather than
   stamped from a committed copy, so a published one-liner cannot default to a version other
   than its own.

The dev container that provides this online environment is `.devcontainer/devcontainer.json`
(`ghcr.io/prefix-dev/pixi`, with `pixi install --locked --all` + `docs-install` + a global `bun add` of opencode
on create).

---

## 🧭 Current v1 Planning and Network Boundary

The current redesign is planned in Backlog milestone **m-0 — v1 standalone cross-platform workflow**. The canonical planning/specification documents are under `backlog/docs/`; `.knowledge/` now keeps historical decisions, raw evidence, and discovery pointers.

### Backlog workstreams

- **TASK-21** — make `init` provider-neutral and platform-specific; use `pixi-sandbox init` with `--github-workflow-path`, `--script-path`, and `--config`; generate only the current platform launcher.
- **TASK-22** — generate a disposable workflow that installs `pixi-sandbox` from `@archont561/pixi-sandbox` and calls the binary directly, without Archont561 composite Actions.
- **TASK-23** — publish Pixi/Conda package variants for `linux-64`, `linux-aarch64`, `osx-64`, `osx-arm64`, and `win-64`, while retaining standalone release binaries for transport bootstrap.
- **TASK-24** — measure standalone transport/restore orchestration and tool deduplication before changing the existing D2/D3 `pixi-pack`/`pixi-unpack` decisions; schema compatibility is required.
- **TASK-25** — move npm-compatible tooling such as Biome and Astro to `package.json`/`bun.lock`, keep Bun in a separate web environment, and decide the Windows Node/Bun fallback.
- **TASK-26** — evaluate Turbo only after multiple JavaScript workspaces justify dependency-aware orchestration and caching.

The v1 proposal is now an accepted architectural decision: backlog `decision-1` is accepted
(through task-24's measurements, recorded in `decision-2` and `doc-7`), with the boundary that
`pixi-pack`/`pixi-unpack` remain pinned helper subprocesses (D2/D3/D4/D12, reaffirmed as D14).

### Work that requires an online Codespace or native CI

- Resolve/download Pixi, Conda, Bun/npm, Turbo, or release assets when they are not already cached.
- Run GitHub Actions and inspect workflow results.
- Publish package variants to prefix.dev.
- Push sandbox transport branches or create GitHub Releases.
- Run native macOS/Windows package and binary jobs.
- Validate installation from the `@archont561/pixi-sandbox` channel.
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
- **Docs tasks could collapse further** (`docs-dev`/`docs-build` → one `docs <mode>` task). Left
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
`pixi run xtask render-relock` as the only way to change it. Also: the session skill's missing
half (`.agents/skills/session/`) — the opening-prompt and session-report templates, and a
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

**Landed locally this session before task-35 work starts:** restore no longer generates
`.pixi/sandbox-env.sh` and removes a legacy copy if it finds one; `scripts/restore.sh` reports
how to use the registered pixi launcher (or the manifest-owned pixi path when registration is
skipped) instead of sourcing an activation hook; `scripts/airlock-gate.sh` now drives Cargo via
`pixi run --frozen -- cargo …` rather than a bare `cargo` resolved from `PATH`; docs, README,
`AGENTS.md`, and the session skill all state the same rule.

**Standing command rule:** pixi is the sole environment entrypoint. Use
`pixi run --frozen <task>` for repository tasks, `pixi run --frozen xtask <subcommand>` for repo
automation, `pixi run --frozen -- cargo <cmd> -p <crate>` for crate-scoped Rust work, and
`pixi run --frozen bun --filter=<workspace-package> run <script>` (or root `pixi run --frozen
bunx <tool>`) for JavaScript package work. Do not source `.pixi/sandbox-env.sh`; new restores do
not generate it.
