---
id: TASK-71
title: Make the generated upgrade lane deliver workflow-file changes safely
status: In Progress
assignee: []
created_date: '2026-10-04 19:49'
updated_date: '2026-10-04 20:23'
labels:
  - ci
  - github-actions
  - init
  - release
dependencies:
  - TASK-47
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/101'
  - 'https://github.com/Archont561/castellan/actions/runs/37228494836'
  - 'https://github.com/Archont561/castellan/pull/13'
  - 'https://github.com/Archont561/castellan/actions/runs/37230019375'
modified_files:
  - crates/pixi-sandbox/src/generated/github_workflow.rs
  - crates/pixi-sandbox/tests/generated_workflow.rs
  - docs/src/content/docs/guides/ci-publishing.mdx
priority: high
type: enhancement
ordinal: 71000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The generated consumer upgrade job reaches bootstrap, self-update, drift detection, and regeneration, but cannot deliver its pull request when the regenerated files include .github/workflows/*.yml. GitHub rejects the default GITHUB_TOKEN because workflow-file updates require Workflows write permission, which the Actions token cannot receive. Castellan reproduced this on the 0.5.2-generated lane while upgrading to 0.5.3: https://github.com/Archont561/castellan/actions/runs/37228494836.

Make the upgrade path honest and reviewable without leaving scheduled consumers red. Support an explicitly configured upgrade credential for workflow-file pushes, fail legibly with a step summary and actionable remedy when it is absent or insufficient, and provide a no-secret fallback that preserves the regenerated files as a reviewable patch/artifact or equivalent operator hand-off. Keep production pins exact; do not bypass review or push directly to main.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A generated upgrade lane with a configured PAT or GitHub App token having Workflows write can push the regenerated workflow files and open a reviewable upgrade PR; checkout, git push, and gh PR creation use the same explicit credential.
- [x] #2 When no suitable upgrade credential is configured, the scheduled/manual lane does not emit an unexplained red failure: it writes the GitHub error and step summary with the exact missing permission/secret and a remediation, and preserves a git-apply-able patch or equivalent artifact containing all regenerated owned files.
- [x] #3 The fallback never pushes main, never weakens exact-version pinning, never edits consumer configuration, and keeps the human review gate and explicit post-merge publish behavior.
- [x] #4 Renderer and fixture tests cover configured-token, absent-token, push-refusal, and fallback paths; generated workflow remains actionlint-clean and init regeneration remains byte-identical.
- [ ] #5 A connected Castellan run proves the repaired lane produces a reviewable PR or documented patch artifact, and the resulting generated files use the exact self-updated version.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 pixi run --frozen fmt
- [x] #2 pixi run --frozen lint
- [x] #3 pixi run --frozen test
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-04 — implemented the workflow-file delivery fix test-first. The generated upgrade lane now accepts optional `PIXI_SANDBOX_UPGRADE_TOKEN` and uses it consistently for checkout, git push, and gh PR creation. The built-in GITHUB_TOKEN remains the checkout/artifact fallback but is never treated as workflow-capable. With no suitable token, or after a refused push/PR creation, the lane stays green, writes a GitHub error and step-summary remediation naming `Workflows: write`, and uploads `pixi-sandbox-upgrade-artifacts` containing a git-apply-able patch. The fallback never pushes main or stages config. Added renderer contracts for token wiring, patch preservation, artifact upload, and non-bare delivery refusal; regenerated the golden consumer workflow and documented the secret/fallback policy in the CI publishing guide.

Evidence: `pixi run --frozen fmt` green; `pixi run --frozen lint` green including actionlint, generated-workflow, and repository checks; `pixi run --frozen test` green at 631 passing / 1 skipped (381 sandbox, 21 git, 142 core, 87 xtask). AC#5 remains open until a connected Castellan run proves the repaired generated lane produces either the reviewable PR with the workflow-capable token or the documented patch artifact without it.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
created: 2026-10-04 20:05
---
2026-10-04 connected evidence from Castellan PR #13: the real generated upgrade lane runs bootstrap, self-update, drift detection, and regeneration, then fails at delivery because GITHUB_TOKEN cannot update .github/workflows/*.yml (runs 37228494836 and 37228565253). The same PR documents five consecutive publish failures after shells joined the bundle, with empty logs caused by set +e followed by a grouped command that re-armed errexit before diagnostics could print. Its reproduced 0.5.3 upgrade was dispatched as run 37230019375 and packed/published green at transport commit 1252ae53, proving the preceding 0.5.2 colon-path failure is fixed; PR #13 remains the reviewable exact-version upgrade artifact.
---
<!-- COMMENTS:END -->
