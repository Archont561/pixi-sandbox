# pixi-sandbox

<p align="center">
  <a href="https://github.com/Archont561/pixi-sandbox/actions/workflows/ci.yml"><img src="https://github.com/Archont561/pixi-sandbox/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/Archont561/pixi-sandbox/actions/workflows/docs.yml"><img src="https://github.com/Archont561/pixi-sandbox/actions/workflows/docs.yml/badge.svg" alt="Docs"></a>
  <a href="https://github.com/Archont561/pixi-sandbox/releases"><img src="https://img.shields.io/github/v/release/Archont561/pixi-sandbox?label=release" alt="Release"></a>
  <a href="https://codecov.io/gh/Archont561/pixi-sandbox"><img src="https://codecov.io/gh/Archont561/pixi-sandbox/branch/main/graph/badge.svg" alt="Coverage"></a>
  <a href="https://archont561.github.io/pixi-sandbox/"><img src="https://img.shields.io/badge/docs-Starlight-6d28d9" alt="Documentation"></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.94%2B-orange.svg?logo=rust" alt="Rust 1.94 or newer"></a>
  <img src="https://img.shields.io/badge/Platforms-linux--64%20%7C%20osx--arm64-brightgreen.svg" alt="Development workspace platforms">
  <a href="https://github.com/Archont561/pixi-sandbox/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT license"></a>
</p>

