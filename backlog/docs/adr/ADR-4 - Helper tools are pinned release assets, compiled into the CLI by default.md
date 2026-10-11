---
id: ADR-4
title: "Helper tools are pinned release assets, compiled into the CLI by default"
status: accepted
tags:
  - decision
---

# ADR-4 — Helper tools are pinned release assets, compiled into the CLI by default

**Decision.** The canonical `crates/pixi-sandbox-core/assets/tools.lock.json` pins version + URL
template + sha256 + linkage per platform and is compiled into every `pixi-sandbox` release.
`pack --fetch-tools` (and release-driven CI) download → verify → cache → execute without a
sidecar file. `--tools-lock PATH` is an explicit, reviewable override for an organisation mirror
or emergency pin change. Nothing is installed system-wide and nothing is taken from `PATH`
unless the caller opts out. Linux embedded tools must be `static`; the packer and `doctor` refuse
ELF `dynamic` ones. Native macOS/Windows binaries are classified as `system` and require their
own platform proof.

**Why.** An installed CLI must remain usable without a loose JSON file beside it, while the
airlock cannot install anything. A tool discovered on `PATH` makes the build machine and the
airlock differ in ways nobody can see. Data embedded at compile time keeps defaults reproducible;
an override keeps private mirrors and urgent security response possible.

**Evidence ✅.** The `~/.pixi/bin` shims that `pixi global install` writes are ~766 KiB
trampolines that exec a dynamically linked binary inside their own prefix: they restore
perfectly and then fail on the airlock with a missing `trampoline_configuration`. The static
musl assets (14.7 MiB `pixi-pack`, 15.7 MiB `pixi-unpack` v0.7.11) run anywhere; a
PATH-stripped run of `--fetch-tools` (download → sha256 → cache → execute) was verified with
the cache empty.

**What would change it.** A project that requires a tool version per repository can use the
explicit override; upstream publishing a static asset for a new platform requires a reviewed
embedded catalogue update and a new CLI release. `tools update` remains the future helper for
preparing that review diff.
