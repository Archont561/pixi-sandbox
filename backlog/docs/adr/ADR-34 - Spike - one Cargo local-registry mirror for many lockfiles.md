---
id: ADR-34
title: 'Spike — one Cargo local-registry mirror for many lockfiles'
date: '2026-10-25'
status: experimental
tags:
  - spike
  - cargo
  - offline
---

# ADR-34 — Spike: one Cargo local-registry mirror for many lockfiles

**Status: experimental.** This record describes a timeboxed spike (TASK-137), not an accepted
product direction. Nothing in the spike is reachable from the CLI, `pack`, `restore` or the
manifest schema; ADR-6 and ADR-33 remain in force. A promoted design requires a new accepted ADR
that answers the open questions below and amends or reaffirms ADR-33 explicitly. m-2 already
anticipates this question: its milestone record keeps the Cargo vendor contract unchanged
"unless the spike proves an amendment".

## Context

ADR-33 gives every project that owns a `Cargo.lock` its own `cargo vendor` tree in one
transport. Shared crates are counted, checked out and verified twice by explicit decision,
because `cargo vendor` keys on one lockfile and merging trees would need a new conflict
policy. The owner proposal recorded in `CONTEXT.md` (2026-10-25) approaches dependency
airlifting differently: instead of per-lock source trees, ship **one read-only registry in
Cargo's native local-registry layout** that any number of lockfiles resolve against through
source replacement. The long-term proposal spans many ecosystems; this spike isolates the
single claim that can be tested cheaply: does the mirror shape work for Cargo at all, and what
does it cost against the ADR-33 baseline?

## What the spike builds

`crates/pixi-sandbox/src/mirror.rs` (spike module, no product wiring):

1. Reads one or more `Cargo.lock` files and takes their registry packages, skipping
   path/workspace members and refusing non-registry sources.
2. Copies each distinct `(name, version)` `.crate` **once** out of a connected-side
   `CARGO_HOME` registry cache (populated by ordinary `cargo fetch`), hashing during the copy
   and refusing any file whose SHA-256 disagrees with the lock's `checksum`.
3. Renders Cargo's local-registry index: length-sharded index files with one JSON line per
   version, dependencies read from each `.crate`'s own `Cargo.toml` (normal, dev, build and
   target-specific; path/git deps excluded), plus the lock-attested `cksum`.
4. Writes a `config.toml` whose `[source.crates-io] replace-with` points at the on-disk
   registry — no daemon, no TLS, no custom certificate authority.

The ignored gate `tests/mirror_offline.rs` (CI step `test-mirror-offline`, same
`sudo unshare -n` namespace shape as `test-vendor-offline`) builds two projects whose locks
share `same-file`, constructs the mirror, then builds both with `cargo build --offline
--locked` under egress denial using a CARGO_HOME that has never held a crate. A cold negative
control removes one `.crate` and asserts the offline build fails. A measurement block prints
mirror bytes against the two-tree `cargo vendor` baseline.

## What would make the spike succeed or fail

It succeeds as a *spike* — evidence, not product — if the runner gate shows:

- both lockfiles building offline against the single mirror with egress denied;
- the shared pair stored once while both builds link it;
- the negative control failing with a fresh cache (no warm-cache theatre);
- a recorded byte/count comparison against two ADR-33 vendor trees.

It falsifies the direction if Cargo's local-registry source replacement cannot serve two
independent locks offline, if index rendering needs information the `.crate`/lock pair does
not contain, or if the measurement shows no meaningful checkout/verify advantage once the
transport packaging is included.

## Why local registry, not a proxy

An intercepting HTTPS proxy would need a custom root CA on every consumer, per-tool proxy
semantics and a live daemon at restore time, contradicting invariant 6 (restore needs only
embedded static binaries and no network). A local registry is static files consumed through
Cargo's documented source-replacement mechanism, so it preserves verify-before-write and the
no-daemon airlock model.

## Open questions a promotion must answer

1. **Transport integration.** Does the mirror ride inside the existing ADR-32 branch (a new
   payload kind with its own manifest oracle and ceilings), or does it need the separate
   branch topology ADR-32 currently forbids for vendor output?
2. **Scope and retention.** One global mirror per ecosystem, or per bundle? Append-only union
   with a garbage-collection policy, or prune to the union of current lock closures? The
   spike takes the second behavior implicitly by building from locks each time.
3. **Lock refresh.** The connected-side solve still exists: the future survey bot must
   resolve closures with egress before downloading them. Offline lock *refresh* against
   shipped index metadata is the capability this model promises but the spike does not test.
4. **Non-registry sources and renamed edges.** Git and path dependencies are deliberately
   excluded; either name them unsupported in the contract or design a separate generic-blob
   mechanism. The index writer also does not yet emit the `package` field for renamed
   registry dependencies, so a promotion needs a rename test case before any consumer uses
   one (the spike's fixture crates have no renamed edges).
5. **Parity evidence.** The spike measures two small fixture projects; a promotion needs the
   comparison repeated on a real multi-crate graph and on at least one lock-bump cycle.

## Consequences while experimental

- Zero impact on existing transports: no schema change, no CLI surface, no pack/restore call
  site; bundles built without the spike are byte-identical.
- One new direct dependency (`flate2`, already present transitively) and one new changes-gated
  CI step that needs egress and `unshare`, matching the existing offline-gate cost pattern.
- The polyglot adapter contract (closure resolver, static layout, offline client config,
  verification oracle) is first exercised here; later ecosystems would implement the same
  four pieces rather than reuse Cargo-specific code.
