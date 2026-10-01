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
test -f .pixi/sandbox-env.sh            # environment already restored?
# if missing — the one-time bootstrap (idempotent, passes --force itself, never fetches):
PIXI_SANDBOX_USER_TOOLS=skip bash scripts/restore.sh   # skip = leave HOME untouched
```

Then, in **every** new shell / tool invocation — state does not survive between calls:

```bash
source .pixi/sandbox-env.sh
```

Facts about this sandbox that shape every command:

- **Egress is filtered**: the crates.io index/API and conda channels (prefix.dev) are
  unreachable; github.com and static file hosts work. Therefore **always** `cargo --offline`,
  and always `pixi run --frozen <task>` so pixi never tries to solve online.
- Everything goes through pixi tasks: `pixi run --frozen fmt` / `lint` / `test`. `test` runs
  in seconds — baseline it at session start and write the number down (it must rise with new
  work, never fall).
- `pixi run docs-install` once, then `pixi run backlog …` and `pixi run skills …` work.
- `convco` (conventional-commit linter) is in the default environment — available after
  sourcing `sandbox-env.sh`.
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

Present a short proposal and **wait for the user to pick** — do not start implementing.

```
Session proposal — <date>

Environment: restored; baseline <N> tests passing, <M> skipped.

Backlog: <X> To Do, <Y> unblocked. Candidates, in recommended order:
1. task-<n> (<priority>, <type>) — <one line: what it delivers and why now>
2. task-<m> …
   …
Not this session: task-<k> (blocked by task-<j>); task-<l> (deferred — <one-line reason>).

Per task, "done" means: its acceptance criteria checked, one focused conventional commit,
gates green (fmt, lint, test), task file completed in house format.
Need from you: confirm the scope (or pick differently) before I start.
```

## 4. While you work — house rules (AGENTS.md is the full list)

- **One focused conventional commit per task.** Before every commit, run manually:
  `pixi run --frozen fmt && pixi run --frozen lint && pixi run --frozen test`, then
  `convco check HEAD~1..HEAD`.
- **Complete the task file in its own format**: every AC `[x]`, status `Done`, bump
  `updated_date`, append `SECTION:NOTES` Implementation Notes and `SECTION:SUMMARY` Final
  Summary — following the task's exact existing section markers (no PLAN edits).
- **Tests target fixtures and tempdirs, never this repository or a real HOME**
  (D10; isolate `HOME`/`USERPROFILE`/`SHELL` per test).
- **Pull requests**: push the working branch, squash-merge, conventional title ending
  `(#N)`. Watch the required checks; when a platform job fails, read that job's log before
  changing code. After merge, watch the post-merge CI and docs runs.
