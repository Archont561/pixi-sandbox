---
id: ADR-32
title: A transport branch is one bundle for one platform, and its payload stays self-contained
date: '2026-10-23'
status: accepted
---

## Decision

Codifies the branch topology the shipped producer already implements, so later work cannot
reopen it by accident:

1. **One branch per `(bundle, platform)` pair** — `{branch_prefix}/{bundle}-{platform}`, e.g.
   `sandbox/developer-linux-64`. A bundle is the only fan-out axis. Two bundles that would
   resolve to the same branch name are refused by config validation.
2. **One manifest per branch, one platform per manifest.** `Manifest.platform` stays a single
   value and tool entries stay platform-scoped (`tools/linux-64/pixi`). No branch carries a
   second platform.
3. **The payload stays inside the branch it describes.** Environments, embedded tools, vendor
   output and the per-file oracle all live under `.pixi-sandbox/` in the same commit, so
   `restore` verifies one manifest before writing anything (invariant 1) and a restore can never
   consume a half-transported pair of branches.
4. **Vendor output is not split into its own branch.** Moving it out would break three
   invariants at once: `restore.sh` derives exactly one branch from the config, verification is a
   single-manifest operation, and the budget/provenance model (`cargo_lock_sha256`, transport
   ceilings) describes one payload. `.pixi-sandbox.toml` states the same rule for operators:
   split a growing transport with another `[[bundle]]`, never by moving payloads outside the
   branch.
5. **A smaller payload is a bundle decision, not a branch surgery.** `cargo_vendor` is already a
   per-bundle override of the global policy, so a bundle that ships only Python or docs can stop
   carrying a vendor tree; a genuinely different lockfile is a different bundle.

## Why (measured context)

- Git stores identical blobs once per repository, so a second platform branch sharing the same
  `Cargo.lock` and the same vendored tree adds checkout and restore work, not new objects. A
  vendor branch would therefore save storage that is already saved, while adding a second thing
  to verify, transfer and keep in lockstep.
- The current published transport is 1,241 MiB across the whole branch tree, of which the vendor
  is 734 MiB (59%) — the largest single contributor and the one a per-bundle `cargo_vendor`
  switch removes first.
- Platform-specific payload is the *toolchain*, and it already comes from the per-platform Conda
  environment. Source distributions (Rust crates, Go modules) are byte-identical across targets;
  NuGet and Maven artifacts are mixed and are handled per provider (TASK-130).

## Consequences

- A consumer who needs several platforms restores several branches, each self-verifying; the
  airlock transfer names the branches the config declares.
- One branch can never satisfy "restore the same project on two platforms"; that is by design,
  and the error is a config-level one (declare the platform in the bundle), not a runtime one.
- Nested Pixi workspaces (TASK-129) extend the payload *inside* one branch under
  `.pixi-sandbox/envs/workspaces/<slug>/<environment>/pack`, so the one-manifest rule survives.
- Adding a multi-platform manifest would supersede this record explicitly; it must not be
  introduced quietly by growing `Manifest.platform` into a list.
