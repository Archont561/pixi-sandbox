---
id: TASK-117
title: Investigate PyPI RECORD mismatches after uv metadata canonicalisation
status: Done
assignee: []
created_date: '2026-10-16 00:00'
updated_date: '2026-10-16 21:33'
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
- [x] #1 Reproduce or otherwise inspect the issue's pinned transport and capture a per-row diff for each reported RECORD, identifying the exact differing fields/bytes and their provenance; record tool versions and whether the original reproduction is independently confirmed.
- [x] #2 Establish and document a root cause for all seven mismatch reports, including whether each is a reproducible restore-time transformation or an integrity/corruption failure; do not infer harmlessness from successful application tests alone.
- [x] #3 If the differences are legitimate regenerated data, add the narrowest shared pack/verify canonicalisation and regression coverage for the specific transformation, with a complement proving unrelated RECORD entries and package-payload tampering still fail. If they indicate corruption or cannot be safely normalised, retain fail-closed behavior and document the evidence and required follow-up instead.
- [x] #4 Validate the affected fixture or pinned artifact against the changed behavior, run `pixi run --frozen fmt`, `pixi run --frozen lint`, and `pixi run --frozen test`, and record exact results; do not claim a real connected round trip unless performed.
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

### 2026-10-16 — root cause proven on the pinned artifact; fix implemented, not landed

**Reproduced, independently, all seven.** The pinned transport `eb87ddadfeffd70dcaecb9fad912272c50805e09`
and consumer source `801a26ce6d4fd5559bfd0bad2e1e0ac4c9403966` were fetched from GitHub (the blocker of
the previous entry is gone) and restored with the transport's own tool: `pixi-sandbox` 0.7.0, `pixi`
0.81.0, `pixi-unpack` 0.7.11, Linux-64, Python 3.14.8. `restore --envs build` fails with 4 records and
`--envs default` with 3, and every *expected* digest equals the one the issue reports. The reported
"got" values differ only because the project path — and therefore the staging prefix length — differs.

