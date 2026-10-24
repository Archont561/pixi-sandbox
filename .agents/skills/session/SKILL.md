---
name: session
description: Run a pixi-sandbox work session on an airlocked machine, start to finish. A session is a sprint by default — one milestone, one version, one release — not one task: bring the offline environment up, survey the backlog, propose the sprint, work its stages, then cut the release, close the milestone and hand the next session its opening prompt. Use at the start of any session on this repository, when the user says to initialize/bootstrap the environment, pick up a sprint or a milestone, or asks what to work on; and again at the end, when the work has landed, the release has run, or the user asks for a session report, a hand-off, or what the next session should start with.
---

# Session lifecycle for pixi-sandbox

A session is a loop with five phases, and this file sequences all of them: **(1)** bring the
environment up, **(2)** survey the backlog, **(3)** propose the **sprint** and stop, **(4)** work
its stages under the house rules, **(5)** cut the release, then report and hand the next session
its opening prompt. The four artifacts those phases produce have templates in
[`standup-template.md`](standup-template.md), cited below as *template §1* … *template §4*.

Two rules span the whole loop. Never propose work you cannot execute — an environment that is
down changes what is possible. And never close on an unproven claim: the proof an airlocked
machine cannot produce is an open item, not a checked box.

## 0. A session is a sprint

The default unit of work is a **sprint**, not a task, and the chain is fixed:

> **sprint → one milestone → one version → one release.**

- A **milestone** is exactly one version plus the exit criteria that make it releasable
  (`backlog/milestones/m-N`). A **sprint** is the work that reaches it, and it is what this
  session proposes and finishes. The plan that orders it lives in
  `backlog/docs/plans/doc-N …`, and [doc-18](../../../backlog/docs/plans/doc-18%20-%20Sprint-Train-and-the-Milestone-Release-Gate.md)
  is the current train plus the list of what must exist for the release lane to run.
- A sprint has **four stages, always in this order**: **spike** (the one unknown that could
  invalidate the rest — timeboxed, output is evidence, not a feature), **build** (the
  milestone's tasks on what the spike proved), **gate** (fmt, lint, the full suite, and
  `airlock linux-64` on the release branch), **release** (version, PR, merge, milestone tag,
  watched release runs).
- A sprint ends when the milestone tag has run the release lane and the milestone file is
  closed — not when the pull request merges. §5 sequences that.

**Task-long is the exception, and it is chosen explicitly.** Run a single task when the user asks
for one, or when the sprint's next proof needs many workflow runs, a native runner or a
maintainer's click — the airlock and release lanes both qualify. Say so in the proposal, name the
run you are waiting on, and do not let a task-long session drift into a sprint.


## 1. Bring the environment up (airlocked machine)

The workspace is a clone of the repository; the pixi environments exist only after a restore.
Check first, restore only if needed:

```bash
test -x "$HOME/.local/bin/pixi" && "$HOME/.local/bin/pixi" --version
# if missing or not managed for this checkout — the one-time bootstrap (idempotent, passes --force itself, never fetches):
bash scripts/restore.sh
export PATH="$HOME/.local/bin:$PATH"   # the running shell did not read the profile edit
pixi --version && pixi sandbox --version
```

**Let the restore register the user tools** (that is the default, `PIXI_SANDBOX_USER_TOOLS=register`;
do not pass `skip` here — `skip` is for shared CI runners). When it registers, it writes managed
`pixi` and `pixi-sandbox` launchers into `~/.local/bin` and adds a marker-delimited PATH block to
the detected shell profile (task-33); the launchers exec the *manifest-verified* tool copies under
`.pixi/tools/<platform>/`.

### Pixi is the only environment entrypoint

**Read the last line the restore printed. It tells you which world you are in**, and it is a fact
about the tree, not a wish:

