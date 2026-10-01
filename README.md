# pixi-sandbox

<p align="center">
  <a href="https://github.com/Archont561/pixi-sandbox/actions/workflows/ci.yml"><img src="https://github.com/Archont561/pixi-sandbox/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://codecov.io/gh/Archont561/pixi-sandbox"><img src="https://codecov.io/gh/Archont561/pixi-sandbox/branch/main/graph/badge.svg" alt="Coverage"></a>
  <a href="https://github.com/Archont561/pixi-sandbox/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.85%2B-orange.svg?logo=rust" alt="Rust"></a>
  <a href="https://pixi.sh"><img src="https://img.shields.io/badge/Pixi-0.81%2B-yellow.svg?logo=condaforge" alt="Pixi"></a>
  <img src="https://img.shields.io/badge/Platforms-linux--64%20%7C%20osx--arm64-brightgreen.svg" alt="Platforms">
  <a href="https://github.com/Archont561/pixi-sandbox/releases"><img src="https://img.shields.io/github/v/release/Archont561/pixi-sandbox?label=release" alt="Release"></a>
  <a href="https://prefix.dev/channels/@archont561/pixi-sandbox"><img src="https://img.shields.io/badge/prefix.dev-%40archont561%2Fpixi--sandbox-5c4ee5" alt="prefix.dev channel"></a>
  <a href="https://archont561.github.io/pixi-sandbox/"><img src="https://img.shields.io/badge/Docs-Starlight-blueviolet?logo=astro" alt="Docs"></a>
  <a href="https://github.com/Archont561/pixi-sandbox/pulls"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg" alt="PRs Welcome"></a>
</p>

<p align="center">
  <strong>Offline sandboxes for pixi projects — pack, publish to an orphan branch, restore on an airlock with zero network.</strong><br/>
  Pure Rust, static musl binaries, checksum-verified, Git-native dedup.
</p>

---

> [!NOTE]
> Because `pixi` discovers `pixi-<command>` on `$PATH`, installing `pixi-sandbox` unlocks native subcommands: `pixi sandbox pack`, `doctor`, `publish`, `restore`, `plan`.

## 🖥️ Platform Support

The badge above mirrors `[workspace] platforms` in `pixi.toml`. A platform only gets a published
bundle in `.pixi-sandbox.toml` once a native runner has passed the airlock proof (decision D11),
so "builds here" and "restores offline there" are tracked separately —
`pixi run xtask check-repository` keeps the badge, `pixi.toml` and the publish plan in agreement.

| Platform | Status | Notes |
|----------|--------|-------|
| `linux-64` | ✅ proven, published | `sandbox/developer-linux-64` is packed, verified and restored offline in CI |
| `osx-arm64` | ✅ proven, published | Packed, verified and restored on a native `macos-14` runner with **egress denied** (`sandbox-exec`, `network-outbound` refused), then gated against the manifest; `sandbox/developer-osx-arm64` is published by the same plan as linux-64 (task-1) |
| `linux-aarch64`, `osx-64` | 🟡 release binaries only | Static binaries ship with every release; no bundle published |
| `win-64` | ❌ not supported | `default` itself no longer needs `bun` (task-25 isolated it in the `web` environment), but `pixi lock` solves **every** environment for **every** platform in `pixi.toml`, and conda-forge has no `bun` build (`pixi lock`: "No candidates were found for bun *"), so Windows is out of `[workspace] platforms` and D11 still lists it as unproven. Until that changes the JS toolchain is unsupported on Windows: use the Rust CLI, and run the JS tools through your own Node.js/Bun install outside the pixi environment |

## 📦 Sandbox Visualization

```text
  connected build machine           orphan branch (Git)                       airlocked machine
  ───────────────────────           ───────────────────                       ─────────────────

  ┌─────────────────────┐           ┌─────────────────────────────┐           ┌────────────────────────┐
  │ pixi project        │   pack    │ sandbox/developer-linux-64  │   fetch   │ airlock                │
  │                     │ ────────► │                             │ ────────► │                        │
  │ pixi.toml           │           │ .pixi-sandbox/manifest.json │           │ .pixi/envs/<env>/      │
  │ pixi.lock           │           │   envs/<env>/pack/          │           │ .pixi/tools/<platform>/│
  │ Cargo.lock          │           │   tools/<platform>/         │           │ .pixi-sandbox/vendor/  │
  │ .pixi-sandbox.toml  │           │   vendor/                   │           │ offline ✔              │
  └─────────────────────┘           └─────────────────────────────┘           └────────────────────────┘
  pack    pixi sandbox pack --cargo-vendor --fetch-tools --self-bin <static>
  fetch   plain git — whole files content-addressed, 262 MB transport → ~110 MB after dedup
  local   ./restore.sh → verify every byte → source .pixi/sandbox-env.sh → build offline
```

