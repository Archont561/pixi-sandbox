# Session standup templates

Fill these in literally — same headings, same order. They are the shape a reviewer expects,
and keeping them out of `SKILL.md` means editing a template never touches the procedure.

## 1. Session proposal (end of startup — then stop and wait)

```
Session proposal — <date>

Environment: restored; baseline <N> tests passing, <M> skipped.
User tools: <registered in ~/.local/bin | NOT registered — bundled pixi-sandbox <v> predates 0.3.7>.

Backlog: <X> To Do, <Y> unblocked. Candidates, in recommended order:
1. task-<n> (<priority>, <type>) — <one line: what it delivers and why now>
2. task-<m> …
   …
Not this session: task-<k> (blocked by task-<j>); task-<l> (deferred — <one-line reason>).

Per task, "done" means: its acceptance criteria checked, one focused conventional commit,
gates green (fmt, lint, test), task file completed in house format.
Need from you: confirm the scope (or pick differently) before I start.
```

## 2. Task hand-off (end of a task, before the commit)

```
task-<n> — <title>

Changed: <file> (<one line why>), …
Evidence: <the command that proves it> → <result>. Suite <N> passing / <M> skipped (was <N₀>/<M₀>).
Gates: fmt ✓ lint ✓ test ✓ convco ✓
Left undone: <anything an AC does not cover, or "nothing">.
```

## 3. Session close

```
Session close — <date>

Landed: <commit subject> (<task>), …
Open: <task ids still To Do that this session touched or unblocked>
Recorded in CONTEXT.md: <the proposals/drafts appended there this session, by heading>
Next session should start with: <one line>
```

A session that produced ideas but no commits still writes section 3 — the ideas go to
`CONTEXT.md` under "Session scratchpad", never into the file they speculate about.
