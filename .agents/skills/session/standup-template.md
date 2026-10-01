# Session lifecycle templates

Four artifacts, one per phase, in the order a session produces them: the **opening prompt** that
starts it, the **standup** that proposes it, the **hand-off** that ends each task inside it, and
the **report** that closes it once the PR has merged. Fill them in literally — same headings,
same order. They are the shape a reviewer expects, and keeping them out of `SKILL.md` means
editing a template never touches the procedure.

The loop closes on itself: the report's last block **is** the next session's opening prompt. A
session that ends without writing one has handed the next session a reconstruction job.

## 1. Session opening prompt (written at close, pasted back at the next start)

```
Restore the sandbox and baseline the suite (expect <N> passing / <M> skipped — <the one or two
environment facts that would otherwise waste the first ten minutes: which pixi-sandbox packed
the published transport, whether anything new needs vendoring before the tree builds offline>),
then read `CONTEXT.md` § Session scratchpad — <the dated heading> lists <K> open items.

<Optional, when the session's first move depends on something only the remote knows: the one
command to run, and what each outcome means. "First, one look at the repo state: `gh release
list`. If vX.Y.Z exists, task-<n>'s last open path is proven by its release.yml run — read the
run, check AC#<k>, complete the task file and close it. If it does not exist, leave the task
In Progress and say so.">

I want to take task-<n> this session — <one line on the shape, plus every decision already made,
spelled out, so the agent does not re-open settled ground>. In slices: <the locally provable
ones> first, <the ones needing pushes, releases or native runners> last, which needs <the
sanction you are reserving>.

Propose the slice and stop. House rules are in `AGENTS.md` (invariant 10: anything you do not
implement goes in `CONTEXT.md`, not into the files it speculates about), the session procedure
and its templates are in `.agents/skills/session/`.
```

What makes this prompt work, and what makes it fail:

- **Name the expected test count.** It is the cheapest possible check that the restore produced
  the tree the last session left; a mismatch is the first thing worth saying out loud.
- **State the sanction boundary.** Which slices may be pushed, and which wait for a click. An
  agent that has to guess will either stall or push something you did not want pushed.
- **Carry the decisions forward, not the deliberation.** "Method A, already decided" saves a
  round trip; "we should decide between A and B" costs one.
- **Point at the scratchpad heading by date**, not at "the scratchpad" — the file grows, and the
  entry that matters is one of several.

## 2. Session standup (end of startup — then stop and wait)

```
Session proposal — <date>

Environment: restored; baseline <N> tests passing, <M> skipped.
User tools: <registered in ~/.local/bin | NOT registered — bundled pixi-sandbox <v> predates 0.3.7>.

Backlog: <X> To Do, <Y> unblocked. Candidates, in recommended order:
1. task-<n> (<priority>, <type>) — <one line: what it delivers and why now>
2. task-<m> …
   …
Not this session: task-<k> (blocked by task-<j>); task-<l> (deferred — <one-line reason>).

Slices: <the locally provable work> first; <anything needing a push, a release or a native
runner> last, and only on your sanction.
Decisions I need before starting: <the ones that change the shape of the work, with a
recommendation each — or "none">.

Per task, "done" means: its acceptance criteria checked, one focused conventional commit,
gates green (fmt, lint, test), task file completed in house format.
Need from you: confirm the scope (or pick differently) before I start.
```

## 3. Task hand-off (end of a task, before the commit)

```
task-<n> — <title>

Changed: <file> (<one line why>), …
Evidence: <the command that proves it> → <result>. Suite <N> passing / <M> skipped (was <N₀>/<M₀>).
Gates: fmt ✓ lint ✓ test ✓ convco ✓
Left undone: <anything an AC does not cover, or "nothing">.
```

## 4. Session report (after the PR merges)

```
Session report — <date>

Merged: PR #<N> "<squash title>" → main at <sha>.
Post-merge runs: ci <verdict, duration>, docs <verdict>, <any path-triggered workflow> <verdict>.
Landed: <commit subject> (<task-id>), …

Tasks: task-<n> Done — every AC checked. task-<m> still In Progress — AC#<k> needs <the proof
this machine cannot produce: a native runner, a cut release, a maintainer's click>.
Suite on merged main: <N> passing / <M> skipped (was <N₀>/<M₀>). Gates: <n> lint gates green.

Recorded in CONTEXT.md: <the headings appended this session>.
Open, in the order a session should consider them: <task id — one line on why it is next, or
what blocks it>, …
Environment facts for next time: <what a restore will print, what the transport carries, what
is not vendored yet>.

Next session should start with:

> <template 1, filled in>
```

A session that produced ideas but no commits still writes a report — "Merged: nothing" is a
result. The ideas go to `CONTEXT.md` § Session scratchpad, never into the file they speculate
about, and the opening prompt says what the next session should decide first.