**Generate publishing and one-command airlock restoration:**

Connected hosts need [Pixi](https://pixi.sh) installed. Install from the canonical channel, then initialise the project:

```bash
pixi global install --channel https://prefix.dev/archont561/pixi-sandbox --channel conda-forge pixi-sandbox
pixi-sandbox init
# commit .github/workflows/publish-sandbox.yml, pixi-sandbox.toml, and restore.sh (restore.ps1 on Windows)

# after transferring Git history with the sandbox branch into the airlock:
./restore.sh
```

---

## 🚀 Key Features

| Icon | Feature | Description |
|------|---------|-------------|
| 🔒 | **Zero-Network Restore** | Verified restore in `unshare -rn` without touching conda channels or crates.io |
| 📦 | **Git-Native Transport** | Orphan branch, whole files content-addressed, free dedup across envs/releases |
| ✂️ | **Automatic Sharding** | Splits >95 MiB into `.partNNN` to respect GitHub 100 MiB blob limit |
| 🛡️ | **Strict Verification** | SHA-256 verified against embedded pins before writing; dynamic tools rejected |
| 🦀 | **Cargo Vendoring** | Bundles `cargo vendor --versioned-dirs` alongside conda envs |
| ⚡ | **Pure Rust CLI** | Static musl Linux, native macOS/Windows, no Python; `pixi sandbox` extension |
| 🤖 | **Generated Native CI** | Project-owned workflow calls the installed CLI directly on native runners |
| 📚 | **Docs like astro-icon** | Starlight + astro-icon + Iconify (lucide, mdi, tabler, simple-icons) |

Docs site uses [Astro Icon](https://www.astroicon.dev/) + [Iconify](https://iconify.design/) — 300k+ icons: https://icon-sets.iconify.design/

---

## ⚡ Quick Start

### 1. Connected Host: Pack, Verify & Publish

```bash
# Build static self-binary (or use verified release)
cargo build -p pixi-sandbox --release

# 1. Pack
pixi sandbox pack \
  --repo-root . \
  --envs dev,docs \
  --output-dir .sandbox-transport \
  --platform linux-64 \
  --cargo-vendor \
  --fetch-tools \
  --self-bin target/release/pixi-sandbox

# 2. Verify (writes nothing)
pixi sandbox doctor --branch-location .sandbox-transport --verify

# 3. Publish as orphan branch (<branch_prefix>/<bundle>-<platform>, as `plan` reports it)
pixi sandbox publish \
  --input-dir .sandbox-transport \
  --branch-name sandbox/developer-linux-64
```

### 2. Disconnected / Airlocked Host: Restore

Transfer a clone or Git bundle containing both the project and sandbox branch, then run the
project-side launcher generated by `pixi-sandbox init`:

```bash
./restore.sh              # Linux/macOS
# .\\restore.ps1          # Windows PowerShell

# in a new shell, pixi and pixi sandbox already resolve (registered user tools);
# for the environment's own binaries:
source .pixi/sandbox-env.sh
pixi install --frozen --offline   # must be no-op
cargo build --offline             # uses vendored crates
```

After verifying the branch *and* the restored tree, the restore registers managed launchers
for `pixi` and `pixi-sandbox` in `~/.local/bin` (Windows: `%USERPROFILE%\.pixi-sandbox\bin`)
and puts that directory on the shell's persistent `PATH` — so `pixi` and `pixi sandbox` work
in a new shell without sourcing anything. `PIXI_SANDBOX_USER_TOOLS=skip` (or
`--user-tools skip`) leaves the home untouched for CI, shared accounts, or locked-down
airlocks; `--user-bin <dir>` relocates the launchers.

The launcher reads the selected config (`pixi-sandbox.toml` by default, with
`.pixi-sandbox.toml` retained as a compatibility fallback) and resolves
`<branch_prefix>/<bundle>-<platform>` for the host — the same branch `pixi-sandbox plan` gives
the publisher — so renaming a bundle or
prefix needs no regenerated launcher. It then uses local `git archive`; it never fetches. It
extracts the branch under `.pixi/.restore-transport` and invokes
`.pixi-sandbox/tools/<platform>/pixi-sandbox`.

```bash
PIXI_SANDBOX_BRANCH=sandbox/developer-linux-64 ./restore.sh  # pin an exact branch
PIXI_SANDBOX_BUNDLE=developer ./restore.sh                   # several bundles cover this platform
```

> [!TIP]
> After restore, `.pixi/envs/*` has relocated prefixes, `.cargo/config.toml` wired to vendored sources with relative path, `CARGO_NET_OFFLINE=true`.

---

## 📖 Using in your project

### Declare what to ship (never infer all envs)

```toml
# pixi-sandbox.toml
schema = 1
branch_prefix = "sandbox"
cargo_vendor = true

[runners]
# linux-aarch64 = "self-hosted-arm64"  # required — no hosted default

[[bundle]]
name = "developer"
environments = ["dev", "docs"]
platforms = ["linux-64", "osx-arm64", "win-64"]

[[bundle]]
name = "minimal"
environments = ["default"]
platforms = ["linux-64"]
cargo_vendor = false
```

Validate:

```bash
pixi-sandbox plan --config pixi-sandbox.toml
pixi-sandbox plan --config pixi-sandbox.toml --json  # GitHub Actions matrix
```

### Without cargo vendoring

```toml
schema = 1
cargo_vendor = false
[[bundle]]
name = "python"
environments = ["dev", "test"]
platforms = ["linux-64", "win-64"]
```

```bash
pixi-sandbox pack --repo-root . --envs dev --output-dir .sandbox-out --platform linux-64 --fetch-tools --self-bin target/release/pixi-sandbox
```

Full guide: https://archont561.github.io/pixi-sandbox/guides/using-in-your-project/

---

## 🤖 Generated GitHub Actions publisher

Run `pixi-sandbox init` to generate a reviewed, project-owned `.github/workflows/publish-sandbox.yml`.
The generated workflow installs the native package from the canonical prefix.dev channel and calls
`plan`, `pack`, `doctor`, and `publish` directly on native runners. Commit that workflow with
`pixi-sandbox.toml`; no repository-owned composite Action or reusable workflow is required.

> **Migration from v0.3.x Actions:** new refs no longer contain `Archont561/pixi-sandbox@…`,
> `Archont561/pixi-sandbox/setup@…`, `Archont561/pixi-sandbox/publish@…`, or the reusable
> `.github/workflows/publish-sandbox.yml`. Immutable older tags and commit SHAs retain those files.
> Regenerate and commit the direct CLI workflow with `pixi-sandbox init`.

## ⚙️ Configuration Reference

### `pixi-sandbox.toml` (`.pixi-sandbox.toml` compatibility fallback)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `schema` | int | **required** `1` | Config schema version |
| `branch_prefix` | string | `"sandbox"` | Branch prefix: `<prefix>/<bundle>-<platform>` |
| `cargo_vendor` | bool | `true` | Default vendor for bundles |
| `runners` | table | `{}` | Platform → runner label override |

**Bundle:**

| Field | Required | Description |
|-------|----------|-------------|
| `name` | yes | Bundle name, used in branch |
| `environments` | yes | Explicit pixi envs, never inferred |
| `platforms` | yes | e.g., `linux-64`, `osx-arm64`, `win-64`, `linux-aarch64` |
| `cargo_vendor` | no | Override top-level |

**Runners:**

| Platform | Default | Notes |
|----------|---------|-------|
| `linux-64` | `ubuntu-latest` | Hosted |
| `linux-aarch64` | **none** | Must provide `[runners]` self-hosted |
| `osx-64` | `macos-13` | Hosted Intel |
| `osx-arm64` | `macos-14` | Apple Silicon |
| `win-64` | `windows-latest` | Hosted |

Full reference: https://archont561.github.io/pixi-sandbox/reference/configuration/

### CLI

```bash
pixi-sandbox pack --repo-root . --envs dev,docs --output-dir DIR --platform linux-64 --cargo-vendor --fetch-tools --self-bin <static>
pixi-sandbox doctor --branch-location DIR --verify [--json] [--envs a,b]
pixi-sandbox publish --input-dir DIR --branch-name NAME --remote origin [--keep N] [--dry-run]
pixi-sandbox restore --branch-location DIR --output-path DIR [--envs a,b] [--verify-only] [--force] [--no-vendor] [--work-dir DIR] [--cargo-config auto|write|print|none]
pixi-sandbox unpack --input-dir X --output-dir PREFIX [--env NAME] [--force] [--work-dir DIR]
pixi-sandbox init [--github-workflow-path PATH] [--script-path PATH] [--config PATH] [--force]
pixi-sandbox plan --config pixi-sandbox.toml [--json]
pixi-sandbox tools list [--tools-lock PATH]
```

- `--output-path` has alias `--path-to-main-repo-code` for backward compat
- Work dir default `.pixi/.restore-work` (same FS, TMPDIR redirected) — never small `/tmp`
- `publish --keep N` retains the N most recent snapshots by *rebuilding* the branch (blobless
  shallow fetch of the kept commits, then one force-push). It bounds what the branch serves,
  not what the server stores — dropped snapshots are unreferenced until an admin prunes

Docs: https://archont561.github.io/pixi-sandbox/reference/cli/

---

## 📂 The Transport Layout

```text
sandbox/<bundle>-<platform>/   # orphan branch
├── .pixi-sandbox/
│   ├── manifest.json           # SHA-256 catalog, source commit, tool versions, vendor
│   ├── envs/<env>/pack/        # pixi-pack --directory-only: channel/*.conda + env.yml + pixi-pack.json
│   ├── tools/<platform>/       # pixi, pixi-unpack, pixi-sandbox (static, verified)
│   └── vendor/                 # cargo vendor --versioned-dirs (loose, deduped by git)
├── README.md                   # human restore instructions
└── AGENTS.md                   # agent instructions
```

Only Markdown files live at the branch root. `pixi-sandbox init` generates exactly one minimal
launcher on the normal project branch: `restore.sh` on Unix or `restore.ps1` on Windows. It
resolves its branch from the selected config, archives the local sandbox branch, and invokes its
manifest-owned binary without network access.

**Sizes (small project, 2 envs, 33 crates, linux-64):**

| Part | Payload |
|------|---------|
| conda envs (.conda) | 133 MB |
| cargo vendor (loose) | 33 MB |
| tools | 96 MB |
| **transport dir** | **262 MB** |
| **orphan branch after git dedup** | **~110 MB** |

Tools dominate small bundles — expected, git stores each tool blob once.

---

## 🏗️ Repository Architecture

| Path | Description |
|------|-------------|
| `crates/pixi-sandbox-core` | Core lib: manifest format, sharding, tool pins, verification |
| `crates/pixi-sandbox-git` | Trait-based Git: `ShellGit` + `FakeGit`/`RecordingRunner` |
| `crates/pixi-sandbox` | CLI binary + fixtures |
| `crates/pixi-sandbox/tests/fixtures/` | Synthetic transport (15 KB, split blob) — tests never point at repo root |
| `crates/xtask` | Typed repository automation behind one `pixi run xtask <subcommand>` task (`check-repository`, `prepare-release`, release artifact gates) |
| `.github/workflows/ci.yml` | CI: lint + test + coverage + docs-build |
| `.github/workflows/release.yml` | Release: 5 tier-1 static binaries + prefix.dev Conda package + GitHub Release |
| `.github/workflows/docs.yml` | Docs → GitHub Pages |
| `.github/dependabot.yml` | Dependabot: cargo, gha, npm (convco prefixes) |
| `scripts/restore.sh` | One-liner offline reconstruction with PATH aliases; branch derived from `.pixi-sandbox.toml`; selects the user-tool registration policy explicitly |
| `.pixi-sandbox.toml` | Reviewed publish plan: bundles, platforms, branch prefix, runner overrides |
| `.devcontainer/devcontainer.json` | Dev container: official pixi image, `git`/`gh` as pixi globals, opencode installed by `.devcontainer/setup.sh` with a global `bun add` |
| `lefthook.yml` | Git hooks — every hook calls a pixi task so hooks and CI cannot drift; `pre-commit` stays formatter-only (no cargo), every Rust gate runs once at `pre-push` |
| `docs/` | Starlight + astro-icon + Iconify docs (12 pages) |
| `package.json` + `bun.lock` | Root bun workspace: `docs` member + repo-wide `backlog.md` / `skills` devDependencies |
| `.knowledge/` | Open Knowledge Format: decisions D1–D11, design, benchmarks |
| `CHANGELOG.md` | Changelog via convco |

---

## 🛠️ Development & Quality Gates

```bash
# Lint (rustfmt, clippy -D warnings, cargo-deny, actionlint, taplo, biome)
pixi run lint

# Test (nextest, fixtures, no network)
pixi run test
pixi run test-doc

# Coverage
pixi run coverage

# Docs (Astro + Starlight + astro-icon)
pixi run docs-dev
pixi run docs-build

# Agent CLI + repo tooling
pixi run backlog           # markdown backlog, from the root bun workspace
pixi run skills            # agent skills CLI, same workspace

# Transport pipeline (pure Rust self-bin)
pixi run sandbox-plan
pixi run sandbox-pack      # pack + vendor + fetch-tools + self-bin target/release/pixi-sandbox
pixi run sandbox-doctor
pixi run sandbox-publish
pixi run sandbox-restore   # one-liner from orphan branch
pixi run test              # fixture-backed doctor → publish → offline restore lifecycle

# Generated artifacts
pixi run xtask render-relock          # rewrite the committed render of .github/workflows/relock.yml

# Changelog via convco
pixi run xtask prepare-release auto   # stamps CHANGELOG.md and every version reference
pixi run xtask commit-release v0.3.8  # commits, tags and pushes a prepared release (--dry-run shows the diff)
pixi run xtask airlock-matrix          # the airlock proof matrix, validated and GitHub-shaped
pixi run xtask stage-release-binary   # strip + stage the release asset (host triple by default)
pixi run xtask release-checksums      # SHA256SUMS over the standalone binaries, verified complete
pixi run build-release-binary         # cargo build -p pixi-sandbox --release (host; --target in CI)

# CI variants (env vars: SANDBOX_PROJECT, ENVS, TRANSPORT, PLATFORM, BRANCH, REMOTE, SELF_BIN)
pixi run sandbox-pack "$PROJECT" "$ENVS" "$TRANSPORT" "$PLATFORM" && pixi run sandbox-doctor "$TRANSPORT"
```

### Adding a dependency from an airlocked machine

Editing `pixi.toml` or a `Cargo.toml` is a text edit any disconnected host can make; the solve
behind it is not, because prefix.dev and the crates.io index are exactly what an airlock cannot
reach (and `pixi add` solves before it writes). So the solve belongs to the connected side:

1. **Edit the manifest and push** a pull request with the lock left stale.
2. **The guard reports.** `relock.yml`'s first job runs `pixi lock --check` before any
   environment is installed, so a stale lock fails with a message about the manifest rather
   than `setup-pixi`'s message about installation.
3. **The bot relocks.** When the guard fails, the second job refreshes `pixi.lock` and
   `Cargo.lock` onto your branch as `pixi-sandbox[bot]` with a `chore(lock):` commit, then
   dispatches `ci.yml` explicitly — a `GITHUB_TOKEN` push triggers no workflow, so that
   dispatch is the only verdict the lock commit gets.
4. **Merge** once CI is green.

> [!IMPORTANT]
> **A merged lockfile does not make a dependency usable in the airlock.** A restored host
> builds against the packed conda environments and `.pixi-sandbox/vendor`, so the new crate or
> package exists for it only once a transport carrying it has been packed and published
> (`pixi run sandbox-pack` → `sandbox-doctor` → `sandbox-publish`). Until then the dependency
> is connected-side only, and an offline `cargo build --offline` will still fail on it.

The workflow is generated, not hand-written: it is the render of
`crates/pixi-sandbox/src/generated/relock_workflow.rs` that `pixi-sandbox init` also writes for
consumers, byte-checked by `pixi run xtask check-repository`. Change the template and run
`pixi run xtask render-relock`; never edit `.github/workflows/relock.yml` directly.

> [!IMPORTANT]
> Integration tests run against synthetic fixtures in `crates/pixi-sandbox/tests/fixtures/`, **never** against this repo itself, ensuring hermetic offline isolation. Two tests enforce it: the fixture must not depend on pixi-pack, and no test may walk out via `..`/`.parent()` — with two reviewed exceptions, `scripts/airlock-gate.sh` and `scripts/restore.sh`, which are artifacts under test rather than fixture data (`tests/restore_script.rs` runs the real bootstrap against a throwaway git repo and the transport fixture).

### Dev container

`.devcontainer/devcontainer.json` is four keys and no Dockerfile — it runs the official
`ghcr.io/prefix-dev/pixi` image as-is. That image is Ubuntu plus the pixi binary: no `git`,
and no C compiler, so the post-create step delegates to a commented script,
`.devcontainer/setup.sh`, which provisions the host tools, materialises the
project environment, installs the bun workspace, and installs the agent CLI:

```jsonc
"postCreateCommand": "bash .devcontainer/setup.sh"
```

`pixi global` installs into `/root/.pixi/bin`, which the official image already has on `PATH`,
so `git`/`gh` are available to Source Control, the lefthook hooks, `scripts/restore.sh` and
the sandbox tasks.

The compiler is the non-obvious one. `ring` (a transitive dependency of the workspace) shells
out to `cc` and `ar` from its build script, so without a C toolchain `pixi run lint` and
`pixi run test` cannot even compile — and CI never sees this, because a hosted runner has a
system `cc`. It is a *global* pixi install on purpose: a `c-compiler` dependency in a feature
would put a ~200 MB toolchain into the packed sandbox branch, to build nothing an airlock
restores. `ar` needs the mapping (`ar=x86_64-conda-linux-gnu-ar`) because conda-forge ships
no unprefixed `ar`; `pixi global list` is the check that `cc`, `gcc` and `ar` are exposed.

The last two steps go through `pixi run` rather than a bare `bun install`: `bun` lives in the
materialised environment, which is on `PATH` inside a pixi task and nowhere else.

opencode runs last and is **not** a pixi task: `.devcontainer/setup.sh` installs it with a
global `bun add opencode-ai@latest` (bun from the materialised `web` environment), links the
binary into `/usr/local/bin` — bun's own global bin dir is on nobody's `PATH` — refreshes the
model catalogue (`~/.cache/opencode/models.json`, which a binary upgrade does not invalidate),
and prints the command a Codespace user starts with:

```bash
opencode -m opencode/big-pickle
```

It lives in the container setup rather than `pixi.toml` because `pixi.toml` describes what the
repository is built, tested and released with, and a 185 MB agent binary is none of those.

### Bun workspace

`package.json` at the root makes this one bun workspace with `docs` as a member, so
`pixi run docs-install` runs at the root and hoists into the root `node_modules`; `docs/` has
no lockfile of its own any more. `bunfig.toml` pins the install to bun's `hoisted` linker
because the workspace default (`isolated`) hides transitive platform packages that Astro
prerenders by name — see that file for the failure it prevents. The root also carries the
repo-wide dev tooling — `backlog.md` and `skills` — reached through one generic `bunx` task:
`pixi run bunx backlog` / `pixi run bunx skills` (and `pixi run bun <args>` for bun itself, in
the `web` environment). The task runs `bun x --bun`: `bun x` because the conda `bun` package
ships no `bunx` shim, `--bun` because the bins carry `node` shebangs and no environment here
carries `node` — adding one would put ~50 MB of developer tooling into the published sandbox
branch. It resolves the workspace install before the registry and depends on `docs-install`,
so the committed lockfile still decides what runs.

---

## 🔖 Changelog & Release

- **Changelog**: `CHANGELOG.md` generated from conventional commits by `pixi run xtask prepare-release`, which stamps it together with every version reference (a bare preview is `pixi run -- convco changelog`).
- **Release**: Tag `v*.*.*` → `release.yml` builds 5 static binaries, builds `pixi-sandbox` as a Conda package, publishes it to [`archont561/pixi-sandbox`](https://prefix.dev/channels/@archont561/pixi-sandbox) with GitHub OIDC, then creates the GitHub Release. Configure prefix.dev Repository Access for this repository's `release.yml` workflow; no long-lived token is stored in GitHub.
  ```bash
  gh workflow run auto-release.yml -f version=vX.Y.Z
  ```
- **Dependabot**: `.github/dependabot.yml` for cargo, gha, npm (docs) — weekly, groups patch/minor, prefixes `chore`/`ci` to pass convco.
- **Pixi deps**: Manual via `pixi update`, `pixi lock --check`.

---

## 📜 License

MIT — see [LICENSE](LICENSE). Third-party packages and vendored crates retain upstream licenses.

---

## 🌟 Docs

Full docs: **https://archont561.github.io/pixi-sandbox/**

- [Installation](https://archont561.github.io/pixi-sandbox/installation/)
- [Quickstart](https://archont561.github.io/pixi-sandbox/quickstart/)
- [Using in your project](https://archont561.github.io/pixi-sandbox/guides/using-in-your-project/)
- [CI Publishing](https://archont561.github.io/pixi-sandbox/guides/ci-publishing/)
- [Airlock Restore](https://archont561.github.io/pixi-sandbox/restore/)
- [CI publishing](https://archont561.github.io/pixi-sandbox/guides/ci-publishing/)
- [Configuration](https://archont561.github.io/pixi-sandbox/reference/configuration/)
- [CLI](https://archont561.github.io/pixi-sandbox/reference/cli/)
- [Actions API](https://archont561.github.io/pixi-sandbox/reference/actions/)
