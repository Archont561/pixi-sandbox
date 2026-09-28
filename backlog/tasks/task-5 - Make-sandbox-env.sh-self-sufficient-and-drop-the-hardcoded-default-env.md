---
id: TASK-5
title: Make sandbox-env.sh self-sufficient and drop the hardcoded default env
status: To Do
assignee: []
created_date: '2026-09-28 22:05'
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
- [ ] #1 Sourcing the generated sandbox-env.sh is sufficient for both the bundled tools and the restored environment binaries to resolve on PATH
- [ ] #2 Environment names come from the manifest and scripts/restore.sh no longer hardcodes envs/default
- [ ] #3 A round-trip test asserts the generated script contains the manifest environment names
- [ ] #4 docs/restore.mdx matches the new single-source workflow
<!-- AC:END -->
