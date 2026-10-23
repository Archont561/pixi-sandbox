---
id: TASK-129
title: Bundle explicitly declared nested Pixi workspaces
status: In Progress
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
- [x] #1 Slice 0 is recorded before any transport code exists: a fixture demonstrates that declaring the nested project as a [workspace] member of the root pixi.toml cannot serve the case (conflicting channels, versions or platforms). If the member route works, this task closes as a documentation change instead.
- [x] #2 Config schema is bumped to 2: a bundle declares explicit nested manifest paths with an optional environment selection, while an unknown or escaping path, a duplicate target, a manifest already claimed by another bundle, a missing lockfile, or a nested Rust workspace with its own Cargo.lock is refused with a named diagnostic pointing at TASK-131. Schema-1 configs keep parsing unchanged, and an older binary refuses a schema-2 config naming the version it needs.
- [ ] #3 plan (text and --json) names every declared manifest, its selected environments, resolved package set and workspace-qualified size estimate, and refuses conflicts (differing channels, platforms or package identities) plus a nested manifest that is a member of the root Pixi workspace; the rendered workflow still derives its matrix from validated plan output.
- [x] #4 pack resolves every selected environment to its own project root for the lock and native preflights, and carries nested environments into the transport with per-environment identity under .pixi-sandbox/envs/workspaces/<slug>/<env>/pack/ — root paths stay byte-identical, and budget ceilings, the per-file oracle and byte verification are unchanged.
- [x] #5 restore materialises each nested environment into its own project's .pixi/envs/<env> with markers and staged-oracle verification; destination resolution lives in core so restore and doctor --verify-restored agree, staging directories are workspace-qualified, a user-supplied environment id is validated against the manifest and never becomes a path component, and the sequential-abort policy (first failure stops the run, staged packs retained) is specified and tested.
- [x] #6 Every new transport writes manifest schema 4, including root-only bundles, while readers keep accepting schemas 1-3; manifest fixtures and goldens cover 4 and the consumer upgrade requirement is documented.
- [ ] #7 Offline evidence asserts both observables: the nested project's pixi install --frozen --offline is a no-op, and a delegated pixi run --manifest-path <nested>/pixi.toml <task> succeeds inside the denied-egress namespace. The no-op is the discriminating assertion; a native-runner airlock result is claimed only from the CI gate that actually ran it.
- [x] #8 Documentation and generated artifacts are updated through their generators (configure-bundles guide, CLI reference, transport layout notes, golden workflow files), and the full suite's pass count rises or holds with fmt, lint and test green.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Focused fixture/tempdir tests and the offline e2e lifecycle pass, with the repository test baseline recorded before and after and never reduced.
- [x] #2 fmt, lint and test are green; generated workflow/branch artifacts are regenerated from their generators and the plan output is validated by its own check.
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
Slice 1–5 landed on 2026-10-23 (config schema 2, plan reporting, pack per-root, restore per-destination, manifest schema 4). AC#3 and AC#7 are **not** checked; see the two entries below.

