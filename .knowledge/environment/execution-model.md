---
type: Measurement
title: "Execution and Git Semantics"
description: How commands actually run here (no tty, stdin closed, no persistent background state) and the git/GitHub specifics that follow.
resource: https://github.com/Archont561/pixi-sandbox
tags: [environment, execution, git]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
verified: { by: process:sandbox-measurement, at: 2026-09-19T21:00:00Z }
legacy: { files: [`SANDBOX_CONSTRAINTS.md`], sections: ["6", "7"] }
---

# Execution and Git Semantics

## 6. Execution model (how commands actually run)

| Property | Behaviour |
|---|---|
| Shell | `bash`, **no controlling TTY** (`tty` → "not a tty"), **stdin closed** |
| `bash` tool timeout | default **30 s**, max **1800 s**; anything longer must be a `start_process` |
| State between calls | `cwd`, env vars, aliases, functions, shell history and **background processes do not survive** a call; only files and git state do |
| Long-lived servers | must use the process tools (`start_process` / `get_process_output` / `stop_process`); a `nohup … &` from `bash` is killed at the timeout |
| Locale / colour | `TERM` unset → tools disable colour; progress bars auto-off (`pixi` does this when stderr is not a TTY) |
| Interactive prompts | will hang or fail; every tool needs non-interactive flags (`-y`, `--no-input`, `--frozen`) |
| `git` identity | `user.name=Archont561`, `user.email=142521648+Archont561@users.noreply.github.com` (repo-local config) |
| Snapshot excludes | `.git/config`, `.git/credentials`, `.git-credentials`, `.netrc` are **not persisted** — do not stash credentials there expecting survival; also `.cache`, `.local`, `.venv`, `node_modules`, `dist`, `build`, `target`, `coverage`, `out`, `__pycache__`, `.next`, `.turbo`, `.pytest_cache`, `.ruff_cache`, etc. |
| Artifact budget | turn-end patchsets are best-effort capped at **≈128 MB / 10 000 files** combined → keep `target/`, `vendor/`, `pixi.lock`-scale caches and generated datasets out of git unless required |
| Persistence | only files under `/home/user` are snapshotted; `/tmp` is scratch (my probes wrote `/tmp/pixitest`, `/tmp/piptest` — those vanish) |
| Editing tools | `write_file` is whole-file; `edit_file` is fuzzy first-match — never rely on replacing "the first line" in a large file |

## 7. Git / GitHub specifics

| Item | Value |
|---|---|
| `origin` | `https://github.com/Archont561/pixi-sandbox.git` (HTTPS, token via `GH_TOKEN` helper) |
| `remote.origin.fetch` | `+refs/heads/main:refs/remotes/origin/main` — **only `main` is tracked**; other branches need an explicit refspec/fetch |
| Branch policy (session-level) | all work must land on `arena/01a0b663-pixi-sandbox`; pushing elsewhere desynchronises the session |
| Default branch on GitHub | `main` |
| Branch protection on `main` | `GET /branches/main/protection` → `403` (no access / not configured for this viewer) — do **not** assume protections exist or are absent; re-check from a maintainer token |
| Actions feasibility | CI YAML can be authored and pushed, but **Actions minutes on a private repo are metered** (2 000 min/mo + 500 MB storage on GitHub Free) and no result can be observed from inside without API reads |
| REST quota | 5 000/hr (`rate_limit` reported `used: 0` at probe time) |

---
