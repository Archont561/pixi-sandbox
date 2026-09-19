---
type: Source Notes
title: "GitHub Actions limits and branch rules"
description: Quotas, pinning, permissions, caching and the branch/ref constraints that bound the CI design.
resource: https://github.com/Archont561/pixi-sandbox
tags: [research, ci]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`WEB_SEARCH_RESULTS.md`], sections: ["7"] }
stale_after: 2026-12-19T00:00:00Z
sources:
  - { id: yonatankracom-7-github-actions-tricks-i-wish-i-knew-, resource: https://yonatankra.com/7-github-actions-tricks-i-wish-i-knew-before-i-started/, title: yonatankra.com/7-github-actions-tricks-i-wish-i-knew-before- }
  - { id: githubcom-orgs-community, resource: https://github.com/orgs/community/discussions/48858, title: orgs/community — discussions/48858 }
  - { id: stackoverflowcom-questions-58139406, resource: https://stackoverflow.com/questions/58139406/only-run-job-on-specific-branch-with-github-actions, title: stackoverflow.com/questions/58139406/only-run-job-on-specifi }
  - { id: mediumcom-alexivenin-understanding-and-overcoming-li, resource: https://medium.com/@alex.ivenin/understanding-and-overcoming-limitations-of-github-actions-52956e9e2823, title: medium.com/@alex.ivenin/understanding-and-overcoming-limitat }
  - { id: oneuptimecom-blog-post, resource: https://oneuptime.com/blog/post/2026-01-25-github-actions-concurrency-control/view, title: oneuptime.com/blog/post/2026-01-25-github-actions-concurrenc }
  - { id: githubcom-orgs-community, resource: https://github.com/orgs/community/discussions/184661, title: orgs/community — discussions/184661 }
  - { id: oneuptimecom-blog-post, resource: https://oneuptime.com/blog/post/2025-12-20-concurrency-control-github-actions/view, title: oneuptime.com/blog/post/2025-12-20-concurrency-control-githu }
  - { id: docsgithubcom-en-repositories, resource: https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches, title: docs.github.com/en/repositories/configuring-branches-and-mer }
---

# GitHub Actions limits and branch rules

## 7. GitHub Actions & branch constraints

### 7.1 Trigger-level filtering

```yaml
on:
  push:
    branches: [main]            # ignore: ['release/**'], paths/paths-ignore also available
    tags: ['v*']
  pull_request:
    branches: [main]
    types: [opened, synchronize, reopened, ready_for_review, converted_to_draft]
  workflow_dispatch:
```

* `pull_request: branches: [main]` filters the **base** branch; it *also* triggers on pushes to the PR's
  head branch [4](https://yonatankra.com/7-github-actions-tricks-i-wish-i-knew-before-i-started/) — a
  classic source of double CI.
* A single `on:` key cannot hold two different `push:` blocks; `if:` at trigger level is not a thing — people
  resort to `startsWith(github.ref, 'refs/tags/')` / `contains(github.event.head_commit.modified, …)` at
  job level [5](https://github.com/orgs/community/discussions/48858).
* **Workflow discovery is default-branch-anchored:** `schedule` and `workflow_dispatch` only work from the
  workflow file present on the **default branch**; a workflow added/edited only on a feature branch won't
  fire on a schedule ("I created a separate branch and tested there… it worked only once pushed to default")
  [3](https://stackoverflow.com/questions/58139406/only-run-job-on-specific-branch-with-github-actions).
  `schedule` is also **throttled/unreliable** (≥5 min, delayed or skipped under load)
  [5](https://medium.com/@alex.ivenin/understanding-and-overcoming-limitations-of-github-actions-52956e9e2823).
* `github.ref_name == 'main'` guards are still needed for deploy jobs:
  `if: github.event_name == 'push' && github.ref == 'refs/heads/main'`
  [1](https://oneuptime.com/blog/post/2026-01-25-github-actions-concurrency-control/view).
* **Fork PRs**: secrets are withheld and workflow runs need approval from someone with write access
  [3](https://github.com/orgs/community/discussions/184661). `pull_request_target` gives write-capable
  context to foreign code — treat as a security flag (pixi-pack's CI literally annotates it:
  `pull_request_target: # zizmor: ignore[dangerous-triggers] safe and needed for PRs from forks`
  ✅ verified locally in `.github/workflows/chore.yml`).

### 7.2 Concurrency per branch

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}   # cancel on PRs, queue on main
```
That expression pattern (PR → cancel; default branch → don't cancel a deploy) is the recommended default
[1](https://oneuptime.com/blog/post/2026-01-25-github-actions-concurrency-control/view),
[2](https://oneuptime.com/blog/post/2025-12-20-concurrency-control-github-actions/view); job-level
`concurrency` with `queue: max` serialises shared resources such as migrations
[1](https://oneuptime.com/blog/post/2026-01-25-github-actions-concurrency-control/view). ✅ pixi-pack uses
exactly `group: ${{ github.workflow }}-${{ github.ref }}` + `cancel-in-progress: true`.

### 7.3 Branch protection / rulesets

Available protections: required PR reviews, **required status checks** (strict = branch must be up to
date; loose = not required to be current; disabled), conversation resolution, **signed commits**,
**linear history**, merge queue, required deployments, lock branch, "do not allow bypassing", restrict who
can push, allow force pushes, allow deletions
[4](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches).
A required status check can be **bound to a specific GitHub App** as the expected source of the check,
otherwise "any source" is accepted [4](https://docs.github.com/…). Enforcement is
**server-side, not git-side** — a blocked push looks like a plain git error (`GH006`) even though the
commits are fine, and **the check name must match exactly** or PRs are blocked forever
[2](https://learn.programmingline.com/learn/git/github-branch-protection).
Rulesets are the modern layering mechanism (org-wide, pattern-based, **bypass lists for bots/apps**, and
as of 2026-05-07 user bypass + branch renaming)
[3](https://github.com/orgs/community/discussions/43460),
[5](https://topictrick.com/blog/github-branch-protection-policies).
> For agent/bot workflows the practical rule is: protect `main`, **allow the bot to bypass on
> `arena/*`-style branches**, and never let an agent force-push a protected branch.

### 7.4 Hard platform limits (matter when designing a pixi/rust matrix)

| Limit | Value |
|---|---|
| Concurrent jobs | Free **20**; Pro 40; Team 60 / Enterprise 180 [1](https://timesofcloud.com/github-actions/pricing-and-limits/), [3](https://github.com/orgs/community/discussions/184661) |
| Job duration | **6 h** hard cap on GitHub-hosted runners (all plans); self-hosted `timeout-minutes` up to **4320** [2](https://markaicode.com/errors/github-actions-timeout-fix/) |
| Whole run | 35 days [1](https://timesofcloud.com/github-actions/pricing-and-limits/) |
| Matrix | **256** job combinations per workflow [1](https://timesofcloud.com/github-actions/pricing-and-limits/) |
| Workflow file | 512 KB [1](https://timesofcloud.com/github-actions/pricing-and-limits/) |
| API | 1 000 requests/hour per repo [1](https://timesofcloud.com/github-actions/pricing-and-limits/) |
| Cache | **10 GB per repo** (+ ~200 cache uploads/min as of 2026-01) [3](https://github.com/orgs/community/discussions/184661) |
| Private-repo minutes | 2 000/mo + 500 MB storage (Free), 3 000 + 2 GB (Pro) [1](https://timesofcloud.com/github-actions/pricing-and-limits/) |
| Artifacts | default retention 90 d; storage quota recalculation lags 6–72 h — "deleted artifacts, still blocked" [4](https://github.com/orgs/community/discussions/169789) |
| Self-hosted | $0.002/min for private/enterprise repos starting 2026-03-01 [3](https://github.com/orgs/community/discussions/184661) |

Because pixi caches are large, the documented mitigation is to **only write caches from the default
branch** [5](https://github.com/Pe44e/setup-pixi), and the ecosystem even ships helpers to keep the
lockfile **out of git** and cache it instead (`Parcels-code/pixi-lock` `create-and-cache` / `restore`
pair, keyed on `pixi.toml` + date) [3](https://github.com/xarray-contrib/pixi-lock).

### 7.5 `prefix-dev/setup-pixi` — the CI primitive

```yaml
- uses: prefix-dev/setup-pixi@v0.10.2
  with:
    pixi-version: v0.81.0          # pin, or 'latest'/'vX.Y' — pin it
    cache: true                    # default on when pixi.lock exists (hashed from the lock)
    cache-write: ${{ github.event_name == 'push' && github.ref_name == 'main' }}
    environments: py311 py312     # else only 'default' is installed+cached  (gotcha!)
    locked: true                  # or frozen: true  → pixi install --locked / --frozen
    auth-host: prefix.dev
    auth-token: ${{ secrets.PREFIX_DEV_TOKEN }}
---
    persist-credentials: false    # runs `pixi auth logout` right after install
    activate-environment: py311
    working-directory: ./packages/my-project   # monorepos
    run-install: false            # only install pixi itself
    # offline / air-gapped:
    pixi-url: https://pixi-mirror.example.com/releases/download/v0.81.0/pixi-x86_64-unknown-linux-musl
    pixi-url-headers: '{"Authorization": "Bearer ${{ secrets.PIXI_MIRROR_BEARER_TOKEN }}"}'
```
[1](https://github.com/prefix-dev/setup-pixi), [2](http://pixi.prefix.dev/v0.48.1/integration/ci/github_actions/),
[5](https://github.com/Pe44e/setup-pixi)

`pixi-url` is the **air-gap lever**: it lets CI install pixi itself from a mirror instead of GitHub
releases. Global-environment caching has no lockfile, so it **expires at the end of each month**
[1](https://github.com/prefix-dev/setup-pixi). On self-hosted runners: `post-cleanup`, and
`pixi-bin-path: ${{ runner.temp }}/bin/pixi` to avoid polluting the runner
[2](http://pixi.prefix.dev/v0.48.1/integration/ci/github_actions/). Docs advice, verbatim in spirit:
**"Pin your action versions"** — and the real-world repos do it by full commit SHA
(`actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1`)
✅ verified locally in pixi-pack's workflows, together with `permissions: read-all` at workflow level and
per-job `contents: read` / `pull-requests: write`, `zizmor` and `codeql` + `scorecard` jobs.

---
