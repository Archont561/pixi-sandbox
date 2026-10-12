---
id: TASK-78
title: Package arbitrary prebuilt binary payloads as installable conda packages
status: To Do
assignee: []
created_date: '2026-10-06 21:01'
updated_date: '2026-10-11 21:48'
labels:
  - packaging
  - conda
  - channels
  - airlock
  - consumer
dependencies: []
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/115'
  - crates/pixi-sandbox/pixi.toml
  - crates/pixi-sandbox/src/commands/pack.rs
  - crates/pixi-sandbox-core/src/transport_budget.rs
  - crates/xtask/src/smoke.rs
  - backlog/docs/adr/README.md
priority: high
type: feature
ordinal: 78000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Pixi channels are a good transport into restricted environments, but some dependencies ship only as prebuilt binary trees rather than source packages. Issue #115 names the concrete case: Playwright browser builds, an executable plus a few hundred MB of shared libraries, resources, locales and data files that normally come from a vendor CDN. Where Git and package APIs are reachable but arbitrary downloads are not, the artifact has to be fetched on a connected host and republished through a channel.

Make an arbitrary prebuilt file or directory a first-class package payload: fetch it connected, wrap it into a platform-correct conda package, publish it to a channel, and let an air-gapped project run pixi add and execute it. The feature must be generic; browsers are the example, not the subject.

This repository is already most of the way there in machinery and none of the way there in generality. crates/pixi-sandbox/pixi.toml builds a real conda package for five platforms, publish-conda uploads the set, xtask check-conda-platforms refuses a release where a platform contributed nothing, and xtask smoke-conda-package installs the built package and runs it. What is missing is a payload path: that manifest uses the pixi-build-rust backend, which compiles a crate, and nothing in the repository wraps bytes that were built elsewhere.

Shape, settled 2026-10-11 and recorded as the proposed `ADR-28` file (`backlog/docs/adr/ADR-28 - *.md`) that AC#1 requires: a new `pixi sandbox wrap-payload` verb in the shipped binary. An xtask subcommand was considered and rejected because `crates/xtask` is a workspace member that appears in no release artifact — the standalone assets are `pixi-sandbox-<target>` (`crates/xtask/src/release_assets.rs`) and the conda package builds only `crates/pixi-sandbox` — so a consumer who installed `pixi` and `pixi-sandbox` could not run it, and the feature would exist for this repository alone. The usual objection to a new verb, a connected fetch inside the offline binary, does not apply here: a config-declared `prepare` command does the fetching and pixi-sandbox only executes it, so `crates/pixi-sandbox/src/github.rs` stays the only socket-touching file.