- `user tools: registered pixi and pixi-sandbox in <dir>` → the launchers exist.
- `user tools: NOT registered — the bundled pixi-sandbox <v> predates --user-tools (0.3.7)` →
  nothing was registered. Registration is a *restore-side* feature, and the binary doing the
  restore comes from the packed branch: a branch packed before 0.3.7 ignores the policy entirely
  (unknown env var, ignored by design). The published branch has carried a registration-aware
  binary (≥ 0.4.0) since 2026-10-01, so this line means the restore came from a stale branch,
  not the published one — check what the transport carries as described below.

**The published branch is rebuilt on every push to main** by the generated
`publish-sandbox.yml` — after a merge, `gh run list` shows a `publish sandbox` run. The
transport names a commit and a tool version, and both move with every repack, so read the
manifest instead of trusting a remembered number:
`git show origin/sandbox/developer-linux-64:.pixi-sandbox/manifest.json` — `source.commit`
should name the tip of main, `tools.pixi-sandbox` the newest release, `linkage: static`. A
lag right after a release is the GITHUB_TOKEN gap (a release-commit push starts no
workflows; auto-release has dispatched the repack since task-44). A lag at any other time
is an incident worth reporting.

A running parent shell cannot be mutated by the restore. For non-interactive agent commands,
prepend the user bin dir once per shell and then use pixi for everything:

```bash
export PATH="$HOME/.local/bin:$PATH"
pixi --version && pixi sandbox --version
```

Do **not** source `.pixi/sandbox-env.sh`: new restores remove or do not generate it, and the
repository no longer supports direct environment activation. Do not run bare `cargo`, `rustc`,
`bun`, `taplo`, `convco`, etc. A command that needs packages or crates must go through pixi:
repository automation through `pixi run --frozen xtask <subcommand>`, Cargo through
`pixi run --frozen -- cargo <cmd> -p <crate>`, docs/package scripts through
`pixi run --frozen bun --filter=<workspace-package> run <script>`, and repo-wide JS CLIs through
`pixi run --frozen bunx <tool>`.

### Every repo task goes through the pixi binary

```bash
pixi run --frozen fmt | lint | test | test-doc | coverage
pixi run --frozen -- cargo check -p pixi-sandbox   # crate-scoped Cargo, not bare cargo
pixi run --frozen -- convco check HEAD~1..HEAD
pixi run --frozen lint-commit < <file>             # what the commit-msg hook runs
pixi run --frozen docs-install                     # once; the bunx lines depend on it too
pixi run --frozen bun --filter=pixi-sandbox-docs run build
pixi run --frozen bunx backlog task list -s "To Do" --plain
pixi run --frozen bunx skills …
pixi run --frozen -- lefthook install              # installs the git hooks in a fresh clone
```

Facts about this sandbox that shape every command:

- **Egress is filtered**, and the exact shape matters: github.com and the npm registry answer
  (so `git fetch`, `gh`, and `pixi run --frozen docs-install` all work), while the crates.io index/API,
  prefix.dev and static.rust-lang.org do not. Therefore **always** `--offline` for cargo — the
  vendored tree under `.pixi-sandbox/vendor` is what builds — and always `pixi run --frozen
  <task>` so pixi never tries to solve online. A restore is the only way to get a toolchain
  here; there is no rustup fallback.
- `pixi run --frozen test` runs in seconds once built — baseline it at session start and write
  the number down (403 passing / 1 skipped on 0.4.0, 2026-10-02; it must rise with new work,
  never fall). The first build after a restore costs about a minute.
- The airlock claim has a local proof and a CI proof, and they are not the same thing. Locally,
  `crates/pixi-sandbox/tests/e2e.rs` is the fixture-backed lifecycle (doctor → publish → restore
  → `doctor --verify-restored`), it needs no network, and on Linux it re-runs the restore inside
  `unshare -rn`; it is already part of `pixi run --frozen test`. The egress-denied gate over a
  *real* packed transport belongs to CI's cross-platform matrix
  (`.github/workflows/airlock.yml`) — never try to reproduce that tier locally. That CI gate is
  the e2e suite's `ci` cargo feature archived by `pixi run --frozen airlock-gate-archive` and
  replayed by `pixi run --frozen airlock-gate-run`; the default local suite must keep those
  tests uncompiled and skipped.
