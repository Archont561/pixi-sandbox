---
type: Measurement
title: "Disk and Performance Budget"
description: Observed throughput, clone costs and space limits that any artifact-packing design must budget against.
resource: https://github.com/Archont561/pixi-sandbox
tags: [environment, budget, performance]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
verified: { by: process:sandbox-measurement, at: 2026-09-19T21:00:00Z }
legacy: { files: [`SANDBOX_CONSTRAINTS.md`], sections: ["10"] }
---

# Disk and Performance Budget

## 10. Disk & performance budget observed

| Measurement | Value |
|---|---|
| `git clone --depth 1 prefix-dev/pixi` | ~3.5 s, 73 MB (mostly `pixi.lock`/assets) |
| `codeload` tarball of the same repo | 19.9 MB, a few seconds |
| `pip download requests` | 73.1 kB at 7.8 MB/s |
| `npm install ms@2.1.3` | 464 ms |
| Idle memory used | 225 MiB of 3.8 GiB |

Practical ceiling: a cold Rust release build of a CLI with `rattler`-class dependencies would be
**hours on 2 vCPU and is very likely to OOM at 3.8 GiB without swap** — even if crates.io were
reachable. Keep dependency graphs small if a CI build is the goal.

---
