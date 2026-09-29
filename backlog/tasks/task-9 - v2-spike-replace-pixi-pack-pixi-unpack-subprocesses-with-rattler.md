---
id: TASK-9
title: 'v2 spike: replace pixi-pack/pixi-unpack subprocesses with rattler'
status: In Progress
assignee:
  - '@agent'
created_date: '2026-09-28 22:05'
updated_date: '2026-09-29 07:50'
labels:
  - core
  - spike
dependencies: []
references:
  - .knowledge/design.md
  - .knowledge/decisions.md
priority: low
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Design §1 notes that re-implementing the pack format on the rattler library is possible but explicitly not v1 work; it would remove the pixi-pack and pixi-unpack subprocesses and their embedded binaries from the transport. Track as a measured spike: produce a decision-level writeup before any code.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A spike note in .knowledge/ measures the trade-off: binary size and blob count saved versus implementation risk and pixi-pack format coupling
- [ ] #2 A go/no-go decision is recorded as a new decision ID in .knowledge/decisions.md before implementation starts
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Headline numbers: dropping pixi-unpack saves 15.0 MiB (1.82 percent) and 1 blob of 10313, once per branch; pixi (76.6 MiB, 81 percent of tools bytes) stays regardless because the airlock still runs pixi install --frozen --offline. pixi-unpack is itself a rattler front-end, so its 15.0 MiB static binary is the empirical proxy for what rattler costs compiled in - the saving is spent on our own binary. Vendored payload grows at 1.79 MiB per added crate measured on this repo, and rattler pulls ~28 crates plus tokio/reqwest/rayon.

Recommendation: no-go, keep D2/D3. Four triggers to revisit are listed, and only option C (replace the install step, keep the pixi-pack format) is worth a prototype - the one categorical win would be peak restore disk, measured today at 2876.8 MiB of scratch for a 1867.5 MiB environment. AC2 is deliberately left unchecked: D12 text is drafted in the note but recording a decision is a maintainer call.
<!-- SECTION:NOTES:END -->
