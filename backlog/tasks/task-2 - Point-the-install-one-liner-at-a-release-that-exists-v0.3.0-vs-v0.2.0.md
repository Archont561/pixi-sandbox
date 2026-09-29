---
id: TASK-2
title: Point the install one-liner at a release that exists (v0.3.0 vs v0.2.0)
status: In Progress
assignee:
  - '@agent'
created_date: '2026-09-28 22:05'
updated_date: '2026-09-29 08:35'
labels:
  - docs
  - release
dependencies: []
references:
  - README.md
  - docs/src/content/docs/quickstart.mdx
  - docs/src/content/docs/guides/using-in-your-project.mdx
priority: high
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
README.md, the docs quickstart and using-in-your-project all curl scripts/init.sh from the v0.3.0 tag, but the latest published release is v0.2.0, so the flagship install one-liner 404s for every new user. Either cut the v0.3.0 release (preferably after the osx-arm64 proof in task-1 lands) or repin every reference to v0.2.0.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every documented install URL resolves with HTTP 200 for the pinned tag
- [x] #2 The pin is consistent across README.md and every docs page that references a version
- [ ] #3 A v0.3.0 release, if cut, ships init.sh plus the five static binaries and SHA256SUMS matching the release workflow contract
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Airlock-completable part done. scripts/init.sh was added after the v0.2.0 tag, so it does not exist at v0.2.0 and defaulted to a non-existent v0.3.0 release - the one-liner 404'd on both the raw script URL and the binary download. Fixed so it works today: fetch init.sh from `main` (the only ref that has it) and pin the installed binary to the existing v0.2.0 release. Changed README.md, docs quickstart.mdx, using-in-your-project.mdx (raw URL -> /main/, prose + PIXI_SANDBOX_VERSION -> v0.2.0) and scripts/init.sh (VERSION default -> v0.2.0). All action `uses:` pins were already v0.2.0, so pins are now consistent (AC#2).

AC#1/AC#3 need an online/CI agent: a proper immutable-tag pin requires cutting a v0.3.0 release that ships init.sh + the five static binaries + SHA256SUMS (via the auto-release/release workflows), then repinning the one-liner from `main` to the v0.3.0 tag. Tracked in CONTEXT.md under "Online / Codespaces agent tasks".


Update: release.yml now ships scripts/init.sh as a release asset (cp into dist/, added to the gh-release files, deliberately not in SHA256SUMS since it's fetched via curl|sh). This lets the one-liner point at an immutable release URL (releases/download/<tag>/init.sh) once v0.3.0 is cut, rather than a branch. Could not add the asset to the existing v0.2.0 release from the airlock (uploads.github.com is blocked), so the one-liner stays on main until an online agent cuts v0.3.0 (or uploads init.sh to v0.2.0) and repins - see CONTEXT.md.
<!-- SECTION:NOTES:END -->
