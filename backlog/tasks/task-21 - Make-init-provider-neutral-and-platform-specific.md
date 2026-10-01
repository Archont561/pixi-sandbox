---
id: TASK-21
title: Make init provider-neutral and platform-specific
status: Done
assignee:
  - '@agent'
updated_date: '2026-10-01 16:05'
created_date: '2026-10-01 09:49'
labels:
  - cli
  - init
  - platform
milestone: m-0
dependencies: []
references:
  - .knowledge/v1-evolution-plan.md
documentation:
  - backlog/docs/plans/v1-platform-workflow-transport/doc-1
priority: high
type: feature
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace init github with pure init. Add --github-workflow-path, --script-path, and --config overrides; prefer pixi-sandbox.toml with dotfile fallback; generate only the current platform launcher; make generated files safely regenerable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 pixi-sandbox init works without a provider subcommand
- [x] #2 Unix generates only the selected POSIX launcher and Windows only the PowerShell launcher
- [x] #3 Both config filenames and explicit --config work
- [x] #4 CLI tests cover force and path overrides
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Removed the positional provider enum: the command is now `pixi-sandbox init`. Its public path controls are exactly `--github-workflow-path`, `--script-path`, and `--config`; the release `install.sh` template invokes the provider-neutral form as well. The generated default config names the native Pixi platform instead of assuming linux-64.

Init resolves configuration as explicit path, existing `pixi-sandbox.toml`, existing `.pixi-sandbox.toml`, then a newly created preferred `pixi-sandbox.toml`. The selected path is shell-quoted into both the workflow and launcher, including custom paths containing spaces. On Unix init writes only the executable POSIX launcher; on Windows it writes only PowerShell. Existing restore lifecycle tests still execute the generated Unix launcher against local orphan-branch objects.

Both generated artifacts carry a common ownership marker. Re-running init replaces marked files without `--force`, refuses an unmarked destination before writing either artifact, and permits the deliberate replacement only with `--force`. The config remains user-owned and is never reset after creation. Colliding workflow, launcher, and config paths are rejected.

CLI tests cover provider-free invocation, native launcher selection, preferred/legacy config precedence, explicit path overrides, quoted paths, marker-based regeneration, user-file refusal, force replacement, and config preservation. Documentation and the install template now show the provider-neutral command, preferred config name, and one-launcher behavior. Verification: actionlint accepts the generated workflow; fmt, Clippy with warnings denied, Taplo, repository consistency, install-script rendering, and all 149 workspace tests pass.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
`pixi-sandbox init` is provider-neutral, platform-specific, config-aware, and safely regenerable. It emits one native launcher plus the workflow, prefers `pixi-sandbox.toml` while retaining the legacy fallback, supports all requested output overrides, and protects user-owned files unless `--force` is explicit.
<!-- SECTION:FINAL_SUMMARY:END -->
