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
- **Publish Workflow (`.github/workflows/publish-sandbox.yml`)**:
  - Automatically triggered via `workflow_run` once `ci` completes with `success`.
  - Also callable manually (`workflow_dispatch`) or as a reusable workflow (`workflow_call`).
  - Packs, verifies with `doctor`, and force-pushes the orphan branch.

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
source .pixi/sandbox-env.sh
pixi install --frozen --offline
cargo build --offline
```

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
- **`deny-egress` as an xtask** (sketched in task-36): one subcommand that re-execs under
  `unshare -n` / `sandbox-exec` would let the airlock workflow's Tier A be a one-line step and
  would make the egress-denied gate reproducible on a developer's Linux box.
- **Docs tasks could collapse further** (`docs-dev`/`docs-build` → one `docs <mode>` task). Left
  alone deliberately: both names appear in CI, docs and the README, and the churn buys one line.

### 2026-10-01 — proposal: a relock bot, so a dependency can be declared offline

**Problem.** Adding a dependency means editing `pixi.toml` (or a `Cargo.toml`) *and* re-solving
the lockfile, and solving needs the network: prefix.dev and the crates.io index are exactly
what an airlocked host cannot reach. `pixi add` solves before it writes, so it is not an option
here either. The manifest edit is the part a human wants to make; the solve is machinery.

**Shape.** Hand-edit the manifest, commit it with a stale lock, and let a workflow produce the
lockfile. Two distinct commands, not one:

| intent | command | churn |
|:---|:---|:---|
| make the lock satisfy a manifest I just edited | `pixi lock` | minimal — only what the new constraint forces |
| move everything to the newest allowed versions | `pixi update` | every dependency that can move |
| the Rust equivalent of the first | `cargo fetch` | resolves the new entry, leaves the rest pinned |
| the Rust equivalent of the second | `cargo update` | bumps the workspace |

`pixi lock --check` exits non-zero when the lock does not satisfy the manifest — that is the
guard CI should carry, because `setup-pixi` defaults to `locked: true` and otherwise fails with
a message about installation rather than about the manifest.

**Proposed `.github/workflows/relock.yml`** (one line per step, SHA-pinned actions, the house
rule). `pixi lock` runs as a bare one-liner, *not* `pixi run <task>`: a pixi task would first
have to solve the very environment whose lock is stale.

```yaml
name: relock
on:
  pull_request:
    paths: [pixi.toml, Cargo.toml, "crates/**/Cargo.toml"]
  workflow_dispatch:
  schedule:
    - cron: "17 5 1 * *"        # monthly refresh, the `pixi update` flavour
concurrency:
  group: relock-${{ github.head_ref || github.ref }}
  cancel-in-progress: true
permissions:
  contents: write
jobs:
  relock:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@<sha>        # v7.0.1 — ref: ${{ github.head_ref }}
      - uses: prefix-dev/setup-pixi@<sha>   # v0.10.2 — with: run-install: false
      - run: pixi lock                      # or `pixi update` on the schedule
      - run: cargo fetch
      - uses: stefanzweifel/git-auto-commit-action@<sha>   # commits pixi.lock + Cargo.lock
      - run: gh workflow run ci.yml --ref ${{ github.head_ref }}
```

**The two traps.**

1. A commit pushed with `GITHUB_TOKEN` does not trigger another workflow — deliberate loop
   prevention. So the bot's lock commit leaves CI unrun unless the last step dispatches it
   explicitly (`gh workflow run`, which *is* allowed with `GITHUB_TOKEN` because
   `workflow_dispatch` is an explicit API call), or the push uses a GitHub App token / PAT.
2. **A new dependency is not usable in the airlock until a transport carries it.** Merging the
   lock only fixes the connected side; a restored host builds against `.pixi-sandbox/vendor`
   and the packed conda envs. The full lane is: edit manifest → bot relocks → merge → pack and
   publish a transport → restore. Any task that adds a dependency (task-38 and `rstest`, for
   one) inherits that ordering.

**Not agreed yet:** whether the bot pushes onto the PR branch (simple, single-maintainer, needs
`contents: write` and breaks for fork PRs) or opens its own PR with
`peter-evans/create-pull-request` (the shape prefix.dev documents, needs "Allow GitHub Actions
to create and approve pull requests" and a non-default token for CI to run on it).

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
