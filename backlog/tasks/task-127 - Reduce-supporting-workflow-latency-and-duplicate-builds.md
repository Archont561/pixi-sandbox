---
id: TASK-127
title: Reduce supporting workflow latency and duplicate builds
status: In Progress
assignee: []
created_date: '2026-10-22'
updated_date: '2026-10-22 16:04'
labels:
  - ci
  - workflow
  - optimization
dependencies: []
documentation:
  - backlog/docs/plans/task-127-supporting-workflows.md
priority: high
type: enhancement
ordinal: 127000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Optimize workflows outside the primary CI job by avoiding repeated Rust compilation, reusing trusted same-commit artifacts, and shortening unnecessary dependency chains. Preserve all correctness, security, offline, native-platform and publication gates.

Related work: TASK-126 owns the main CI optimization work; TASK-118/119/120/121/122 cover the earlier generated-workflow improvements. Do not reimplement their completed bootstrap, concurrency or path-filtering changes. This task targets the current source-built publisher, release, airlock, docs, auto-release, starter and relock workflows. Audit the already-small post-release asset-boundary proof, but change it only if measurements justify doing so.

### Observed opportunities (2026-10-22)

- `publish-sandbox.yml` builds a release-mode planner, then builds a target-specific release bootstrap in a separate job. Neither job has a Rust cache action. Planning does not need an optimized binary, but the transported Linux bootstrap still needs the static musl build.
- `release.yml` makes the built-artifact consumer proof depend on the entire five-platform build matrix, including package builds and smoke tests, although that proof only consumes the Linux standalone binary. Start it when that artifact is ready; publication must still wait for every platform and the proof.
- Several automation jobs compile `xtask` without a Rust cache, including auto-release, starter and the built-artifact consumer proof. Since `xtask` depends on the product CLI, even small commands can pull in the native runtime dependency graph.
- `airlock.yml` builds an optimized source planner before the proof matrix. Its nextest archive is already built once and replayed for both proof tiers; retain that reuse and the actual egress-denied tier.
- Main CI and `docs.yml` both build the site. A single trusted build could run every docs quality gate and produce the artifact deployed to Pages. CLI snippet checks must still exercise the matching source revision, not an unrelated released binary.
- The relock guard can reuse Cargo download caches without replacing `pixi lock --check` or `cargo fetch --locked` with weaker validation. Its workflow is generated and must be changed through the template.
- The post-release `consumer-proof.yml` already downloads, checksums and executes the published asset without compiling the repository; avoid adding infrastructure to a cheap job without evidence.

### Implementation order

1. Record baseline timings, cache results and critical paths for the affected workflows.
2. Add missing, correctly scoped Rust caches and make publisher/airlock planning cheaper.
3. Decouple the Linux release consumer proof from unrelated matrix completion.
4. Consolidate the trusted docs build and deployment artifact flow.
5. Evaluate same-commit automation artifacts or a lightweight automation dependency boundary only if remaining compilation costs warrant it. Keep typed, tested automation rather than duplicating policy in workflow shell.

The existing sandbox publisher failure after PR #189 is a separate correctness issue, not evidence of a performance improvement. Repair or establish an explicit failing baseline before comparing successful publisher runs; never make that workflow faster by removing its failing verification.

### Constraints

