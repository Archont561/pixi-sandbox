---
type: Measurement
title: "Host Inventory and Verdict"
description: CPU, memory, disk, OS, and the exact toolchain present or absent on the reference sandbox - with the headline verdict.
resource: https://github.com/Archont561/pixi-sandbox
tags: [environment, inventory]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
verified: { by: process:sandbox-measurement, at: 2026-09-18T21:20:00Z }
stale_after: 2026-10-19T00:00:00Z   # a re-probed sandbox may differ; re-run /environment/reproducing.md
legacy: { files: [`SANDBOX_CONSTRAINTS.md`], sections: ["1", "2", "3"] }
---

# Host Inventory and Verdict

## Verdict in one line

This is a **GitHub + npm + PyPI sandbox, not a Rust/pixi sandbox**: `git`, `gh`, Node and npm work (bun only
*via* npm); the Rust toolchain, crates.io, prefix.dev and conda channels, `pixi.sh` and GitHub release-asset
downloads are all unreachable. So the `pixi`/`cargo` half of the `pixi`-`cargo`-`bun` stack cannot be installed
or executed here — it can only be *designed for* and validated in CI. One measured exception worth remembering:
**`bun` 1.4.2 installs fine via `npm i bun`**, because its npm postinstall pulls the binary from
`registry.npmjs.org` (see §3 below).

Every number in this area was measured on **2026-09-18 ~21:20 UTC** from inside the sandbox; the commands that
produce them are collected in [Reproducing These Measurements](/environment/reproducing.md).

# 1. Identity and layout

| Item | Value |
|---|---|
| Repository | `Archont561/pixi-sandbox` (private) |
| Working dir / git root | `/home/user/pixi-sandbox` |
| Current branch | `arena/01a0b663-pixi-sandbox` (branched from `main` @ `4daee79a`) |
| Repo content at start | `README.md` only (15 bytes), 1 commit, `size: 0` KB reported by API |
| Home directory | `/home/user` (only this tree is snapshotted/persisted) |
| User | `uid=1001(user) gid=1001(user) groups=1001(user),27(sudo),100(users)` |
| Effective privilege | **passwordless `sudo` works** (`sudo -n id` → `uid=0(root)`), but `/root` is not readable |
| Hostname | `e2b.local` (`127.0.1.1 e2b.local`) |
| Runtime platform | E2B code-interpreter style sandbox, `init` = `systemd` (present but not usable as a service manager for us; no `pidof systemd` match on the classic socket path) |

## 2. Machine parameters

```
Linux e2b.local 6.1.158+ #1 SMP PREEMPT_DYNAMIC Mon May 11 18:48:24 UTC 2026 x86_64 GNU/Linux
```

| Parameter | Measured value | Notes for build/CI work |
|---|---|---|
| OS | Debian GNU/Linux 12 (bookworm), `VERSION_ID=12` | Frozen userland; base image pinned to a `snapshot.debian.org` date of `20260610` |
| Kernel | `6.1.158+`, SMP, PREEMPT_DYNAMIC | Built on May 11 2026 |
| Architecture | `x86_64` (32/64-bit op modes) | `linux-64` in conda terms, `x86_64-unknown-linux-gnu`/`-musl` in Rust terms |
| vCPUs | **2** (`nproc`=2, CPU(s): 2, 1 core × 2 threads) | Parallel `cargo`/`pixi` builds will be `j=2` at best |
| Model | Intel(R) Xeon(R) Processor @ 2.60GHz, family 6 model 106 (Ice Lake-SP class) | 54 MiB shared L3, 1.3 MiB L2, 48 KiB L1d |
| CPU flags | **AVX-512 present** (`avx512f/dq/cd/bw/vl/vnni/bitalg/vbmi/vpopcntdq`, `vaes`, `gfni`, `sha_ni`), plus `hle`/`rtm` (TSX) | Safe to use `sha2`/`aes` accelerated code paths; irrelevant for CI runners, which differ |
| Virtualisation | `Hypervisor vendor: KVM`, `Vulnerability … Mitigation; Enhanced/Automatic IBRS` | Full VM, not a container share of a host |
| Memory | `MemTotal 4034452 kB` (**3.84 GiB**), `MemAvailable ≈ 3.6 GiB` | **No swap (`Swap: 0B`)** — OOM is fatal, not slow |
| Disk | `/dev/root` 21 G total, 813 M used, **20 G available (4%)** | Enough for one Rust `target/` dir, not several |
| Inodes | 5 714 800 total, 24 967 used (1%) | Not a constraint |
| cgroup | cgroup **v2 unified**, self cgroup `0::/user`; `/sys/fs/cgroup/user/cpu.max` = `max 100000` (no quota) | CPU count is limited by cpuset (`2`), not by a quota |
| NUMA | 1 node (`node0: 0,1`) | No cross-node tuning needed |
| Locale | `LANG=` empty, `LC_CTYPE="POSIX"` | **`POSIX` locale** — mind Unicode in shell scripts/tests; set `LANG=C.UTF-8` explicitly |
| Timezone | UTC (`Fri Sep 18 21:21:17 UTC 2026`) | Matches the user's stated local TZ |
| Open processes | ~79 tasks at idle | Headroom is large |

