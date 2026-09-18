---
type: Design Spec
title: "Error Taxonomy and UX Contract"
description: Exit codes, message voice, and the promise that every refusal names the check that failed.
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, ux, errors]
status: stable
confidence: reasoned
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`DESIGN.md`], sections: ["13"] }
sources:
  - { id: oneuptimecom-blog-post, resource: https://oneuptime.com/blog/post/2026-01-25-error-types-thiserror-anyhow-rust/view, title: oneuptime.com/blog/post/2026-01-25-error-types-thiserror-any }
---

# Error Taxonomy and UX Contract

## 13. Error taxonomy and UX contract

`thiserror` in `sandbox-core`, exit codes in the binary (the split I verified upstream: `thiserror` for
typed library errors, `anyhow`-style aggregation only at the `main` boundary ✅
[oneuptime](https://oneuptime.com/blog/post/2026-01-25-error-types-thiserror-anyhow-rust/view)) — but with
**distinct, documented exit codes**, because this tool is meant to be driven by scripts and agents:

| Code | Class | Example | Contract |
|---|---|---|---|
| 0 | ok | — | stdout = result (or `sandbox.lock.json` diff) |
| 1 | usage | bad flag combination | fixable by the caller alone |
| 2 | plan refused | `--offline` but action needs network | prints the `requires-hosts` list |
| 3 | **stale** | `pixi.lock` ≠ manifest; `vendor/` ≠ `Cargo.lock`; `bun.lock` ≠ `package.json` | remediation = a `sync` verb |
| 4 | **integrity** | sha256 mismatch; `dist-manifest` missing an entry | remediation = re-`fetch`; **never auto-heals** |
| 5 | **unavailable** | `bun` has no `win-64` conda package; `pixi-pack` absent | remediation = the `fallback` chain, printed verbatim |
| 6 | **policy/egress** | `index.crates.io: TLS reset` | remediation = "run in CI" / `vendor sync` elsewhere |
| 7 | **not-detected** | `--env training` on a repo whose inventory has no such environment; `--target win-64` where P0/P1 veto ([§7.4](/spec/toolchain-resolution.md#74-platform-validation-does-this-platform-actually-exist)) | remediation = `pixi sandbox environments` (what *does* exist), `pixi lock` (write the platform block), or the manifest edit — distinct from 5 because **the request**, not the upstream, is what's wrong |
| 8 | **reconstruction failed** | all of R1→R4 tried, verification trio still red | payload = `reconstruct-report.json` with each rung's failure; `--allow-partial` may downgrade to 0 **only** when the caller explicitly wants a prefix-not-a-workspace result |

Why 5 vs 7 vs 8 must stay separate for an agent driver: **5** means "change your platform set or accept a
degraded kit", **7** means "ask for something that exists", **8** means "the *target* is broken — stop
retrying and page a human". Collapsing them is how you get an agent that retries a doomed bootstrap 20 times.

Every error renders as `error[E-CODE]: what\n = where\n = why\n = try: <exact command>` and, with
`--json`, as `{code, class, message, artifact, remediation: [cmd…]}`. **Never** `expect()` in library
code; `unwrap` is confined to tests. Exit codes are the API for CI (`|| exit 0` is a smell; `--strict`
opts into treating warnings as errors for gating).

---