- Distinguish end-to-end wall time, critical-path time and total runner/billed minutes. Parallelism may improve latency without reducing compute cost.
- Preserve checksum verification, static/standalone validation, all five release platforms, Conda smoke tests, consumer lifecycle proof, transport/file-oracle verification, actual network-denied airlock checks, lock validation and ADR-29 main integration checks.
- Retain fail-closed publication gates and serialized, non-cancelled release/publication operations. Superseded PR checks may remain cancellable.
- Cache keys must distinguish relevant OS, architecture/target, compiler, dependency and build-flag inputs. Do not restore credentials or promote untrusted PR-built executables/artifacts into privileged publishing jobs. Separate instrumented coverage outputs where applicable.
- Artifacts reused across jobs/workflows must match the intended repository, trusted producer, source SHA, platform and build configuration. A shared cache hit alone is not provenance.
- Keep generated workflow changes in their generators, update golden files and regenerate committed renders. Preserve one-command workflow steps and existing narrowly approved bootstrap exceptions.
- Do not trigger a production release, publish/overwrite a transport, alter protections or widen token permissions merely to benchmark. Obtain authorization for live side effects.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Record before/after run links, commit SHAs, runner OS/target, relevant compiler versions, cache hit/miss state and step/job timings. Compare successful cold-cache and warm-cache scenarios where practical, reporting wall time separately from aggregate runner time and actual billed minutes when available. Do not present estimates or failed runs as measured savings.
- [ ] #2 Add correctly scoped Rust caching to source publisher planning/build jobs and the uncached automation jobs identified by the baseline. Document invalidation and trust boundaries; demonstrate warm-cache reuse without accepting stale binaries after relevant source/toolchain/flag changes.
- [ ] #3 Publisher and airlock planning no longer require an optimized/LTO product build solely to produce a matrix. Preserve source-fresh config/override validation, platform mappings and equivalent plan output. Actual transported/released Linux bootstraps remain static musl and pass standalone verification.
- [ ] #4 The release consumer lifecycle proof can start once its matching Linux standalone artifact is available, without waiting for unrelated matrix legs or package smoke tests. The publication job still requires all five platform artifacts, package smoke tests and the consumer proof; missing/failed artifacts or a failed proof prevent publication.
- [ ] #5 Main CI and Pages deployment use one trusted same-revision docs build for the shared path, with frontmatter, links, accessibility and CLI snippet checks still enforced. Direct/manual docs builds retain a safe fallback. Deployments reject missing, mismatched or untrusted artifacts and cannot elevate fork/PR-produced artifacts into privileged execution.
- [x] #6 Evaluate the remaining cost of compiling the full product graph for small xtask commands. Either implement a tested lightweight dependency boundary or verified same-commit automation artifact reuse, or document measured reasons to defer it. Preserve typed automation behavior and avoid duplicating business logic in YAML/shell.
- [x] #7 Optimize relock dependency-download reuse without weakening locked fetches, stale/edited/deleted-lock detection, fork refusal, repair-loop safeguards or the verdict. Change its generator and verify the committed render matches. Keep the small post-release asset-boundary proof unchanged unless measured savings justify a change.
- [ ] #8 Public workflow/generator and automation CLI seams have regression coverage for changed behavior, agreed before implementation. Verify success and refusal paths for artifact identity, missing artifacts, failed matrix legs and publication gating; native/offline guarantees are demonstrated on the real applicable runners, not simulated labels.
- [ ] #9 Actionlint, generated-workflow consistency, repository checks, formatting, lint and tests pass. Attach representative workflow evidence showing the optimizations preserve security and correctness, and document remaining bottlenecks and any latency-versus-cost tradeoffs without promising unmeasured minute savings.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 pixi run --frozen fmt --check
- [x] #2 pixi run --frozen lint
- [x] #3 pixi run --frozen test
- [ ] #4 Relevant workflow executions and before/after measurements linked; no unapproved production release/publication performed
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Approved seams: generated workflow output and public automation CLI. Capture existing run metadata and failing publisher baseline; add scoped caches and cheaper planning; decouple Linux consumer proof from packaging completion without weakening final publication; consolidate trusted same-SHA docs artifacts with manual fallback; optimize generated relock download caching. Use vertical red/green slices, then cleanup and gates. Leave native/live evidence ACs open until actually demonstrated.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-22: user approved workflow/generator and automation CLI seams. No production release/publication authorized for benchmarking. TASK128 is implemented separately.

2026-10-22 implementation: scoped caches added to source planner/static publisher and automation jobs; source plan dev builds preserve config and override JSON versus release. Linux binary upload now unblocks consumer proof independently of Linux packaging and the remaining four native legs; final release still requires every build/package/proof and completeness checks. Main CI quality-checks one SHA-named docs artifact; Pages adopts only successful same-repository main-push matching-SHA CI output without executing artifact code, or uses a main-only manual build with all quality gates. Relock generator caches source downloads in guard and repair without caching CARGO_HOME credentials/config/target binaries; committed render regenerated. Post-release consumer-proof.yml unchanged. Ledger contains baseline run links/SHAs/job/step timings and explicit unknown compiler/cache/billing fields; failed publisher baseline retained, not called a performance sample. Lightweight crate split deferred based on documented measured historical costs and lack of comparable current-native warm/cold data.

Validation: full lint 11/11, no cache; focused relock generator 7 passed; workflow public CLI 8 passed including refusal paths. Observed red/green for the new cache, CLI contract, optional failed platform, repair cache and main-only fallback cases. Full uncached workspace before last manual-branch regression 1174 passed/2 skipped; final hook run pending. Local docs build/frontmatter/links/a11y/snippets passed: 29 pages, 62 commands, 123 flags. No production release, sandbox publication or starter push performed. TASK127 remains In Progress: AC1/2 need same-source successful cold/warm/cache-invalidation/billing evidence; native/static built-artifact proof, actual artifact overlap, Pages reuse and representative after-run evidence for AC3/4/5/8/9 remain to be collected. Structural CLI tests are not native-runner proof. TASK125 native-artifact obligations are unchanged.

Final current-tree pre-push proof: all 5 normal hooks passed (main integration, workspace Clippy, repository consistency, generated workflows, full uncached tests). Test baseline now 1175 passed/2 skipped: 54 git +295 core +575 product +251 xtask. No hook bypass. Evidence artifact name uses run ID/attempt so artifacts-only dispatches from branch names containing slashes remain uploadable. Safe PR CI/airlock evidence pending; do not close task on these local checks alone.
<!-- SECTION:NOTES:END -->