- Missing system tools: no `/usr/bin/time`, no `file(1)` — use `date +%s`, `readelf`.
- `gh` works against github.com when authenticated; workflow dispatch/rerun may be
  forbidden for the token (`403 Resource not accessible by integration`) — then ask the user to
  click it. Pushes still work through a one-off credential helper, and a **tag push starts the
  tag-triggered workflows even when dispatch does not** — that is how v0.4.0's release was
  re-run after the tag was moved.
- `git push` has no credential helper in this container. Push with
  `git -c "credential.helper=!f() { echo username=x-access-token; echo password=\$GITHUB_TOKEN; }; f" push …`
  rather than writing git config.
- `pixi-sandbox init` **strips every comment from `pixi.toml`** (it round-trips the manifest
  through `toml::Table`). Run it, then `git checkout pixi.toml` unless the channel addition is
  wanted — a 325-line hand-commented manifest comes back reformatted and 100 lines shorter.
- `pixi-sandbox` installs globally with
  `pixi global install -c https://prefix.dev/archont561/archont561 -c conda-forge pixi-sandbox`
  (prefix.dev answers from here); the launcher lands in `~/.pixi/bin`.

## 2. Survey the backlog

1. **Sync before proposing.** `git fetch origin` (github.com is reachable), then confirm the
   working branch is based on `origin/main`'s tip — if it is behind, say so before anything
   else. Never edit on a stale base.
2. **Read the standing context** (skim, do not quote back): `AGENTS.md` (repo map, invariants,
   task commands), `CONTEXT.md`, and `backlog/docs/adr/README.md` — the accepted decisions
   (every file under `backlog/docs/adr/` with `status: accepted`) are load-bearing; if you think one is wrong, bring a measurement, not an opinion.
3. **List the open work**:
   `pixi run --frozen bunx backlog task list -s "To Do" --plain` — or read `backlog/tasks/*.md` directly;
   tasks are markdown files whose frontmatter carries `dependencies`, `priority`, `ordinal`,
   `type`, and `documentation` links (spec docs live under `backlog/docs/`).
4. **Filter honestly.** A task is a candidate only when every dependency has status `Done`.
   Order candidates by priority (high → low), then `ordinal`. A spike that unblocks several
   tasks may jump the queue — say so explicitly when you propose it.
5. **Do not re-propose finished work.** Check `git log --oneline -15`, recently merged PRs
   (`gh pr list --state merged --limit 5`), and the latest release tag. Done tasks, merged
   decisions, and published releases stay done; re-running a release that already exists is
   an error, not work.

## 3. Propose the sprint, then stop

Fill in the **Sprint proposal** (template §2) in [`standup-template.md`](standup-template.md)
and **wait for the user to pick** — do not start implementing.

A sprint proposal names, at minimum: the **milestone and the version** it targets, the **four
stages** with the tasks in each, the **spike** that starts it and the decision it settles, the
**gates** that will prove it, the **release step**, and every **decision the user owns** (tag
vocabulary, sprint scope, whether the release automation lands in this sprint). Propose the whole
sprint even when the user named a single task: say which stage that task belongs to and what the
sprint around it would be, then let them choose the narrower scope.

Board changes are proposed, not assumed. Attaching tasks to the milestone, closing a task whose
criteria are already met, and filing the milestone's release-gate task are all separate edits;
list them under "what the sprint needs" and wait.

That file carries all four phases of the lifecycle, and a session is only closed when every one
of them has been produced: the **opening prompt** (template §1) that started this session,
the **sprint proposal** (template §2) you are writing now, a **task hand-off** (template §3) at
the end of each task inside the sprint, and the **sprint report** (template §4) after the release
— which ends by emitting the next opening prompt.
Use them at those points rather than inventing a shape; the templates are the hand-off protocol
between sessions, and the loop only closes because the last artifact is the first one.

