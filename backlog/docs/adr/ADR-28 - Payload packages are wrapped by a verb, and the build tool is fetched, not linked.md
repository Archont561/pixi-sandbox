---
id: ADR-28
title: "Payload packages are wrapped by a verb, and the build tool is fetched, not linked"
date: '2026-10-14'
status: accepted
tags:
  - decision
---

# ADR-28 — Payload packages are wrapped by a verb, and the build tool is fetched, not linked

**Decision.** A prebuilt binary payload (TASK-78) becomes an installable conda package through a
new verb on the shipped binary, `pixi sandbox wrap-payload`. Its pure rules (payload validation, budget preflight, recipe rendering, provenance) live in
`pixi-sandbox-core::payload` and are tested without I/O. Building the package runs `rattler-build`
as a subprocess. It is fetched on demand into the tools cache by the existing tool-fetch machinery,
and it is never linked and never embedded. It stays out of `tools.lock.json`, and `Cargo.lock` does
not change. Channel resolution stays in pixi's own configuration, through `[mirrors]`,
`PIXI_CONFIG_FILE` or `PIXI_NO_CONFIG` (ADR-17), and the consumer's `pixi.toml` is never rewritten.
pixi-sandbox never downloads the payload. A config-declared `prepare` command does, and its declared
source SHA256 is checked against what it produced. The feature therefore adds no network code of its
own: its one download is `rattler-build` itself, through the existing tool-fetch adapter
(`tool_fetch::HttpDownload`).

## Alternatives excluded

- **An xtask subcommand.** `crates/xtask` is a workspace member that appears in no release artifact.
  The standalone assets are `pixi-sandbox-<target>` (`crates/xtask/src/release_assets.rs`), and the
  conda package builds only `crates/pixi-sandbox`. A consumer who holds `pixi` and `pixi-sandbox`
  could not run an xtask, so the feature would exist for this repository alone.
- **A documented recipe only.** AC#2, AC#3 and AC#6 of TASK-78 need tested Rust. Validation, budget
  arithmetic and provenance have to be checked by tests, not by a reader who follows prose.
- **Linking the `rattler` crate.** Measured and declined in ADR-12: about 1.79 MiB of transport per
  vendored crate across roughly 28 crates, plus tokio, reqwest and rayon. Linking it would also
  change `Cargo.lock` and need a relock cycle.
- **Embedding rattler-build in `tools.lock.json`.** rattler-build v0.76.1 is 31.1 MiB as a static
  musl binary, against the 15.0 MiB of `pixi-unpack` that ADR-12 already declined to absorb. The
  catalogue is packed into every transport, so the cost would recur on every restore.

## Upstream status

`crates/pixi-sandbox/pixi.toml` declares `preview = ["pixi-build"]`, because the pixi-build backend
is still a preview feature. That is the reason to generate a recipe for payloads rather than to
extend the backend.

## Shape of the package

- **Install location.** The payload is installed at `$PREFIX/libexec/<name>/`, a prefix-relative
  path fixed by the package name.
- **Launcher.** An optional entry point is exposed as `$PREFIX/bin/<basename>`. It is a POSIX shell
  script that `exec`s the real executable in place, so relative runtime lookups inside the payload
  keep working. A copy of the executable is never made.
- **Empty directories.** A conda package carries files and symlinks only, so an empty source
  directory cannot survive the round trip and is deliberately absent from the installed tree.
  TASK-78 AC#3 is scoped to files and symlinks for this reason. Measured on rattler-build 0.76.1
  (2026-10-13): `info/paths.json` records only `hardlink` and `softlink` entries, and even an
  explicit `$RATTLER_BUILD_PACKAGE_FILES` list drops the directories. Upstream
  `prefix-dev/rattler-build#371` was closed as won't-fix; the suggested workaround is a hidden
  placeholder file, which would leave a non-payload byte in the consumer's prefix and so is not
  used.
- **Native payloads.** A payload is native when it contains an ELF, Mach-O or PE file, detected by
  magic bytes. Declaring such a payload `noarch` is refused. So is an executable whose format does
  not match the platform's operating system. Architecture is not checked.
- **Budgets.** The preflight is conservative. Uncompressed bytes are an upper bound for the `.conda`
  blob. The transport is counted twice, once as the channel package and once as the unpacked prefix
  files, because `record_file` keeps a blob per path. `wrap-payload` does not re-measure the built
  `.conda`. Its exact bytes meet the same budgets when the environment holding it is published: the
  publish pipeline's `doctor --budget-config` phase measures the packed transport before `publish`.
- **Provenance.** `libexec/<name>/pixi-sandbox-payload.json` (schema 1) records the upstream source
  URL, the SHA256 of the fetched artifact, and the payload's file count and byte total. The source
  URL must be `https://`.
- **Channel.** The channel is named once, as a real URL, in the committed manifest and lockfile.
  Local iteration redirects it through `[mirrors]` in the uncommitted `<project>/.pixi/config.toml`.

## Why

The transport already carries whatever pixi resolved into an environment. `pack_environment`
records `.pixi/envs/<name>/pack` verbatim, so a wrapped package flows through the existing pack,
restore and verify path without a change to the transport. A channel package carries one dependency
into the lockfile. The transport branch carries a whole environment. Neither replaces the other.

## Verified assumptions and known limits

- **`[mirrors]` and `file://`.** *Verified 2026-10-12 on pixi 0.81.0.* A `file://` mirror target
  does not work: pixi rewrites the channel to the `file://` URL and then rejects it with "URL
  scheme is not allowed". An `http://` target does work — a consumer whose `pixi.toml` names the
  channel as `https://example.invalid/demo-channel` and whose uncommitted `.pixi/config.toml`
  mirrors that to `http://127.0.0.1:8899` resolved the package, and the written `pixi.lock` kept
  the canonical `https://` URLs rather than the mirror. A direct `file://` entry in
  `[workspace].channels` also resolves, but it is a machine-specific absolute path and so cannot be
  committed. Local iteration therefore serves the local channel over HTTP; the committed channel
  URL stays a real URL.
- **rattler-build recipe semantics.** *Verified 2026-10-12 on rattler-build 0.76.1.* `source:
  path:` and `$RECIPE_DIR` behave as assumed: a payload staged beside `recipe.yaml` built to
  `libexec/<name>/`, and `rattler-build build --output-dir` writes the `.conda` plus its own
  repodata, which is the indexing step this decision relies on.
- **Windows.** The install script is POSIX. `win-64` launchers are refused, and `win-64` payloads
  are not otherwise verified.
- **Chromium.** Playwright's Chromium is the worked example in documentation and tests only. No code
  path names it.

## Where it lives

- **The verb.** `crates/pixi-sandbox/src/commands/wrap_payload.rs` (black-box from `tests/`) over
  the promoted `crates/pixi-sandbox/src/wrap_payload.rs`.
- **The pure rules.** `pixi-sandbox-core::payload`: validation, the budget preflight, recipe
  rendering, provenance, and `verify_artifact`, the `prepare` command's SHA256 gate.
- **The `prepare` command.** `crates/pixi-sandbox/src/prepare.rs` runs it and calls
  `verify_artifact` on the artifact it declared.
- **The build tool.** `crates/pixi-sandbox/src/rattler_build.rs` pins `rattler-build` in code
  (`pins()`) and fetches it through `tool_fetch`; it is never in `tools.lock.json`.
