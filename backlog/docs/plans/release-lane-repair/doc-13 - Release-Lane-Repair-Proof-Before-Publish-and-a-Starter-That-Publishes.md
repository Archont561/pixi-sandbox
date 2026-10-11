---
id: doc-13
title: 'Release Lane Repair: Proof Before Publish, and a Starter That Publishes'
type: specification
created_date: '2026-10-11 10:30'
---

# Release Lane Repair: Proof Before Publish, and a Starter That Publishes

## Status

**Plan only — no code changed.** Every claim below was measured on 2026-10-11 against
`main` at `a79f82df` (v0.6.1) with the restored 0.6.1 binary; the commands that produced each
observation are quoted so a successor can re-derive rather than trust.

Three defects in the release lane, one ordering change, and one template addition. They are
planned together because they touch the same four files and because fixing one without the
others moves the failure rather than removing it.

## 1. What is broken

### 1.1 The starter publishes an empty tree, and the lane is green

`Archont561/pixi-sandbox-starter` at `main` (`945ed1a7`, "chore: regenerate starter for
pixi-sandbox v0.6.1", 2026-10-11T09:59:35Z) contains **one file**: `README.md`, 1893 bytes.
The tag `pixi-sandbox-v0.6.1` points at that same commit, so the immutable evidence tag seals
an empty template. Run `38130262129` — which produced it — completed `success` on both jobs.

Cause, with the line:

- `starter::publish` (`crates/xtask/src/starter.rs:782`) stages exactly
  `git.worktree_status_files(root)` and commits that list.
- `ShellGit::worktree_status_files` (`crates/pixi-sandbox-git/src/shell.rs:551`) runs
  `git status --porcelain --untracked-files=no` — untracked files are invisible to it.
- The starter repository's `Initial commit` (`d3b0ce07`) tracked only `README.md`, so all six
  generated files were untracked and never staged.

Reproduced locally with the real `xtask` against a bare-remote fixture seeded the same way:

```
git status --porcelain                      →  M README.md  + 6 × ??
git status --porcelain --untracked-files=no →  M README.md          ← all publish() sees
xtask starter-publish --dry-run             →  publishing 1 file(s) for v0.0.1
                                               would publish README.md
pushed clone, git ls-files                  →  README.md
tag created                                 →  pixi-sandbox-v0.0.1
```

Why no gate caught it: `starter-verify` asserts the *directory* holds `REQUIRED_FILES`
(`crates/xtask/src/starter.rs`, 7 entries) and it did, on the runner. Nothing compares the
*staged set* to the required set. `REQUIRED_FILES` also omits `restore.sh` entirely, so the
launcher is unverified even once staging is fixed.

### 1.2 `consumer proof` fails deterministically on v0.6.1+

Run `38130260877`, step `Render and regenerate consumer workflow`, exit code 1. The step's
line 88 asserts:

```
grep -F 'Archont561/pixi-sandbox/releases/download' .github/workflows/publish-sandbox.yml
```

TASK-76 replaced that raw `curl` with `pixi-sandbox fetch-release --dest … --version …`
(`crates/pixi-sandbox/src/generated/github_workflow.rs:172` and `:242`), so the generated file
contains no `releases/download` literal. Reproduced offline with the released 0.6.1 binary in a
tempdir fixture: `init --force` ×2 → 0, `cmp` → 0, the `releases/download` grep → **1**, the
`SHA256SUMS` grep → 0. The generated workflow is correct; the assertion is stale.

### 1.3 The release publishes without checking the binary set is complete

The conda side is gated: `xtask check-conda-platforms dist/conda` fails unless every platform
in `SUPPORTED_PLATFORMS` (`crates/xtask/src/conda_platforms.rs:27`, derived from
`Platform::ALL`) contributed exactly one package.

The binary side is not. `release_checksums` (`crates/xtask/src/release_assets.rs:161`) hashes
whatever `pixi-sandbox-*` files exist, bails only when that set is **empty**, then verifies each
*present* binary has a line. A release missing one platform therefore produces a green
`SHA256SUMS` and publishes. Run `38061777852` came within one artifact upload of exactly that:
`build x86_64-apple-darwin` failed with `Failed to CreateArtifact: … ENOTFOUND`, the publish job
was skipped, and no release existed until the failed job was re-run (green at 09:58:37Z, release
published 09:58:15Z with 5 binaries, 5 conda packages, `SHA256SUMS`).

## 2. Target topology

```
release.yml  (on: push tags v*.*.*, or dispatch with version)
├─ build (matrix ×5)              → binary-<target>, conda-<platform> artifacts
├─ consumer-proof                 needs: build
│    download-artifact: binary-x86_64-unknown-linux-musl     ← artifact, not release asset
│    permissions: contents: read      (job-level; it *replaces* the workflow-level block)
│    NOT in `environment: release`
│    init ×2 + cmp, then pack → doctor → publish → fetch → offline restore
└─ release (publish)              needs: [build, consumer-proof]
     download artifacts
     → xtask check-conda-platforms        (exists)
     → xtask check-release-binaries       (new — §3.3)
     → xtask release-checksums → prefix.dev → GitHub Release
     → dispatch docs, dispatch starter    (unchanged, post-publish)

consumer-proof.yml (on: release: published)   ← slimmed to the asset-boundary smoke
     curl releases/download/<tag>/{binary,SHA256SUMS} → sha256sum --check → --version

starter.yml (workflow_dispatch, from release.yml after the Release exists)  ← unchanged trigger
     scaffold (+ .devcontainer/) → init --script-path scripts/restore.sh → lock
     → init --check → actionlint → fresh-clone dev proof → verify → publish
```

Ordering rationale, per the owner's direction: the proof gates the publish, and the starter is
regenerated only after the release succeeds — which is already true today
(`starter.yml` header: "Triggered explicitly by release.yml after the GitHub Release exists";
observed 2026-10-11: release published 09:58:15Z → starter dispatched 09:58:19Z → starter commit
09:59:35Z) and stays true, because `starter.yml` resolves the tag against the published release.

What a pre-release proof **cannot** cover: the `releases/download/<tag>` asset boundary, which
by definition does not exist before publish. That boundary keeps a home — the slimmed
post-release smoke — and a red smoke blocks the *next* release by policy rather than its own.

Cost on the release critical path: the last four successful consumer-proof runs took 27–32 s
(`37516425291`: 19:06:15→19:06:43Z), against a ~5 min build.

## 3. Changes

### 3.1 `crates/xtask/src/starter.rs` — stage the required set, then prove it

Do **not** change `worktree_status_files`. It has two production callers —
`crates/xtask/src/prepare_release.rs:246` and `starter.rs:782` — and `prepare_release` needs the
tracked-only semantics to refuse unaccounted modifications. Changing the shared primitive would
silently widen the release-commit path.

Instead, in `publish`:

1. Build the stage list explicitly: `SCAFFOLD_FILES ∪ REQUIRED_FILES ∪ devcontainer paths`,
   plus any tracked deletion/modification `worktree_status_files` reports (so removing a file
   from the template still publishes).
2. `git.add_files` already accepts untracked paths (`git add --` with explicit paths), so no new
   git primitive is needed.
3. After staging, assert the staged set (`git diff --cached --name-only`) is a superset of
   `REQUIRED_FILES` and disjoint from `FORBIDDEN_PATHS`. Fail with the missing names — this is
   the gate that was absent, and it is the one that turns a silent README-only publish into a
   red lane.
4. `REQUIRED_FILES` grows from 7 to include `scripts/restore.sh`,
   `.devcontainer/devcontainer.json`, `.devcontainer/setup.sh`.
5. `SCAFFOLD_FILES` grows from 3 (`pixi.toml`, `.gitignore`, `README.md`) to add the two
   devcontainer files.

Tests (tempdir fixtures, D10 — never this checkout):

- a bare remote seeded with a README-only initial commit + a complete working tree publishes
  **all** required files, not one (regression test for §1.1; must fail against today's code);
- a working tree missing one required file is refused naming that file;
- a `.pixi/` present in the tree is never staged;
- a tracked file removed from the template is staged as a deletion.

### 3.2 `starter.yml` — `scripts/restore.sh`, devcontainer, pixi pin from the release

- Pass `--script-path scripts/restore.sh` to **both** `init` invocations (`--force` and
  `--check`), or `--check` reports drift against the root-level default. Verified working today
  with the released 0.6.1 binary, no core change needed:

  ```
  pixi-sandbox init --project-root <t> --script-path scripts/restore.sh --force
    → generated offline launcher at <t>/scripts/restore.sh
    → tree: .github/workflows/{publish-sandbox,relock}.yml, pixi-sandbox.toml, pixi.toml,
            scripts/restore.sh
    → upgrade lane carries: --script-path scripts/restore.sh
  ```

  `init`'s writer creates parent directories (`crates/pixi-sandbox/src/commands/init.rs:421`,
  `create_dir_all`), so `scripts/` needs no new plumbing. The default stays root `restore.sh`
  for ordinary consumers; only the starter opts into `scripts/`.
- Add a step that reads the pixi version the *release* pins and writes it to `$GITHUB_OUTPUT`
  (one-line `xtask starter-pixi-version -- --binary … --output "$GITHUB_OUTPUT"`, mirroring how
  `release.yml` writes `matrix=`), then pass it to `starter-scaffold` as `--pixi-version`.
  Source of truth is the released binary's own embedded lock — `pixi-sandbox tools list` prints
  `pixi 0.81.0` — so the template never hardcodes a version that the next release invalidates
  (`starter.yml`'s own contract: "This workflow never invents content").
- `.devcontainer/devcontainer.json`: `image: ghcr.io/prefix-dev/pixi:<that version>`,
  `postCreateCommand: bash .devcontainer/setup.sh`.
- `.devcontainer/setup.sh`: `git`, `gh`, and `pixi-sandbox` only — **no C toolchain**:

  ```bash
  pixi global install git gh
  pixi global install -c https://prefix.dev/archont561/archont561 -c conda-forge pixi-sandbox
  pixi install --locked
  ```

  (This repository's own `.devcontainer/setup.sh` installs `c-compiler` and `opencode`; the
  starter deliberately does not — it is a consumer template, not the sandbox's build
  environment.)
- Extend the evidence artifact's `path:` list with `starter/scripts/restore.sh` and
  `starter/.devcontainer/`.

### 3.3 `crates/xtask` — `check-release-binaries`

Mirror `conda_platforms.rs`: expected asset names from
`pixi_sandbox::self_update::assets::SUPPORTED_HOSTS` (`crates/pixi-sandbox/src/self_update/
assets.rs:19`, built from `Platform::ALL` in a const block; `xtask` already depends on
`pixi-sandbox`), so the list is derived once and cannot drift from the platform set. Report
every missing or unexpected name rather than the first. Wire it into the `release` job between
`check-conda-platforms` and `release-checksums`. Tests: fixture directories with all five,
four-of-five, and an unexpected extra.

### 3.4 `release.yml` + `consumer-proof.yml` — proof before publish

- New `consumer-proof` job: `needs: build`, `permissions: contents: read`, job-level block
  (it *replaces* the workflow-level `contents: write` / `actions: write` — the trap documented
  in the generated publisher and `relock.yml`), not in `environment: release`. It downloads
  `binary-x86_64-unknown-linux-musl` with `actions/download-artifact` and runs the lifecycle.
- `release` job: `needs: [build, consumer-proof]`.
- Drop the post-publish `dispatch-consumer-proof` step; keep `dispatch-starter` and
  `dispatch-docs` after `Create GitHub Release`.
- `consumer-proof.yml` keeps `on: release: published` and shrinks to the asset-boundary smoke
  (download asset + `SHA256SUMS`, `sha256sum --check`, `--version` under `env -i`), which is the
  only part that needs the release to exist. Its line-88 assertion is replaced by what the
  refactored template actually guarantees — the exact-version `fetch-release` bootstrap and the
  `SHA256SUMS` verification — plus a renderer contract test in
  `crates/pixi-sandbox/tests/generated_workflow.rs` so the assertion cannot drift from the
  template again.

**Shape decision (recommended):** port the consumer-proof recipe into `xtask` subcommands the
way `starter.rs` already did, and make the new job one-line steps. `starter.yml`'s header names
the alternative as the debt it avoided — "consumer-proof.yml proves the same recipe but spends
~60 lines of `run:` blocks behind `multiline-run-allowed` opt-outs" — and task-36's rule is that
a step is a pinned `uses:` or a one-line pixi task. The cheap interim is to move the existing
`run:` blocks verbatim into the new job; that ships the gate but carries the debt forward.

## 4. Slices

| # | slice | gates | provable here? |
| --- | --- | --- | --- |
| 1 | fix the stale `consumer-proof.yml` assertion + renderer contract test | `fmt`, `lint` (actionlint, generated-workflow, repository), `test` | yes |
| 2 | `starter::publish` stages the required set + staged-set assertion; `REQUIRED_FILES` grows | same, plus new xtask fixtures | yes (bare-remote fixtures) |
| 3 | `scripts/restore.sh` + `.devcontainer/` in the scaffold, pixi pin from `tools list` | same | yes |
| 4 | `xtask check-release-binaries` + wire into the publish job | same | yes |
| 5 | `consumer-proof` as a `needs: build` gate; slim `consumer-proof.yml` | actionlint, `lint-generated-workflow`, repository checks | yes (render/lint); the run itself is not |
| 6 | end-to-end: a real release, then inspect the starter tree and the proof verdict | a cut release | no — needs a release and owner sanction |

Baseline to hold at every commit: **935 passing / 1 skipped** (35 git / 219 core /
525 pixi-sandbox / 156 xtask), measured 2026-10-11 on the restored tree. It must rise, never
fall.

## 5. Owner actions this plan cannot do

1. **Delete the starter tag `pixi-sandbox-v0.6.1`.** It names an empty tree, and
   `publish_refusals` refuses an existing tag ("starter revisions are immutable"), so the
   corrected tree cannot be published for v0.6.1 while it stands.
2. **Confirm `PIXI_SANDBOX_STARTER_TOKEN` carries `Workflows: write` on the starter repository.**
   The README-only push succeeded; a push carrying `.github/workflows/*` into another repository
   is the case issue #101 was filed about. Unverified from this sandbox.
3. **Re-dispatch the starter lane** for v0.6.1 once 1–2 are done, or let the next release carry
   the corrected template.
4. **Sanction the release** that slice 6 needs.

## 6. Risks and open decisions

- **Devcontainer is scaffold-owned, not `init`-owned.** `init --check` will not police it, so
  `starter-scaffold` must re-render it every run from the release's pixi version, and
  `starter-verify` must require it. Acceptable, and cheaper than teaching `init` about
  devcontainers — but it means the starter has two generators, and the file list in
  `REQUIRED_FILES` is the only place that says so.
- **Artifact retention.** A pre-release gate consumes `binary-*` artifacts inside the same run,
  so the 7-day retention never bites. It would bite a design that reused merge-time artifacts at
  tag time — which is the reason this plan keeps the build tag-triggered rather than moving it
  to `push: [main]`.
- **The slimmed smoke is not a gate on its own release.** If it goes red, the release is already
  public. That is the honest cost of proving before publishing, and it is why the smoke stays in
  the tree rather than being deleted.
- **`--untracked-files=no` stays.** Worth a comment on `worktree_status_files` naming the starter
  bug, so the next caller does not read "status files" as "everything that would be committed".

## 7. Out of scope

TASK-73's owner-operated GitHub App proof (needs the App installed and a Castellan run),
TASK-88 AC#8 (needs a connected re-pack of `qgis-rust`'s transport), TASK-78 (conda packaging of
arbitrary payloads), and moving the release build to `push: [main]` — rejected in §6.
