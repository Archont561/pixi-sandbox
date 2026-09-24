# pixi-sandbox

<p align="center">
  <a href="https://github.com/Archont561/pixi-sandbox/actions/workflows/ci.yml"><img src="https://github.com/Archont561/pixi-sandbox/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://codecov.io/gh/Archont561/pixi-sandbox"><img src="https://codecov.io/gh/Archont561/pixi-sandbox/branch/main/graph/badge.svg" alt="Coverage"></a>
  <a href="https://github.com/Archont561/pixi-sandbox/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.85%2B-orange.svg?logo=rust" alt="Rust"></a>
  <a href="https://pixi.sh"><img src="https://img.shields.io/badge/Pixi-0.81%2B-yellow.svg?logo=condaforge" alt="Pixi"></a>
  <img src="https://img.shields.io/badge/Platforms-linux--64%20%7C%20osx--arm64%20%7C%20win--64-brightgreen.svg" alt="Platforms">
  <a href="https://github.com/Archont561/pixi-sandbox/releases"><img src="https://img.shields.io/github/v/release/Archont561/pixi-sandbox?label=release" alt="Release"></a>
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

## 📦 Sandbox Visualization

```text
  connected build machine               orphan branch (Git)              airlocked machine
  ───────────────────────               ─────────────────                ──────────────────

  ┌─────────────────┐      pack        ┌──────────────────┐     fetch     ┌─────────────────┐
  │  pixi project   │  ───────────►    │ sandbox/linux-64 │  ──────────►  │    airlock      │
  │                 │                 │                  │               │                 │
  │  pixi.toml      │                 │  manifest.json   │               │  .pixi/envs/*  │
  │  pixi.lock      │                 │  envs/*/pack/    │               │  vendor/        │
  │  Cargo.lock     │                 │  tools/*/        │               │  .pixi/tools/   │
  │  [dev, docs]    │                 │  vendor/         │               │  offline ✔      │
  └─────────────────┘                 └──────────────────┘               └─────────────────┘
         │                                      │                                │
         │ pixi sandbox pack                    │ git content-addressed          │ pixi install --frozen --offline
         │ --cargo-vendor --fetch-tools         │ 262 MB → 110 MB dedup          │ cargo build --offline
         │ --self-bin <static>                  │ verify before write            │ source .pixi/sandbox-env.sh
```

**One-liner offline reconstruction (PATH aliases wired like `setup-pixi`):**

```bash
bash scripts/restore.sh sandbox/linux-64   # or: pixi run sandbox-restore
# does: fetch + worktree add /tmp/sb + root ./restore.sh + doctor --verify + restore + source .pixi/sandbox-env.sh
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
| 🤖 | **Short Action Refs** | `owner/repo@vX`, `owner/repo/setup@vX`, `owner/repo/publish@vX` in same repo |
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

# 3. Publish as orphan branch
pixi sandbox publish \
  --input-dir .sandbox-transport \
  --branch-name sandbox/linux-64
```

### 2. Disconnected / Airlocked Host: Restore

```bash
# Fetch
git fetch origin sandbox/linux-64:sandbox/linux-64
git worktree add /tmp/sb origin/sandbox/linux-64 --force

# Restore with the executable at the branch root
/tmp/sb/pixi-sandbox restore \
  --branch-location /tmp/sb \
  --output-path . --force
# or use the generated thin wrapper:
/tmp/sb/restore.sh .

# A bare root binary is read-only: it runs doctor --verify and prints the restore command.
/tmp/sb/pixi-sandbox

git worktree remove /tmp/sb --force
source .pixi/sandbox-env.sh
pixi install --frozen --offline   # must be no-op
cargo build --offline             # uses vendored crates
```

> [!TIP]
> After restore, `.pixi/envs/*` has relocated prefixes, `.cargo/config.toml` wired to vendored sources with relative path, `CARGO_NET_OFFLINE=true`.

---

## 📖 Using in your project

### Declare what to ship (never infer all envs)

```toml
# .pixi-sandbox.toml
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
pixi-sandbox plan --config .pixi-sandbox.toml
pixi-sandbox plan --config .pixi-sandbox.toml --json  # GitHub Actions matrix
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

## 🤖 GitHub Actions (like `setup-pixi`)

Both actions live in **same repo** with short references.

### Setup (download verified release binary)

```yaml
# Root = setup (like prefix-dev/setup-pixi)
- uses: Archont561/pixi-sandbox@v0.2.0
  with:
    version: v0.2.0
- run: pixi-sandbox --version

# Explicit short path
- uses: Archont561/pixi-sandbox/setup@v0.2.0
  id: setup
  with:
    version: v0.2.0