### ulimits (`ulimit -a`)

| Limit | Value | Why it matters |
|---|---|---|
| `open files (-n)` | **1024** | The classic failure mode for big `node_modules` / `cargo` link steps; raise with `ulimit -n 65536` inside a task if needed |
| `max user processes (-u)` | 15734 | Plenty |
| `stack size (-s)` | 8192 KB | Deep recursion in `toml_edit`/proc-macro code can still blow it; `RUST_MIN_STACK` if so |
| `max locked memory (-l)` | 8192 KB | Irrelevant here |
| `core file size (-c)` | **0** | **No core dumps** — a segfault leaves nothing to inspect |
| `virtual memory (-v)`, `cpu time (-t)`, `data seg (-d)`, `file size (-f)` | unlimited | No hard wall beyond RAM/disk |
| `pipe size (-p)` | 8 × 512 B | Inter-process streaming is throttled |

## 3. Toolchain inventory (the part that decides the plan)

### Present

| Tool | Version | Path |
|---|---|---|
| `git` | 2.39.5 | `/usr/bin/git` |
| `gh` | 2.23.0 (Debian build, `+dfsg1-1`) | `/usr/bin/gh` — old, but authenticated |
| `curl` | 7.88.1 (OpenSSL 3.0.20, http2, brotli, zstd) | `/usr/bin/curl` |
| `wget` | present | `/usr/bin/wget` |
| `python3` | 3.11.2 + `pip 23.0.1`, `venv`/`ensurepip` OK | `/usr/bin/python3` |
| `node` | **v22.22.3** | `/usr/local/bin/node` |
| `npm` | **10.9.8** (latest published is 12.0.2 — upgrade is possible, see §5) | `/usr/local/bin/npm` |
| `gcc`, `make` | present (`/usr/bin/gcc`, `/usr/bin/make`) | C toolchain exists, so `cc`-sys-based crates *could* compile given a rustc |
| `rg` (ripgrep 13.0.0), `jq 1.6` | present | `ripgrep`/`fd`/`yq`/`taplo`/`just`/`pre-commit` names are **absent** |

### Absent — and not installable inside the sandbox

| Tool | Status | Why the usual install path fails |
|---|---|---|
| **`pixi`** | ✗ absent | `pixi.sh` blocked · `prefix.dev` blocked · GitHub release assets blocked (see §5) |
| **`cargo` / `rustc` / `rustup`** | ✗ absent (no `~/.cargo`, no `~/.rustup`, no `/usr/local/cargo`) | `sh.rustup.rs` blocked · `apt` repos unreachable (port 80) · `index.crates.io`/`static.crates.io` blocked |
| **`bun`** | ✗ not preinstalled — **but installable here** ✅ (measured) | `bun.sh/install` is blocked, yet `npm i bun@1.4.2` works: its `postinstall` (`node install.js`) fetches `@oven/bun-linux-x64` **from `registry.npmjs.org`**, not from GitHub releases. Verified end-to-end: `bun --version` → `1.4.2`, `bun install` resolved+extracted 4 packages and wrote a text `bun.lock`, and `bun ci` → *"Checked 4 installs across 5 packages (no changes)"*. |
| `conda` / `mamba` / `micromamba` | ✗ absent | `conda.anaconda.org`, `micro.mamba.pm`, `repo.anaconda.com` all blocked |
| `docker` | ✗ absent (and no nested containerisation) | N/A |
| `go`, `cmake`, `clang`, `uv`, `pipx`, `taplo`, `just`, `pre-commit`, `fd`, `yq` | ✗ absent | — |

> [!WARNING]
> **There is no way to install a Rust or pixi toolchain in this sandbox.** Every route is cut:
> crates.io, prefix.dev, `pixi.sh`, `sh.rustup.rs`, apt mirrors, `ghcr.io`, and GitHub *release
> asset* storage. Only source **clones** from github.com succeed. Plan for *authored-and-reviewed-in-repo*
> work, verified by real CI, not by local execution.

---
