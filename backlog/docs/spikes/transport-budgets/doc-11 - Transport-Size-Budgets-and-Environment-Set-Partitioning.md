---
id: doc-11
title: Transport Size Budgets and Environment-Set Partitioning
type: other
created_date: '2026-10-06 10:47'
updated_date: '2026-10-06 10:47'
tags:
  - spike
  - transport
  - airlock
  - measurement
---
# Spike — transport size budgets and environment-set partitioning

**Status:** measured spike plus production gate, for TASK-72. The transport format stays schema 2,
payloads stay self-contained, and the partition boundary is still a reviewed `[[bundle]]`
environment set. No external immutable-payload channel is introduced.

**Question.** Where should the tool draw hard lines for transport growth — Git object size,
transport bytes, repository/push bytes, and restore disk — before a generated publisher refuses
to overwrite the last healthy branch? If a consumer hits those lines, is environment-set
partitioning enough, or does the format need an external payload mechanism?

**Environment.** Measurements were taken 2026-10-06 from this restored Linux sandbox (pixi
0.81.0, pixi-sandbox 0.5.3 in the published transports). GitHub was reachable for shallow
branch clones. General package registries were not used. The clone/worktree measurements used
`git clone --depth 1 --single-branch --no-checkout` followed by `git worktree add --detach`; the
publish measurements used `pixi-sandbox publish --dry-run`, and the fixture also used a real
local bare remote.

---

## 1. Measurements

| Transport | Source | Envs | Largest stored Git object | Largest declared payload | Declared transport bytes | Snapshot/push bytes | Git object store | Checkout | Fetch + checkout | Restore preflight |
|:---|:---|:---|---:|---:|---:|---:|---:|---:|---:|---:|
| Fixture | checked-in test fixture | `demo` | 77,213 B (0.1 MiB) | 77,213 B (0.1 MiB) | 90,985 B (0.1 MiB) | 100,001 B (0.1 MiB) | 1 MiB after local publish | 1 MiB local checkout | 0.023 s + 0.008 s (`file://`) | 265,542 B (0.3 MiB) |
| pixi-sandbox | `sandbox/developer-linux-64` @ `01db0c5`, manifest source `145bbfb` | `default` | 99,614,720 B (95.0 MiB) | 173,347,196 B (165.3 MiB), sharded | 851,987,337 B (812.5 MiB) | 854,174,217 B (814.6 MiB) | 502 MiB | 839 MiB | 11.62 s + 2.93 s | 2,934,433,753 B (2,798.5 MiB) |
| Castellan | `sandbox/developer-linux-64` @ `434e2ea`, manifest source `a693650` | `default,shells` | 99,614,720 B (95.0 MiB) | 173,347,196 B (165.3 MiB), sharded | 1,947,248,816 B (1,857.0 MiB) | 1,952,445,144 B (1,862.0 MiB) | 829 MiB | 1,923 MiB | 19.68 s + 6.50 s | 7,744,060,400 B (7,385.3 MiB) |

Payload split for the two connected transports:

| Transport | Env packs | Tools | Vendored Cargo | Vendor crates |
|:---|---:|---:|---:|---:|
| pixi-sandbox | 436.0 MiB | 95.2 MiB | 280.5 MiB | 167 |
| Castellan | 1,151.0 MiB | 95.2 MiB | 605.4 MiB | 489 |

Publish behavior:

- Fixture local publish created one orphan commit, reported `published 17 file(s), 0.1 MiB`, and
  the local branch object store remained `0.1 MiB` inside a 1 MiB bare repository. A shallow
  `file://` clone plus detached worktree checkout took 0.023 s + 0.008 s and produced 1 MiB
  `.git` and checkout directories.
- pixi-sandbox dry-run reported the existing orphan push shape (`commit-tree` without a parent,
  `push --force`) and `10827 file(s), 814.6 MiB`. The one-file difference from the measurement
  table is the `.git` pointer present in a Git worktree checkout, not in a pack output.
- Castellan dry-run reported the same orphan/force-push shape and `26167 file(s), 1862.0 MiB`
  (again including the worktree `.git` pointer). It is below 2 GiB but close enough that one more
  large environment family should be partitioned before publish.

The current risk is not an individual Git blob: both real transports already shard the 165.3 MiB
Rust package into 95.0 MiB parts. The risk is total branch/push size and restore preflight bytes,
especially when multiple independent environment families (`default` plus `shells`) ride one
branch.

---

## 2. Reviewed hard budgets

