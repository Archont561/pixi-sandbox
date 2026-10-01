---
id: TASK-32
title: Remove install script and make the Pixi channel canonical
status: Done
assignee: []
created_date: '2026-10-01 16:30'
updated_date: '2026-10-01 14:45'
labels:
  - release
  - packaging
  - docs
  - cleanup
milestone: m-0
dependencies:
  - TASK-22
  - TASK-23
references:
  - .github/workflows/release.yml
  - templates/install.sh
  - scripts/render-install.sh
  - scripts/lint-repo-consistency.sh
  - README.md
  - docs/src/content/docs/installation.mdx
priority: high
type: enhancement
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Retire the downloaded `install.sh` bootstrap now that pixi-sandbox is published as native Pixi/Conda packages for every supported platform. Make `pixi global install` from the `archont561/pixi-sandbox` prefix.dev channel the single connected-host installation path. Keep standalone GitHub release binaries and checksums because transports still need a verified native bootstrap on the airlock; remove only the redundant shell installer.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The release workflow no longer renders or uploads `dist/install.sh`, while standalone binaries, `SHA256SUMS`, all five Conda packages, prefix.dev publication, and GitHub release notes remain intact
- [x] #2 `templates/install.sh` and `scripts/render-install.sh` are deleted, and repository-consistency/release checks contain no install-template policy
- [x] #3 README and user docs present one tested canonical command using the actual `archont561/pixi-sandbox` prefix.dev channel followed by `pixi-sandbox init`
- [x] #4 Active documentation contains no `curl ... install.sh | sh` path and clearly states that connected hosts need Pixi installed; historical changelog and completed-task evidence remain untouched
- [x] #5 Generated publishing workflows install the same channel package rather than depending on the removed installer or repository-owned setup action
- [x] #6 Release and documentation checks fail if the canonical channel command or package name drifts
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
First complete the native package proof in TASK-23. Remove the render/upload path from release.yml and delete the template and renderer. Delete the corresponding numbered repository-consistency rule instead of porting dead policy to xtask. Replace connected-side installation examples with one verified `pixi global install` command, retain binary/checksum documentation only for transport bootstrap, and align TASK-22's generated workflow with the same channel command.
<!-- SECTION:PLAN:END -->


## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Removed the release workflow's render step and `dist/install.sh` upload while preserving all five standalone binaries, SHA256SUMS, all five native Conda packages, prefix.dev publication with attestation, and generated release notes. Deleted `templates/install.sh`, the retired `render-install` xtask command, its pure renderer, and the old consistency rule.

README and active installation, quickstart, and project guides now require Pixi on connected hosts and use the single canonical command `pixi global install --channel https://prefix.dev/archont561/pixi-sandbox pixi-sandbox`, followed by `pixi-sandbox init`. Standalone binaries and checksums are documented only as transport-bootstrap or pinned-CI assets. The generated publisher already installs from the same package-specific channel and remains fixture-tested. Repository consistency check 5 now fixture-tests drift in the README, installation guide, and generated-workflow source instead of enforcing deleted installer policy.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
Connected-host installation is now one native prefix.dev package command followed by init. The redundant curl-piped installer and all of its release, rendering, and consistency machinery are gone, while verified standalone transport assets and the five-platform package release remain intact.
<!-- SECTION:SUMMARY:END -->