**Per-row diff and provenance.** Every failing `RECORD` is a strict superset of its wheel's `RECORD`
(the wheels travel in the transport at `.pixi-sandbox/envs/<env>/pack/pypi/*.whl`): zero rows changed,
zero missing. The added rows are installer metadata (`INSTALLER`, `REQUESTED`, `direct_url.json`,
`uv_cache.json` — the last two already collapsed by #128/#93) plus one row per entry-point script:
`../../../bin/coverage`, `coverage-3.14`, `coverage3`, `hypothesis`, `mako-render`, `pytest-bdd` — six
rows across the four files in `build`, and one each for hypothesis, mako and pytest-bdd in `default`.
For each of those rows, the recorded sha256 is the hash of the script **as installed with the staging
prefix in its shebang** (measured row by row: `sha256(script with the staging prefix) == row hash`,
while the relocated on-disk script hashes differently). The script *file* passes verification because
canonicalisation replaces prefix spellings in content; a recorded digest cannot be canonicalised.

**Root cause, all seven, no corruption.** Installer-written entry-point rows are a second
prefix-dependent row class that the #128/#93 canonicalisation never covered. Two-sided proof from the
delivered bytes alone: pack builds its oracle by unpacking into
`<out>/.pixi-sandbox-verify-<env>/stage/<env>` (`pack.rs`), and the generated publisher passes
`--transport-dir "${{ runner.temp }}/pixi-sandbox-transport"`, so the pack-time prefix is
`/home/runner/work/_temp/pixi-sandbox-transport/.pixi-sandbox-verify-<env>/stage/<env>`. Substituting
that prefix into the delivered scripts and recomputing the rows reproduces the manifest's *expected*
digest exactly for all four `build` records; the restore-time staging prefix
(`<project>/.pixi/.restore-work/stage-<env>/<env>`) reproduces the reported *got* digest exactly; the
same holds for the three `default` records. Discriminator: only uv/pypi-installed dist-infos carrying
such a row fail (4 in `build`, 3 in `default`). The 13 / 22 conda-installed dist-infos that also have
`../../../bin/` rows match byte for byte, because their `RECORD` ships inside the `.conda` — which is
also why `coverage` is absent from the default set.

**Fix (AC#3).** `files_manifest` gains the second collapsed row class — a row naming a script directly
inside the prefix's script directory (`<prefix>/bin/<name>`, `<prefix>/Scripts/<name>` on Windows),
recognised by the exact spelling the installer writes (one `../` per component between the dist-info's
directory and the prefix), collapsing hash **and** size to `@pixi-sandbox-prefix-stamped@`. Row
presence, order, path and every other row stay compared, and the script file remains a fully
content-verified entry of the oracle.

**The rule travels in the document (measured, not assumed).** Applying the new class unconditionally
makes a fixed verifier report **17** failures on the untouched 0.7.0-packed transport — the original 4
plus 13 conda `RECORD`s whose recorded digests were written without the collapse — and would newly
fail a pre-fix transport that verifies clean today. So `DOC_SCHEMA` goes 1 → 2, `SUPPORTED_DOC_SCHEMAS`
accepts both, and `RecordRows::for_schema` selects the collapse (schema 1 is compared exactly as 0.7.0
compared it). The three `pack-reference` fixtures move to schema 2 (schema byte + blob digest only),
recorded as a deliberate exception in their README; the synthetic transport fixture stays schema 1 on
purpose, which keeps the older-generation read path exercised.

**Confirmed with the transport's own verifier.** The pack-time bytes are reconstructible, so the
root cause is not an inference: for every script row the recorded hash equals sha256 of the script as
staged under the *restore* prefix (verified row by row, size included), and swapping that prefix for
the pack-time one reproduces the manifest's expected digest. Rewriting *only* those rows — nothing
else in the tree — makes the transport's own 0.7.0 verifier accept both environments: 10,588 `build`
entries and 64,904 `default` entries, 0 failures each, where the same binary reported 4 and 3 before.
A control with one byte of the prefix moved puts exactly the touched `RECORD` back in the failure
list. That fixes the per-file inventory too: six rows in `build` (coverage ×3, hypothesis, mako,
pytest_bdd) and three in `default` (hypothesis, mako, pytest_bdd), all of them installer-written
console-script rows; no other byte of any other file differs, and no conda-shipped `RECORD` is
involved.

**Evidence against the real artifact** (fixed debug binary, `/home/user/repro/`):

* untouched transport → exactly the original 4 `build` failures with the same expected/got values: the
  fix changes nothing for a transport packed before it, which is also the honest limit of a
  verifier-only change;
* a simulated repack (`/home/user/repro/repack_proof.py`, oracle only: schema 2 plus the digests a
  fixed packer records, manifest blob size/sha256 updated, payload blobs untouched) →
  `doctor --verify-restored` reports `OK` for **build (10,588 entries)** and **default (64,904
  entries)**, exit 0;
* complements: tampering with `bin/coverage` — the very file whose row is collapsed — is still caught
  by its own entry; a changed payload row inside that `RECORD` and a changed payload file are still
  caught;
* the 0.7.0 binary refuses the schema-2 document loudly (`files manifest schema 2 is not supported`),
  which is the intended pairing: a transport carries the tool that restores it.

**Consequence for the report.** The seven failures cannot be repaired by a verifier alone; the
reported transport needs a **repack by a fixed tool** — the consumer's publisher, after its
`PIXI_SANDBOX_VERSION` pin moves to a release that carries this change.

**Validation.** Focused suite 19 → 26 tests. Workspace `pixi run --frozen test`: **1,198 passing / 2
skipped** (was 1,191 / 2; the +7 are the new rows-and-schema tests). `pixi run --frozen fmt` and
`pixi run --frozen lint` (11 tasks, forced re-run) both exit 0. No real connected round trip was
performed and none is claimed: the repack simulation rewrites the oracle by hand, it is not a pipeline
pack.

**Closed on this branch's pull request.** Every acceptance criterion above is checked on evidence
gathered on this machine, and the fix lands through the pull request this branch carries, so the task
is `Done` there rather than left open on an unpushed commit. What remains afterwards is the path that
repairs the *reported artifact*: a release carrying this change, then the consumer's publisher
raising its `PIXI_SANDBOX_VERSION` pin so the transport is repacked by a fixed tool — a verifier-only
change cannot repair bytes a 0.7.0 packer hashed. Also deliberately not done here: the answer on
issue #170 (it accompanies the landing), a line in `create-verify.mdx` saying a pre-fix transport must
be repacked, and an ADR-13 note recording the collapsed row classes (the precedent — #128/#93 —
recorded theirs in the module docs only).

<!-- SECTION:NOTES:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Root cause and artifact evidence are recorded in this task.
- [x] #2 Any code change is constrained to proven nondeterministic metadata and preserves integrity checks; if no safe code change is justified, the fail-closed result and next diagnostic step are recorded.
- [x] #3 Formatting, lint, and workspace tests pass, or any unavailable external proof is explicitly left open with its evidence boundary named.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Obtain the consumer fixture and pinned transport commit from issue #170, without relying on the mutable published branch.
2. Reproduce final verification in an isolated fixture/worktree; compare the pack-time and restored RECORD bytes row by row, tracing changed entries to their source files and installer behavior.
3. Decide per the evidence whether a specific regenerated field is missing from the existing shared canonicalisation or whether verification correctly detects a real mismatch. Add a regression fixture and a tampering complement only for a proven safe normalization; otherwise preserve the failure and document the finding.
4. Run the task's validation gates, record exact outputs and proof limitations, and leave any unavailable connected-host confirmation explicit.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
The `*.dist-info/RECORD` rows that the *installer* writes for entry-point scripts — `../../../bin/<name>`
on unix, `../../Scripts/<name>.exe` on Windows — record the sha256 **and size** of a script whose
shebang names the install prefix. Pack's verification unpack and the airlock's restore stage into
different prefixes, and a digest cannot be canonicalised by replacing path spellings, so those rows
made seven `RECORD`s of the pinned transport unreproducible while every file they named verified
(issue #170). The oracle now models them as their own row class: presence, order, the path field and
every other row stay compared, the two recorded fields collapse to `@pixi-sandbox-prefix-stamped@` —
a sentinel distinct from the prefix-path and uv-metadata ones — and the script file itself remains a
fully content-verified entry of the list.

The rule is chosen by the document that recorded the digests: the file list gains `schema: 2`,
`RecordRows::for_schema` maps schema 1 to the pre-#170 rule and schema 2 to the new one, and a newer
document is refused rather than misread under the wrong rule. That is what keeps the change from
rewriting history — measured on the pinned artifact, the untouched 0.7.0-packed transport still
reports exactly its original four `build` failures with byte-identical expected digests, while a
transport whose file list is rewritten to schema 2 verifies 10,588 `build` and 64,904 `default`
entries with zero failures. The complements hold on that same tree: appending to `bin/coverage` — the
very file whose row is collapsed — to a payload file, or to a payload row inside that `RECORD`, is
caught three times out of three, one failure apiece.

The limit is stated rather than hidden: a transport packed before this change cannot be repaired by a
reading side, only by a repack with a fixed tool — the consumer's publisher does that on its next
release. Focused suite 19 → 26 tests; workspace `pixi run --frozen test` **1,198 passing / 2 skipped**
(from 1,191/2), `pixi run --frozen fmt` and `pixi run --frozen lint` clean. Only the unix `bin/` case
is measured against the real artifact; the Windows `Scripts/` spelling collapses through the same
mechanism and is covered synthetically.
<!-- SECTION:SUMMARY:END -->
