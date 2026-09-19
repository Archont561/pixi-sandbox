---
type: Playbook
title: "Publishing Your Environment: the sandbox-environment Template"
description: The copy-paste workflow that puts your repo's pixi environments on an orphan dist branch - the canonical integration format for pixi-sandbox, its five knobs, and the traps GitHub bakes into workflow files.
resource: https://github.com/Archont561/pixi-sandbox
tags: [workflow, template, integration, ci, dogfooding]
status: draft
confidence: mixed
generated: { by: arena-agent/agent-mode, at: 2026-09-19T10:30:00Z }
sources:
  - { id: githubcom-actions-runner-895, resource: https://github.com/actions/runner/issues/895, title: actions/runner #895 — the github context is not accessible from step.uses (expressions in `uses:` are rejected by the workflow interpreter) ✅ }
---

# Publishing Your Environment: the sandbox-environment Template

**The promise:** copy **one file** into your repository, edit the marked knobs, push to `main` — and your
pixi environments (plus, optionally, your vendored crate graph) are published to an orphan branch of *your*
repo, digest-pinned, and reconstructible on a machine whose only network access is `git`. The target side
needs nothing new: the `pixi-sandbox` binary is fetched **inside this workflow, in CI**, and ships *inside
the kit* it publishes — the sealed machine clones one branch and reassembles with **one command** (§5).

This workflow **is** the integration format. It is the same file `pixi-sandbox`'s own repository runs
([The Repository's Workflow Set §7](/spec/ci-workflows.md#7-sandbox-environmentyml--the-dogfood-publish-d4s-ci-half)),
so "how do I use it" and "how does the author use it" have one answer, and every improvement to the author's
dogfood loop ships to users as the same diff.

> [!IMPORTANT]
> **Status: draft.** The YAML below is the reviewed template shape, not a shipped file — the repo is
> markdown-only until M1, and the `pin` job's mechanics may be simplified once the action's real outputs
> settle ([The Action Shape](/spec/action-shape.md) already defines `manifest`, `revision`, `bytes` ✅).
> Everything the template *leans on* is verified: the four CI-push rules ✅, the setup-pixi knobs ✅, the
> measured `uses:`-is-a-literal limitation ✅ (runner #895), and the reconstruction semantics of the nine
> oracle outcomes ✅ ([Running the Action](/workflows/action-run.md#the-nine-outcomes-measured-in-this-sandbox)).

## 1. The template

```yaml
# .github/workflows/sandbox-environment.yml
# Publishes this repository's pixi environments to the orphan branch named below,
# then re-fetches and reconstructs them in a clean container — the publish is only
# believed after the verify job proves it. Copy, edit the KNOBs, push.
name: Sandbox environment
on:
  push:
    branches: [main]
    paths:                                  # KNOB ① — what can change the artifacts.
      - pixi.toml                           # (the lockfile→digest map is the full table:
      - pixi.lock                           #  /workflows/lockfile-digest-map.md)
      - .github/workflows/sandbox-environment.yml
  schedule:
    - cron: "0 3 1 * *"                     # KNOB ② — monthly warm-up + drift catch. Delete to disable.
  workflow_dispatch: {}

env:
  DIST_BRANCH: pixi-sandbox-dist            # KNOB ③ — the orphan branch the action pushes.
  ENVIRONMENTS: default                     # KNOB ④ — space-separated pixi environments to pack.
  PLATFORMS: linux-64                       # KNOB ⑤ — see the runner table below before adding more.

concurrency: { group: sandbox-dist, cancel-in-progress: false }  # serialize; never cancel mid-publish
permissions: { contents: write }            # exactly what pushing the branch needs — nothing more

jobs:
  publish:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2 — pin by SHA
      # THE ONE LITERAL LINE. GitHub does not interpolate expressions into `uses:`
      # (actions/runner#895 ✅), so the action ref cannot be an env knob. Pin a commit
      # SHA of Archont561/pixi-sandbox in production; @v1 tracks the floating major.
      - id: sandbox
        uses: Archont561/pixi-sandbox@v1
        with:
          branch: ${{ env.DIST_BRANCH }}
          environments: ${{ env.ENVIRONMENTS }}
          platforms: ${{ env.PLATFORMS }}
          ship-pixi: true
          # bundle-crates: true             # also ship vendor/ — only when targets must compile offline

  verify:
    needs: publish
    runs-on: ubuntu-latest
    container: { image: alpine:latest }     # git + tar only — the airlock rehearsal
    timeout-minutes: 15
    steps:
      - run: apk add --no-cache git         # the container is bare on purpose
      - name: fetch and verify what publish just pushed
        run: |
          git clone --depth 1 --single-branch --branch "$DIST_BRANCH" \
            "$GITHUB_SERVER_URL/$GITHUB_REPOSITORY" kit
          (cd kit && sha256sum -c SHA256SUMS)
      - name: reconstruct and prove it works
        run: |
          sh kit/assemble --print-rung      # the exact command an airlocked box runs — dogfooded, not a variant
          . ./.pixi/assemble.env
          pixi list                         # non-empty output is the floor
          pixi run -e default -- lint       # KNOB ⑥ — a real task proves more than a version

  pin:
    needs: [publish, verify]
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
      - name: record the dist revision on main
        run: |
          git fetch --depth 1 origin "$DIST_BRANCH"
          git show "origin/$DIST_BRANCH:dist-manifest.json" > sandbox.lock.json
          git config user.name  "pixi-sandbox-bot"
          git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
          git add sandbox.lock.json
          git diff --cached --quiet || \
            git commit -m "chore(dist): pin artifacts @ ${{ needs.publish.outputs.revision }} [skip ci]"
          git push
```

## 2. The knobs

| # | Knob | Default | When to change |
|---|---|---|---|
| ① | `paths:` filter | the two pixi files + this workflow | always — add `pixi-sandbox.toml` if you have one, `Cargo.toml`/`Cargo.lock` when `bundle-crates: true`. The full keying table is [One Lockfile, One Digest](/workflows/lockfile-digest-map.md); a missing input-side filter is the stale-artifact trap |
| ② | `schedule:` cron | monthly | delete for quiet repos; weekly if upstream drift matters to you |
| ③ | `DIST_BRANCH` | `pixi-sandbox-dist` | one branch per *audience*: `…-staging` for experiments — the action takes `branch:` verbatim ✅ |
| ④ | `ENVIRONMENTS` | `default` | the envs your `pixi.toml` defines; `pixi-sandbox environments --json` is the ground truth (M1) |
| ⑤ | `PLATFORMS` | `linux-64` | **one platform per runner** — a pack built on `ubuntu-latest` unpacks only on `linux-64` ✅ ([The Action Shape](/spec/action-shape.md)); `osx-arm64` needs `macos-14`, `win-64` needs `windows-latest`, and a platform with no runner is recorded `omitted[]` (D10), not failed |
| ⑥ | the verify task | `lint` | any task your `pixi.toml` defines; this is the job that turns "pushed" into "proven" |
| — | the `uses:` ref | `@v1` | pin the commit SHA you validated; [step 5 of the release flow](/spec/ci-workflows.md#5-releaseyml--ship-a-version-tag--marketplace) exists so a SHA always resolves to oracle-green binaries |

## 3. What you are accepting when you copy this (all four D19 rules, restated)

1. **`permissions: contents: write`** — the minimum that can push a branch; the workflow touches no
   secrets, and `GITHUB_TOKEN` pushes **do not retrigger workflows** ✅, so there is no loop.
2. **Fork pull requests never trigger it** — `on: push: branches: [main]` only, and forks have no write
   token anyway ✅ (D19 rule 3).
3. **The branch is force-updatable and its blobs are forever** ✅ (D19 rule 4) — payload budget numbers live
   in [Git as the Artifact Registry §10.4](/spec/git-registry.md#104-repo-bloat-is-the-real-cost--so-budget-it-explicitly); a
   typical python/rust workspace pack is tens of MB, not hundreds.
4. **The verify job is not optional decoration.** It re-fetches the branch through the *consumer's* door
   (clone → `sha256sum -c` *before* executing anything → reconstruct → run) — T6's semantics
   ([Running the Action](/workflows/action-run.md#the-nine-outcomes-measured-in-this-sandbox)) on every
   push. Deleting it turns the workflow into a publisher that trusts itself.

## 4. How this file relates to the author's own

The canonical user template lives **here** (this concept, and after M1 as a real file in the action repo).
The author's `.github/workflows/sandbox-environment.yml` is the *dogfood rendering* of it with exactly two
substitutions, marked in the file so the rewrite is mechanical:

| Marker in the author's file | Template value | Why it differs |
|---|---|---|
| `uses: ./  # [template: Archont561/pixi-sandbox@v1]` | the published ref | the author must exercise the *unreleased* action on every push — §7.7's strictest dogfood ✅ ([Dogfooding](/workflows/dogfooding.md)) |
| the extra `dogfood-released` job | absent | users have only one ref; the author re-runs the released `@v1` on cron as the G9 tier |

`ci.yml`'s L0 job carries the drift check: applying the two substitutions to the author's file must yield
exactly the fenced template above — "drift between the two renderings is a test failure, not a judgement
call", the same rule the [oracle vector table](/spec/testing-strategy.md#2-the-oracle-one-vector-table-two-implementations)
uses. The docs site renders this concept unchanged, so the copy-paste source of truth is one markdown file.

## 5. What you get on the far side: one command

The binary never travels to the airlock by itself — **nothing on the sealed machine is fetched except the
kit**. Your CI does the fetching: the action's first step pulls the digest-pinned `pixi-sandbox` binary
(from the author's `pixi-sandbox-bin` branch — a CI-only channel) and `publish` embeds a copy in the kit's
`bin/`, covered by `SHA256SUMS` like every other payload. The airlocked machine therefore clones **exactly
one branch** and runs **one command**:

```sh
git clone --depth 1 --single-branch --branch pixi-sandbox-dist https://github.com/<you>/<repo>.git kit
sh kit/assemble                  # add: --with-vendor · --env <name> · --print-rung · --promote-path
```

`kit/assemble` is a ~30-line POSIX **shim** the action writes next to the payloads (mode `100755`, its own
digest in `SHA256SUMS`). It has no logic of its own — three responsibilities:

1. `sha256sum -c SHA256SUMS` — integrity **before** executing anything; the check gates the lines that
   follow, so the shim's own execution is covered too (the oracle's step-1 rule ✅);
2. pick the host's binary: `bin/pixi-sandbox-<triple>` via `uname -m` — no arch names typed by humans, and
   one kit can carry both P0 triples;
3. `exec` it: `reconstruct --from <this kit> "$@"` — the rung ladder, `--print-rung`, `assemble.env` — all
   of it the assembler binary's contract
   ([The Assembler Binary §2](/spec/assembler-binary.md#2-the-one-command),
   [nine oracle outcomes](/workflows/action-run.md#the-nine-outcomes-measured-in-this-sandbox)).

After it finishes, `pixi run <task>` works: the command prints the source line for the current shell
(`. ./.pixi/assemble.env`), and `--promote-path` makes it permanent, idempotently. Windows has no POSIX `sh`
by default, so there the one command is the direct call the kit layout makes obvious:
`kit\bin\pixi-sandbox-x86_64-pc-windows-msvc.exe reconstruct --from kit`.

The `pin` job's `sandbox.lock.json` on `main` is what lets the box answer *"is my kit current?"* without
trying to build
([Republishing When a Lockfile Changes §3.3](/workflows/ci-republish.md#33-what-the-consumer-of-the-next-commit-types-nothing)),
and the manual ladder in [Reconstructing on an Airlocked Machine](/workflows/airlock-clone.md) remains what
it always was — the specification of what the command just did, and the fallback for a kit too old to carry
the shim.

## 6. Unproven here

* The whole template is 🚧 until M1 — no workflow has executed (the `assemble` shim included); the L0 layer
  (`val.py`, `js-yaml`) is the only thing this sandbox can run against it ✅.
* `env` context inside a step's `with:` is ⚠️ reasoned (documented context-availability, not re-verified
  here); if a future runner refuses it, the knobs degrade to literals on the `with:` lines — same file,
  no structure change.
* The `pin` job's `git show … > sandbox.lock.json` is the simple form ⚠️; if the M1 action grows a first-class
  `sandbox.lock.json` output, the job shrinks to consuming it.