`rattler-build` is invoked as a subprocess and never linked. Cargo.lock is therefore untouched: no relock cycle and no vendored growth, which is the arithmetic ADR-12 turned down (this repository's transport costs ~1.79 MiB per vendored crate, and rattler pulls a ~28-crate tree plus tokio/reqwest/rayon). It is fetched on demand into the tools cache by the machinery behind `pixi sandbox tools` and `tool_fetch.rs`, and it must not enter the embedded `tools.lock.json`, which is packed into every transport — rattler-build v0.76.1 is 31.1 MiB static musl, more than double the 15.0 MiB of `pixi-unpack` that ADR-12 declined to absorb. `pixi-build` remains an upstream preview feature (`preview = ["pixi-build"]` in that manifest), which is a reason to generate a recipe rather than extend the backend.

Nothing changes on the transport side, and that is the point. `pack_environment` records `.pixi/envs/<name>/pack` verbatim (`crates/pixi-sandbox/src/commands/pack.rs`), so any conda package pixi resolved into the prefix is already carried, and `restore` materialises and verifies it without ever re-solving (invariant 5). Pack, publish, restore, doctor and the manifest schema are untouched; the airlock resolves no channel at all. Note that `record_file` (`crates/pixi-sandbox-core/src/shard.rs`) hashes each path independently and keeps a blob per path, so a payload pixi hardlinked into the prefix occupies its full size in the transport — the sizing AC#6 has to judge.

Channel resolution is the one part that is not local. Pixi requires **absolute** `file://` paths for filesystem channels, and `pixi.lock` records the channel URL per environment plus the full URL per package, so a machine-specific path cannot be committed and a frozen install reads the URLs already in the lock rather than the manifest's channel list. The channel is therefore named once, as a real URL, in the committed manifest and lockfile; local iteration redirects it through `[mirrors]` in `<project>/.pixi/config.toml`, which `.gitignore` already excludes, and per-invocation overrides exist as `PIXI_CONFIG_FILE` and `PIXI_NO_CONFIG`. Pixi-sandbox never rewrites the consumer's `pixi.toml` (ADR-17). Open and worth testing before any code: whether `[mirrors]` accepts a `file://` target at all, since it is documented for HTTP mirrors — if it does not, serve the local channel over HTTP for iteration.

Keep the boundary against the sandbox transport explicit. The transport carries a whole environment on an orphan branch; this carries one dependency through a channel so it participates in the lockfile and in pixi add. They are not substitutes, and the documentation has to say which to reach for.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A reviewed decision records the implementation shape before code lands: a new `pixi sandbox wrap-payload` verb. The alternatives are excluded with reasons rather than left open — an xtask subcommand, because xtask ships in no release artifact and a consumer could not run it; a documented-recipe-only answer, because AC#2, AC#3 and AC#6 need tested Rust. It names the upstream preview status of pixi-build, how rattler-build is fetched and why it is never linked or embedded, and confirms no new vendored crate is added without a relock cycle; the decision goes in a new `ADR-N` file under backlog/docs/adr/ tagged `decision` (ADR-28 at the time of writing); the README there is only the anchor map.
- [ ] #2 A payload directory or single file becomes a conda package for a named platform subdir such as linux-64 or linux-aarch64, built from inputs already on disk with no network access at build time; a native payload declared as noarch is refused with a named error rather than silently mispackaged.
- [ ] #3 Executable bits, symlinks including relative and dangling ones, nested directories and empty directories survive the round trip, proven by packing a generated fixture tree, installing it into a temporary prefix, and comparing modes and link targets entry by entry.
- [ ] #4 The payload installs at a deterministic prefix-relative location, and an optional entry point can be exposed on PATH as a launcher that resolves the real executable inside the payload rather than a copy of it, so relative runtime lookups inside the payload still work.
- [ ] #5 An air-gapped consumer can run pixi add against a channel carrying the package, install it from the lockfile with no network, and execute the exposed entry point successfully in a clean temporary prefix.
- [ ] #6 Large payloads are handled deliberately: a payload that exceeds the configured blob or transport budget fails with the budget name, the measured size and a remediation, instead of producing a package the channel will reject at upload time.
- [ ] #7 The published package records the provenance of what was wrapped, at minimum the upstream source URL and the SHA256 of the fetched artifact, so an auditor can tell which external bytes entered the channel and verify them independently.
- [ ] #8 No payload byte is committed to Git. Tests build their fixture tree in a tempdir and never point at this checkout or a real HOME per ADR-10, and the documented workflow fetches on a connected host and publishes rather than vendoring the artifact into the repository.
- [ ] #9 Nothing in the implementation is browser-specific or Playwright-specific. Firefox and Playwright appear only as a worked example in documentation and tests; the code paths take a payload, a name, a version and a platform.
- [ ] #10 A user-facing guide documents the connected fetch, build and publish steps and the air-gapped pixi add step, and states the boundary against the sandbox transport: a channel package carries one dependency into the lockfile, the transport branch carries a whole environment, and neither replaces the other. It also documents the ordering the flow actually requires (install the driver, wrap, add the package, install again) and that `pack --self-bin` needs the standalone release binary rather than a `pixi global install` trampoline (issue #81).
- [ ] #11 The channel is named once, as a URL every connected environment resolves identically, in the committed manifest and lockfile. No committed or generated file carries a machine-specific absolute path; local iteration redirects the channel through `[mirrors]` in the uncommitted `<project>/.pixi/config.toml`; and pixi-sandbox never rewrites the consumer's `pixi.toml` (ADR-17). Whether `[mirrors]` accepts a `file://` target is verified, not assumed, before the design depends on it.
- [ ] #12 The build tool is fetched on demand into the tools cache and never enters the embedded tool catalogue or a published transport: `tools.lock.json` and the packed payload are unchanged by a wrap, a missing tool fails with a named remedy rather than a bare spawn error, and adding the capability grows no vendored crate.
- [ ] #13 pixi-sandbox never downloads the payload. A config-declared `prepare` command does, and its declared source SHA256 is checked against what it produced, so a changed upstream fails the run instead of packing different bytes behind a green pipeline.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
<!-- DOD:END -->
