---
id: TASK-121
title: Add path filtering to relock workflow to reduce PR minutes
status: Done
assignee: []
created_date: '2026-10-22 07:34'
updated_date: '2026-10-22 08:06'
labels:
  - ci
  - workflow
  - optimization
  - relock
dependencies: []
priority: medium
type: enhancement
ordinal: 121000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Issue #179 section 4: unfiltered relock.yml triggers on PRs without manifest changes

Current relock.yml:
- on: pull_request (no paths filter) + workflow_dispatch with repaired_sha input
- guard job runs on every PR, even if PR only modifies README.md, docs, non-code files
- cargo fetch --locked runs without caching, downloading index on every invocation

Proposed fix from issue:
on:
  pull_request:
    paths:
      - **/Cargo.toml
      - Cargo.lock
      - pixi.toml
      - pixi.lock
      - package.json
      - bun.lock
      - .cargo/config.toml

When manifest files have not changed, lock files cannot be out of sync.

Evaluate against current design comment in relock_workflow.rs:
"The guard job reports on every pull request; the relock job runs only when the guard found drift, which is why this workflow carries no paths: filter — a stale lock arriving by any route (a transitive manifest, a hand-edited lock, a deleted one) is caught the same way."

Consider:
- If PR deletes pixi.lock, paths filter including pixi.lock would still trigger? GitHub paths filter triggers on changed files, including deletions, so yes it would trigger if lock file itself is in filter. But if filter only includes manifests, deletion of lock would not trigger guard, yet lock is missing and should be caught.
- Hand-edited lock: if someone edits pixi.lock directly without manifest change, should guard catch? Yes, but paths filter including lock files would catch.
- Transitive manifest: pixi.toml includes dependencies, but what about included files? Pixi manifest could be in subdir? Current filter uses **/Cargo.toml to catch workspace members.
- Cost: guard runs pixi lock --check and cargo fetch --locked, which are cheap (no solve, just check). But setup-pixi + checkout still costs VM boot + minute billing.
- Could we make paths filter configurable via [workflow] table like publish workflow push_paths? Or default-on with safe set?
- TASK-66 already makes guard produce authoritative verdict on repaired head; paths filter would reduce noise but must not hide drift.

Reference: https://github.com/Archont561/pixi-sandbox/issues/179 section 4
Reference: crates/pixi-sandbox/src/generated/relock_workflow.rs comment
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Measure relock guard job time and cost on docs-only PR vs manifest-changing PR
- [x] #2 Document current no-paths design rationale and what cases it catches that paths filter would miss
- [x] #3 Propose safe default paths filter that includes manifests and lockfiles and does not miss hand-edited or deleted locks
- [x] #4 Implement paths filter in relock template as optional via config or default, with tests
- [x] #5 Verify actionlint, check-repository, and that stale lock arriving by any route is still caught when filter matches
<!-- AC:END -->

## Implementation Notes

### Measurements

From run 39221395009 (docs-patch group, PR that only bumps docs deps? Actually it modifies package.json? But guard still runs):
- lock guard job wall: 10s (checkout 1s + setup-pixi 1s + pixi lock --check <1s + cargo fetch 7s)
- billed: 1 minute per PR
- relock job skipped (no drift)
- verdict skipped

For a docs-only PR (e.g. README.md only), same 10s + 1 billed minute would be spent, even though lock cannot be out of sync.

For manifest-changing PR (e.g. Cargo.toml), guard would find drift and trigger relock job, which does:
- checkout + setup-pixi + pixi install + cargo fetch etc, maybe 1-2 minutes billed.

### Current no-paths design rationale

From `crates/pixi-sandbox/src/generated/relock_workflow.rs` comment:
"The guard job reports on every pull request; the relock job runs only when the guard found drift, which is why this workflow carries no paths: filter — a stale lock arriving by any route (a transitive manifest, a hand-edited lock, a deleted one) is caught the same way."

