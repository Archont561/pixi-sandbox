---
id: TASK-118
title: Optimize publish workflow bootstrap to eliminate double installation
status: In Progress
assignee: []
created_date: '2026-10-22 07:34'
updated_date: '2026-10-22 07:49'
labels:
  - ci
  - workflow
  - optimization
dependencies: []
priority: high
type: enhancement
ordinal: 118000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Issue #179 identifies double installation in publish-sandbox.yml publish job:
1. pixi global install pixi-sandbox==VERSION from conda
2. pixi-sandbox fetch-release --dest BIN --version VERSION which downloads standalone binary

Current design is intentional: conda install provides the tool that does checksum-verified fetch of static binary (ADR-4 static linkage, TASK-76 bootstrap exception). The standalone binary is then embedded as transport bootstrap and must be static.

Evaluate:
- Whether conda binary can be used directly as pipeline entrypoint and self-bin when static linkage is already satisfied (e.g. linux musl conda package may already be static, or dynamic is acceptable for connected publish job but not for embedded bootstrap)
- Whether fetch-release can be made to not require a prior pixi-sandbox install (e.g. curl + SHA256SUMS verification in shell, or a small standalone fetcher)
- Measure time saved: conda solve + install vs curl download, and billing minutes
- Ensure security: checksum verification must remain, no unverified execution
- Ensure static linkage guarantee for embedded bootstrap is preserved

Reference: https://github.com/Archont561/pixi-sandbox/issues/179 section 1
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Measure current publish job time breakdown: setup-pixi, conda install, fetch-release, pipeline
- [x] #2 Document why double install exists: static linkage, checksum verification, ADR-4, TASK-76
- [x] #3 Propose one bootstrap path that preserves verification and static guarantee while reducing minutes
- [ ] #4 Implement chosen path in generated workflow template with tests and golden file update
- [ ] #5 Verify actionlint, check-repository, and consumer-proof still pass
<!-- AC:END -->

## Implementation Notes

### Measurements (consumer template, from Archont561/pixi-sandbox-starter run 38804893239)

plan job (linux-64, default env):
- setup-pixi: 1s (20:48:24 -> 20:48:24)
- Install pixi-sandbox CLI (conda): 2s (20:48:24 -> 20:48:26)
- plan --json: <1s
Total plan: ~4s

publish job:
- setup-pixi: 1s (20:48:36 -> 20:48:37)
- Install pixi-sandbox CLI (conda): 10s (20:48:37 -> 20:48:47) — `pixi global install --channel ... pixi-sandbox==VERSION`
- Fetch pinned release: <1s (20:48:47 -> 20:48:47) — downloads standalone static binary + SHA256 verification via Rust
- Pack, verify, publish: 15s (20:48:47 -> 20:49:02)
- Upload artifact: 1s
Total publish: ~31s, bootstrap overhead 10s (32%)

For this repo's source-built variant (run 39132270578):
- plan: Build source planner 56s (13:36:22 -> 13:37:18)
- publish: Build source 74s + pipeline 99s = ~3min

### Why double install exists

- ADR-4 static linkage: transport bootstrap (self-bin) must be static musl binary to run on any host without glibc. Conda package for linux-64 is built against glibc (conda-forge), not guaranteed static. Standalone release is explicitly static (4.6 MiB).
- TASK-76 bootstrap exception: only one step may download pinned binary via checksum-verified command. That command currently is `pixi-sandbox fetch-release` which itself requires pixi-sandbox binary to run. Hence conda install provides the verifier that fetches the static binary.
- Security: checksum verification must remain, no unverified execution. Current Rust implementation verifies SHA256SUMS from GitHub release, same file that signs all assets.

### Proposed path (preserves verification + static guarantee, reduces minutes)

Replace conda install + Rust fetch-release with direct curl + sha256sum verification in shell:

1. setup-pixi remains (provides pixi for `pipeline` which runs `pixi install --frozen` per env)
2. New step "Fetch pinned pixi-sandbox release" becomes:
   ```bash
   curl -fsSL -o $BIN https://github.com/Archont561/pixi-sandbox/releases/download/v$VERSION/pixi-sandbox-<platform>
   curl -fsSL -o $BIN.sha256 https://github.com/Archont561/pixi-sandbox/releases/download/v$VERSION/SHA256SUMS
   grep <asset> $BIN.sha256 | sha256sum -c -
   chmod +x $BIN
   ```
   Verification same as Rust: SHA256SUMS from release, asset name resolved per runner (runner.os mapping). No conda install needed.

Savings:
- Consumer: plan 2s + publish 10s = 12s saved per workflow run (~30% of bootstrap, ~15% of total publish workflow)
- This repo: plan build 56s could be replaced by same curl if we make plan job also use standalone binary (currently builds from source for dogfooding, but consumer path could use curl)

Tradeoffs:
- Curl approach loses Rust's robust asset resolution (currently uses Platform::asset_name), but can be reimplemented in shell with case statement (old approach) or small Python. However TASK-76 explicitly removed shell case statements to avoid literal duplication. We need to keep asset resolution in Rust? Alternative: keep Rust binary but make it not require conda install — e.g., distribute a tiny standalone fetcher as shell script that is itself checksum-verified via cosign? Or make `fetch-release` available as standalone shell script in repo that does not need pixi-sandbox install.
- Simplest: keep current double install for now, but document that future optimization is to make fetch-release a curl-based step, and implement it as part of TASK-118.

Decision: Keep conda install for now for safety, but propose curl+sha256 verification as next step. If implemented, must update `crates/pixi-sandbox/src/generated/github_workflow.rs` to replace "Install pixi-sandbox CLI" step with direct curl, and update tests `the_render_embeds_no_release_asset_names` (currently asserts no asset names embedded) — curl approach would embed asset names again, requiring ADR update.

Next action: implement curl-based fetcher in separate PR, with new ADR explaining why asset name literals are acceptable when paired with SHA256 verification.

### Billing impact

GitHub Actions bills per minute. Saving 12s per run = 0.2 minutes. For a repo with 100 pushes/month, saves 20 minutes/month. For starter (1 env, 1 platform), total workflow ~35s, saving 12s = 34% reduction. For multi-platform (e.g. 4 platforms), saving 10s per publish matrix job = 40s total per run.
