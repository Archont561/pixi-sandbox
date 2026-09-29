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
   from a Linux airlock.
2. **Implement `pixi-sandbox tools update`** (backlog **task-4**). The command can be *written and
   unit-tested* offline, but its live behaviour — resolving/downloading/sha256-verifying the latest
   `pixi`, `pixi-pack`, `pixi-unpack`, `rattler-index` pins — needs network, so end-to-end
   verification belongs in Codespaces/CI.

The dev container that provides this online environment is `.devcontainer/devcontainer.json`
(`ghcr.io/prefix-dev/pixi`, with `pixi install --locked --all` + `docs-install` + `setup-opencode`
on create).

---

## 📚 Key References

| Resource | Purpose |
|:---|:---|
| [`.knowledge/decisions.md`](.knowledge/decisions.md) | Architectural Decisions D1–D12 with empirical lab measurements |
| [`.knowledge/design.md`](.knowledge/design.md) | In-depth design specification and airlock invariants |
| [`.knowledge/rust-bootstrap.md`](.knowledge/rust-bootstrap.md) | Rust static bootstrap binary strategy and validation checklist |
| [`.knowledge/README.md`](.knowledge/README.md) | Open Knowledge Format index |