**What is implemented and measured.** `CONFIG_SCHEMA` is 2 with `[[bundle.workspaces]]` (`manifest` + optional `environments`; omitted means the nested project's `default`). Shape rules refuse an absolute path, a `..` or `.` component, a backslash, a non-`pixi.toml` name, the repository root's own manifest, a duplicate declaration, one manifest claimed by two bundles, and a root environment named `workspaces` (the reserved transport segment). Filesystem rules live in `SandboxConfig::load` and refuse a missing manifest, a missing nested `pixi.lock`, and a nested `Cargo.lock` with TASK-131 named as the follow-up. Schema-1 configs parse unchanged; a newer schema is still refused with the supported range, and two tests that hardcoded `schema = 2` as "newer" were rewritten to derive from `CONFIG_SCHEMA` — they had stopped testing anything the moment the constant moved.

`plan` resolves the declarations against the repository and reports, in text and under `workspaces` in `--json`, the manifest, its environments, the package count its lock resolves for the bundle's platforms, and the summed artifact sizes. It refuses a nested manifest with no `workspaces.platforms` list, a platform the bundle publishes that the nested manifest does not declare, an environment its lock cannot resolve, and a directory the root `pixi.toml` lists as a workspace member. `include` remains the only thing the generated workflow reads, so the matrix is unchanged.

`pack` resolves every `--envs` entry through the declaration — `default` stays the repository root, `subproject/default` becomes a target rooted at `subproject/` — and runs the PyPI and native preflights against *that* root's lock. Nested payloads land at `.pixi-sandbox/envs/workspaces/<slug>/<env>/pack/…` with the oracle beside them; root payloads are byte-identical to an un-nested pack (asserted by comparing bytes against the fixture channel). The per-file oracle, shard recording and budget accounting are untouched.

`restore` asks `Manifest::env_destination` — now in core, shared with `doctor --verify-restored` — so the root environment returns to `.pixi/envs/<env>` and a nested one to `<project>/<directory>/.pixi/envs/<env>`. Staging is workspace-qualified (`pack-<id with / -> -->`), so root `default` and nested `default` cannot share scratch, and the sequential abort keeps its existing contract: the first failure stops the run, earlier siblings stay installed, staged packs are retained.

Manifest schema is 4 for every transport, root-only included; readers still accept 1–3 (covered by a parametrised test) and the upgrade requirement is in the guide.

Evidence: full suite 1,218 passed / 2 skipped (585 → 593 product, 295 → 315 core, 54 git, 256 xtask), against 1,190 / 2 before this change — nothing reduced. `fmt`, `lint-deps`, `lint-clippy`, `actionlint`, `taplo`, generated workflows, `plan` and `check-repository` all pass, and `lint-docs` is clean.

**AC#3 is left unchecked on purpose.** Destination, platform, environment-resolution and root-membership conflicts are refused, but the criterion also names *differing channels or package identities*. Those cannot be refused without contradicting the design this task ratified: each nested workspace keeps its own lock and its own prefix, so two projects that pin different versions of a package do not conflict — that disagreement is the feature. Refusing it would re-impose the merged-workspace cost slice 0 exists to avoid. This needs the owner's reading: either the parenthetical is stale, or it names a check this implementation does not have.

**AC#7 was attempted and could not be asserted here.** A first version of the test ran `pixi install --frozen --offline -m <nested>/pixi.toml` after a restore. Pixi rejected the fixture's lock with `missing field/fields 'subdir'` — the packer fixture writes a hand-built v7 lock that is enough for `pack` but not a lock pixi itself will load, so the run says nothing about the feature. Proving the no-op needs a lock pixi generated (a local `file://` channel solve), which the fixture does not have; the test was removed rather than weakened into an assertion that could pass while proving nothing.

**AC#7 is left unchecked.** The two observables it asks for — the nested project's `pixi install --frozen --offline` is a no-op, and a delegated `pixi run --manifest-path …` succeeds in a denied-egress namespace — were not run. The local fixture proves transport identity and destination layout, which is not the same claim, and the native-runner airlock tier is a CI gate, not something this machine can assert.

**Premise re-confirmed:** the nested directory is refused when the root `pixi.toml` lists it under `[workspace] members` — pixi 0.83 rejects that key outright, so the check is currently inert by construction and exists for the version that accepts it.

Slice 0 (premise check) executed on 2026-10-23 against pixi 0.83.0 with a local file channel; full evidence table is `backlog/docs/plans/task-129-nested-workspaces.md` § Slice 0.

Findings: (1) pixi 0.83 rejects `[workspace] members` outright, so the "declare it as a member" route does not exist; (2) a nested pixi.toml is an independent workspace with its own lock and `python/.pixi/envs/default`; (3) packing that workspace separately works today — restored, `pixi install --frozen --offline` is a no-op and `pixi run hello` succeeds with the channel directory renamed away; (4) removing the restored env reproduces issue #194 exactly (`failed to fetch native-fixture-1.0-0.tar.bz2`), which is the negative control; (5) the nearest member equivalent — merging the subproject into the root manifest as `[feature.py]` + `[environments] py = ["py"]` — also packs and restores offline, so the merge route can technically serve the goal; (6) per-feature and per-environment channels exist (`pixi workspace channel add -f/-e`), so differing channels alone do not defeat a merge.

Conclusion: TASK-129 is not the only way to be offline. It is the unification — one bundle, one branch, one restore, subproject manifests untouched — and the alternatives cost either a consumer restructuring (one manifest, one lock, renamed environments, tasks moved) or one branch/restore per workspace with no config support. AC#1 is deliberately left unchecked pending the owner ruling between proceeding as scoped, closing as documentation, or re-scoping plan reporting first.

Product defect found and filed as TASK-132: `native_pack::writer` requires an absolute URL for every locked `conda:` entry, but pixi writes bare paths for file channels, so a mirrored/offline project cannot pack without hand-editing the lock (`file://` plus an explicit `subdir`).
<!-- SECTION:NOTES:END -->
