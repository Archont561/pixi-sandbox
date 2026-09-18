---
type: Source Notes
title: "pixi: manifest, lockfile, subcommands"
description: What pixi 0.81.0 actually promises: manifest fields, tasks, environments, and the external-subcommand contract a pixi-sandbox binary rides on.
resource: https://github.com/Archont561/pixi-sandbox
tags: [research, pixi]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`WEB_SEARCH_RESULTS.md`], sections: ["1", "6"] }
stale_after: 2026-12-19T00:00:00Z
sources:
  - { id: githubcom-prefix-dev-pixi, resource: https://github.com/prefix-dev/pixi/, title: prefix-dev/pixi }
  - { id: pixiprefixdev-v0672-switching_from, resource: https://pixi.prefix.dev/v0.67.2/switching_from/conda/, title: pixi.prefix.dev/v0.67.2/switching_from/conda/ }
  - { id: pixiprefixdev-latest-first_workspace, resource: https://pixi.prefix.dev/latest/first_workspace/, title: pixi.prefix.dev/latest/first_workspace/ }
  - { id: pixiprefixdev-latest-concepts, resource: https://pixi.prefix.dev/latest/concepts/conda_pypi/, title: pixi.prefix.dev/latest/concepts/conda_pypi/ }
  - { id: pydevtoolscom-handbook-reference, resource: https://pydevtools.com/handbook/reference/pixi/, title: pydevtools.com/handbook/reference/pixi/ }
  - { id: pixiprefixdev-latest-reference, resource: https://pixi.prefix.dev/latest/reference/pixi_configuration/, title: pixi.prefix.dev/latest/reference/pixi_configuration/ }
  - { id: pixiprefixdev-latest-reference, resource: https://pixi.prefix.dev/latest/reference/cli/pixi/workspace/channel/add/, title: pixi.prefix.dev/latest/reference/cli/pixi/workspace/channel/ }
  - { id: githubcom-quantco-pixi-pack, resource: https://github.com/Quantco/pixi-pack, title: Quantco/pixi-pack }
---

# pixi: manifest, lockfile, subcommands

## 1. pixi

**What it is.** pixi is a cross-platform, **multi-language** package manager and workflow tool built on
the conda ecosystem, from prefix.dev, written in Rust on top of the `rattler` libraries; it aims for a
`cargo`/`npm`-like experience for any language [3](https://github.com/prefix-dev/pixi/) — "Supports
multiple languages including Python, C++, and R using Conda packages", "Always includes an up-to-date
lock file", "Entirely written in Rust and built on top of the rattler library".

**Model: workspace, not environment.** A pixi workspace = a folder with a manifest (`pixi.toml` or
`pyproject.toml`), a `pixi.lock` lock-file, and a `.pixi` directory holding the environments. Pixi
"builds upon the foundation of the conda ecosystem, introducing a workspace-centric approach rather
than focusing solely on environments", and manages *multiple environments for multiple platforms in a
single workspace* [5](https://pixi.prefix.dev/v0.67.2/switching_from/conda/). Minimal manifest
[2](https://pixi.prefix.dev/latest/first_workspace/):

```toml
[workspace]
channels = ["conda-forge"]
name = "my_workspace"
platforms = ["osx-arm64", "linux-64", "win-64"]

[dependencies]
[tasks]
```

**Two dependency universes.** Conda deps go in `[dependencies]`, PyPI deps with
`pixi add --pypi httpx` → `[pypi-dependencies]` [2](https://pixi.prefix.dev/latest/first_workspace/).
Resolution order matters and bites people: pixi resolves **conda first**, then maps conda packages to
PyPI names and resolves the rest with uv internally — so if a package exists on both sides, **the
conda package wins** [8](https://pixi.prefix.dev/latest/concepts/conda_pypi/),
[4](https://pydevtools.com/handbook/reference/pixi/). You can override the mapping with
`[workspace.conda-pypi-map]` (e.g. `conda-forge = { mapping = { pytorch = "torch" } }`)
[8](https://pixi.prefix.dev/latest/concepts/conda_pypi/).

**Tasks are the glue** (a Makefile replacement): `[tasks]` entries support `cmd`, `depends-on`, `cwd`,
`args`, `input`/`output` (content-hash based caching!), `env`, and minijinja templating
[2](https://pixi.prefix.dev/latest/first_workspace/).

**The command surface** (from the CLI help in the upstream README) is broad and deliberately cargo-ish:
`add`, `auth`, `build`, `clean`, `completion`, `config`, `exec`, `global`, `info`, `init`, `import`,
`install`, `list`, `lock`, `reinstall`, `remove`, `run`, `search`, `shell`, `shell-hook`, `task`,
`tree`, `update`, `upgrade`, `upload`, `workspace` — plus **global options that matter for CI and for
sandboxes like ours**: `--offline`, `--locked`, `--frozen`, `--tls-no-verify`, `--no-progress`,
`--color`, and `--list` ("List all installed commands (built-in and extensions)")
[3](https://github.com/prefix-dev/pixi/).

**Configuration layering** — 9 priority levels, CLI > `.pixi/config.toml` (project) > `$PIXI_HOME/config.toml`
> user > rattler shared > system-wide [10](https://pixi.prefix.dev/latest/reference/pixi_configuration/).
Notable fields: `default-channels`, `offline`, `detached-environments`, `[cache]` with **per-kind paths
and netfs redirection** (HPC-shaped), `[concurrency] downloads = 50 / solves`, `[mirrors]`, and
`[pypi-config] index-url / allow-insecure-host`
[10](https://pixi.prefix.dev/latest/reference/pixi_configuration/).

**Mirrors are the air-gap story.** `[mirrors]` can rewrite any channel URL to a list of alternatives,
and the alternatives may be **OCI registries (`oci://ghcr.io/channel-mirrors/conda-forge`) or S3**
[10](https://pixi.prefix.dev/latest/reference/pixi_configuration/).

**Offline semantics** (important, subtle) — `--offline` / `PIXI_OFFLINE` / `offline = true`:
"Any HTTP request that would still slip through is rejected by the HTTP client itself"; network-only
commands (`self-update`, `upload`, `publish`) fail fast; the restriction applies **to the solve for the
host platform only** (other platforms solve from cached repodata); `pixi add`/`pixi upgrade` offline
record bounds derived from *local* availability (i.e. they rewrite your manifest based on what happens
to be cached); **PyPI solving via uv is only "offline connectivity", not restricted to cached wheels**;
and offline mode is **not enforced for build backends** (they are separate processes with their own HTTP
clients) [3](https://pixi.prefix.dev/latest/reference/pixi_configuration/),
[1](https://pixi.prefix.dev/latest/reference/cli/pixi/workspace/channel/add/).

**Supply-chain niceties.** `exclude-newer = "7d"` (added ~0.67.0) delays adoption of freshly released
packages, workspace-wide with per-channel/per-package overrides
[4](https://pydevtools.com/handbook/reference/pixi/). Lock-file **version 7** is required for named
platforms (e.g. `platforms = [{ name = "jetson", platform = "linux-aarch64", cuda = "13" }]`)
[3](https://github.com/Quantco/pixi-pack).

**Global tools** behave like pipx/condax: `pixi global install bat` — one isolated environment per tool,
binaries linked onto the global path; the docs explicitly warn "Never install pip with `pixi global`"
[5](https://pixi.prefix.dev/v0.67.2/switching_from/conda/).

**pixi *as a build system*.** `preview = ["pixi-build"]` unlocks `[package]` +
`[package.build] backend = { name = "pixi-build-rust" | "pixi-build-python" | "pixi-build-cmake" |
"pixi-build-ros" | "pixi-build-mojo" | "pixi-build-rattler-build" }`, producing conda packages
[2](https://prefix-dev.github.io/pixi/v0.63.2/build/backends/pixi-build-rust/),
[3](https://deepwiki.com/prefix-dev/pixi-build-backends/5-usage-guide). `pixi-build-rust` shells out to
`cargo install --locked --root "$PREFIX" --path . --no-track`, auto-detects **sccache** as
`RUSTC_WRAPPER`, wires OpenSSL, and inherits metadata from `Cargo.toml`; limitations: release-mode only,
no custom cargo profiles, "limited workspace support for multi-crate projects"
[2](https://prefix-dev.github.io/pixi/v0.63.2/build/backends/pixi-build-rust/). Compilers are selected
with `compilers = ["rust","c","cxx","fortran","go","cuda"]`.

> [!WARNING]
> **Live-version reality check.** Search snippets still show pixi 0.46/0.48/0.55/0.59/0.63/0.66/0.67.
> The **actual latest release today is `v0.81.0`, released 2026-09-15** — ✅ verified locally via
> `api.github.com` and the repo's own `CHANGELOG.md` ("0.81.0 - 2026-09-15: `pixi run --script` can now
> run `conda-script` files directly from HTTP or HTTPS URLs, including GitHub Gists"). Upstream also
> declares `rust-version = "1.90"`, `edition = "2024"` and ~60 internal crates
> (`pixi_cli`, `pixi_core`, `pixi_manifest`, `pixi_build_rust`, `pixi_global`, `pixi_pty`, …) —
> ✅ verified locally. Docs URLs like `pixi.sh/latest/…` vs `prefix-dev.github.io/pixi/vX/…` are the
> main source of stale-flag confusion: **always pin `pixi-version:` in CI.**

## 6. pixi-prefixed executables: `pixi-pack` as a subcommand

This is a real, cargo-style **external subcommand** mechanism — and I confirmed it in source
(✅ verified locally, `crates/pixi_cli/src/command_info.rs`, pixi v0.81.0):

```rust
// clap setup on the top-level parser:
#[clap(arg_required_else_help = true, disable_help_flag = true, allow_external_subcommands = true)]
...
#[command(external_subcommand)]
```
```rust
/// Find a specific external subcommand by name
/// Based on cargo's find_external_subcommand function
pub fn find_external_subcommand(cmd: &str) -> Option<PathBuf> {
    let command_exe = format!("pixi-{}{}", cmd, env::consts::EXE_SUFFIX);
    search_directories().and_then(|dirs| dirs.into_iter()
        .map(|dir| dir.join(&command_exe)).find(|path| path.is_executable()))
}
---
```
`find_external_commands()` walks the search directories, takes every executable whose name starts with
`pixi-`, strips the prefix (and `.exe` on Windows), and that set powers `pixi --list` ("List all installed
commands (built-in and extensions)"); dispatch is `Command::External(args) =>
command_info::execute_external_command(args)`. There is even a Levenshtein-ish "suggestions" helper for
typos.

**Consequence:** `pixi pack …` is not a pixi feature — it is a **name resolution convention**. Any
executable named `pixi-<thing>` on `PATH` becomes `pixi <thing>`, exactly like `cargo-<thing>`. That's
why the docs say:

> "You can also write `pixi pack` (and `pixi unpack`) if you have `pixi`, and `pixi-pack` and
> `pixi-unpack` installed globally." [2](https://pixi.prefix.dev/latest/deployment/pixi_pack/),
> ✅ mirrored in the upstream `docs/deployment/pixi_pack.md` (kept in sync with the pixi-pack README).

**Three ways to get a prefixed executable, in decreasing permanence:**

| Style | Command | Environment lifetime | Where the PATH comes from |
|---|---|---|---|
| ephemeral | `pixi exec pixi-pack` / `pixi x <cmd>` | temp env per invocation, **nothing left behind** | pixi creates the env, prepends it to `PATH` for the child |
| global | `pixi global install pixi-pack pixi-unpack` | persistent, pipx-like (one env per tool) | `~/.pixi/bin` on PATH → `pixi-pack` also becomes `pixi pack` |
| project | declare in a `pixi.toml` `[dependencies]`, then `pixi run pixi-pack` | per-workspace, in `.pixi/envs/<env>/bin` | activation by `pixi run`/`pixi shell` |

`exec` is documented in the CLI as "**Run a command and install it in a temporary environment**"
[aliases: `x`] [3](https://github.com/prefix-dev/pixi/), and there are `exec-environments` cache entries
in the config (`cache.exec-environments`, redirected to node-local scratch on network filesystems)
[10](https://pixi.prefix.dev/latest/reference/pixi_configuration/) — i.e. repeated `pixi exec` runs are
cached, so `pixi exec pixi-pack` in CI is cheap.

> [!TIP]
> Design consequence for our own tool: shipping a binary named **`pixi-<name>`** (and putting it on PATH)
> gives you `pixi <name>` for free — plus `pixi --list` discovery — with **zero pixi-side registration**.
> Pair it with `clap_complete` for shell completion (pixi-pack generates completions via
> `clap_complete::generate`) and with a `--create-executable`-style offline artifact.
> Note the historical CLI break: old pixi docs show `pixi exec pixi-pack pack` / `pixi-pack pack
> --manifest-file pixi.toml` [3](http://pixi.prefix.dev/v0.43.1/deployment/pixi_pack/) versus today's
> flat `pixi-pack <manifest>` + separate `pixi-unpack`. **Pin and re-verify any pixi-pack version you
> automate against.**

---
