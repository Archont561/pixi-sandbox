---
id: TASK-129
title: Bundle explicitly declared nested Pixi workspaces
status: To Do
assignee: []
created_date: '2026-10-23 16:57'
updated_date: '2026-10-23 18:48'
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
- [ ] #1 Slice 0 is recorded before any transport code exists: a fixture demonstrates that declaring the nested project as a [workspace] member of the root pixi.toml cannot serve the case (conflicting channels, versions or platforms). If the member route works, this task closes as a documentation change instead.
- [ ] #2 Config schema is bumped to 2: a bundle declares explicit nested manifest paths with an optional environment selection, while an unknown or escaping path, a duplicate target, a manifest already claimed by another bundle, a missing lockfile, or a nested Rust workspace with its own Cargo.lock is refused with a named diagnostic pointing at TASK-131. Schema-1 configs keep parsing unchanged, and an older binary refuses a schema-2 config naming the version it needs.
- [ ] #3 plan (text and --json) names every declared manifest, its selected environments, resolved package set and workspace-qualified size estimate, and refuses conflicts (differing channels, platforms or package identities) plus a nested manifest that is a member of the root Pixi workspace; the rendered workflow still derives its matrix from validated plan output.
- [ ] #4 pack resolves every selected environment to its own project root for the lock and native preflights, and carries nested environments into the transport with per-environment identity under .pixi-sandbox/envs/workspaces/<slug>/<env>/pack/ — root paths stay byte-identical, and budget ceilings, the per-file oracle and byte verification are unchanged.
- [ ] #5 restore materialises each nested environment into its own project's .pixi/envs/<env> with markers and staged-oracle verification; destination resolution lives in core so restore and doctor --verify-restored agree, staging directories are workspace-qualified, a user-supplied environment id is validated against the manifest and never becomes a path component, and the sequential-abort policy (first failure stops the run, staged packs retained) is specified and tested.
- [ ] #6 Every new transport writes manifest schema 4, including root-only bundles, while readers keep accepting schemas 1-3; manifest fixtures and goldens cover 4 and the consumer upgrade requirement is documented.
- [ ] #7 Offline evidence asserts both observables: the nested project's pixi install --frozen --offline is a no-op, and a delegated pixi run --manifest-path <nested>/pixi.toml <task> succeeds inside the denied-egress namespace. The no-op is the discriminating assertion; a native-runner airlock result is claimed only from the CI gate that actually ran it.
- [ ] #8 Documentation and generated artifacts are updated through their generators (configure-bundles guide, CLI reference, transport layout notes, golden workflow files), and the full suite's pass count rises or holds with fmt, lint and test green.
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

0. Premise proof (AC#1): a fixture showing the [workspace]-member route cannot serve the case. If it can, stop — this task becomes a docs change.
1. Agreement (AC#2, AC#6): config schema-2 surface, manifest schema-4 policy, plan shape, and the public test seams.
2. Config + validation (AC#2): parsing, named refusals, schema-1 compatibility.
3. Plan reporting (AC#3): text and JSON over a fixture repo, including conflict refusals.
4. Transport identity (AC#4): per-environment roots for preflight, nested blob namespace, oracle untouched.
5. Restore (AC#5): core-owned destination resolution, qualified staging, validated ids, sequential abort.
6. Evidence and docs (AC#7, AC#8): the two AC#6 observables, docs, regenerated artifacts, full gates.

Evidence boundary: the local denied-egress fixture tier is no substitute for a native-runner airlock run; that claim waits for the CI gate that actually executes it.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Slice 0 (premise check) executed on 2026-10-23 against pixi 0.83.0 with a local file channel; full evidence table is `backlog/docs/plans/task-129-nested-workspaces.md` § Slice 0.

Findings: (1) pixi 0.83 rejects `[workspace] members` outright, so the "declare it as a member" route does not exist; (2) a nested pixi.toml is an independent workspace with its own lock and `python/.pixi/envs/default`; (3) packing that workspace separately works today — restored, `pixi install --frozen --offline` is a no-op and `pixi run hello` succeeds with the channel directory renamed away; (4) removing the restored env reproduces issue #194 exactly (`failed to fetch native-fixture-1.0-0.tar.bz2`), which is the negative control; (5) the nearest member equivalent — merging the subproject into the root manifest as `[feature.py]` + `[environments] py = ["py"]` — also packs and restores offline, so the merge route can technically serve the goal; (6) per-feature and per-environment channels exist (`pixi workspace channel add -f/-e`), so differing channels alone do not defeat a merge.

Conclusion: TASK-129 is not the only way to be offline. It is the unification — one bundle, one branch, one restore, subproject manifests untouched — and the alternatives cost either a consumer restructuring (one manifest, one lock, renamed environments, tasks moved) or one branch/restore per workspace with no config support. AC#1 is deliberately left unchecked pending the owner ruling between proceeding as scoped, closing as documentation, or re-scoping plan reporting first.

Product defect found and filed as TASK-132: `native_pack::writer` requires an absolute URL for every locked `conda:` entry, but pixi writes bare paths for file channels, so a mirrored/offline project cannot pack without hand-editing the lock (`file://` plus an explicit `subdir`).
<!-- SECTION:NOTES:END -->
