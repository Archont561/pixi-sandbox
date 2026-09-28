---
name: backlog
description: Managing project work as Markdown tasks with Backlog.md. Use when creating, breaking down, tracking, or updating tasks; when the user mentions the backlog, a kanban board, a task ID (task-N), acceptance criteria (AC), or a Definition of Done (DoD).
---

Backlog.md keeps every task as a plain Markdown file under `backlog/`, versioned with the code, so task state is a git commit. Drive it through the CLI: prefer `backlog` commands over hand-editing task files, so IDs, filenames, and metadata stay consistent.

## Running it in this repo

This project has `backlog.md` as a root bun-workspace devDependency, exposed as a pixi task — run it as:

```
pixi run backlog <args>          # = bun x backlog
```

- Run `pixi run docs-install` once first. The `backlog` binary has a `#!/usr/bin/env node` shebang and the `default` pixi env carries no node, so a bare `backlog` (or `node_modules/.bin/backlog`) fails with `env: node: No such file or directory` — go through `bun x` / the pixi task.
- If `backlog/` does not exist yet, run `pixi run backlog init` once to scaffold it.

## Agent mode

Read commands default to an **interactive TUI** that never returns in a non-interactive shell. Always run non-interactively:

- Add `--plain` to every read (`task list`, `task <id>`, `search`) for stable text.
- Add `--json` when you will parse the output; `--json` is versioned and machine-readable.
- Never launch `backlog board` (bare), `backlog browser`, or `backlog task <id>` without `--plain` — they block on a UI.

## The loop

1. **Find work** — `pixi run backlog task list -s "To Do" --plain`, or `pixi run backlog search "<query>" --plain`.
2. **Read before coding** — `pixi run backlog task <id> --plain`. Read the acceptance criteria and any plan first; match test and interface names to the task's vocabulary.
3. **Claim and plan** — `pixi run backlog task edit <id> -s "In Progress" -a @me --plan "approach"`.
4. **Record progress in notes** (execution log), not comments — `pixi run backlog task edit <id> --notes "..."` then `--append-notes "..."` for more lines.
5. **Verify against AC** — mark each criterion with `--check-ac <n>`; write a PR-ready `--final-summary "..."` when the work is done.
6. **Close** — set `-s Done`, or `pixi run backlog task complete <id>` during cleanup (keeps the record and dependency links). Use `pixi run backlog task archive <id>` for cancelled, duplicate, or invalid work.

## Creating tasks

```
pixi run backlog task create "Title" -d "description" --ac "First,Second" -l area --priority high --dep task-1
```

Sub-task: add `-p <parent-id>`. Draft: add `--draft`, or `backlog draft create "..."` then `backlog draft promote <id>`. Full flag surface is in [`REFERENCE.md`](REFERENCE.md).

## Input gotchas

- **Multi-line** description/plan/notes: repeat the `--append-*` variant once per line (works in every shell, including agent sandboxes), or put real newlines inside double quotes. Do **not** use `$'line1\nline2'` — tree-sitter agent sandboxes reject it.
- **Literal backticks** in task text: single-quote the argument (`'Document `backlog init` setup'`), or the shell runs command substitution before Backlog.md sees the text.
- **Notes vs comments**: implementation notes and the final summary carry execution progress; comments are append-only review discussion (`--comment "..." --comment-author @you`). A standalone `---` line is reserved as a comment delimiter.

## More

For the exhaustive command and flag table (task edit AC/DoD operations, dependencies, milestones, drafts, board/export, browser, config, JSON shapes) see [`REFERENCE.md`](REFERENCE.md), and trust `pixi run backlog <command> --help` as the live source of truth over any cached list here.
