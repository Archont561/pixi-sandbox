---
type: Design Spec
title: "Testing Strategy (for the Rust assembler + action)"
description: A five-layer test plan for the D21 Rust binary and its action — what each layer proves, where it can run, and which layer this sandbox is structurally locked out of.
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, testing, ci, d21]
status: stable
confidence: reasoned
generated: { by: arena-agent/agent-mode, at: 2026-09-19T23:55:00Z }
sources:
  - { id: repo, resource: https://github.com/Archont561/pixi-sandbox, title: Archont561/pixi-sandbox }
---

# Testing Strategy (for the Rust assembler + action)

> [D21](/spec/decisions.md) is accepted: the v1 artifact is the Rust assembler binary, and this is its test
> plan. The shell-era strategy in [Testing Without a Compiler](/spec/testing.md) stays valid for everything this
> sandbox can still do (L0); this page owns what only CI can prove.

## 1. The split that organises everything

This sandbox has **no route to `rustc`** (measured: crates.io, rustup hosts and conda all unreachable ✅), so
no Rust test can run here — and pretending otherwise is how bad confidence labels are born. Every test
therefore carries a *where*:

| Layer | What it proves | Runs | Today |
|---|---|---|---|
| **L0 — design checks** | bundle integrity, `action.yml` parses, fixture-tree shapes, the oracle vectors are internally consistent | **this sandbox** | ✅ measured |
| **L1 — unit** | manifest parse, branch-sharding planner, rung selection, digest verify, exit-code mapping, template rendering | CI (`cargo test`) | 🚧 needs rustc |
| **L2 — integration** | the nine oracle outcomes against the real binary; multi-branch lazy fetch against a local bare repo; tamper ⇒ exit 4; split-blob round-trip | CI (`cargo test` + system git) | 🚧 |
| **L3 — end-to-end** | real `pixi-pack`/`cargo vendor`/`pixi-unpack`; clean-container reconstruction; `pixi list` + `pixi run test` offline; zero non-git egress | CI runner + containers | 🚧 (the M1.5 gate) |
| **L4 — action/security lint** | `actionlint`, `zizmor`, `shellcheck`, pin-by-SHA audit, permissions minimality | CI | 🚧 (tools not installable here ❌ measured) |

L0 is the only layer this box contributes *executed* evidence to; L1–L4 are specified here and executed in CI.
That is an honest division of labour, not a gap to apologise for: the design's riskiest claims (rung 2 works,
vendor tree is complete, pixi runs offline) were always CI-shaped ([What Is Still Unproven](/workflows/unproven.md)).

## 2. The oracle: one vector table, two implementations

The measured `assemble.sh` prototype (nine outcomes ✅ — [Running the Action](/workflows/action-run.md)) is
frozen as a **vector table**: `fixture → expected exit code → expected stdout/stderr markers → expected files
on disk`. Both implementations are tested against the *same* table:

* the shell script passed it by execution (this sandbox, stubbed drivers);
* the Rust assembler must pass it in L2 as `#[test]`s with identical fixtures.

Rule: a vector may only change by editing the table in one place (a fixture directory in the future repo,
mirrored as the outcome table in [Running the Action](/workflows/action-run.md) here) and regenerating both
sides' expectations. If the Rust binary "improves" a behaviour, the table changes first, and the shell oracle
is re-run against it — drift between the two implementations is a test failure, not a judgement call.

## 3. Layer detail

**L1 — unit (no network, no pixi, no git binary).** Everything behind a trait with an in-memory fake:
`proptest` on the sharding planner (arbitrary payload size lists ⇒ every blob ≤ `max-blob-bytes`, every branch
≤ `max-branch-bytes`, and reassembly is byte-identical — the properties the 100 MB GitHub limit makes
load-bearing), `insta` snapshots for rendered `README.md`/`AGENTS.md`/`dist-manifest.json`, and an exhaustive
exit-code mapping table (0/1/4/5/7/8, never a foreign status leaking through — the exact bug the shell draft
had when `set -eu` leaked `tar`'s 2).

**L2 — integration (tempdir + system git, still no pixi).** The vectors from §2 run against the compiled
binary with stubbed drivers (the same stubbing the shell harness used — the script/binary contains *no* solver
logic, only selection, verification and wiring, which is exactly what stubs exercise). Plus three flows the
shell harness proved manually and the binary must reproduce automatically: publish → bare-repo push → clone →
lazy-fetch reconstruct; one flipped byte ⇒ exit 4 *before* anything executes; a blob forced over a tiny budget
⇒ split → reassemble → identical sha.

**L3 — end-to-end (the gate that can falsify the design).** One CI job, real tools, throwaway dist branch:
pack a fixture workspace on `ubuntu-latest`, publish, then in a **clean alpine container** (nothing installed,
git only) run the one-liner and assert: `pixi list` non-empty, `pixi run <task>` exits 0, and — when the fixture
carries `Cargo.lock` with bundled crates — `cargo test` runs offline against the vendored directory. The egress
assertion from [M4](/spec/roadmap.md) runs here too: `strace`/allowlist proves reconstruction makes **zero**
non-git network calls. Containers beyond alpine (debian-slim, a glibc-less oddity) are added only when a user
needs one — each is a vector, not a promise.

**L4 — the action is attack surface.** `zizmor` for action-injection patterns, `actionlint` for schema,
pin-by-SHA audit of every `uses:`, and a check that the workflow's permissions never exceed
`contents: write` (+`pages`/`id-token` for the docs job). These run on the action repo's own CI, on every push.

## 4. Dogfooding stays the promotion gate

[D17's tiers](/spec/decisions.md) still order trust: L0/L2 fixtures run anywhere; the compiled binary is
exercised in CI on throwaway `pixi-sandbox-wip-<topic>` branches; and the **D4 promotion rule** is unchanged —
a release is cut only when the released binary's behaviour equals the oracle's expectation, demonstrated by an
actual CI run, not by review. The assembler binary ships in kits only after L3 has passed for its revision, and
`dist-manifest.json` records which run built it, so a kit's provenance is auditable back to a green pipeline.
