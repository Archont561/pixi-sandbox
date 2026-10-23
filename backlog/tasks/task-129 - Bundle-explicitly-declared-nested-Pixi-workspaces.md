---
id: TASK-129
title: Bundle explicitly declared nested Pixi workspaces
status: To Do
assignee: []
created_date: '2026-10-23 16:57'
updated_date: '2026-10-23 18:29'
labels:
  - pack
  - restore
  - transport
  - config
  - plan
  - offline
dependencies: []
references:
  - crates/pixi-sandbox-core/src/sandbox_config.rs
  - crates/pixi-sandbox-core/src/manifest.rs
  - crates/pixi-sandbox/src/commands/plan.rs
  - crates/pixi-sandbox/src/commands/pack.rs
  - crates/pixi-sandbox/src/commands/restore.rs
  - crates/pixi-sandbox/src/pixi_lock.rs
  - docs/src/content/docs/guides/configure-bundles.mdx
documentation:
  - backlog/docs/plans/task-129-nested-workspaces.md
priority: high
type: feature
ordinal: 129000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Opt-in bundling of nested Pixi workspaces (GitHub issue #194; issue #193 is a duplicate report of the same request).

**Problem.** A sandbox bundle declares environments of the *root* `pixi.toml` only. A repository whose root tasks delegate to subprojects (`pixi run --manifest-path python/pixi.toml …`) restores the root environment and then re-solves and fetches inside the subproject, which cannot work on an airlocked machine. Reported against a real project whose `developer` bundle declares `environments = ["default"]` while its root test/lint tasks delegate to `python/`, `typescript/` and `go/` manifests: after an offline restore the Python task tried to fetch conda-forge packages and stopped.

**Requested outcome.** Declare the root and selected nested workspaces once, pack them, restore on a machine with no channel access, and run tasks that use those restored environments without a solve or network fetch. Existing root-only bundles keep their current behaviour.

**Constraints from the report.** Scanning every `pixi.toml` is explicitly not wanted: nested projects may carry conflicting versions, channels, platforms or intentionally separate environments. The inclusion list is explicit and reviewed; the pack plan must show which manifests, environments and resolved package sets will be included, and restore must either materialise them or clearly report the conflicts that prevent it.

**Non-goals.** No automatic workspace discovery, no flattening of nested environments into one prefix, no change to the schema-3 transport guarantees (verify-before-write, per-file oracle, budgets), and no non-Conda ecosystem vendoring (that is TASK-130, GitHub issue #195).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Seams are agreed with the user before implementation: the config surface, the plan text/--json shape, and the public library boundaries the tests will use (fixture projects and tempdirs, never this repository — ADR-10).
- [ ] #2 A bundle can declare explicit nested manifests with an optional environment selection; an unknown or escaping path, a duplicate target, a nested manifest without a lockfile, or one already claimed by another bundle is refused with a named diagnostic, and configs without the new declaration keep parsing unchanged under schema 1.
- [ ] #3 plan (text and --json) names every declared manifest, its selected environments and resolved package set, and refuses conflicts (differing channels, platforms or package identities) instead of silently merging them; the rendered workflow still derives its matrix from validated plan output.
- [ ] #4 pack carries the declared nested environments into the transport with per-environment identity, so the manifest distinguishes the root prefix from each nested prefix without weakening budget ceilings, the per-file oracle or byte verification.
- [ ] #5 restore materialises every declared nested environment into its own prefix with Pixi markers and staged-oracle verification, keeps verify-before-write (invariant 1) and never leaves a half-populated nested environment; the failure policy for one failed nested environment is the agreed one and is tested.
- [ ] #6 Offline lifecycle is proven on fixtures locally: a nested workspace packs, restores and runs a delegated task inside a denied-egress namespace with no solve or channel fetch. Native-runner airlock replay is claimed only from the CI gate that actually ran it, never inferred from fixture tests.
- [ ] #7 Documentation and generated artifacts are updated through their generators (configure-bundles guide, CLI reference, transport layout notes, golden workflow files), and the full suite's pass count rises or holds with fmt, lint and test green.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Focused fixture/tempdir tests and the offline e2e lifecycle pass, with the repository test baseline recorded before and after and never reduced.
- [ ] #2 fmt, lint and test are green; generated workflow/branch artifacts are regenerated from their generators and the plan output is validated by its own check.
- [ ] #3 The task stays open until every acceptance criterion has evidence; anything this machine cannot prove (native-runner airlock tier, real consumer lifecycle) is named as an open item rather than checked.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Slice order, each locally provable before the next starts:

1. Agreement (AC#1): the config surface, the plan/--json shape and the public test seams. Open decision: one bundle-level list of explicit manifest paths with an optional environment selection (recommended) versus per-bundle named workspace entries.
2. Config + validation (AC#2): parsing, refusals and the unchanged schema-1 shape, as library tests over tempdir fixtures.
3. Plan reporting (AC#3): text and JSON output over a fixture repository, including the conflict refusals; the rendered workflow keeps consuming validated plan output.
4. Transport (AC#4): per-nested-environment identity in the manifest and packing, without touching budget or oracle policy.
5. Restore (AC#5): materialise each nested prefix with markers and staged-oracle verification, including the agreed failure policy.
6. Offline proof and docs (AC#6, AC#7): denied-egress fixture lifecycle, documentation, regenerated artifacts, full gates.

Evidence boundary: the local denied-egress fixture tier is no substitute for a native-runner airlock run; that claim waits for the CI gate that actually executes it.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design recorded in `backlog/docs/plans/task-129-nested-workspaces.md`, with ADR-32 fixing the branch topology. No code written yet; the slice order above is unchanged.

Settled from the code, not from intent: a nested workspace is another project root for `native_pack::preflight/write`, which already read `<project>/pixi.lock`; blobs stay namespaced per environment, so nested ones take `.pixi-sandbox/envs/workspaces/<slug>/<env>/pack/…` (a fixed, checkout-safe segment — `python:default` inside the branch would be illegal on NTFS) while root paths stay byte-identical. Each nested environment restores into its own project `.pixi/envs/<env>`, the layout `pixi run --manifest-path …` reads. The per-file oracle, markers, budget accounting and verify-before-write keep working per environment; `ensure_destinations_available` must widen to every declared destination so a refused restore still leaves the project untouched. CLI env ids become `<workspace>/<env>`; a bare name still means the root environment. `plan --json` gains `workspaces[]` and stays the workflow matrix source, so `publish-sandbox.yml` keeps one-line steps. `plan` must also refuse a nested `pixi.toml` that is a member of the root Pixi workspace, since it would duplicate a prefix.

Open decisions before slice 1: (1) schema — recommended bump to 4 only when nested workspaces are declared, so an older binary refuses loudly instead of restoring the root envs and reporting success; (2) restore failure policy — recommended keeping today's sequential abort with staged packs retained rather than inventing an all-or-nothing transaction; (3) per-workspace Cargo vendor — `Manifest.vendor` is a single `Option<Vendor>`, so a nested Rust workspace with its own `Cargo.lock` needs either a per-workspace vendor entry, a single repository-wide lockfile, or an explicit deferral.
<!-- SECTION:NOTES:END -->
