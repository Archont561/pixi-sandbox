---
name: session
description: Start a pixi-sandbox work session on an airlocked machine — bring the offline environment up, survey the backlog, and propose what to do this session. Use at the start of any session on this repository, when the user says to initialize/bootstrap the environment, pick up tasks from the backlog, or asks what to work on.
---

# Session startup for pixi-sandbox

Two jobs, in this order: **(1)** bring the environment up, **(2)** survey the backlog and
propose the session. Never propose work you cannot execute — an environment that is down
changes what is possible.

## 1. Bring the environment up (airlocked machine)

The workspace is a clone of the repository; the pixi environments exist only after a restore.
Check first, restore only if needed:

```bash
test -f .pixi/sandbox-env.sh   # environment already restored?
# if missing — the one-time bootstrap (idempotent, passes --force itself, never fetches):
bash scripts/restore.sh
```

**Let the restore register the user tools** (that is the default, `PIXI_SANDBOX_USER_TOOLS=register`;
do not pass `skip` here — `skip` is for shared CI runners). When it registers, it writes managed
`pixi` and `pixi-sandbox` launchers into `~/.local/bin` and adds a marker-delimited PATH block to
the detected shell profile (task-33); the launchers exec the *manifest-verified* tool copies under
`.pixi/tools/<platform>/`.

### When you can stop sourcing `sandbox-env.sh` — and when you still cannot

**Read the last line the restore printed. It tells you which world you are in**, and it is a fact
about the tree, not a wish:

- `user tools: registered pixi and pixi-sandbox in <dir>` → the launchers exist.
- `user tools: NOT registered — the bundled pixi-sandbox <v> predates --user-tools (0.3.7)` →
  nothing was registered. Registration is a *restore-side* feature, and the binary doing the
  restore comes from the packed branch: a branch packed before 0.3.7 ignores the policy entirely
  (unknown env var, ignored by design). **This is the situation on the current published branch**
  — `sandbox/developer-linux-64` carries pixi-sandbox 0.3.6 — so until a 0.3.7+ transport is
  packed and published, `source .pixi/sandbox-env.sh` in every shell remains the only way.

Once a 0.3.7+ branch is restored, dropping the sourcing is safe when all three hold:

1. the restore printed the `registered` line (not `skip`, not a refusal over an unmanaged `pixi`
   already sitting in `~/.local/bin` — that refusal needs `--force`);
2. `~/.local/bin` is on `PATH` in the shell you are actually in;
3. everything you run is `pixi …` — i.e. every repo-management command goes through the pixi
   binary, not through a tool you expect to find loose on `PATH`.

Point 2 is the one that bites an agent: the profile block is read by a **new login/interactive**
shell, and a non-interactive `bash -c …` — which is what every tool invocation here is — reads
no profile at all. So instead of sourcing a path inside this checkout, prepend the user bin dir:

```bash
export PATH="$HOME/.local/bin:$PATH"   # once per shell; nothing checkout-specific about it
pixi --version && pixi sandbox --version
```

The same applies to the shell that *ran* the restore: a running parent shell cannot be mutated,
so use the `export` above for the rest of that session.

`.pixi/sandbox-env.sh` is now an **escape hatch, not a step**: it is only needed when you want
`cargo`, `rustc`, `bun`, `taplo` or `convco` loose on `PATH`. Don't — run them through pixi,
which is what the hooks and CI do.

### Every repo task goes through the pixi binary

```bash
pixi run --frozen fmt | lint | test | test-doc | coverage
pixi run --frozen -- cargo <anything>        # e.g. cargo check --offline, cargo nextest run -p …
pixi run --frozen -- convco check HEAD~1..HEAD
pixi run --frozen lint-commit < <file>       # what the commit-msg hook runs
pixi run docs-install                        # once; then the two below work
pixi run backlog task list -s "To Do" --plain
pixi run skills …
pixi run -- lefthook install                 # installs the git hooks in a fresh clone
```

Facts about this sandbox that shape every command:

- **Egress is filtered**, and the exact shape matters: github.com and the npm registry answer
  (so `git fetch`, `gh`, and `pixi run docs-install` all work), while the crates.io index/API,
  prefix.dev and static.rust-lang.org do not. Therefore **always** `--offline` for cargo — the
  vendored tree under `.pixi-sandbox/vendor` is what builds — and always `pixi run --frozen
  <task>` so pixi never tries to solve online. A restore is the only way to get a toolchain
  here; there is no rustup fallback.
