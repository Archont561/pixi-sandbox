---
type: Design Spec
title: "CI Design"
description: The workflow set, triggers, pins and permissions that build, verify and publish kits.
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, ci]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`DESIGN.md`], sections: ["12"] }
sources:
  - { id: githubcom-prefix-dev-setup-pixi, resource: https://github.com/prefix-dev/setup-pixi, title: prefix-dev/setup-pixi }
---

# CI Design

## 12. CI design

```yaml
name: ci
on:
  pull_request:
    branches: [main]
    types: [opened, synchronize, reopened, ready_for_review]
  push:
    branches: [main]
concurrency:
  group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
---
permissions:
  contents: read
```

Jobs:

| Job | Contents | Why it's there |
|---|---|---|
| `lint` | `cargo fmt --check`, `cargo clippy -- -D warnings`, `taplo fmt --check`, `prettier --check`, `zizmor`, `typos` | mirrors upstream pixi-pack's lint feature set ✅ verified; cheap, catches everything I can't compile-check locally |
| `test` | `cargo test --locked --all-features`, `--offline` where `vendor/` exists | proves §8.2's offline claim in CI, not just in docs |
| `plan-golden` | run every verb with `--dry-run --json`, diff against `tests/golden/*.json` | the **only** behavioural test obtainable without the sub-tools; this is what makes a plan/apply split worth its cost |
| `pack` | `setup-pixi` → `pixi install --frozen` → `pixi sandbox pack --all-envs` → `sha256sum -c` → `pixi sandbox dist push --no-push` (dry) | exercises pixi-pack for real on `linux-64/osx-arm64/win-64` |
| `detect-golden` | run `inventory`/`environments`/`platforms` against `tests/fixtures/{bare,pyproject,alien-workspace,multi-env}/` and diff JSON | the **only** guard for requirements 1–3: detection is a spec, and specs rot silently. Includes a fixture whose `win-64` is deliberately missing (the bun case, §7.4) |
| `reconstruct-e2e` | build a kit → `container: { image: alpine:latest }` with **only `git` + `tar` + `zstd`**, no `pixi` on PATH → clone dist branch → `bootstrap.sh` → `reconstruct --mode auto` → `pixi run <trivial-task>` → assert `printenv` sees the env | proves (e) end-to-end, and which rung (R1–R5) actually survived. **Three containers, one per rung family** (R1 cache copy · R2 `file://` channel · R5 tar-only), because one container cannot hold all three states. Alpine/busybox are *deliberate*: if it works there, `tar`-only R5 is real |
| `vendor-sync` | `cargo vendor --locked --sync vendor && git diff --exit-code vendor` | **the** guard against the stale-vendor class of failure |
| `docs` (`if: [docs].enabled`) | `pixi run docs-check` = sync → `astro build` → assert `pages ≥ *.md count` → `starlight-links-validator` | the validator fails the build on 104 findings ✅ measured — and it is the *only* checker here that follows `FILE.md#anchor` across files, which `val.py` cannot do alone |
| `build×3` | `x86_64-unknown-linux-musl` (musl-tools), `aarch64-apple-darwin`, `x86_64-pc-windows-msvc` | static-musl is what makes the git-branch install path work on any Linux |
| `dist` (needs build×3, `if: github.ref == 'refs/heads/main'`) | `pixi sandbox dist push --with-self` | self-hosting: the repo publishes its own binary to its own dist branch |
| `airlock-sim` | same three containers, but every step runs behind `HTTPS_PROXY` to a deny-all proxy allowlisting only the hosts in the egress fixture, **and asserts the negatives** (`curl -sSf https://static.crates.io/…` must fail) | a "passed" test with unrestricted egress proves nothing about an airlock; the negative assertion is what makes the simulation honest (🚧 whether hosted runners allow this, see D17) |
| `dogfood` | `git clone --depth 1 --branch pixi-sandbox-dist` of *this repo* → `sha256sum -c` → `bootstrap.sh` → `pixi-sandbox kit verify` using **the released binary, not the one just built** | G9: yesterday's tool must still do today's job; this is the only job that fails when the *format* changes incompatibly rather than the code |
| `mirror-refresh` (schedule, default branch only ✅) | `pixi sandbox mirror selfhost --check` → compares `bin/pixi-*` against the recorded upstream digests + `pixi --version`; opens an issue body on drift | ⚠️ `schedule` never runs on non-default branches ✅ — so this job is meaningless on a feature branch, which is worth saying out loud in the file |

Facts baked in from the research: pin actions **by commit SHA** (`actions/checkout@<sha> # v7.0.1` etc.,
copied from upstream's real pins ✅); `cache-write: ${{ github.event_name == 'push' && github.ref_name ==
'main' }}` to protect the **10 GB** cache; matrix ≤ 256 combos and 6 h/job are hard ceilings; **private
repos get 2 000 min/month** so `pack` should `paths:`-filter to `pixi.toml`, `pixi.lock`,
`pixi-sandbox.toml`, `Cargo.*`, `crates/**`; and the repo here is **private**, so assume the minute budget
is the binding constraint, not concurrency. ✅ [setup-pixi](https://github.com/prefix-dev/setup-pixi),
[limits](/research/github-actions.md#7-github-actions--branch-constraints).

`pull_request_target` is **not** used anywhere; if a fork-PR flow is ever needed, it must carry the
`# zizmor: ignore[dangerous-triggers]` + justification comment upstream uses ✅ verified, and stay
read-only.

---
