# Session lifecycle templates

Four artifacts, one per phase, in the order a session produces them: the **opening prompt** that
starts it, the **sprint proposal** that opens it, the **hand-off** that ends each task inside it,
and the **sprint report** that closes it once the release has run. Fill them in literally — same
headings, same order. They are the shape a reviewer expects, and keeping them out of `SKILL.md`
means editing a template never touches the procedure.

A session is a sprint by default (SKILL.md §0): one milestone, one version, one release. A
task-long session still uses §3 and §4, and says in its §2 that it is deliberately narrower.

The loop closes on itself: the report's last block **is** the next session's opening prompt. A
session that ends without writing one has handed the next session a reconstruction job.

The hand-off happens **in chat**: both artifacts are printed in the final message after the
merge. No follow-up `docs(context)` PR is opened for them — the scratchpad entry that feeds the
report rides the work PR before it merges, and the post-merge facts live in the printed prompt
(SKILL.md §5b, steps 4–5).

## 1. Session opening prompt (written at close, pasted back at the next start)

```
Restore the sandbox and baseline the suite (expect <N> passing / <M> skipped — <the one or two
environment facts that would otherwise waste the first ten minutes: which pixi-sandbox packed
the published transport, whether anything new needs vendoring before the tree builds offline>),
then read `CONTEXT.md` § Session scratchpad — <the dated heading> lists <K> open items.

This session is sprint <n> of milestone m-<n> (<version>) — <one line naming the milestone's
theme and its exit criteria>. The plan is `backlog/docs/plans/doc-<n> …`: stage A is the spike
(<the one unknown it settles>), stage B is <tasks>, stage C is the gate (<the proof that
counts>), stage D is the release (<selector> then the milestone tag). <Carry every decision
already made, spelled out, so the agent does not re-open settled ground.>

<Optional, when the first move depends on something only the remote knows: the one command to
run, and what each outcome means. "First, `gh release list --limit 3`. If vX.Y.Z is already
there, the sprint is closed — check the milestone, update it, and propose the next sprint
instead.">

Stage D needs <the sanction you are reserving: the merge, the tag push, the dispatch>. Propose
the sprint and stop. House rules are in `AGENTS.md` (invariant 10: anything you do not implement
goes in `CONTEXT.md`), the procedure and these templates are in `.agents/skills/session/`.
```

What makes this prompt work, and what makes it fail:

- **Name the expected test count.** It is the cheapest possible check that the restore produced
  the tree the last session left; a mismatch is the first thing worth saying out loud.
- **Name the milestone and the version.** A sprint prompt that only lists tasks lets the session
  end after the merge, with no release.
- **State the sanction boundary.** Which stages may be pushed, and which wait for a click — the
  tag push and the release dispatch are exactly where an agent should not guess.
- **Carry the decisions forward, not the deliberation.** "Tag vocabulary `milestone-vX.Y.Z`,
  already decided" saves a round trip; "we should decide between the two" costs one.
- **Point at the scratchpad heading by date**, not at "the scratchpad" — the file grows, and the
  entry that matters is one of several.

## 2. Sprint proposal (end of startup — then stop and wait)

```
Sprint proposal — <date>

Environment: restored; baseline <N> tests passing, <M> skipped.
User tools: <registered in ~/.local/bin | NOT registered — bundled pixi-sandbox <v> predates 0.3.7>.

Milestone: m-<n> — <version>, "<theme>" (backlog/milestones/m-<n> …). Plan: doc-<n> ….
Board: <X> To Do, <Y> unblocked. The sprint's tasks, by stage:
  A — spike:  task-<n> (<the unknown it settles, and the timebox>)
  B — build:  task-<m>, task-<k>
  C — gate:   <the proof that counts: fmt/lint/suite + airlock linux-64 on the release branch>
  D — release: prepare-release -- <selector>, release PR, merge, milestone tag, release runs
Not this sprint: task-<k> (blocked by task-<j>); task-<l> (belongs to m-<n+1>, after its own spike).

What the sprint needs first: <milestone attachment on the tasks; the release-gate task;
  doc-18 §5.2's tag trigger; a task closed that is already finished> — each one a board or
  workflow edit I am not making without your word.

Slices: <the locally provable ones> first; <anything needing a push, a release or a native
runner> last, and only on your sanction.
Decisions I need before starting: <the ones that change the shape of the sprint — tag
  vocabulary, scope, whether the release automation lands here — with a recommendation each,
  or "none">.

Per task, "done" means: its acceptance criteria checked, one focused conventional commit,
gates green (fmt, lint, test), task file completed in house format. Per sprint, "done" means:
the milestone tag ran the release lane and the milestone file is closed.

Task-long instead? <only when the user asked for one task, or when the next proof needs several
workflow runs, a native runner or a maintainer's click — name which, and say what it blocks.>

Need from you: confirm the sprint scope (or pick differently) before I start.
```

## 3. Task hand-off (end of a task, before the commit)

```
task-<n> — <title>   [sprint stage: <A|B|C>]

Changed: <file> (<one line why>), …
Evidence: <the command that proves it> → <result>. Suite <N> passing / <M> skipped (was <N₀>/<M₀>).
Gates: fmt ✓ lint ✓ test ✓ convco ✓
Left undone: <anything an AC does not cover, or "nothing">.
```

## 4. Sprint report (after the release has run)

```
Sprint report — <date>

Sprint: <n> of milestone m-<n> (<version>) — <theme>.
Merged: PR #<N> "<pull request title>" → main at <sha>.  (or "Merged: nothing")
Release: <vX.Y.Z published by release.yml run <id> | not cut — <the blocker> and run <id>>.
Release assets: <five static binaries + conda packages + SHA256SUMS verified>.
Milestone: m-<n> <closed | still open — <the exit criterion that is not met>>.
Post-merge runs: ci <verdict, duration>, docs <verdict>, airlock <verdict both tiers>,
publish sandbox <verdict>, release <verdict>.
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

> <template 1, filled in — the next sprint of m-<n+1> when this milestone closed, or this
> milestone's remaining stages when it did not>
```

A session that produced ideas but no commits still writes a report — "Merged: nothing" is a
result. So does a sprint whose code merged but whose release did not run: report the merged work,
the blocker and the run id, and leave the milestone open. The ideas go to `CONTEXT.md` § Session
scratchpad, never into the file they speculate about, and the opening prompt says what the next
session should decide first.