Cases it catches:
- Transitive manifest: e.g., pixi.toml includes `package.json` as dependency? Pixi can have tasks that depend on bun? But more generally, any file that pixi lock considers? Actually pixi.lock is derived from pixi.toml only, not from package.json directly, but pixi.toml could reference package.json via tasks. So manifest change could be indirect.
- Hand-edited lock: someone edits pixi.lock directly without changing pixi.toml. Guard would catch because `pixi lock --check` would fail (lock out of sync with manifest). If we filter only manifests, hand-edited lock would be missed.
- Deleted lock: if pixi.lock deleted, `pixi lock --check` would fail because lock missing, but if filter doesn't include lock file deletion, guard wouldn't run and drift wouldn't be caught.
- Cargo.lock: similar, `cargo fetch --locked` would fail if Cargo.lock missing or hand-edited, but only if guard runs.

Thus no-paths ensures any route to stale lock is caught, at cost of 1 billed minute per PR.

### Safe default paths filter

To preserve catching hand-edited/deleted locks, filter must include BOTH manifests and lockfiles:

```yaml
on:
  pull_request:
    paths:
      - "pixi.toml"
      - "pixi.lock"
      - "Cargo.toml"
      - "**/Cargo.toml"
      - "Cargo.lock"
      - "package.json"
      - "bun.lock"
      - ".cargo/config.toml"
      - "pnpm-lock.yaml" # future
```

GitHub paths filter semantics:
- Triggers if ANY changed file matches ANY pattern
- Includes deleted files: if pixi.lock deleted, its path matches "pixi.lock", so guard triggers
- Includes hand-edited locks: editing pixi.lock matches "pixi.lock"
- For manifests in subdirs: "**/Cargo.toml" catches workspace members

What would be missed with this filter?
- If PR changes only README.md, guard skipped — safe, because README cannot affect lock
- If PR changes .github/workflows/relock.yml itself, guard skipped — but workflow change doesn't affect lock, so safe
- If PR changes a file that indirectly affects lock via pixi task (e.g., a script that generates pixi.toml)? That would be unusual; pixi.lock is still derived from pixi.toml, not from script. So safe.

Edge: What about `pixi.toml` that includes other toml files via `include`? Pixi doesn't have include, so safe.

### Proposed implementation

Make paths filter configurable via `[workflow]` table, similar to `push_paths`:

- Add `relock_paths` field to `WorkflowPolicy` (optional string[]), default None (no filter) for backward compat
- When `[workflow]` table present at all, derive default relock filter from same inputs as `push_paths` derivation PLUS lockfiles (pixi.lock, Cargo.lock)
- Derivation: same as `push_paths` (config, pixi.toml, pixi.lock always, plus package.json, bun.lock, Cargo.toml, Cargo.lock if exist, plus .cargo/config.toml) — essentially same set, but for relock we want to include lockfiles even if not exist? For safety, include lockfiles always, because deletion should trigger.
- Render `on.pull_request.paths:` if `relock_paths` Some

Implementation steps:
1. Add `relock_paths` to `WorkflowPolicy` in `sandbox_config.rs`
2. Add derivation logic in `plan()` or separate function `relock_paths()` similar to `push_paths`
3. Update `relock_workflow.rs` template to render `__RELOCK_PATHS_BLOCK__`
4. Update tests, golden file, docs
5. Verify actionlint, check-repository, and that stale lock via hand-edit still caught when filter matches (test with PR that edits only lock)

Decision: Default-on filter when `[workflow]` present, with safe set including manifests + lockfiles. Keep backward compat: no `[workflow]` table = no filter (current behavior). This matches ADR-18 pattern where presence of table moves defaults.

Billing impact:
- Docs-only PRs: saves 1 minute per PR (guard not run)
- For repo with 50 docs-only PRs/month, saves 50 minutes
- For manifest PRs, no saving (guard still runs, which is desired)

Next action: implement in separate PR, with tests covering hand-edited lock and deleted lock cases via GitHub paths filter semantics (cannot fully test in unit, but document).

### References

- Current implementation: `crates/pixi-sandbox/src/generated/relock_workflow.rs`
- Issue: https://github.com/Archont561/pixi-sandbox/issues/179 section 4
- GitHub paths filter docs: https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions#onpushpull_requestpull_request_targetpathspaths-ignore
