# pixi-sandbox

<p align="center">
  <a href="https://github.com/Archont561/pixi-sandbox/actions/workflows/ci.yml"><img src="https://github.com/Archont561/pixi-sandbox/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://codecov.io/gh/Archont561/pixi-sandbox"><img src="https://codecov.io/gh/Archont561/pixi-sandbox/branch/main/graph/badge.svg" alt="Coverage"></a>
  <a href="https://github.com/Archont561/pixi-sandbox/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.85%2B-orange.svg?logo=rust" alt="Rust 1.85+"></a>
  <a href="https://pixi.sh"><img src="https://img.shields.io/badge/Pixi-0.81%2B-yellow.svg?logo=condaforge" alt="Pixi"></a>
  <img src="https://img.shields.io/badge/Platforms-linux--64%20%7C%20osx--arm64%20%7C%20win--64-brightgreen.svg" alt="Platforms">
  <a href="https://github.com/Archont561/pixi-sandbox/pulls"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg" alt="PRs Welcome"></a>
</p>

---

**pixi-sandbox** packages [pixi](https://pixi.sh) environments and vendored Cargo dependencies into an isolated Git orphan branch on a connected machine, enabling **100% offline, bit-for-bit airlock restoration** on disconnected target machines with zero network access.

> [!NOTE]
> Because `pixi` automatically discovers `pixi-<command>` binaries on `$PATH`, installing `pixi-sandbox` unlocks native subcommands: `pixi sandbox pack`, `doctor`, `publish`, and `restore`.

---

## ⚡ Quick Start

### 1. Connected Host: Pack, Verify & Publish
```bash
# 1. Pack environments and vendor Cargo crates
pixi sandbox pack \
  --repo-root . \
  --envs dev,docs \
  --output-dir .sandbox-transport \
  --platform linux-64 \
  --cargo-vendor \
  --fetch-tools

# 2. Verify manifest and file integrity
pixi sandbox doctor --branch-location .sandbox-transport --verify

# 3. Publish to a dedicated Git orphan branch
pixi sandbox publish \
  --input-dir .sandbox-transport \
  --branch-name sandbox/linux-64
```

### 2. Disconnected / Airlocked Host: Restore
```bash
# Fetch and restore the sandbox payload without internet access
git fetch origin sandbox/linux-64:sandbox/linux-64
pixi sandbox restore \
  --branch-location sandbox/linux-64 \
  --output-path .
```

> [!TIP]
> After `restore`, all conda environments are unpacked under `.pixi/envs/` with binary prefix paths relocated, and `.cargo/config.toml` is wired to vendored crate sources. Both `pixi install --frozen --offline` and `cargo build --offline` succeed immediately.

---

## 🚀 Key Features

- **🔒 Zero-Network Airlock Restoration**: Verified restore in network-severed environments (`unshare -rn`) without touching external conda channels or crates.io.
- **📦 Git-Native Transport**: Published to a Git orphan branch. Whole files are content-addressed by Git, providing free deduplication across environments and releases.
- **✂️ Automatic Blob Sharding**: Splits files larger than 95 MiB into `.partNNN` chunks to safely respect GitHub's 100 MiB per-file blob limit.
- **🛡️ Strict Checksum Verification**: Cryptographically verifies every package and tool payload against embedded SHA-256 pins before writing to disk.
- **🦀 Cargo Vendoring Integration**: Bundles Cargo workspace dependencies alongside Pixi conda environments.
- **🤖 Pure Native Action & Test Harness**: GitHub Actions implemented without runtime Python dependencies; full test suite running under `cargo nextest`.

---

## 📂 The Transport Layout

```text
sandbox/<platform>/
├── .pixi-sandbox/
│   ├── manifest.json       # Content catalog with SHA-256 digests, parts, and tool metadata
│   ├── envs/<env>/pack/    # Local Conda channel containing .conda/.tar.bz2 packages
│   ├── tools/<platform>/   # Pinned static helper binaries (pixi, pixi-unpack, pixi-sandbox)
│   └── vendor/             # Vendored Cargo crates deduplicated by Git object storage
├── README.md               # Human-readable restore instructions
└── AGENTS.md               # Machine-readable automation instructions
```

---

## 🤖 GitHub Actions (like `setup-pixi`)

Both actions live in the **same repo** with short references:

```yaml
# Setup: download verified release binary (like prefix-dev/setup-pixi)
- uses: Archont561/pixi-sandbox@v0.2.0           # root = setup
  with:
    version: v0.2.0
- run: pixi-sandbox --version

# Same setup via explicit short path
- uses: Archont561/pixi-sandbox/setup@v0.2.0
  with:
    version: v0.2.0

# Publish: pack + doctor + publish one native bundle
- uses: Archont561/pixi-sandbox/publish@v0.2.0
  with:
    project: .
    environments: dev,docs
    platform: linux-64
    branch: sandbox/dev-linux-64
    self-bin: ${{ steps.setup.outputs.path }}
    remote: https://github.com/OWNER/REPO.git
    output-dir: /tmp/transport
```

Pin to immutable SHA for supply-chain security:

```yaml
- uses: Archont561/pixi-sandbox@<sha>            # setup (root)
- uses: Archont561/pixi-sandbox/setup@<sha>      # setup (explicit)
- uses: Archont561/pixi-sandbox/publish@<sha>    # publish
```

Long-form paths still work:

```yaml
- uses: Archont561/pixi-sandbox/.github/actions/setup-pixi-sandbox@<sha>
- uses: Archont561/pixi-sandbox/.github/actions/publish-pixi-sandbox@<sha>
```

Reusable workflow publisher for other repos (uses `.pixi-sandbox.toml`):

```yaml
jobs:
  publish:
    uses: Archont561/pixi-sandbox/.github/workflows/publish-sandbox.yml@<sha>
    with:
      release-repository: Archont561/pixi-sandbox
      release-ref: <sha>
      release-version: v0.2.0
      # cargo-vendor: false  # disable if you only need conda envs
    secrets:
      SANDBOX_PUSH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

To publish multiple envs without cargo vendoring:

```toml
# .pixi-sandbox.toml
schema=1
cargo_vendor=false
[[bundle]]
name="python"
environments=["dev","test"]
platforms=["linux-64","win-64"]
```

## 🏗️ Repository Architecture

| Path | Description |
|:---|:---|
| `crates/pixi-sandbox-core` | Core library: manifest format, sharding, tool pins, validation logic |
| `crates/pixi-sandbox-git` | Trait-based Git operations: `ShellGit` and in-memory `RecordingRunner` |
| `crates/pixi-sandbox` | The native CLI binary (`pack`, `publish`, `restore`, `doctor`, `plan`) |
| `crates/pixi-sandbox/tests/` | Pure Rust integration suites: `cli.rs`, `e2e.rs`, `actions.rs`, `fixtures.rs` |
| `action.yml` + `setup/action.yml` + `.github/actions/setup-pixi-sandbox` | Setup: downloads & verifies release binary (short refs: `owner/repo@vX` and `owner/repo/setup@vX`) |
| `publish/action.yml` + `.github/actions/publish-pixi-sandbox` | Publish: pack, doctor, publish one bundle (short ref: `owner/repo/publish@vX`) |
| `.github/workflows/ci.yml` | Unified single-job CI with Cargo caching (`Swatinem/rust-cache`) |
| `.github/workflows/publish-sandbox.yml` | Automated branch publisher (unified, triggers on ci success, dispatch, call) |
| `.github/workflows/release.yml` | Multi-platform release: builds 5 static binaries + SHA256SUMS |
| `.knowledge/` | Open Knowledge Format architectural decisions (D1–D11) and research benchmarks |

---

## 🛠️ Development & Quality Gates

All development tasks are managed via Pixi:

```bash
# Run linting (rustfmt, clippy -D warnings, cargo-deny, actionlint, taplo, biome)
pixi run -e dev lint

# Run the full test suite with nextest
pixi run -e dev test

# Run doc tests
pixi run -e dev test-doc

# Generate test coverage report (lcov.info)
pixi run -e dev coverage

# Build the documentation site
pixi run -e dev docs-build
```

> [!IMPORTANT]
> Integration tests deliberately run against synthetic fixtures in `crates/pixi-sandbox/tests/fixtures/`, **never** against this repository itself, ensuring hermetic, offline test isolation.

---

## 📜 License

Distributed under the [MIT License](LICENSE). Third-party packages and vendored crates retain their respective upstream licenses.