**Self-contained, verifiable offline environments for [Pixi](https://pixi.sh) projects.**
Pack a locked environment on a connected machine, publish it to a Git orphan branch, and restore
it on an airlocked host without contacting package registries. The transport carries the packages,
explicitly bundled executables, integrity manifest, and optionally vendored Rust crates—not just instructions for
recreating them online.

The CLI is published on [prefix.dev](https://prefix.dev/channels/@archont561/archont561) and as
[standalone release binaries](https://github.com/Archont561/pixi-sandbox/releases).
Documentation is at <https://archont561.github.io/pixi-sandbox/>.

## Native transport upgrade

This source tree replaces external environment pack/unpack helpers with native rattler/UV
installation. New transports use schema 3; format-1 packs under schemas 1–3 restore natively.
The `tools` command and helper-management flags are retired. Bundle Pixi/bootstrap executables
explicitly with `--pixi-bin` / `--self-bin` when needed. Source builds require Rust 1.94+ and
packing accepts version-7 Pixi locks with wheel-only PyPI dependencies. Legacy opaque transports
retain their recorded unpacker compatibility path.

Read the [native migration checklist](https://archont561.github.io/pixi-sandbox/guides/migration/)
before upgrading producer scripts. Version `0.7.0` predates this interface; install a release
containing the native migration before using the new flags.

## Quick start

### Start a new project

Use the **[Pixi Sandbox starter](https://github.com/Archont561/pixi-sandbox-starter)**:
choose **Use this template**, create your repository, then clone it and run:

```bash
pixi run dev
```

The starter includes a locked, language-neutral Pixi project, release-pinned publishing and
relock workflows, a devcontainer, and `scripts/restore.sh`. You do not need to run `init` or
copy workflows from this repository. It currently targets `linux-64`.

> [!NOTE]
> Projects created from the v0.6.2 starter inherited an incorrect Cargo-vendoring default.
> Before pushing such a project, set `cargo_vendor = false` in its `pixi-sandbox.toml`: the
> starter contains no Rust workspace or `Cargo.lock`. The v0.7.0 starter ships the corrected
> setting, and its own publisher
> [completed successfully](https://github.com/Archont561/pixi-sandbox-starter/actions/runs/38804893239).

Commit your changes and push to `main` to run the included publisher. Its repository-scoped
`GITHUB_TOKEN` publishes the transport to `sandbox/developer-linux-64`; no Archont561-owned
secret is needed for that transport publication. Wait for the publisher to succeed before
attempting a restore.

### Add it to an existing project

On a connected host with Pixi installed:

```bash
pixi global install --channel https://prefix.dev/archont561/archont561 --channel conda-forge pixi-sandbox
pixi-sandbox init
pixi-sandbox plan
```

Review `pixi-sandbox.toml`, the generated workflows under `.github/workflows/`, and the restore
launcher before committing. Enable Cargo vendoring only for a Rust project with a usable
`Cargo.lock`; with the v0.6.2 CLI, explicitly set `cargo_vendor = false` for other projects.
After changing that setting, run `pixi-sandbox init` again to refresh the generated relock policy.

When `pixi-sandbox` is on `PATH`, the same commands are available as `pixi sandbox init`,
`pixi sandbox plan`, and so on. See the [quickstart](https://archont561.github.io/pixi-sandbox/quickstart/)
and [existing-project guide](https://archont561.github.io/pixi-sandbox/guides/using-in-your-project/).

### Restore on an airlocked host

Transfer the project **and its published sandbox branch** into the restricted environment
(for example, through an approved Git mirror or Git bundle). In a starter-derived checkout:

```bash
bash scripts/restore.sh
# Open a new shell, or add the registered user tools to this shell:
export PATH="$HOME/.local/bin:$PATH"
pixi install --frozen --offline
pixi run --frozen dev
```

For an existing project initialized with default paths, the launcher is `restore.sh`
(`restore.ps1` on Windows), rather than `scripts/restore.sh`.
Restore verifies the transport before writing, relocates the environment, verifies the restored
files, and registers user tools. It does not install missing host OS libraries or turn an
application that needs a network connection into an offline application.

The [restore guide](https://archont561.github.io/pixi-sandbox/restore/) covers branch transfer,
verification, tool registration, and shared-machine policies.

## What it provides

| Capability | What you get | Guide |
| :-- | :-- | :-- |
| Git-native transport | Self-contained orphan branches, whole-file deduplication, and sharding for large blobs | [Design](https://archont561.github.io/pixi-sandbox/design/) |
| Verified restore | SHA-256 checks before writing and a per-file check of the relocated environment | [Airlock restore](https://archont561.github.io/pixi-sandbox/restore/) |
| Explicit executables | Verified release bootstrap downloads; local Pixi/standalone CLI bundled only when selected, with recorded byte hashes | [Installation](https://archont561.github.io/pixi-sandbox/installation/) |
| Generated CI | Native publish matrix, pack/doctor/publish pipeline, lockfile refresh, and reviewed upgrades | [CI publishing](https://archont561.github.io/pixi-sandbox/guides/ci-publishing/) |
| Cargo vendoring | Optional vendored crates and sandbox-owned Cargo configuration for offline Rust builds | [Configuration](https://archont561.github.io/pixi-sandbox/reference/configuration/) |
| Publish budgets | Limits for blobs, transport size, repository pushes, and restore disk requirements | [Configuration](https://archont561.github.io/pixi-sandbox/reference/configuration/) |
| Host diagnostics | Declare and check OS-level prerequisites that the packed environment cannot supply | [Host requirements](https://archont561.github.io/pixi-sandbox/guides/host-requirements/) |

## How it works

```text
Connected host                  Git transport                    Airlocked host

locked Pixi project  -- pack --> sandbox/<bundle>-<platform>
                                manifest + packages + tools
                     -- doctor  (optional vendored crates)
                     -- publish          |
                                         +-- transfer / restore --> verified environment
```

`pack` builds the transport. `doctor` checks integrity and budgets without restoring it.
`publish` writes a dedicated orphan branch, not your source branch. `restore` verifies and
materializes the payload; `doctor --verify-restored` audits it afterward.

The generated publisher runs these phases in order and does not publish a failed pack or
verification. For manual operation and the exact options, use the
[CLI reference](https://archont561.github.io/pixi-sandbox/reference/cli/).

## Platforms and guarantees

**CLI releases and this repository's development sandbox are different support surfaces.**
Releases contain native binaries and Conda packages for `linux-64`, `linux-aarch64`, `osx-64`,
`osx-arm64`, and `win-64`. Linux standalone binaries use static musl; macOS and Windows use
native system linkage.

The badge above describes this repository's Pixi workspace, not every CLI release target:

| Workspace platform | Development transport |
| :-- | :-- |
| `linux-64` | Routinely packed, verified, and published as `sandbox/developer-linux-64` |
| `osx-arm64` | Native airlock proof exists; omitted from routine publishing to avoid the additional runner cost |
| `win-64` | Not a development-workspace target: conda-forge has no `bun` build; native proof is required by ADR-11 before this workspace can claim support |

Consumer projects declare their own environments and platforms. A released binary for a
platform is not evidence that every consumer's dependencies can be packed or restored there.
The airlock CI uses native runners and egress-denied restore checks; local fixture tests are
not a substitute for that connected-platform proof.

A sandbox branch must be available locally before a fully disconnected restore. Integrity
verification detects changed bytes; it does not replace your trust policy for the source
repository, Git refs, or release publisher.

## Repository layout

This is a Rust workspace with Bun/Turbo orchestration and an Astro documentation site.

| Path | Role |
| :-- | :-- |
| `crates/pixi-sandbox` | CLI, transport lifecycle, generated workflows and launchers |
| `crates/pixi-sandbox-core` | Manifest, sharding, integrity checks, platform and publish-plan models |
| `crates/pixi-sandbox-git` | Git protocol and implementation |
| `crates/xtask` | Tested repository, release, starter, and CI automation |
| `docs` | Astro + Starlight documentation, published to GitHub Pages |
| `scripts/restore.sh` | Bootstrap for this repository's packed development environment |
| `backlog` | Tasks, specifications, ADRs, milestones, and implementation plans |

## Development

On a restricted host, restore the published development environment first:

```bash
bash scripts/restore.sh
export PATH="$HOME/.local/bin:$PATH"
```

Then use Pixi as the environment entrypoint:

```bash
pixi run --frozen docs-install
pixi run --frozen fmt
pixi run --frozen lint
pixi run --frozen test
pixi run --frozen test-doc
pixi run --frozen coverage
pixi run --frozen docs dev
pixi run --frozen docs build
```

Use `pixi run --frozen -- cargo <command> --offline -p <crate>` for focused Rust work after a
restore. Tests use committed fixtures and isolated temporary directories, never this checkout
as their payload. Keep generated environments, transports, build output, and vendored crates
out of source commits.

On an airlocked machine, editing a dependency manifest does not make the dependency available:
lock refresh and repacking happen on a connected host. See the
[repository guide](https://archont561.github.io/pixi-sandbox/repository/) for setup and automation.

## Contributing

Report bugs and feature requests through [GitHub issues](https://github.com/Archont561/pixi-sandbox/issues).
Before opening a pull request:

- Read [AGENTS.md](AGENTS.md) for repository invariants, test conventions, and task commands.
- Add regression coverage at the public boundary of the behavior you change.
- Run formatting, lint, and tests through Pixi.
- Update the user-facing documentation when behavior changes.
- Use focused Conventional Commits and record design decisions in the repository.

Release history is in [CHANGELOG.md](CHANGELOG.md). Release automation builds and tests native
artifacts, runs the consumer lifecycle proof before publication, then publishes packages and
standalone assets. A successful starter-generation run alone does **not** prove the generated
starter's publisher works; that requires its own end-to-end run.

## License

[MIT](LICENSE). Bundled packages and vendored crates retain their upstream licenses.