- `pixi run --frozen test` runs in seconds once built — baseline it at session start and write
  the number down (206 passing / 1 skipped on 0.3.7; it must rise with new work, never fall).
  The first build after a restore costs about a minute.
- The airlock claim has a local proof and a CI proof, and they are not the same thing. Locally,
  `crates/pixi-sandbox/tests/e2e.rs` is the fixture-backed lifecycle (doctor → publish → restore
  → `doctor --verify-restored`), it needs no network, and on Linux it re-runs the restore inside
  `unshare -rn`; it is already part of `pixi run --frozen test`. The egress-denied gate over a
  *real* packed transport belongs to CI's cross-platform matrix
  (`.github/workflows/airlock.yml`) — never try to reproduce that tier locally.
  Today that gate is still `scripts/airlock-gate.sh`, driven from one e2e test; **task-35**
  moves it into `tests/e2e.rs` behind the `ci` cargo feature (skipped locally, run by the
  matrix) and deletes the script. Until that lands, add no new logic to the shell script.
- Missing system tools: no `/usr/bin/time`, no `file(1)` — use `date +%s`, `readelf`.
- `gh` works against github.com when authenticated; workflow dispatch/rerun may be
  forbidden for the token (`403 … by integration`) — then ask the user to click it.

## 2. Survey the backlog

1. **Sync before proposing.** `git fetch origin` (github.com is reachable), then confirm the
   working branch is based on `origin/main`'s tip — if it is behind, say so before anything
   else. Never edit on a stale base.
2. **Read the standing context** (skim, do not quote back): `AGENTS.md` (repo map, the nine
   invariants, task commands), `CONTEXT.md`, and `.knowledge/decisions.md` — the decisions
   D1–D15 are load-bearing; if you think one is wrong, bring a measurement, not an opinion.
3. **List the open work**:
   `pixi run backlog task list -s "To Do" --plain` — or read `backlog/tasks/*.md` directly;
   tasks are markdown files whose frontmatter carries `dependencies`, `priority`, `ordinal`,
   `type`, and `documentation` links (spec docs live under `backlog/docs/`).
4. **Filter honestly.** A task is a candidate only when every dependency has status `Done`.
   Order candidates by priority (high → low), then `ordinal`. A spike that unblocks several
   tasks may jump the queue — say so explicitly when you propose it.
5. **Do not re-propose finished work.** Check `git log --oneline -15`, recently merged PRs
   (`gh pr list --state merged --limit 5`), and the latest release tag. Done tasks, merged
   decisions, and published releases stay done; re-running a release that already exists is
   an error, not work.

## 3. Propose the session, then stop

Fill in the **Session proposal** template in [`standup-template.md`](standup-template.md) and
**wait for the user to pick** — do not start implementing. That file also carries the task
hand-off and session-close templates; use them at those points rather than inventing a shape.

## 4. While you work — house rules (AGENTS.md is the full list)

- **One focused conventional commit per task.** The hooks are split by cost, so match your
  rhythm to them:
  - while committing: `pixi run --frozen fmt`. `pre-commit` adds taplo, biome and actionlint
    over staged files only — no cargo, by design.
  - before pushing: `pre-push` runs clippy, the repo-consistency and generated-workflow xtask
    checks, and the whole-workspace test suite, in that order, stopping at the first failure.
    Run `pixi run --frozen lint && pixi run --frozen test` yourself first when you want the
    answer sooner, or when the change touches what only the full `lint` covers (`deny`,
    `sandbox-plan --json`, `lint-toml`, `lint-docs`).
  - message check: `pixi run --frozen -- convco check HEAD~1..HEAD`.
- **Complete the task file in its own format**: every AC `[x]`, status `Done`, bump
  `updated_date`, append `SECTION:NOTES` Implementation Notes and `SECTION:SUMMARY` Final
  Summary — following the task's exact existing section markers (no PLAN edits).
- **Tests target fixtures and tempdirs, never this repository or a real HOME**
  (D10; isolate `HOME`/`USERPROFILE`/`SHELL` per test).
- **Pull requests**: push the working branch, squash-merge, conventional title ending
  `(#N)`. Watch the required checks; when a platform job fails, read that job's log before
  changing code. After merge, watch the post-merge CI and docs runs.