```

Pinned SHA (supply-chain secure):

```yaml
- uses: Archont561/pixi-sandbox@<sha>
- uses: Archont561/pixi-sandbox/setup@<sha>
- uses: Archont561/pixi-sandbox/.github/actions/setup-pixi-sandbox@<sha>
```

### Publish (pack + doctor + publish one native bundle)

```yaml
- uses: Archont561/pixi-sandbox/publish@v0.2.0
  with:
    project: .
    environments: dev,docs
    platform: linux-64
    branch: sandbox/dev-linux-64
    self-bin: ${{ steps.setup.outputs.path }}
    remote: https://github.com/OWNER/REPO.git
    output-dir: /tmp/transport
    cargo-vendor: "true"
    push-token: ${{ secrets.GITHUB_TOKEN }}
```

### Full workflow — just two actions (recommended)

No `release-repository`/`release-version` inputs. Your `uses: @v0.2.0` pin *is* the version.

```yaml
# .github/workflows/publish-sandbox.yml in your project
name: publish sandbox
on:
  push: { branches: [main] }
  workflow_dispatch:

jobs:
  plan:
    runs-on: ubuntu-latest
    outputs: { matrix: ${{ steps.plan.outputs.matrix }} }
    steps:
      - uses: actions/checkout@v7.0.1
      - uses: Archont561/pixi-sandbox/setup@v0.2.0
        id: setup
        with: { version: v0.2.0 }
      - id: plan
        run: echo "matrix=$(${{ steps.setup.outputs.path }} plan --config .pixi-sandbox.toml --json)" >> $GITHUB_OUTPUT

  publish:
    needs: plan
    strategy: { matrix: ${{ fromJSON(needs.plan.outputs.matrix) }}, fail-fast: false }
    runs-on: ${{ matrix.runner }}
    steps:
      - uses: actions/checkout@v7.0.1
      - uses: prefix-dev/setup-pixi@v0.10.2
      - uses: Archont561/pixi-sandbox/setup@v0.2.0
        id: setup
        with: { version: v0.2.0 }
      - uses: Archont561/pixi-sandbox/publish@v0.2.0
        with:
          project: .
          environments: ${{ matrix.environments }}
          platform: ${{ matrix.platform }}
          branch: ${{ matrix.branch }}
          self-bin: ${{ steps.setup.outputs.path }}
          remote: https://github.com/${{ github.repository }}.git
          output-dir: /tmp/transport
          push-token: ${{ secrets.GITHUB_TOKEN }}
```

Single-platform without `plan`:

```yaml
- uses: Archont561/pixi-sandbox/setup@v0.2.0
  id: setup
  with: { version: v0.2.0 }
- uses: Archont561/pixi-sandbox/publish@v0.2.0
  with:
    project: .
    environments: dev,docs
    platform: linux-64
    branch: sandbox/developer-linux-64
    self-bin: ${{ steps.setup.outputs.path }}
    remote: https://github.com/${{ github.repository }}.git
    output-dir: /tmp/transport
    push-token: ${{ secrets.GITHUB_TOKEN }}
```

- `plan --json` validates `.pixi-sandbox.toml` and gives native `runner` per `bundle × platform`
- `setup` verifies `SHA256SUMS` before `chmod +x`; `publish` verifies `doctor --verify` before push
- Token via `http.extraheader` (never URL), rejects cross-packing

<details>
<summary>Alternative: reusable workflow</summary>

```yaml
jobs:
  publish:
    uses: Archont561/pixi-sandbox/.github/workflows/publish-sandbox.yml@v0.2.0
    with:
      config: .pixi-sandbox.toml
      release-repository: Archont561/pixi-sandbox
      release-version: v0.2.0
    secrets:
      SANDBOX_PUSH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

</details>

Docs: https://archont561.github.io/pixi-sandbox/guides/ci-publishing/ and https://archont561.github.io/pixi-sandbox/guides/actions/

---

## ⚙️ Configuration Reference

