---
id: TASK-134
title: The airlock Tier B gate fails on every 2026-10-23 branch
status: Done
assignee: []
created_date: '2026-10-23 21:20'
labels:
  - ci
  - transport
  - airlock
dependencies: []
references:
  - .github/workflows/airlock.yml
priority: high
type: bug
ordinal: 134000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`airlock linux-64` fails at **Tier B — gate with the network reachable** (`Process completed with exit code 100`) on every branch after the v0.8.0 release — not only the two branches pushed on 2026-10-23:

| Run / job | Branch | Tier B |
| --- | --- | --- |
| `39833817145` / `120017862784` | 2026-10-22 `b4a9cf6a` | success (90 s) |
| `40008533829` / `120603532347` | `arena/5a48024f` (TASK-132, PR #196) | failure (0 s, exit 100) |
| `40024797158` / `120658973177` | `arena/542c12a9` (TASK-129, PR #199) | failure (0 s, exit 100) |

The job log is unreadable from this machine (`gh run view --log-failed` returns an EOF from job-log blob storage). Only the check-run annotation is retrievable, and it says exactly `Process completed with exit code 100.` — nextest's "the test binary ran and a test failed", so the launch path is fine and one test panics in milliseconds.

### Root cause 1 — the gate demanded a tool the native format removed

`crates/pixi-sandbox/tests/e2e.rs`'s `ci_gate` module required **three** manifest-owned tools — `pixi`, `pixi-sandbox` and `pixi-unpack` — in `assert_gate_tools_are_first_on_path_and_execute`. `pixi-unpack` stopped existing the moment v0.8.0 shipped the native migration (TASK-125, `rattler` 0.48 replacing `pixi-pack`/`pixi-unpack`): a format-1 transport is installed by rattler inside the process, so the packer embeds no unpacker. The published transport proves it — `.pixi-sandbox/manifest.json` at schema 4 declares `tools` = `pixi` + `pixi-sandbox` only.

So the gate demanded a binary that the transport's own restore path never uses. `gate_tool` panics before any restored prefix is examined, which is why the step dies in six milliseconds instead of the green run's ninety seconds.

### Why the boundary is v0.8.0, not TASK-129 or TASK-132

`resolve-release-tag` proves the version declared in `Cargo.toml`, and v0.8.0's assets were already downloadable as a draft at `2026-10-22T19:06:31Z` (published `19:27:27Z`). The first red run started `19:04:44Z`, whose plan job resolved the tag after that draft appeared; the plan job emitted no "declared but not published yet" notice, so it proved `v0.8.0`. The last green run (`39833817145`, started `18:43Z`) still proved `v0.7.0`, the last release with no native format. Nothing in TASK-129 or TASK-132 touches this: the gate is stale, and it went red on `main` with them.

### Root cause 2 — the gate was run against a *moved* pixi environment

With the tool contract fixed, Tier B still died in one second, now on the first test nextest
executes (`airlock_gate_cargo_check_uses_the_pixi_entrypoint`). The job log is unreadable here,
but check-run annotations are not, so the Tier B output was temporarily routed into an
annotation (commit `4453f5e4`, reverted in `3522163d` per the repo's "the gate is the proof"
rule). It said, exactly:

```
pixi run -- cargo check --offline failed with exit status: 1
x the environment directory has moved from
  '.../project/.pixi/envs/package' to '.../restored/.pixi/envs/package'
```

`cp -r project restored` copied the project's live `.pixi` into the restore destination. Pixi
environments are non-relocatable by design and pixi enforces it; `restore` replaces the
environments the transport carries (`default`) but knows nothing about the `package`
environment the job itself created with `pixi run -e package`, so the copy kept a stale
prefix path and the first `pixi run` inside it refused to work. The copy is the bug, not the
restore: an airlock reconstructs a checkout, not a fat clone.

### Fixes

`gate_transport_installs_natively` mirrors the product's `support::uses_native_pack` rule (schema ≥ 3 ⇒ native). `pixi` and `pixi-sandbox` stay mandatory; `pixi-unpack` is required only for a legacy transport, where it is still part of the tool contract and still executed. The native branch prints a notice naming the contract it checked, so a future format change cannot silently weaken the gate without saying so.

The restore destination is now seeded with `rsync -a --exclude .pixi`, so every environment
under `restored/.pixi` comes from the transport. That is also what makes the gate's
"`pixi install` is a no-op" assertion mean something: it can no longer pass because the host's
own environment leaked into the copy.

### Reproduction (Tier B command, verbatim)

Against the real published transport `sandbox/developer-linux-64` (`289d017c`, schema 4, 29,955 blobs):

```
pixi run --frozen airlock-gate-run /tmp/airlock-gate.tar.zst /tmp/restored /tmp/airlock-checkout default false
```

Before: exit 100, `panicked at crates/pixi-sandbox/tests/e2e.rs:573:5: bundled pixi-unpack is missing or not executable under /tmp/restored/.pixi/tools/linux-64`.
After: exit 0, `Summary [45.539s] 7 tests run: 7 passed, 6 skipped`.

### Proof

Run `40111562031` on branch head `3522163d`: `validate airlock plan` success, **`airlock linux-64` success** — Tier B (network reachable) and Tier A (egress denied through `xtask deny-egress`) both green for the first time since the v0.8.0 release. Local gates on the same tree: `cargo fmt --check` clean, `pixi run --frozen lint` 7/7, `pixi run --frozen test` 1,218 passed / 2 skipped (unchanged from the TASK-129 baseline).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The Tier B failure is reproduced locally with the same command the workflow runs, and the failing assertion is named.
- [x] #2 The cause is either fixed, or shown to be pre-existing and filed separately with the evidence that predates it.
- [x] #3 `airlock linux-64` is green on a branch that carries the fix.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 The fix is proven by the CI gate that actually runs it, not by a local substitute.
- [x] #2 fmt, lint and test stay green and the suite count does not drop.
<!-- DOD:END -->
