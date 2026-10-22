# Native transport release readiness — 2026-10-22

## Version decision

Recommend **v0.8.0**, a breaking pre-1.0 minor release from v0.7.0.
`.versionrc` explicitly sets `preMajor: true`, and the published compatibility policy says
pre-1.0 breaking changes advance the minor. Removed CLI interfaces, explicit rather than implicit
bundling, schema-3 output and Rust 1.94 source MSRV are incompatible changes. This is not a patch;
1.0.0 would additionally promise a stable public API and is not justified just by this migration.
Wheel-only PyPI input already existed in v0.7.0; document it as a retained limit, not a new break.

Do not bump source manifests by hand or dispatch production from this cleanup. After merging and
checking this documentation change, preview explicitly:

```bash
gh workflow run auto-release.yml --ref main -f version=v0.8.0 -f dry-run=true
```

Inspect the prepared diff. The maintainer's separate real cut uses the same explicit version and
`dry-run=false`; auto-release commits/tags and the tag push starts release.yml. Do not manually
start a second publishing release for the same tag. Real auto-release requires the configured
workflow-capable RELEASE_PUSH_TOKEN. No credential/protection changes are proposed here.

`CHANGELOG.md` has a curated Unreleased summary for review; prepare-release regenerates the
versioned changelog from Conventional Commits. The migration guide is the durable runbook. A
breaking-change commit note should explicitly record the removed interfaces and MSRV so the
generated release notes cannot present this as merely a docs-only patch. Version manifests and
generated version stamps intentionally remain 0.7.0 until release preparation runs.

## Verified source and live evidence

All following executions target source merge **0d15c4d8b1564b1733774b0831b014551666739b**:

| Evidence | Outcome |
| --- | --- |
| [CI 39824086495](https://github.com/Archont561/pixi-sandbox/actions/runs/39824086495) | Success: strict lint, full workspace coverage, doctests, explicit connected payload/network-denied consumer step, docs gates, actual Rust 1.94 and Windows replacement |
| [Publisher 39824086542](https://github.com/Archont561/pixi-sandbox/actions/runs/39824086542) | Success: source-built static Linux bootstrap, real frozen install, native pack, doctor verification and transport publish |
| [Release validation 39826041383](https://github.com/Archont561/pixi-sandbox/actions/runs/39826041383) | Success: all five native binary/Conda package builds and smoke tests; fresh Linux artifact consumer lifecycle; publication correctly skipped |
| [Docs 39825855879](https://github.com/Archont561/pixi-sandbox/actions/runs/39825855879) | Success: main same-revision CI artifact adoption and Pages deployment; content correctness cleanup is in this branch |
| [Auto-release 39828722470](https://github.com/Archont561/pixi-sandbox/actions/runs/39828722470) | Success: preparation, prepared-repository validation and dry-run diff; commit/tag/push skipped |
| [Airlock 39822123949](https://github.com/Archont561/pixi-sandbox/actions/runs/39822123949) | Success on PR head 4a5955c3: immutable released-binary compatibility proof, not falsely counted as the new native binary proof |

The release consumer proof ran while unrelated native build/package legs were still in progress.
This demonstrates scheduling overlap, not a cold/warm cost comparison. Job API completion times
were observed to be normalized at workflow end; do not infer exclusive subtask timing from them.

The published sandbox manifest was inspected directly (GitHub blob
45e0df7c9619a6ac12d355408b39baa80ef489e5): schema 3, source.commit `0d15c4d`, only `pixi` and
`pixi-sandbox` tool entries. No pixi-pack/pixi-unpack dependency or executable is bundled.
Restore of that transport in this session verified 29,953 blobs and 5,516 restored file entries
with zero failures. The tools total was 75.2 MiB; no budget ceiling was relaxed.

Pre-cleanup local baseline: 1,186 pass / 2 ignored; exact ordinary/instrumented inventories
match 1,188 entries across 96 binaries. Current cleanup changes docs/comments plus the wheel-refusal diagnostic wording: an existing
public black-box CLI regression was tightened, observed failing on the old helper attribution,
then passed after changing only that message to "native packing". No installer algorithm,
CLI option, dependency graph, schema or release automation behavior is changed.

## Acceptance reconciliation

ADR-31 supersedes the original TASK125 helper-acquisition assumptions. Compare original Conda
archive/wheel bytes and installed/oracle behavior, not an identical helper-produced directory
layout. Record the unavoidable transitive reqwest and scoped runtime-policy exceptions rather
than claiming an empty reqwest graph. Preserve the independent payload builder and the explicit
legacy opaque reader. Historical tests were retired with owner approval; current counts are not
presented as unchanged from the old synthetic/helper suite.

TASK125 runtime evidence is now available. Documentation review/merge is the remaining delivery
step for this cleanup; mark completion with the merged docs and their validation, not with an
unsupported statement that all five platforms ran an offline consumer. All five did native build,
package and smoke; the fresh-binary offline lifecycle was Linux. Development workspace support
remains linux-64 with osx-arm64 lock coverage; release asset availability does not widen it.

TASK126/127 controlled warm/cold cache invalidation, starter/manual fallback live cases and billed
minute comparisons remain separate optimization evidence. Do not turn those into a release
correctness blocker or close them by claiming fixture checks prove live billing savings.

## Documentation changes

Correct the home page, design/repository map, transport layouts, restore ordering and per-prefix
transaction scope, glossary, configuration validation, diagnostic terminology and security
provenance claims. Remove obsolete helper flag passthrough advice, explain native wheel installation,
label old size numbers historical, and add the native migration/API/MSRV checklist. Preserve
legitimate format marker names, explicit overrides and historical composite-Action migration.

## Cleanup validation

- Normal frozen workspace formatting and full lint (7/7 Turbo gates), including deny, clippy,
  generated-workflow, publish-plan and repository consistency checks: passed.
- Full forced ordinary suite: 1,186 passed, 2 skipped; full instrumented workspace coverage:
  1,186 passed, 2 skipped; workspace doctests: passed (0 examples).
- The existing black-box wheel-refusal CLI test was observed red on the old `pixi-pack`
  attribution, then green on the native diagnostic; environment/package/location and no-output
  assertions remain intact. No new private seam or synthetic installer fixture was introduced.
- Docs production build, frontmatter, internal links, accessibility and command snippets: passed;
  29 pages, 62 CLI commands, 120 flags, zero failures. `git diff --check`: passed.

These are local cleanup checks. Normal PR CI and merge are still required before dispatching any
release; neither protected-gate success on an older source nor local checks bypasses that step.