## 4. While you work — house rules (AGENTS.md is the full list)

- **Work the sprint stage by stage**, spike first. Do not start stage B on an unproven stage A
  assumption: if the spike cannot settle its question, that is a proposal amendment for the user,
  not a reason to build on a guess. Stage C's hygiene tasks run on spare capacity and never at
  the cost of the stage that gates the release.
- **One focused conventional commit per task.** The hooks are split by cost, so match your
  rhythm to them:
  - while committing: `pixi run --frozen fmt`. `pre-commit` adds taplo, biome and actionlint
    over staged files only — no cargo, by design.
  - before pushing: `pre-push` first runs the local ADR-29 main-integration convenience (CI
    remains the repository-wide authority), then clippy, the repo-consistency and
    generated-workflow xtask checks, and the whole-workspace test suite, in that order, stopping
    at the first failure. Run `pixi run --frozen lint && pixi run --frozen test` yourself first
    when you want the answer sooner, or when the change touches what only the full `lint` covers
    (`deny`, `sandbox-plan --json`, `lint-toml`, `lint-docs`).
  - message check: `pixi run --frozen -- convco check HEAD~1..HEAD`.
- **Complete the task file in its own format**: every AC `[x]`, status `Done`, bump
  `updated_date`, append `SECTION:NOTES` Implementation Notes and `SECTION:SUMMARY` Final
  Summary — following the task's exact existing section markers (no PLAN edits).
- **Tests target fixtures and tempdirs, never this repository or a real HOME**
  (ADR-10; isolate `HOME`/`USERPROFILE`/`SHELL` per test).
- **Pull requests**: push the working branch and merge on GitHub once `ci` is green — squash,
  or a merge commit when the branch carries several focused commits — with a conventional title
  ending `(#N)`. That is the only way `main` takes more than one commit (AGENTS.md invariant 12,
  ADR-29); `pixi run --frozen xtask check-main-push` judges a direct push before you make it.
  Watch the required checks; when a platform job fails, read that job's log before changing
  code. What happens after the merge is § 5, not an afterthought.

## 5. Close the sprint — cut the release, then report, then hand the next session its prompt

A merge is not the end of a session; it is the first moment you can tell the truth about it. Even
that is only true for a task-long session: a sprint-long one closes at the release, so this phase
starts one step earlier.

### 5a. Cut the release (sprint-long sessions)

The version is the sprint's deliverable, so the release is part of the work, not a follow-up.
In order, and stopping at the first failure:

1. **Prepare.** `pixi run --frozen xtask prepare-release -- <auto|minor|patch|vX.Y.Z>`. It bumps
   the workspace version, refreshes `Cargo.lock`, regenerates `CHANGELOG.md` and restamps the
   committed relock render, reporting every file it touched. Read that list; nothing else may be
   in the release commit.
2. **Review the bump.** The selector must match the milestone's version policy (`backlog/milestones/m-N`):
   pre-1.0, a breaking transport or manifest change is a **minor** bump. A `vX.Y.Z` selector
   bypasses convention — use it only when the milestone names the version explicitly, and say so
   in the release PR.
3. **Open the release PR**, house style, with the prepared diff, the airlock verdict for the
   release branch and the suite count in the body. The release PR is the last place the version
   can still be questioned.
4. **Gate it for real.** `ci` green is not an airlock (§5.7 of
   [doc-18](../../../backlog/docs/plans/doc-18%20-%20Sprint-Train-and-the-Milestone-Release-Gate.md)):
   if the release PR touches no airlock path, run `airlock.yml` deliberately and wait for both
   tiers, or say plainly that the release ships without that proof. Never claim it by association.
5. **Merge, then tag.** `auto-release.yml` today triggers only on `workflow_dispatch` and
   `workflow_call`; nothing reacts to a milestone tag until doc-18 §5.2 lands. So either dispatch
   it (`gh workflow run auto-release.yml -f version=vX.Y.Z`) or push the milestone tag once that
   trigger exists — then watch `release.yml` (five platforms, conda, GitHub Release) and
   `publish-sandbox.yml` (transport repack) to their verdicts.
