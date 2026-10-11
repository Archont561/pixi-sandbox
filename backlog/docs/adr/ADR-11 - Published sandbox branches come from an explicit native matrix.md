---
id: ADR-11
title: "Published sandbox branches come from an explicit native matrix"
status: accepted
tags:
  - decision
---

# ADR-11 — Published sandbox branches come from an explicit native matrix

**Decision.** A project declares publishable environment bundles in `.pixi-sandbox.toml`.
`pixi-sandbox plan --json` validates that declaration and emits one job per
(bundle × platform). The reusable `publish-sandboxes.yml` workflow downloads a
checksum-verified standalone `pixi-sandbox` release and runs each job on its matching native
runner; it never silently cross-packs a platform.

**Why.** A Pixi manifest commonly contains environments that are experimental, CI-only, or not
supported on every platform. Packing "all environments" is an unsafe implicit publish policy.
One GitHub Actions job has one OS/architecture, so a custom action cannot honestly create and
airlock-test a Mac or Windows payload from an unrelated runner. Explicit bundles, branch names,
and runners make that reviewable.

**Evidence.** The Linux real transport path is proven under a network namespace. macOS and
Windows are not yet proven, so the planner requires a named native runner and complete compiled
helper pins; the publish action compares its detected platform with the requested platform before
any bytes are packed.

**What would change it.** A measured, supported cross-packing implementation for
`pixi-pack`/`pixi-unpack` plus equivalent target execution tests could relax the native-runner
constraint. Until then, an action that loops platform labels on one runner is only pretending to
provide multi-platform sandboxes.
