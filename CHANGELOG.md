# Changelog

All notable changes to this project are documented in this file. It is generated via [convco](https://github.com/convco/convco) from conventional commits (`pixi run -e dev changelog`).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and adheres to [Conventional Commits](https://www.conventionalcommits.org/).

## [Unreleased]

## [0.2.0] - 2026-09-21

### Features

- **setup & publish short refs in same repo** — both GitHub Actions now usable with short references:
  - `Archont561/pixi-sandbox@vX` → setup (root, like `prefix-dev/setup-pixi`)
  - `Archont561/pixi-sandbox/setup@vX` → setup explicit alias
  - `Archont561/pixi-sandbox/publish@vX` → publish one native bundle
  - Root `action.yml` now wraps `.github/actions/setup-pixi-sandbox` (DRY); `setup/action.yml` and `publish/action.yml` wrap their respective composites. Long-form `.github/actions/...` paths still work.
  - Updated README, `quickstart.mdx`, and action READMEs.

- **one-liner offline reconstruction with PATH aliases** — `scripts/restore.sh` + `pixi run sandbox-restore`:
  - `git fetch origin sandbox/linux-64 && git worktree add /tmp/sb ... && /tmp/sb/.pixi-sandbox/tools/.../pixi-sandbox restore --output-path . --force; git worktree remove /tmp/sb --force; source .pixi/sandbox-env.sh; export PATH="$PWD/.pixi/envs/dev/bin:$PATH"`
  - Wires `pixi()` function + tools + dev bin like `setup-pixi`.

- **setup action like `prefix-dev/setup-pixi`** — root `action.yml` proxy with optional `repository`/`version` defaults, so minimal usage is `uses: Archont561/pixi-sandbox@v0.2.0`.

- **pure Rust self-bootstrap** — `sandbox-pack` now builds Rust binary via `sandbox-build-self` (`cargo build -p pixi-sandbox --release`) and embeds `target/release/pixi-sandbox` as `--self-bin` instead of Python reference implementation.

- **two-action user workflow** — recommended publishing is now just `Archont561/pixi-sandbox/setup@vX` + `Archont561/pixi-sandbox/publish@vX` with `plan --json` matrix; no `release-repository`/`release-version` duplication. Reusable workflow `publish-sandbox.yml` kept as alternative.

### Fixes

- **unify publish workflows** — `publish-sandbox.yml` now handles `workflow_run` (after ci.yml success), `workflow_dispatch`, and `workflow_call`; uses composite actions `setup-pixi-sandbox` (verified download) and `publish-pixi-sandbox` (install, pack with `--self-bin`, doctor, publish). Removed `publish-sandboxes.yml`.

- **release workflow** — new `release.yml` builds 5 tier-1 static binaries (musl Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64), strips, generates `SHA256SUMS` via `sha256sum`, creates GitHub Release with `generate_release_notes: true`. Contract matches `setup-pixi-sandbox` README (`pixi-sandbox-{target}{exe}`).

- **rename flag with alias** — CLI `--output-path` with alias `--path-to-main-repo-code` (backward compat). Updated `restore.rs`, `pack.rs`, `publish.rs`, `cli.rs`, tests, fixtures, and `docs/restore.mdx`.

- **scratch leak** — `shell.rs` fix: move scratch repo outside transport (same filesystem, never `/tmp`) to avoid leaking `.pixi-sandbox-publish-<pid>/` because git only ignores `.git`, not custom `GIT_DIR` names.

- **remove all Python script references** — deleted `.knowledge/research/pixi_sandbox.py`, `make_fixture_transport.py`, `reproduce.sh`, `scripts/write_release_checksums.py`; replaced with Rust tasks (`sandbox-proof`, `ci-proof` pure bash using embedded binary) and `sha256sum` for checksums.

### Documentation

- Removed Python refs from `.knowledge/README.md`, `AGENTS.md`, `design.md`, `publish-automation.md`, `rust-bootstrap.md`, `repository.mdx`, `quickstart.mdx`, `restore.mdx`, `fixtures/README.md`, `transport/README.md`, `EVIDENCE.md`, `REPRODUCE-TRANSCRIPT.md`.
- Updated `quickstart.mdx` to use Rust self-bin and unified publisher.
- Added `setup/README.md` and `publish/README.md` short-alias docs.
- Removed `.knowledge` links from user-facing docs (`design.mdx`, `repository.mdx`) — docs now standalone.
- Updated `README.md`, `quickstart.mdx`, `guides/ci-publishing.mdx`, `guides/actions.mdx` to make two-action workflow the primary example; reusable workflow moved to alternative.

### CI

- `pixi.toml` `sandbox-proof` and `ci-proof` rewritten in bash using embedded Rust binary (replaces old `reproduce.sh`).
- `lint-commit = "convco check --from-stdin"` via lefthook commit-msg hook enforces conventional commits.
- New tasks: `changelog = "convco changelog > CHANGELOG.md"` and `changelog-preview`.

## [0.1.0] - 2026-09-20

### Initial

- Initial commit with core crates:
  - `pixi-sandbox-core`: manifest format, sharding, tool pins, validation
  - `pixi-sandbox-git`: trait-based Git ops (`ShellGit` + `FakeGit`/`RecordingRunner`)
  - `pixi-sandbox`: CLI (`pack`, `publish`, `restore`, `doctor`, `plan`, `tools`)
- Fixtures: `demo-project` (real pixi project) + `transport` (synthetic payload with split blob)
- GitHub Actions: `ci.yml`, `docs.yml`, `publish-sandbox.yml`
- Docs: Astro + Starlight site, `.knowledge/` Open Knowledge Format (D1–D11)
- Tooling: `pixi.toml` tasks, `lefthook.yml`, `taplo`, `biome`, `actionlint`, `cargo-deny`, `convco`

### CI

- Upgrade workflow actions to Node 24 native versions
- Add codecov configuration and status badge

---

Generated with `pixi run -e dev changelog-preview` (convco). To update: `pixi run -e dev changelog`.