The production gate enforces these defaults through `[budgets]` in `pixi-sandbox.toml` or
`.pixi-sandbox.toml`:

| Config field | Default | What it measures | Refusal remedy |
|:---|---:|:---|:---|
| `max_blob_mib` | 95 MiB | Largest stored Git object: an unsplit blob, a split part, an embedded tool, or an oracle file. | Lower `--shard-limit-mib` or remove the single oversized payload. A single file cannot be fixed by environment-set partitioning. |
| `max_transport_mib` | 2048 MiB | Declared bytes `doctor --verify` would check: env pack blobs, file oracles, tools, and vendor blobs. | Split the reviewed environment set into separate `[[bundle]]` entries. |
| `max_repository_push_mib` | 2048 MiB | File bytes the publisher stages into the orphan snapshot before Git compression. | Split the reviewed environment set into separate `[[bundle]]` entries. |
| `max_restore_required_mib` | 8192 MiB | Restore preflight estimate: packed + unpacked envs, plus two vendor copies. | Split the reviewed environment set into separate `[[bundle]]` entries. |

Why these numbers:

- 95 MiB is the existing GitHub-safe shard ceiling. It is also the observed largest stored object
  in both connected transports.
- 2048 MiB catches the Castellan class of growth before a branch crosses a 2 GiB push/worktree
  footprint. Castellan is currently 1,857.0 MiB declared / 1,862.0 MiB snapshot bytes, leaving
  roughly 9% headroom.
- 8192 MiB catches the matching restore-disk risk. Castellan is currently 7,385.3 MiB, leaving
  roughly 10% headroom.

The gate is intentionally hard: a generated publisher fails before `publish`, leaves the prior
healthy branch untouched, and prints the exceeded field plus the partition remedy. A consumer can
lower any budget; `max_blob_mib` cannot be raised above 95 MiB because larger stored Git blobs are
not a project policy choice.

---

## 3. Implementation surface

- `pixi_sandbox_core::transport_budget` measures a manifest and optional checkout snapshot bytes.
  Its transport byte definition matches the trust boundary of `doctor --verify`, including
  file-oracle blobs and embedded tools rather than only `Manifest::payload_bytes()`.
- `SandboxConfig` accepts an optional `[budgets]` table with reviewed defaults. Existing configs
  without the table continue to load under schema 1; new `init` scaffolds write explicit defaults.
- `doctor --budget-config <path>` loads the reviewed config, measures the transport checkout, and
  fails read-only when any hard threshold is exceeded. JSON output includes the measurements,
  byte ceilings, and violation list.
- The generated publisher now runs
  `doctor --branch-location <transport> --verify --budget-config <config>` before `publish` on
  both Bash and PowerShell lanes, so an over-budget pack never force-pushes over the last healthy
  transport. This repository's source-built publisher does the same with `.pixi-sandbox.toml`.
- The planner already represents partitioning as multiple `[[bundle]]` environment sets; tests now
  lock that `default` and `shells` style partitions become separate self-contained branches such
  as `sandbox/developer-linux-64` and `sandbox/shells-linux-64`.

---

## 4. Decision

External immutable payloads are **not necessary** for the measured state. Castellan's combined
`default+shells` branch is near the new limits but still under them; if it grows, the first remedy
is to publish `shells` as a separate environment-set bundle, preserving one complete manifest and
one complete branch per restored unit.

A payload-format migration should only reopen if environment-set partitioning cannot bring a real
consumer under all four budgets, or if a real remote refuses pushes below these thresholds. Until
then, the hard gate plus `[[bundle]]` partitioning is smaller, testable offline, and compatible
with existing schema-2 transports.

## 5. Reproduction

```bash
# connected transport clone measurements (outside the repository working tree):
git clone --quiet --no-checkout --depth 1 --single-branch \
  --branch sandbox/developer-linux-64 https://github.com/Archont561/castellan.git repo
git -C repo worktree add --quiet --detach checkout HEAD
du -sm repo/.git checkout
python3 - <<'PY'
# parse checkout/.pixi-sandbox/manifest.json: sum env blobs + files oracles + tools + vendor,
# count split parts by part size for largest stored object, and compute restore preflight as
# sum(env.packed + env.unpacked) + 2 * vendor.size.
PY
pixi-sandbox publish --input-dir checkout --branch-name sandbox/developer-linux-64 --dry-run

# fixture publish behavior:
git init -q --bare remote.git
pixi-sandbox publish --input-dir crates/pixi-sandbox/tests/fixtures/transport \
  --branch-name sandbox/demo-linux-64 --remote remote.git
```
