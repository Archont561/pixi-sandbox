---
type: Source Notes
title: "convco: conventional-commit linting and changelog generation"
description: The convco CLI (check/changelog/version/commit) from upstream docs and its conda-forge packaging - the tool this repository adopts for commit linting and release notes, via pixi and the conda-forge channel.
resource: https://github.com/Archont561/pixi-sandbox
tags: [research, convco, conventional-commits, changelog, ci]
status: stable
confidence: mixed
generated: { by: arena-agent/agent-mode, at: 2026-09-19T11:10:00Z }
stale_after: 2026-12-19T00:00:00Z
sources:
  - { id: convco-github-io, resource: https://convco.github.io/, title: convco documentation site — commands, install, the official GitHub Actions recipe ✅ }
  - { id: convco-github-io-check, resource: https://convco.github.io/check/, title: convco check — full usage output ✅ }
  - { id: convco-github-io-changelog, resource: https://convco.github.io/changelog/, title: convco changelog — full usage output ✅ }
  - { id: githubcom-conda-forge-convco-feedstock, resource: https://github.com/conda-forge/convco-feedstock, title: conda-forge/convco-feedstock — v0.7.2 bot-automerge merged 2026-09-03 ✅ }
  - { id: githubcom-convco-convco, resource: https://github.com/convco/convco, title: convco/convco — source repository }
  - { id: docsrs-crate-convco, resource: https://docs.rs/crate/convco/latest, title: docs.rs convco — 0.6.4 (2026-05-24) as crawled; the feedstock is ahead at 0.7.2 ⚠️ }
---

# convco: conventional-commit linting and changelog generation

**Adopted 2026-09-19** (user: "add convco via the conda-forge channel via pixi, so I have changelog and
commit linting done for repo"). This concept records what the tool is, what its packaging actually offers,
and how it slots into [The Repository's Workflow Set](/spec/ci-workflows.md) — the `lint` feature gets a
`commit-lint` job, `release.yml` gets its release notes from the same commit history the guard just
validated. One convention (`type[scope]: description`), three consumers: PR guard, changelog, semver.

> [!NOTE]
> **Everything on this page is read from upstream docs and the feedstock — nothing has been executed
> here.** The sandbox cannot install convco: `conda.anaconda.org` and `prefix.dev` are unreachable ❌
> (measured, [Network Model](/environment/network-model.md)) and there is no `pixi` on this box ✅. Flag
> tables below are ✅ **verified** (copied from the tool's own `--help` output published on
> convco.github.io); the integration sketches are ⚠️ **reasoned** until the first `pixi install` in CI.

## The tool (verified from upstream docs)

`convco` is a single-binary Rust CLI for [Conventional Commits](https://www.conventionalcommits.org/).
It ships musl-static release archives per target, a Docker image (`convco/convco`, also on ghcr.io),
Homebrew, and `cargo install` ✅ — the same single-static-binary distribution shape as pixi-pack and this
project's own assembler.

| Command | What it does | Flags this repo uses |
|---|---|---|
| `convco check [REV]` | exits non-zero when any commit in range breaks the convention; `REV` may be a range `<commit>..<commit>` | `--merges` (include merge commits), `--ignore-reverts`, `--ignore-message-pattern <re>` (+ `CONVCO_IGNORE_MESSAGE_PATTERN`), `--from-stdin`, `-n/--max-count`, `-C <path>` |
| `convco changelog [REV]` | renders a changelog from tagged conventional commits (handlebars template, default ships with the tool; config follows the conventional-changelog config spec) | `-o/--output <file>`, `-s/--skip-empty`, `-m/--max-versions <n>`, `-u/--unreleased [title]`, `-p/--prefix` (default `v`), `--paths` (monorepo pathspecs), `--ignore-prereleases`, `--no-links` |
| `convco version` | prints the current version (latest tag); `--bump` computes the **next** major/minor/patch from the commits since | `--bump`, `--major/--minor/--patch` (overrule the convention) |
| `convco commit` | interactive helper that *writes* a conventional commit | (developer-side; not used in CI) |
| `convco config --default` | prints the effective/default configuration | the way to discover every config key before editing `.convco` |

## conda-forge packaging (verified from the feedstock)

* **Current version: `0.7.2`** — bot-automerge PR merged **2026-09-03** ✅. Note the crawl discrepancy:
  docs.rs/crates.io snippets captured `0.6.4` (2026-05-24) ⚠️ — the feedstock is ahead of the crawled
  docs pages, so pin with a floor (`convco >=0.7`), never a hard equality.
* **All six conda subdirs build: `linux-64`, `linux-aarch64`, `linux-ppc64le`, `osx-64`, `osx-arm64`,
  `win-64`** ✅ (read from the feedstock's `.ci_support/` matrix). Contrast [node-bun](/research/node-bun.md):
  bun has **no win-64** build, convco does — a `lint` feature that carries convco suffers no D10
  `omitted[]` degradation on any platform this repo's runners use.
* Install is the D8 conda-first pattern, one line: `pixi add convco` (channel `conda-forge`, which is
  already this workspace's channel ✅).

## How this repository uses it (⚠️ reasoned until the first pixi install)

**1. The pixi feature** ([The Repository's Workflow Set §2](/spec/ci-workflows.md#2-the-workspace-the-workflows-assume)):

```toml
[feature.lint.dependencies]
convco = ">=0.7"                      # conda-forge; all six subdirs ✅ — no win-64 gap

[feature.lint.tasks]
commit-lint = { cmd = "convco check" }              # range appended by the caller, e.g. origin/main..HEAD
changelog   = { cmd = "convco changelog -s --output CHANGELOG.md" }
```

**2. The CI guard** — a dedicated `commit-lint` job in `ci.yml`, PR-only, using the range from the event
payload (the range form is exactly the official recipe's
`${{ github.event.pull_request.base.sha }}..${{ github.event.pull_request.head.sha }}` ✅):

```yaml
commit-lint:
  if: github.event_name == 'pull_request'
  steps:
    - { uses: actions/checkout@<sha>, with: { fetch-depth: 0 } }   # the range needs both ends
    - { uses: ./.github/actions/setup-env, with: { environments: lint } }
    - run: pixi run -e lint commit-lint -- "${{ github.event.pull_request.base.sha }}..${{ github.event.pull_request.head.sha }}"
```

**3. Release notes and the semver gate** — `release.yml` stops generating prose by hand: notes come from
`convco changelog -s -m 1` at the tag (the newest version section, exactly), and the gate set gains a
convco cross-check — the pushed tag must satisfy `convco version` at the tag, and *should* match
`convco version --bump` computed over the commits since the previous tag; a tag that jumps a minor
without a `feat!` is precisely the accident this catches (a soft warning is the documented fallback if
that gate ever fights a deliberate skip).

**4. The repo convention file** — lands with the code phase (markdown-only until M1 ✅); sketch, keys to be
verified against `convco config --default` on the first real run:

```yaml
# .convco  — verify every key against `convco config --default` before trusting it ⚠️
scopes: []            # free-form scopes; tighten to the bundle's area names when the repo has code
```

**Why the pixi route and not the official recipe's install:** the docs' GitHub Actions example
`curl`s a tarball from GitHub release assets and untars it by hand ✅ — that works in CI, but it puts a
second dependency channel next to the one `setup-pixi` already caches, and release-asset fetching is
precisely the transport this project's constraint set distrusts (measured blocked in the airlock ❌,
[Constraints](/overview/constraints.md#3-constraints-that-shape-the-design)). `pixi add convco` keeps
one dependency manager, one lockfile assertion (`pixi.lock` pins the convco build like everything else),
and one cache. The tool's own single-binary philosophy is untouched — conda-forge just becomes its
distributor here.

## What is unproven here

* **convco has never run in this sandbox** 🚧 — no conda route ❌, so exit codes, the exact `FAIL` output
  shape, and the `pixi run` argument passthrough for the commit range are doc-verified ⚠️, not measured.
  The first `commit-lint` CI job is the experiment; if `--` is needed before the range, the fix is one line.
* **The `.convco` config sketch is unvalidated** 🚧 — the docs enumerate flags, not every config key;
  `convco config --default` is the ground truth, available on the first installable machine.
* **The conda-forge version pin `>=0.7`** ⚠️ — chosen from the feedstock's 0.7.2 automerge; `pixi.lock`
  will freeze the actual build, which is the real pin.
