---
id: TASK-117
title: Investigate PyPI RECORD mismatches after uv metadata canonicalisation
status: In Progress
assignee: []
created_date: '2026-10-16 00:00'
updated_date: '2026-10-16 07:24'
labels:
  - bug
  - verification
  - pypi
dependencies: []
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/170'
  - 'https://github.com/Archont561/pixi-sandbox/issues/128'
  - backlog/tasks/task-88 - Stop-recording-uv-regenerated-install-metadata-in-the-per-file-oracle.md
  - backlog/tasks/task-93 - Also-collapse-uv-s-path-dependent-direct_url-json-in-the-per-file-oracle.md
priority: high
type: bug
ordinal: 117000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A reported `pixi-sandbox` 0.7.0 restore of `Archont561/qgis-sdk` materialises all environments
but fails final per-file verification on seven `*.dist-info/RECORD` digests. The pinned transport
is commit `eb87ddadfeffd70dcaecb9fad912272c50805e09`, built from consumer source commit
`801a26ce6d4fd5559bfd0bad2e1e0ac4c9403966`; affected packages are coverage 7.16.2, hypothesis
6.169.4, mako 1.4.3, and pytest-bdd 9.0.0 under Python 3.14. No `uv_cache.json` or
`direct_url.json` mismatch was reported. This resembles issue #128, but TASK-88/TASK-93 already
canonicalise those known uv-regenerated files and RECORD rows; the cause of these seven
mismatches is not established. Determine whether the differences are legitimate reinstall or
relocation variance, or evidence of actual corruption. Any oracle change must preserve detection
of altered package payloads and must be limited to proven regenerated data; do not broadly exempt
RECORD files or weaken verification to silence the report.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Reproduce or otherwise inspect the issue's pinned transport and capture a per-row diff for each reported RECORD, identifying the exact differing fields/bytes and their provenance; record tool versions and whether the original reproduction is independently confirmed.
- [ ] #2 Establish and document a root cause for all seven mismatch reports, including whether each is a reproducible restore-time transformation or an integrity/corruption failure; do not infer harmlessness from successful application tests alone.
- [ ] #3 If the differences are legitimate regenerated data, add the narrowest shared pack/verify canonicalisation and regression coverage for the specific transformation, with a complement proving unrelated RECORD entries and package-payload tampering still fail. If they indicate corruption or cannot be safely normalised, retain fail-closed behavior and document the evidence and required follow-up instead.
- [ ] #4 Validate the affected fixture or pinned artifact against the changed behavior, run `pixi run --frozen fmt`, `pixi run --frozen lint`, and `pixi run --frozen test`, and record exact results; do not claim a real connected round trip unless performed.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->

### 2026-10-16 — pinned artifact unavailable; cause remains unproven

The issue report was inspected in full. It supplies seven expected/observed RECORD digests and
package names, but not the RECORD files or a per-row diff. The reported transport commit
`eb87ddadfeffd70dcaecb9fad912272c50805e09` is not available from this checkout: it is not in
the local object database, `git fetch origin <sha>` returns `upload-pack: not our ref`, and the
GitHub commit endpoint reports no commit for that SHA. The current
`sandbox/developer-linux-64` ref is `fbb7471e` and its manifest names source commit `21a238a`,
so it is not the reported artifact and cannot stand in for it. The consumer source repository
also could not be read here: Git fetch returned HTTP 403 and the GitHub integration API returned
`Resource not accessible by integration`.

Consequently the mismatch was **not independently reproduced**, none of the seven rows could be
compared, and no root cause (benign reinstall variance versus integrity failure) is established.
No production normalization was added: broadening the RECORD exemption without evidence would
weaken the integrity boundary. The existing `files_manifest` implementation only canonicalises
rows naming `uv_cache.json` and `direct_url.json`, retaining every other row byte-for-byte; its
current 19-test suite passes but does not establish what changed in issue #170.

Validation on this checkout: `pixi run --frozen fmt`, `pixi run --frozen lint`, and
`pixi run --frozen test` all exited 0. Lint printed the known offline `cargo-deny` yanked-index
warnings because the crates.io index cannot be queried from this environment. The focused
`pixi run --frozen -- cargo test --offline -p pixi-sandbox-core --test files_manifest` also
passed (19/19). These validate current code only, not the unavailable issue artifact.

**Next required evidence:** an owner or connected session must provide access to the pinned
transport commit (or an archive containing its `files.json`, affected `RECORD` files, and
restored counterparts) and the consumer source/environment needed to reproduce the install.
Then compare the seven CSV files row-by-row, identify exact differences and provenance, and add
a narrowly scoped regression only if the differing rows are proven to be regenerated metadata;
otherwise retain the current fail-closed behavior and document the corruption diagnosis.

<!-- SECTION:NOTES:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Root cause and artifact evidence are recorded in this task.
- [ ] #2 Any code change is constrained to proven nondeterministic metadata and preserves integrity checks; if no safe code change is justified, the fail-closed result and next diagnostic step are recorded.
- [ ] #3 Formatting, lint, and workspace tests pass, or any unavailable external proof is explicitly left open with its evidence boundary named.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Obtain the consumer fixture and pinned transport commit from issue #170, without relying on the mutable published branch.
2. Reproduce final verification in an isolated fixture/worktree; compare the pack-time and restored RECORD bytes row by row, tracing changed entries to their source files and installer behavior.
3. Decide per the evidence whether a specific regenerated field is missing from the existing shared canonicalisation or whether verification correctly detects a real mismatch. Add a regression fixture and a tampering complement only for a proven safe normalization; otherwise preserve the failure and document the finding.
4. Run the task's validation gates, record exact outputs and proof limitations, and leave any unavailable connected-host confirmation explicit.
<!-- SECTION:PLAN:END -->
