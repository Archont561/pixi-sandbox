# TASK127 — supporting-workflow evidence and remaining proof

Recorded 2026-10-22. This is an implementation/evidence ledger, not a measured savings claim.

## Baseline captured before changes

GitHub run/job/step metadata was read with `gh run list --workflow <file>` and
`gh api repos/Archont561/pixi-sandbox/actions/runs/<id>/jobs?per_page=100`.
Durations below are job timestamps, not invoiced minutes: span is earliest job start to
latest job completion (including scheduling/approval gaps); aggregate sums non-skipped job
durations (including setup/cache actions). Neither is GitHub's actual billed amount.

| Workflow / run | Source SHA | Verdict | Runner labels | Job span | Aggregate job time |
| --- | --- | --- | --- | ---: | ---: |
| [publish-sandbox.yml](https://github.com/Archont561/pixi-sandbox/actions/runs/39794596433) | `38228afc26cefe8bca71aed384730b045a247ece` | failure | ubuntu-latest | 330s | 327s |
| [airlock.yml](https://github.com/Archont561/pixi-sandbox/actions/runs/39791166856) | `a85859680de040b539593c72b2fc36c95b7f6619` | success | ubuntu-latest | 715s | 712s |
| [release.yml](https://github.com/Archont561/pixi-sandbox/actions/runs/38798448933) | `d54ae974f487e1946d059ee6707dc018724ee32b` | success | macos-14, macos-15-intel, ubuntu-24.04-arm, ubuntu-latest, windows-latest | 847s | 2120s |
| [auto-release.yml](https://github.com/Archont561/pixi-sandbox/actions/runs/38798313628) | `648f6435c030e62f0305dbc19bf1182d11d25b09` | success | ubuntu-latest | 56s | 56s |
| [starter.yml](https://github.com/Archont561/pixi-sandbox/actions/runs/38804692103) | `e16884a2b8615aeb2c2de7da8b0a3abca33898bb` | success | ubuntu-latest | 89s | 86s |
| [docs.yml](https://github.com/Archont561/pixi-sandbox/actions/runs/39794596726) | `38228afc26cefe8bca71aed384730b045a247ece` | success | ubuntu-latest | 354s | 152s |
| [relock.yml](https://github.com/Archont561/pixi-sandbox/actions/runs/39791166958) | `a85859680de040b539593c72b2fc36c95b7f6619` | success | ubuntu-latest | 12s | 12s |

Compiler versions, runner CPU model, exact cache hit/miss state, and billed minutes are not
present in the job metadata captured here. They are **unknown**, not assumed cold/warm. Release,
auto-release and starter rows predate the native runtime migration and cannot establish a
same-source performance comparison. The publisher is an explicitly failing baseline, not a
successful performance sample: source build succeeded, then Pack/verify/publish failed.
Its correctness issue remains separate; no failing verification was removed.

Selected observed step durations:

- Publisher source planner: 152s; source static bootstrap: 159s; failing pack/verify/publish: 3s.
- Airlock source release planner: 153s (cache action present; hit/miss unknown).
- Release Linux standalone: 48s; Linux Conda build: 50s. Consumer proof waited until all build
  legs ended, starting at 20:11:39Z, although the Linux build job ended at 20:03:02Z.
- Release Intel macOS standalone: 202s; Conda build: 284s. Early Linux proof cannot remove this
  unrelated critical-path cost, and must not release before its package smoke test passes.
- Starter dispatch policy: 24s; starter release verification: 34s, including compilation/API
  overhead rather than measured pure Rust compilation. Auto-release prepare step: 25s.
- Docs build: 7s; matching-source CLI snippet check: 115s in the separate docs job.
- Relock locked fetch: 5s. This is already cheap; source-download caching remains conservative.

## Implementation and trust boundaries

- Publisher/airlock matrices still come from the checked-out source, now built in the dev
  profile without LTO. The source-derived native matrix and override validation are unchanged.
  A plan executable is never transported: Linux transported/released binaries still use musl
  release builds and retain verification/consumer/airlock gates.
- Missing publisher/automation Rust caches are scoped by job purpose, native platform/target,
  toolchain environment and lock inputs. Swatinem rust-cache also fingerprints compiler and
  Cargo dependency/build configuration; explicit `pixi.lock` keys distinguish the Pixi toolchain
  where used. Source is rebuilt through Cargo every time: no `cache-hit` skips the build.
  Airlock PR runs may restore their accessible branch/base caches but do not save them.
  Native host automation, transported static binaries and CI coverage caches use separate
  shared-key namespaces. Caches are acceleration only, never artifact provenance.
- Linux release binary upload ends a dedicated job. Linux packaging and consumer lifecycle
  proof then run independently, alongside the other four native build/package jobs.
  Publication depends on **all** binary/package jobs and the successful consumer proof.
  Downloads remain scoped to this run; final five-platform completeness checks remain.
  The extra Linux packaging job adds runner setup/cache overhead: it is a latency scheduling
  change, **not** a claimed reduction in aggregate/billed minutes. Conda builds are not replaced
  with musl release artifacts: their build configurations are different.
- Main CI builds the site once and runs frontmatter, link, accessibility and matching-source
  CLI snippet checks before uploading a SHA-named artifact. Pages observes completed CI runs,
  accepting only successful main-push producers from this repository at the workflow's current
  source SHA. Explicit repository/run/SHA binding prevents selecting an unrelated artifact.
  The reuse job executes no checkout, shell or artifact code and has no Pages-write permission.
  Missing artifacts fail download and cannot reach deployment. Fork/PR producers and stale
  source SHAs are skipped without deployment. A main-only manual dispatch builds and checks its own
  source; it does not fall back from a failed or untrusted CI artifact.
  Deployment requires exactly one successful producer and retains the github-pages environment.
  Pages may wait for the full CI verdict: this favors provenance/compute reuse over earliest
  deployment latency. Manual fallback is deliberate and may duplicate a build if requested.
- Relock caches only registry index/archive downloads and Git source databases, not all of
  CARGO_HOME, credentials, configuration or target executables. OS/architecture/lock keys and
  GitHub's branch-scoped cache isolation apply. Both the guard and repair reuse downloads;
  locked validation, stale/edited/deleted lock checks, fork refusal, repair loop and verdict
  remain unchanged. It is generated in the template and re-rendered, never edited by hand.
- The small post-release asset-boundary consumer proof is unchanged.

## Lightweight automation decision

Defer a new crate split or cross-workflow executable handoff. The measured old starter dispatch
cost was 24s and prepare-release 25s, while the current native-runtime source planners were
152–153s optimized builds. First remove unnecessary optimized planning and add missing caches.
The historical starter/release data is not a current-native-graph cold/warm benchmark; it cannot
justify a cross-cutting library split. Splitting typed Git/release/consumer policies or handing
executables into privileged jobs would add API/provenance maintenance without a demonstrated
remaining same-source cost. Revisit after current-revision warm/cold runs identify the residual
bottleneck; AC6's evaluation is documented, not a claim that current compile cost was eliminated.

## Validation and open evidence

Local regression checks use the approved public generator and automation CLI boundaries.
They exercise success/refusal for incomplete release dependencies, ignored failed platform
legs, missing/wrong/cross-run artifact names, relaxed Pages producer identity, artifact execution,
missing quality gates and deployment bypasses. These are structural policy checks, **not**
simulations of real native or network-denied runners.

No production release, sandbox publication, starter push or protection change was triggered
for measurement. TASK125's new-native built-artifact proof is not claimed by these changes.

Still required before TASK127 can be Done:

1. Same-revision successful cold and warm supporting-workflow runs, compiler/cache identities,
   step/job timings and billed minutes where obtainable. Do not compare the failing publisher
   against success or use the pre-native release as measured savings.
2. Artifacts-only five-platform release run proving Linux proof overlap, each Conda smoke gate
   and final dependency verdict. A production tag/publication is not needed for that proof.
3. Actual native/network-denied airlock proof; retain the distinction from fixture success.
4. On-main Pages reuse plus explicit/manual fallback, wrong/missing artifact refusal, and
   successful publisher correctness baseline before publisher performance comparison.
5. Real warm cache reuse and invalidation on source/toolchain/build-flag changes.

Local command results and PR/native run links are recorded in TASK127 Implementation Notes as
available. Unknown/unrun evidence must stay unchecked rather than inferred from YAML or tests.

## Local results (2026-10-22)

- TASK128: four CLI characterization cases passed before extraction; 38 focused native/prefix
  cases passed after extraction; package Clippy passed; uncached workspace 1166 passed/2 skipped.
- TASK127 vertical slices observed red then green: missing relock download cache; missing CLI
  scheduling/trust checks; optional failed platform leg; missing repair download reuse; unsafe
  manual docs branch. Final focused generator checks: 7 passed; workflow CLI checks: 8 passed.
- Full lint: 11/11 successful, 0 cached (including actionlint, generated render, repository
  consistency, formatting, Cargo deny and package Clippy). The first attempt caught the missing
  AGENTS promoted-module entry from TASK128; it was corrected in that task's commit and rerun.
- Final uncached pre-push workspace suite: **1175 passed/2 skipped**, up from the 1154/2
  session baseline. All five normal pre-push gates passed; no hook bypass was used.
- Docs build/frontmatter/links/accessibility/snippets: passed; 29 built pages, 62 CLI commands
  and 123 flags verified. This is a local build, not evidence of live Pages artifact adoption.
- Dev and release planner JSON compared equal on the current config and explicit custom
  environment/platform/vendor overrides. No production transport was written or published.

Commands needing the larger native dependency graph used `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=0` and `CARGO_NET_OFFLINE=true`. Turbo used
`--env-mode=loose --force` so these settings actually reached Cargo and results were not cached.
These are sandbox disk accommodations, not CI performance measurements.

## Post-merge evidence and publisher incident follow-up (2026-10-22)

Merge SHA `97bb24c68475d7e29e519cb2fffb22abcd145a57`:

- CI [39813564355](https://github.com/Archont561/pixi-sandbox/actions/runs/39813564355)
  succeeded, including MSRV and Windows replacement. Linux job 707s; observed overall job span
  829s and aggregate job runtime 928s, not billed minutes.
- Pages [39816408308](https://github.com/Archont561/pixi-sandbox/actions/runs/39816408308)
  adopted the trusted same-SHA CI artifact, skipped the manual build, and deployed successfully.
  This supplies the previously missing real main-path adoption evidence; manual/refusal paths
  still have structural/local evidence, not newly dispatched live runs.
- Publisher [39813564308](https://github.com/Archont561/pixi-sandbox/actions/runs/39813564308)
  failed after the successful 2m40s static musl source compile. Frozen Pixi install succeeded;
  pack failed with `invalid locked Conda record: missing field build_number`. Doctor/publish
  did not run. This is a compact-lock decoding bug, not authentication or a compile failure.
  The follow-up has local public-boundary red/green coverage and retains verification gates;
  successful live publication remains required. Publisher caches now save on failure so a
  subsequent packing incident does not discard successful dependency compilation. Cache scopes,
  permissions, serialization and the source-fresh/static bootstrap requirement are unchanged.

See the TASK126 follow-up plan for main-CI measurements and optimizations. Failed/successful runs
are not comparable performance samples. TASK127 remains In Progress with native/cold/warm/billing
and publisher evidence outstanding.

## Live-evidence update after artifacts-only release (2026-10-22)

The previous missing publisher/main Pages/new-artifact correctness evidence is now supplied by
publisher 39824086542, CI 39824086495, Pages 39825855879 and release validation 39826041383 on
0d15c4d8. The release validates five native build/package/smoke legs and the freshly built Linux
consumer lifecycle; production publish was skipped. Early Linux proof overlapped the remaining
platform work. Airlock compatibility run 39822123949 remains an older-release adapter proof.

See `task-125-release-readiness.md` for the complete run links, exact scope and inspected native
manifest. The earlier "still required" list is historical: controlled cold/warm/cache invalidation,
billing and additional live fallback cases remain open; the successful publisher, same-SHA main
Pages reuse and artifacts-only native release validation are no longer unknown. No numerical
savings or all-platform offline-consumer claim is made.
