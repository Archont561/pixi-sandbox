# m-2 sprint plan — polyglot dependency vendoring (0.10.0)

Status: proposal, opened when m-1's release was still waiting on its scheduled airlock
confirmation (2026-10-25). Nothing here is implemented; it exists so m-1's closure does not
hand the next session a planning job (doc-18 §6).

## Why this sprint

TASK-130 (GitHub issue #195) is the board's standing polyglot request and was explicitly
named out of scope for m-1. The m-1 work built the machinery this sprint reuses rather than
reinvents:

- **ADR-33's per-workspace tree model** — one vendored tree per project that owns a lockfile,
  qualified transport paths, per-tree verification and per-project wiring, shared content
  counted not deduped. A Go/NuGet/Maven provider is a new *kind* of tree, not a new topology.
- **The ignored-proof gate pattern** — `test-nested-offline` and `test-vendor-offline` show
  the accepted shape: real toolchains, genuine lockfiles, `unshare -n`, isolated caches,
  cold negative controls, changes-gated CI steps, hermetic fake-wrapper tests for the
  ordinary suite.
- **The milestone tag release lane** — proven live on 2026-10-25 (`milestone-v0.9.0`,
  auto-release run `40205465298`). Its trade-offs (no reviewed release PR; bot commit to
  main) are recorded in TASK-136 and must be chosen deliberately again, not inherited.

## Stage A — the spike (TASK-130), questions that must return evidence

1. **Go (first, native mechanics).** Does `go mod vendor` against `go.mod`/`go.sum` produce
   a tree content-hashable the way `cargo vendor --versioned-dirs` output is? How does
   `go build -mod=vendor` behave with `GOPROXY=off` and an empty module cache inside
   `unshare -n`? What is the cold negative control — removing the tree must fail the build,
   mirroring the TASK-131 proof. Output: an end-to-end /tmp recipe with exact commands.
2. **NuGet (second, local package source).** A locked `dotnet restore` into a bundled local
   package source/cache: what does lockfile-driven mean for `packages.lock.json`, how is the
   source wired via `NuGet.Config` without touching the consumer's own config (the
   cargo-home/activation-hook precedent), and what does verification hash when nupkgs are
   zips?
3. **Maven (last, the hard one).** No standard lockfile. The design must name the explicit
   artifact list source and per-artifact integrity checks, or recommend deferral. A
   half-designed Maven provider is the failure mode to avoid; an opt-in contract that
   refuses an undeclared plugin graph is acceptable, silent partial vendoring is not.
4. **Transport decision.** Do provider trees live beside `vendor/` under new qualified
   paths (e.g. per-ecosystem siblings inside each workspace's `.pixi-sandbox/`), and does
   that need an ADR-34 or an ADR-33 amendment? The spike answers with a concrete layout and
   its schema impact; expect a schema bump with the same old-reader reasoning ADR-33 used.
5. **Config surface.** Opt-in naming per provider in `pixi-sandbox.toml`, plan reporting of
   inputs and approximate sizes (the TASK-131 lesson: no plan-time byte fields without an
   honest measurement), and refusal-before-write semantics for incomplete graphs.

## Stage B/C — build and gate shape (to be confirmed by the spike)

- One focused PR per provider, Go first; each leaves every existing bundle byte-identical
  when its config is unused (the ADR-33 root-only identity rule, re-applied).
- Ordinary suite: hermetic tests with fake tool wrappers only. Real-toolchain claims live
  in ignored Linux gates wired into `ci.yml` beside the two existing offline proofs, each
  with a negative control; the suite count must hold or rise.
- Budget honesty follows ADR-33: every tree's bytes count in the transport ceiling and
  twice in the restore preflight, overlap analysis extended only where the content model
  supports it.

## Stage D — release

- Create the m-2 release-gate task (the TASK-136 analogue) before cutting anything.
- Decide the lane explicitly: reviewed release PR vs. human-pushed `milestone-v0.10.0` tag.
  m-1's two open-at-ship items are the decision inputs — the skipped release-PR review and
  the scheduled-tier airlock landing after publication.
- Close m-2 only against run-id evidence; write m-3's plan the same way this document was
  written before m-1 closed.

## Explicit non-goals and carried risks

- No changes to Cargo vendoring or the ADR-6/ADR-33 contract absent spike evidence.
- The m-1 accepted risk stands: no transport containing a nested workspace has yet cycled
  through the released-binary publish/fetch path. If m-2 touches publish/fetch plumbing,
  the deferred airlock harness extension becomes release-blocking instead of a recorded
  boundary.
- TASK-126/TASK-127 remain hygiene work, not milestone exit criteria, as in m-1.
