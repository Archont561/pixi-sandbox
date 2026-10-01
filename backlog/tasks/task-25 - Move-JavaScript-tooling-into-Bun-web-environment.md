---
id: TASK-25
title: Move JavaScript tooling into Bun web environment
status: Done
assignee:
  - '@me'
created_date: '2026-10-01 09:49'
updated_date: '2026-10-01 11:09'
labels:
  - bun
  - tooling
  - windows
milestone: m-0
dependencies: []
references:
  - .knowledge/v1-evolution-plan.md
documentation:
  - backlog/docs/plans/v1-platform-workflow-transport/doc-1
priority: medium
type: chore
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Move npm-compatible development tools such as Biome and Astro to package.json/bun.lock, keep Bun out of the default Windows-compatible Pixi environment, and document the Node.js plus prefix-local Bun fallback for Windows if supported.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Default Pixi environment has no Bun-only Conda dependency
- [x] #2 Web tasks install from the committed Bun lockfile
- [x] #3 Windows CI does not require Bun in the default environment
- [x] #4 Bun fallback is tested or explicitly documented as unsupported
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Move bun and biome out of the default environment into a dedicated web feature. Bun stays conda-pinned there; biome becomes a package.json devDependency so the JS toolchain installs from the committed bun.lock. Put every bun-consuming task under [feature.web.tasks] so pixi runs it in the web environment even when it is a depends-on of the default-env lint aggregate, and let the cross-platform default env resolve without bun.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Verified: bun x needs a node shebang handler that no conda env here provides, so the bun-consuming tasks hand their JS entry point to the bun runtime by path instead.

Done. [feature.docs] is now [feature.web] and holds only bun; the default environment is [rust, utils, sandbox], and "command -v bun" inside it comes back empty (verified with pixi run -e default). Biome and Astro come from the root bun workspace, so "bun install --frozen-lockfile" is what installs them and bun.lock is the only pin. Every bun-consuming task moved under [feature.web.tasks]; pixi runs those in the web environment on its own, so "pixi run lint", "pixi run backlog" and "pixi run docs-build" keep working with no -e flag, and ci.yml / docs.yml just install "default web" up front.

Incidental fix, and the reason the invocation is written the way it is: the npm bin of biome, backlog.md and skills is a "#!/usr/bin/env node" shim, and no conda environment in this repo carries node, so "bun x <tool>" dies with "env: node: No such file or directory". The tasks therefore hand the shim to the bun runtime by path (bun node_modules/<pkg>/<entry>), which needs nothing on PATH. lint-docs-write is the same command with --write, so the lefthook hook and the lint gate cannot drift onto different biome invocations. The old "bun x backlog" form was equally broken here; it just had a conda biome masking the shape of the problem for lint-docs.

Measured, and the thing task-21 needs to know: isolating bun is NOT sufficient to put win-64 in the root [workspace] platforms. Copying the manifest to a scratch dir, adding win-64 and running pixi lock fails with "failed to solve requirements of environment web for platform win-64 / No candidates were found for bun 1.3.11.*" - pixi solves every environment for every platform in the manifest, not just the ones a given consumer selects. So the remaining win-64 blocker is exactly one environment, and unblocking it means moving the web feature into its own manifest (the pattern crates/pixi-sandbox/pixi.toml already uses) rather than deleting bun. AC#4 is answered in the README platform table: the JS toolchain is documented as unsupported on Windows today, with the reason and the workaround (Rust CLI in the pixi env, Node.js/Bun installed outside it).
<!-- SECTION:NOTES:END -->