### `.pixi-sandbox.toml`

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
| `osx-64` | `macos-15-intel` | Hosted Intel |
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
pixi-sandbox plan --config .pixi-sandbox.toml [--json]
pixi-sandbox tools list [--tools-lock PATH]
```

- `--output-path` has alias `--path-to-main-repo-code` for backward compat
- Work dir default `.pixi/.restore-work` (same FS, TMPDIR redirected) — never small `/tmp`

Docs: https://archont561.github.io/pixi-sandbox/reference/cli/

---

## 📂 The Transport Layout

```text
sandbox/<bundle>-<platform>/   # orphan branch
├── pixi-sandbox                # root self-bootstrap convenience copy
├── restore.sh / restore.ps1    # thin wrappers around the root binary
├── .pixi-sandbox/
│   ├── manifest.json           # SHA-256 catalog, source commit, tool versions, vendor
│   ├── envs/<env>/pack/        # pixi-pack --directory-only: channel/*.conda + env.yml + pixi-pack.json
│   ├── tools/<platform>/       # pixi, pixi-unpack, pixi-sandbox (static, verified)
│   └── vendor/                 # cargo vendor --versioned-dirs (loose, deduped by git)
├── README.md                   # human restore instructions
└── AGENTS.md                   # agent instructions
```

**Sizes (small project, 2 envs, 33 crates, linux-64):**

| Part | Payload |
|------|---------|
| conda envs (.conda) | 133 MB |
| cargo vendor (loose) | 33 MB |
| tools | 96 MB |
| **transport dir** | **262 MB** |
| **orphan branch after git dedup** | **~110 MB** |

The root `pixi-sandbox` has the same bytes as the manifest's nested self-binary, so Git stores
only one blob. It is intentionally an extra convenience file; `doctor --verify` still verifies
the manifest copy. On Windows the names are `pixi-sandbox.exe` and `restore.ps1`.

Tools dominate small bundles — expected, git stores each tool blob once.

---

## 🏗️ Repository Architecture

| Path | Description |
|------|-------------|
| `crates/pixi-sandbox-core` | Core lib: manifest format, sharding, tool pins, verification |
| `crates/pixi-sandbox-git` | Trait-based Git: `ShellGit` + `FakeGit`/`RecordingRunner` |
| `crates/pixi-sandbox` | CLI binary + fixtures |
| `crates/pixi-sandbox/tests/fixtures/` | Synthetic transport (15 KB, split blob) — tests never point at repo root |
| `action.yml` + `setup/action.yml` + `.github/actions/setup-pixi-sandbox` | Setup action (short refs `owner/repo@vX`, `owner/repo/setup@vX`) |
| `publish/action.yml` + `.github/actions/publish-pixi-sandbox` | Publish action (short ref `owner/repo/publish@vX`) |
| `.github/workflows/ci.yml` | CI: lint + test + coverage + docs-build |
| `.github/workflows/publish-sandbox.yml` | Unified publisher: workflow_run + dispatch + call, native runners |
| `.github/workflows/release.yml` | Release: 5 tier-1 static binaries + SHA256SUMS + GitHub Release |
| `.github/workflows/docs.yml` | Docs → GitHub Pages |
| `.github/dependabot.yml` | Dependabot: cargo, gha, npm (convco prefixes) |
| `scripts/restore.sh` | One-liner offline reconstruction with PATH aliases |
| `docs/` | Starlight + astro-icon + Iconify docs (13 pages) |
| `.knowledge/` | Open Knowledge Format: decisions D1–D11, design, benchmarks |
| `CHANGELOG.md` | Changelog via convco |

---

## 🛠️ Development & Quality Gates

```bash
# Lint (rustfmt, clippy -D warnings, cargo-deny, actionlint, taplo, biome)
pixi run -e dev lint

# Test (nextest, fixtures, no network)
pixi run -e dev test
pixi run -e dev test-doc

# Coverage
pixi run -e dev coverage

# Docs (Astro + Starlight + astro-icon)
pixi run -e dev docs-dev
pixi run -e dev docs-build

# Transport pipeline (pure Rust self-bin)
pixi run -e dev sandbox-plan
pixi run -e dev sandbox-pack      # pack + vendor + fetch-tools + self-bin target/release/pixi-sandbox
pixi run -e dev sandbox-doctor
pixi run -e dev sandbox-publish
pixi run -e dev sandbox-restore   # one-liner from orphan branch
pixi run -e dev sandbox-proof     # full cold proof

# Changelog via convco
pixi run -e dev changelog-preview
pixi run -e dev changelog

# CI variants (env vars: SANDBOX_PROJECT, ENVS, TRANSPORT, PLATFORM, BRANCH, REMOTE, SELF_BIN)
pixi run -e ci ci-pack && pixi run -e ci ci-doctor && pixi run -e ci ci-publish
```

> [!IMPORTANT]
> Integration tests run against synthetic fixtures in `crates/pixi-sandbox/tests/fixtures/`, **never** against this repo itself, ensuring hermetic offline isolation. Two tests enforce it: fixture must not depend on pixi-pack, and no test may walk out via `..`/`.parent()`.

---

## 🔖 Changelog & Release

- **Changelog**: `CHANGELOG.md` generated via `convco changelog` from conventional commits. Tasks: `pixi run -e dev changelog`.
- **Release**: Tag `v*.*.*` → `release.yml` builds 5 static binaries, `SHA256SUMS`, creates GitHub Release with `generate_release_notes: true`.
  ```bash
  git tag v0.2.0 -m "v0.2.0"
  git push origin main --tags
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
- [GitHub Actions](https://archont561.github.io/pixi-sandbox/guides/actions/)
- [Configuration](https://archont561.github.io/pixi-sandbox/reference/configuration/)
- [CLI](https://archont561.github.io/pixi-sandbox/reference/cli/)
- [Actions API](https://archont561.github.io/pixi-sandbox/reference/actions/)
