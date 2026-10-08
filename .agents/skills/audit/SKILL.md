---
name: audit
description: >-
  Conduct evidence-based code-quality audits of source code, changes, modules, or repositories.
  Use whenever the user asks to audit, scan, assess, or review code for maintainability,
  architecture, readability, duplication, complexity, code smells, or refactor opportunities,
  including a plain request to "audit this" in a coding context. Use the refactor and
  clean-code-principles skills, report concrete findings, and automatically create or update a
  focused backlog task for every validated actionable follow-up. Classify behavior-preserving
  refactors separately from bugs, missing features, and contract changes. If code, tests,
  documentation, contracts, or requirements conflict, or intended behavior is unclear, pause and
  seek clarification through grill-me before deciding the finding or task scope. This skill is not
  a substitute for security, compliance, or other domain-specific audits.
---

# Code-quality audit

An audit discovers and triages maintainability problems; it does not silently change production
code. The deliverable is a concise evidence-backed report and, by default, a backlog task for every
validated actionable follow-up. Track behavior-preserving cleanup as refactor work and track bugs,
missing features, or approved contract changes under their correct task types.

## 1. Establish scope and constraints

- Follow the user's requested target: named files, diff, module, package, or repository. If the
  request is simply "audit this" and the target is not clear from context, ask one scope question
  rather than silently scanning the whole repository. State any reasonable scope assumption.
- Read the applicable `AGENTS.md`, `CONTEXT.md`, architecture notes, and decisions before judging
  project-specific design. Inspect `git status --short` and the relevant diff so existing user
  changes are not mistaken for audit findings.
- Keep the scan read-only. Do not edit source, auto-format, or start implementing a fix during an
  audit. Honor an explicit instruction not to create tasks or not to inspect particular paths.
- Keep the requested scope bounded. Exclude generated, vendored, or unrelated areas unless the
  user explicitly includes them. Say what was and was not inspected.

## 2. Load the review criteria

For each code-quality audit, read:

- [refactor](../refactor/SKILL.md) for behavior-preserving refactoring, small steps, and avoiding
  speculative restructuring.
- [clean-code-principles](../clean-code-principles/SKILL.md) for principle selection. Read the
  specific rule files that apply to evidence found (for example `rules/core-dry.md`,
  `rules/core-kiss-simplicity.md`, or `rules/solid-srp-function.md`); do not cite planned or
  unavailable rules as if they were implemented.

Use the project's own conventions and architecture as the controlling context. A principle is a
lens, not proof: line count, a design-pattern opportunity, or a personal style preference alone is
not an actionable finding. Prefer the simplest fix; do not recommend abstractions or patterns
without a concrete problem they solve.

When a finding is a candidate for a refactor task, also read [tdd](../tdd/SKILL.md) and
[backlog](../backlog/SKILL.md). The implementation task must preserve the public behavior and use
the repository's test-first workflow. Read [grill-me](../grill-me/SKILL.md) only when a clarification
gate below is reached.

## 3. Scan and verify findings

Inspect the relevant implementation together with its callers, public interfaces, tests, and nearby
documentation. Use existing, scoped linters or analyzers when they are available and relevant; do
not add dependencies or run a repo-wide gate merely to make an audit look comprehensive. Follow
repository tooling instructions (for this repo, use Pixi entry points rather than bare tool
commands). Record the exact checks run and their results, including environmental blockers.

Keep a finding only when the evidence supports it. For each candidate:

1. Identify the observable maintenance problem and cite `path:line` (or the exact tool output).
2. Select the relevant clean-code rule and explain the concrete cost or risk in this codebase.
3. Check whether tests, docs, an ADR, public API, wire shape, error mapping, ownership/threading
   rule, or another explicit contract changes the interpretation.
4. Recommend the smallest behavior-preserving refactor that addresses the problem. Separate a
   confirmed issue from a hypothesis or optional improvement.

Do not label a behavior bug, missing feature, or contract change as a refactor. Report it separately
and, once its expected behavior and scope are clear, track it under the appropriate task type in §5.
If the expected behavior or follow-up scope is unclear, clarify before creating a task.

## 4. Clarify contradictions before acting

Pause when a finding depends on unclear intent, or when implementation evidence conflicts with a
test, documented requirement, ADR, public contract, or the user's constraints. Also pause if a
proposed refactor could change observable behavior and the expected behavior is not settled.

At that point, use the repository's `grill-me` skill to ask a focused clarification question and
wait for the answer before deciding the finding or creating a task. The current `grill-me` wrapper
is user-invoked and delegates to a `grilling` skill. If the agent environment cannot invoke that
skill directly, ask the same concise question in chat and pause; do not guess, bury the conflict in
an assumption, or create a task whose acceptance criteria encode one side of the dispute. A clear,
objective code smell with no behavioral ambiguity does not need a clarification round.

## 5. Automatically create focused backlog task(s)

Unless the user explicitly asks for report-only output, automatically create or update a backlog
entry for every validated, actionable audit follow-up once its scope and expected behavior are
clear. Do not ask for a second confirmation merely to create a task after the user requested an
audit. Group findings only when they form one cohesive implementation; do not create tasks for
speculation, style preferences, or findings whose scope remains unresolved.

- Search the backlog first. Reuse or update an existing task instead of creating a duplicate; if
  existing tasks overlap or conflict, clarify ownership/scope before changing them.
- Use the correct task type: behavior-preserving structural work is a refactor task; a behavior bug,
  missing feature, or approved contract change must be tracked as the appropriate bug/enhancement,
  never disguised as a refactor. After its expected behavior is clarified, create that task without
  another task-creation permission prompt.
- Follow [backlog](../backlog/SKILL.md) and its CLI. In this repository use `pixi run backlog ...`
  and non-interactive read options. Never hand-edit backlog task metadata. If the CLI or required
  environment is unavailable, report the exact blocker and provide a ready-to-enter task draft
  instead; do not silently claim the task was created.
- Make each task implementation-focused: include finding/evidence, intended behavior to preserve or
  establish, bounded scope, non-goals, and verifiable acceptance criteria.
- Read [tdd](../tdd/SKILL.md) before writing criteria. Require agreement with the user on public
  test seam(s) before tests are written; add or extend a behavior test at the agreed seam first,
  observe the red result when applicable, make one small change at a time, and run focused tests and
  the relevant package gate. Keep structural cleanup separate from behavior changes and record
  exact blockers rather than claiming unrun checks passed.
- Leave created tasks open for implementation. The audit does not implement their production fixes
  or mark them Done.

If no actionable follow-up is confirmed, say so and create no task.

## 6. Report

Use this compact structure, omitting empty sections:

```markdown
## Audit
**Scope:** ...
**Summary:** ...

### Findings
- **[severity] [rule-id]** `path:line` — evidence, impact, and proposed behavior-preserving change.

### Follow-up tasks
- `task-id` — title; or explain why no task was created / why creation was blocked.

### Checks
- `command` — result, or exact reason it could not run.

### Scope limits
- Paths or concerns not examined.
```

Use severity only to describe practical impact, not to inflate ordinary maintainability issues.
Clearly distinguish verified evidence, uncertainty, and suggestions. Never imply a test or scan ran if
it did not.
