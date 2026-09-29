---
id: TASK-5
title: Make sandbox-env.sh self-sufficient and drop the hardcoded default env
status: Done
assignee:
  - '@agent'
created_date: '2026-09-28 22:05'
updated_date: '2026-09-29 06:49'
labels:
  - restore
  - ux
dependencies: []
references:
  - crates/pixi-sandbox/src/commands/restore.rs
  - scripts/restore.sh
priority: medium
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The generated .pixi/sandbox-env.sh (write_sandbox_env in restore.rs) wires only .pixi/tools/<platform> and the pixi() function, so cargo, bun and rustc stay off PATH unless the caller also exports .pixi/envs/<env>/bin — and scripts/restore.sh does that with a hardcoded envs/default, which is wrong for any project whose restored environment has another name. Observed live: after a successful restore, sourcing sandbox-env.sh alone leaves cargo: command not found.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Sourcing the generated sandbox-env.sh is sufficient for both the bundled tools and the restored environment binaries to resolve on PATH
- [x] #2 Environment names come from the manifest and scripts/restore.sh no longer hardcodes envs/default
- [x] #3 A round-trip test asserts the generated script contains the manifest environment names
- [x] #4 docs/restore.mdx matches the new single-source workflow
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
TDD: add a CLI integration test in tests/cli.rs that restores the fixture transport and, in a fresh shell, sources only .pixi/sandbox-env.sh then checks a real restored env binary resolves on PATH (confirmed RED). Fix write_sandbox_env in restore.rs to append each restored environment bin dir to PATH using the actually-restored environment list (confirmed GREEN, full suite plus clippy clean). Drop the hardcoded envs/default PATH export in scripts/restore.sh now that sourcing alone is sufficient, replacing it with a PATH diff report. Update restore.mdx to describe the self sufficient sandbox-env.sh.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
RED: added sourcing_the_generated_sandbox_env_resolves_restored_environment_binaries in crates/pixi-sandbox/tests/cli.rs, chose fixture bin/freetype-config over lzmainfo because this sandbox has a system lzmainfo that would let the test pass for the wrong reason. GREEN: write_sandbox_env now takes the restored environment list and prepends envs/name/bin for each one before PATH; verified with an env -i fresh shell against a locally built binary restoring the real sandbox/developer-linux-64 transport, cargo and rustc resolved with zero extra PATH setup. Also removed the hardcoded envs/default export in scripts/restore.sh, replaced with a PATH diff report; note the currently published sandbox branch still embeds the pre-fix pixi-sandbox binary so scripts/restore.sh will only show the full PATH gain after the next publish. Full cargo test -p pixi-sandbox and clippy are clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed the airlock PATH gap via TDD: sandbox-env.sh now wires every restored environment bin dir onto PATH by itself, and scripts/restore.sh no longer hardcodes envs/default. Docs updated to match.
<!-- SECTION:FINAL_SUMMARY:END -->
