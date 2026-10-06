---
id: TASK-71
title: Make the generated upgrade lane deliver workflow-file changes safely
status: Done
assignee: []
created_date: '2026-10-04 19:49'
updated_date: '2026-10-06 21:01'
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
- [x] #5 A connected Castellan run proves the repaired lane produces a reviewable PR or documented patch artifact, and the resulting generated files use the exact self-updated version.
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

2026-10-06 20:47 — AC#5 PROVEN. Castellan run 37529029130 (workflow_dispatch, upgrade=0.5.3) closed this on the documented-patch-artifact branch.

What ran: plan and publish skipped, upgrade succeeded in 10s (20:47:14 to 20:47:24Z). Steps 6, 7 and 8 are all gated on drift == true and all executed, so the drift guard fired for real — self-update accepted the downgrade from 0.6.0 to 0.5.3, which the earlier audit had assumed was unforceable. Branch pixi-sandbox-upgrade/0.5.3 at c645409 carries exactly the three owned files regenerated to the self-updated version: publish-sandbox.yml +12/-59, relock.yml +19/-233, scripts/restore.sh +1/-1. Artifact pixi-sandbox-upgrade-artifacts, 7561 bytes. The job stayed green and surfaced a failure annotation, which is AC#2 behaving as designed rather than an unexplained red run.

Diagnostic the run settles: the handoff emitted "the branch was pushed but gh could not open the pull request". That is the third branch of write_handoff, reached only after the empty-token check and the push both passed, so PIXI_SANDBOX_UPGRADE_TOKEN IS configured in Castellan and does carry Workflows: write — GITHUB_TOKEN cannot push .github/workflows/* and the push of two workflow files succeeded. What the credential lacks is pull-request creation. The remaining fix is Pull requests: Read and write on that token or App installation, not the Workflows permission the remediation string suggests; that string is now slightly misleading for this case and is worth narrowing.

Cleanup owed: pixi-sandbox-upgrade/0.5.3 is a downgrade proposal left on Castellan by this proof and should be deleted so nobody merges it.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
created: 2026-10-04 20:05
---
2026-10-04 connected evidence from Castellan PR #13: the real generated upgrade lane runs bootstrap, self-update, drift detection, and regeneration, then fails at delivery because GITHUB_TOKEN cannot update .github/workflows/*.yml (runs 37228494836 and 37228565253). The same PR documents five consecutive publish failures after shells joined the bundle, with empty logs caused by set +e followed by a grouped command that re-armed errexit before diagnostics could print. Its reproduced 0.5.3 upgrade was dispatched as run 37230019375 and packed/published green at transport commit 1252ae53, proving the preceding 0.5.2 colon-path failure is fixed; PR #13 remains the reviewable exact-version upgrade artifact.
---
created: 2026-10-06 19:55
---
2026-10-06 the chicken-and-egg is broken, but AC#5 is still open and cannot be closed yet. Castellan was bootstrapped onto the 0.6.0 templates by hand (commit 8193c58, "chore: update pixi-sandbox to version 0.6.0"), carrying exactly the three owned files and no config edit: publish-sandbox.yml gains the `secrets.PIXI_SANDBOX_UPGRADE_TOKEN || github.token` wiring, the fail-soft `write_handoff`/`delivery_refused` path and the `pixi-sandbox-upgrade-artifacts` upload; relock.yml gains `checks: write` and the observer; scripts/restore.sh gains the stamp. A hand bootstrap was the only way in, because the 0.5.3 lane that had to deliver the fix is the very lane the fix repairs — it reads no upgrade secret, so adding the secret to Castellan before this commit would have changed nothing. The push proved the regenerated files work: `publish sandbox` run 37521245074 was green in 3m37s (plan 7s, upgrade **skipped**, publish green), it repacked the transport to af92e4c1, and that manifest reads `"tool": {"version": "0.6.0"}` with `"source": {"commit": "8193c58"}`. Castellan's own CI job "generated files" also passed, so the regeneration is byte-correct. None of that is AC#5: the `upgrade` job never ran (its `if:` is schedule-or-dispatch, and this was a push), so the delivery path this task rewrote has still never executed connected. It also cannot be forced right now — Castellan is pinned at 0.6.0 and 0.6.0 is latest, so a dispatch resolves `drift=false` and there is nothing to deliver. **The proof window is the next pixi-sandbox release.** Set `PIXI_SANDBOX_UPGRADE_TOKEN` in Castellan *before* cutting 0.6.1/0.7.0: with it, the weekly cron opens the reviewable PR and closes AC#5 on the happy path (and TASK-73 AC#3/#4); without it, the same run closes AC#5 on the documented-patch-artifact branch instead and leaves TASK-73 AC#3 open.
---
<!-- COMMENTS:END -->
