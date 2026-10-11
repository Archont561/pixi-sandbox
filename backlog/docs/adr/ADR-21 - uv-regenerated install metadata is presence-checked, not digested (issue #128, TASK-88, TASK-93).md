---
id: ADR-21
title: "uv-regenerated install metadata is presence-checked, not digested (issue #128, TASK-88, TASK-93)"
status: accepted
tags:
  - decision
---

# ADR-21 — uv-regenerated install metadata is presence-checked, not digested (issue #128, TASK-88, TASK-93)

**Decision.** Files at `*.dist-info/uv_cache.json` and `*.dist-info/direct_url.json` are
**presence-only** in the per-file oracle, exactly like a `conda-meta` record: listed, so a
missing or forged install is still caught, but carrying no content digest. A file at
`*.dist-info/RECORD` keeps its full content digest, computed with the rows describing those
regenerated files collapsed to a literal sentinel before hashing. Either metadata file may
additionally appear in a restored prefix without being listed. Both rules are computed by one
shared, path-aware entry point (`canonical_file_sha256`) that `scan_prefix` and
`check_restored_entry` both call, so the pack side and the verify side cannot drift. **No
manifest or `files.json` schema change.**

**Why.** `pixi-pack` transports a pypi dependency as the *wheel*, not as installed bytes
(ADR-2's unit is whatever `pixi-pack` emitted), so `pixi-unpack` **reinstalls** it with uv at
restore time. Three entries then vary with when and where that install ran: `uv_cache.json`
opens with a wall-clock `timestamp` field; `direct_url.json` records the local `file://` wheel
path uv resolved; and the package's own `RECORD` carries a `sha256=`/`size` row for each file.
The oracle recorded digests for bytes the restore path is *designed* to regenerate, so the
restored-tree check failed deterministically for every project with a pypi dependency — and
because `restore` exits non-zero after unpacking correctly, it also skipped user-tool
registration, and the documented next command failed with `pixi: command not found` (issue
#128, measured against `Archont561/qgis-rust`: 18 failures, two files × nine packages, while
a conda-only env in the same transport verified clean at 76366 entries).

Presence is the sound check for the same reason it already is for `conda-meta` records: the
content is a function of the installing host, not of the transport, so no digest over it could
mean "the tree the packer described". ADR-13's own honesty rule — a check that cannot be made
true is not a check — cuts both ways: an unsatisfiable digest is worse than no digest,
because it reports corruption where there is none and costs the consumer their exit code.

**Why `RECORD` is normalised rather than exempted.** Exactly the rows for uv's regenerated
metadata are collapsed. The other rows are the package's own installed files with their hashes
and sizes, and they are transported content. Exempting the whole file would trade a
deterministic false positive for a silent loss of coverage across every file of every pypi
package — a much larger hole than the one being closed, and one no test would notice.
Normalising only those rows keeps the check where it still means something; the guard test
(`a_changed_record_row_still_changes_the_digest`) fails if the rule is ever widened.

**Why the directory component is required.** `is_uv_install_metadata` matches only inside a
`*.dist-info/`. A file that merely shares the name elsewhere in the prefix is transported
content and keeps its digest — the rule names uv's install metadata, not a basename.

**Evidence.** TASK-88 added seven tests, suite 928/1 → 935/1 (core 212 → 219). The two
regression tests were confirmed to fail with the rules neutralised, so they test the defect
and not the implementation. The `restored::world()` fixture now carries a uv-installed wheel,
so every pre-existing restored-tree test runs against a prefix with pypi content instead of a
conda-only one. Compatibility is structural rather than tested-into-existence: presence-only
is carried by the *absence* of `h`, which is already how `conda-meta` records work, so an
older binary reading a list this one writes passes.

TASK-93 adds three tests, suite 982/1 → 985/1 (core 219 → 222): a `direct_url.json` row's
hash and size no longer alter its `RECORD` digest, the file itself scans as presence-only, and
a `verify_restored` fixture passes across different pack-time and restore-time local wheel
paths while the pre-existing payload-tampering complement still fails. Each new behaviour was
first run with the production extension neutralised and failed, then passed when restored.

**Honest limits.** An **already-packed transport keeps failing until it is re-packed**: its
recorded digest was computed over bytes the restore path regenerates, and no comparison can
rescue that. A lenient reading — ignore a recorded digest for either uv metadata file — would
still fail `RECORD`, which is not a coherent behaviour to ship, so the consequence is stated
instead of hidden. TASK-93's connected proof (a real `pixi-pack` → `pixi-unpack` round trip of
a wheel) is **not** produced on this airlocked host: `pixi lock` cannot reach `prefix.dev`.
Issue #128's third suggestion — registering user tools even when the final check fails — is
deliberately **not** here; it changes when `restore` mutates `$PATH`, which is a different
invariant and wants its own decision.

**What would change it.** A `pixi-pack` mode that transports installed pypi bytes rather than
wheels would remove the reinstall entirely and make the whole question moot. uv making either
metadata file reproducible (or dropping it) would let the matching row exemption narrow.
Evidence that a different installer regenerates further files inside `site-packages` would
extend the rule — and per-install metadata in other ecosystems is the plausible next case,
which is why the rule is expressed as "inside a `.dist-info`" rather than as a fixed filename
list.
