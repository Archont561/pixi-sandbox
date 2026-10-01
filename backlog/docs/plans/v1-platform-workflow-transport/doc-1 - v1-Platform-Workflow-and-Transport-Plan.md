---
id: doc-1
title: 'v1 Platform, Workflow, and Transport Plan'
type: specification
created_date: '2026-10-01 09:48'
updated_date: '2026-10-01 09:49'
tags:
  - v1
  - platform
  - transport
  - workflow
---
# v1 Platform, Workflow, and Transport Plan

## Status

Proposed plan. The existing `.knowledge/` decisions remain authoritative until the relevant spike and acceptance tests are complete.

## Source context

This plan consolidates the current `.knowledge/` base and the design discussion. Background: `.knowledge/design.md`, `.knowledge/decisions.md`, `.knowledge/publish-automation.md`, `.knowledge/rust-bootstrap.md`, `.knowledge/rattler-spike.md`, and `.knowledge/v1-evolution-plan.md`.

## Desired user experience

```text
pixi global install -c https://prefix.dev -c https://prefix.dev/channels/@archont561/pixi-sandbox pixi-sandbox
pixi-sandbox init [--github-workflow-path PATH] [--script-path PATH] [--config PATH]
```

`init` is provider-neutral and generates a regenerable workflow plus exactly one platform-appropriate airlock launcher. The workflow installs and invokes `pixi-sandbox` directly; it does not reference Archont561 composite Actions. `pixi-sandbox.toml` is the preferred config name, with `.pixi-sandbox.toml` retained as a compatibility fallback.

## Workstreams

### 1. Init and configuration

- Remove the positional `github` provider from `init`.
- Add `--github-workflow-path`, `--script-path`, and `--config` overrides.
- Generate only `restore.sh` on Unix or `restore.ps1` on Windows by default.
- Resolve config in the order explicit path, `pixi-sandbox.toml`, `.pixi-sandbox.toml`.
- Add regeneration markers and safe `--force` behavior.

### 2. Generated connected workflow

- Install Pixi and the channel package from `@archont561/pixi-sandbox`.
- Run `pixi-sandbox plan`, `pack`, `doctor`, and `publish` directly.
- Preserve native bundle/platform matrix behavior.
- Make the file disposable and fully regenerable.

### 3. Airlock bootstrap and transport

- Keep the generated script thin: resolve branch, extract transport, execute embedded `pixi-sandbox restore`.
- Keep restore verification, relocation, Pixi markers, Cargo vendor handling, and final checks in Rust.
- Measure the proposed standalone v1 transport before replacing D2/D3; schema changes require explicit compatibility or migration behavior.
- Prevent duplicate `pixi-sandbox` copies: one canonical bootstrap binary, with any environment dependency promoted or excluded deliberately.

### 4. Platform packages and binaries

- Publish Conda/Pixi package variants for linux-64, linux-aarch64, osx-64, osx-arm64, and win-64 using native runners.
- Retain the Rust standalone binary matrix for transport bootstrap and GitHub Release assets.
- Add install smoke tests for every published package variant.
- Keep cargo-dist optional; do not add it as a runtime dependency.

### 5. JavaScript development tooling

- Move Biome, Astro, Starlight, TypeScript, and other npm-compatible development tools to `package.json`/`bun.lock`.
- Keep Bun in a separate Pixi `web` environment, not the cross-platform default environment.
- For Windows, evaluate a Node.js plus prefix-local Bun installation only if the Bun npm distribution provides a supported executable.
- Add Turbo only after multiple JS workspaces need dependency-aware scheduling and caching.

## Acceptance strategy

1. Existing Linux and macOS restore tests remain green.
2. Generated files are tested for platform selection, config precedence, regeneration, and absence of repository-owned Actions.
3. Generated workflows pass actionlint and execute the direct CLI path.
4. Package artifacts are produced and installed on every target platform.
5. Transport manifests reject duplicate tools and unsupported schemas.
6. A native airlock proof verifies the embedded binary and restored environment with network disabled.
7. JS jobs use the web environment and do not require Bun in Windows default CI.

## Sequencing

1. Configuration resolver and provider-neutral init.
2. Generated workflow and platform launcher tests.
3. Package matrix and platform smoke tests.
4. Standalone transport spike and decision update.
5. Any transport implementation/schema migration.
6. Bun environment migration.
7. Turbo adoption only if workspace growth justifies it.

## Explicit non-goals

- Do not silently replace the measured D2/D3 transport with rattler.
- Do not use global npm installs in CI or airlock environments.
- Do not require Bun in the Windows default Pixi environment.
- Do not hand-edit generated workflows or restore launchers.