6. **Close the milestone** — its release-gate task's criteria checked, the milestone file's exit
   criteria satisfied — and write the **next** sprint's plan document and milestone file. A sprint
   that ends without the next milestone written has handed the next session a planning job.

A release that cannot be cut in this session (no token, a red platform job, a missing asset) is
reported as an open item with the run id that shows why: the sprint stays open, never quietly
"done".

### 5b. Report and hand over

Do these in order, and do not skip to the report:

1. **Watch the post-merge runs.** `gh run list --branch main --limit 5`, then read the verdict
   of every workflow the merge triggered (`ci`, `docs`, and — since 2026-10-01 —
   `publish sandbox`, which repacks the transport on every push to main; anything
   path-triggered). A red post-merge run belongs to **this** session — fix it before reporting,
   never leave it as the next session's surprise. `gh run view <id> --log` often cannot stream
   from this sandbox; `gh run view <id>` alone still gives job verdicts and annotations, which
   is usually enough to tell a real failure from a flake; a running run's job logs are also
   fetchable with `gh api repos/<owner>/<repo>/actions/jobs/<id>/logs`.
2. **Close the task files** in house format — in the pull request itself while it is still open,
   since a follow-up PR after the merge is not part of the hand-off (step 5): every AC `[x]`, status `Done`, `updated_date`
   bumped, `SECTION:NOTES` and `SECTION:SUMMARY` appended, no PLAN edits. **An AC you could not
   prove is not a checked AC.** When the remaining proof needs a native runner, a cut release
   or a maintainer's click, leave the task `In Progress`, name the AC and the missing proof in
   the notes, and carry that same sentence into the report — a task closed on an unproven AC
   is how a stale claim enters the record.
3. **Re-baseline on the merged tree.** `git fetch origin && git log --oneline origin/main -1`
   to confirm what merged, then `pixi run --frozen test`. The number goes in the report and
   into the next opening prompt; it must rise with new work, never fall.
4. **Record what you did not implement on the branch that is merging, before it merges.** Write
   the `CONTEXT.md` § Session scratchpad entry — dated heading, newest last (invariant 10):
   proposals, alternatives weighed, measurements worth keeping, open items — and end it with the
   next session's opening prompt filled in with everything knowable pre-merge, so the file in the
   tree and the message in the chat say the same thing. The entry rides the work PR (or the
   release PR) as a `docs(context): …` commit **inside that PR** — never as a pull request of its
   own after the merge. A scratchpad entry is not, by itself, a reason to open a PR.
5. **Hand over in chat; the hand-off is a printed message, not a follow-up PR.** Once the
   post-merge runs are watched and the tree is re-baselined (steps 1 and 3), print both artifacts
   in the final message: the **Session report** (template §4) and, inside it, the **Session
   opening prompt** (template §1), now completed with the post-merge facts — merged SHA, run
   verdicts, the fresh baseline. The test is literal: the user should be able to paste that
   prompt back as the first message of the next session and have an agent reach the same
   understanding you have now, without reading the diff. Do not push a follow-up branch or open a
   `docs(context)` PR once the merge has landed — that retired pattern (a standalone hand-off PR:
   one markdown file costing a whole extra CI cycle) is exactly what this rule forbids. If the
   post-merge facts must also be stamped into the tree, the only legal vehicle is invariant 12's
   single hand-made direct-push commit — markdown-only, nothing under `airlock.yml`'s
   `pull_request.paths`, judged first with `pixi run --frozen xtask check-main-push`. That is an
   exception, never the default; normally the printed prompt carries the post-merge facts and the
   tree keeps the pre-merge entry from step 4.

A session that merged nothing still reports — "Merged: nothing", the ideas in the scratchpad,
and an opening prompt that says what to decide first. So does a sprint that shipped its code but
could not cut its version: report the merged work, the release blocker and the run that shows it,
and leave the milestone open.
