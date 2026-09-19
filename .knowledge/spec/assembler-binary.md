---
type: Design Spec
title: "The Assembler Binary (D21)"
description: One Rust binary with two surfaces — the CI verbs the action runs and the assembler the kit ships — prebuilt, musl-static, mirrored like the pixi drivers, installed by nobody.
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, action, rust, distribution, d21]
status: stable
confidence: reasoned
generated: { by: arena-agent/agent-mode, at: 2026-09-19T10:30:00Z }
sources:
  - { id: quantco-pixi-pack, resource: https://github.com/Quantco/pixi-pack, title: Quantco/pixi-pack — v0.7.11 ships musl-static binaries for 8 triples with sha256 digests ✅ }
  - { id: repo, resource: https://github.com/Archont561/pixi-sandbox, title: Archont561/pixi-sandbox }
---

# The Assembler Binary (D21)

> **Status: decided (accepted 2026-09-19).** This is the v1 shape of the artifact. The distribution model of
> [The Action Shape](/spec/action-shape.md) stands; the deliverable is the Rust binary described here, and the
> 183-line `assemble.sh` embedded there is its frozen behavioural oracle.

## 1. One binary, two surfaces

The whole product is one Rust crate (`pixi-sandbox`) with two deployment surfaces of the *same* compiled
program:

| Surface | Where it runs | Verbs | Who invokes it |
|---|---|---|---|
| **CI** | the action's runner (networked) | `pack`, `publish`, `plan --json`, `verify` | the composite action's steps |
| **target** | the sealed machine (git-only) | `reconstruct` (plus `--print-rung`, `--self-test`, `doctor --offline`) | the one-liner |

Developing the crate develops both halves at once — that is the point the proposal exists for. The action's
`action.yml` steps call the CI verbs; `publish` copies the very binary it is running as into the kit's `bin/`,
so the target surface is always the *same revision* that built the kit (recorded in `dist-manifest.json`).

## 2. The one command

```sh
git clone --depth 1 --single-branch --branch pixi-sandbox-dist <repo-url> kit
sh kit/assemble         # sha256 check → pick the host's binary → reconstruct --from kit
. ./.pixi/assemble.env  # printed by the assembler: PATH only, never PIXI_HOME (--promote-path = permanent)
pixi run test           # any task the project's pixi.toml defines; cargo test if rust is in the env
```

The reconstruction is still the binary's job — the kit root merely carries `assemble`, a ~30-line POSIX
**shim** that `publish` writes next to the payloads: it runs `sha256sum -c SHA256SUMS` first (the check
gates its own later lines — verify-before-execute includes the shim), picks `bin/pixi-sandbox-<triple>` via
`uname -m` (no arch names typed by humans; one kit can carry both P0 triples), and `exec`s it with
`reconstruct --from <its own kit dir> "$@"`. Everything measured stays measured: the rung ladder,
`--print-rung`, `assemble.env`, the exit-code API — the nine oracle outcomes ✅.

*(Amendment 2026-09-19, user direction — "the binary shouldn't be fetched on the airlocked host; the user
only defines `sandbox-environment.yml` and calls one command": the earlier draft had the user type the
arch-suffixed path by hand and offered a same-blob `assemble` copy as an optional extra. Rejected: a
compiled binary is arch-specific, so "regardless of triple" only held when the host matched the publishing
runner — exactly the class of quiet assumption this corpus exists to catch. The *fetch* of the binary
happens in CI — the action's digest-pinned fetch step — and `publish` embeds the bytes in the kit; the
airlock clones the kit branch and contacts nothing else 🚧 until M1.)*

No installer, no conda route, no git-install of the tool, no `chmod` (git preserves mode `100755` — the
publishing plumbing already commits executables with `update-index --cacheinfo 100755` ✅). On Windows — no
POSIX `sh` by default — the shim's job is done by hand:
`kit\bin\pixi-sandbox-x86_64-pc-windows-msvc.exe reconstruct …` — one codebase where the shell design needed a
second script.

The one known hazard: `noexec` mounts (some `/tmp`-style volumes, hardened containers). The assembler cannot
fix a kernel policy; the remediation is a named error with the workaround (`git clone` into a mounted-exec
path), which the shell script could not detect either.

## 3. Distribution: mirrored, pinned, verified — exactly like the pixi pair

* **Built where network exists.** CI compiles per triple — `x86_64-unknown-linux-musl` and
  `aarch64-unknown-linux-musl` (P0), `x86_64-pc-windows-msvc` (P1), macOS triples (P2). Static musl ⇒ zero
  runtime dependencies on any Linux, which is precisely the shape pixi-pack's own release assets use
  (8 triples × `pixi-pack`/`pixi-unpack`, every asset with a `sha256` digest ✅ — see
  [Research: pixi-pack §16.7](/research/pixi-pack.md)). We copy the pattern, not just the idea.
* **Pinned by digest.** The action repo publishes the built binaries to an orphan `pixi-sandbox-bin` branch;
  `action.yml` pins the digest it will run; each kit's `SHA256SUMS` covers its copy. Verification before
  execution is the same gate `assemble.sh` already enforces ([§1 of the Action Shape](/spec/action-shape.md)).
* **The bootstrap stays dead.** D11's ceremony existed because a Rust CLI had to arrive on a machine that could
  not build it. A mirrored blob has no such problem: nothing is compiled on the target, the "who builds the
  builder" chain terminates in CI (which reaches crates.io ✅ — the airlock applies to targets, not to CI), and
  the first-ever build is a one-time CI event whose output is pinned forever. The seed cache and `--self-test`
  mode are not resurrected by this decision ⚠️ *(the risk register disagrees until M1.5 runs; see
  [Risks](/spec/risks.md))*.

## 4. The contract the binary must reproduce

The measured `assemble.sh` prototype (nine outcomes ✅, embedded in
[The Action Shape](/spec/action-shape.md)) is frozen as the **behavioural oracle**: the Rust assembler passes
acceptance when it reproduces, vector for vector —

1. integrity before execution (perturbed byte ⇒ exit 4, nothing unpacked);
2. the rung ladder 3 → 2 → 4 → 5 with an explicit refusal (exit 5) rather than a blind `tar` of a
   channel-shaped pack;
3. manifest-driven lazy fetch of payload branches (only what the requested envs need, shallow, digest-verified,
   split blobs reassembled and re-verified);
4. vendor wiring that never edits the user's repo (kit-scoped `.cargo/config.toml`, `cargo metadata --locked
   --offline` ⇒ exit 8 on 101);
5. PATH exposure (`$GITHUB_PATH` append, sourceable `assemble.env`, idempotent `--promote-path`, and never a
   `PIXI_HOME` rewrite);
6. the exit-code API itself: 0 · 1 usage · 4 integrity · 5 unavailable · 7 not-detected · 8
   reconstruction-failed ([Error Taxonomy](/spec/error-taxonomy.md)).

Where the binary may *exceed* the script: parallel branch fetches, native JSON manifest parsing (the fragile
`awk`/`cut` TSV reads disappear), typed errors with remediation text, Windows. Where it may not differ: exit
codes, rung semantics, warning prefixes (`W-…`), and the refusal list. The test vectors live in
[Testing Strategy](/spec/testing-strategy.md).

## 5. What changes in the kit layout

```
pixi-sandbox-dist/            # kit branch (small: index + docs + drivers)
├── assemble                  # the POSIX entry shim: sha256 check → host binary → reconstruct
├── bin/
│   ├── pixi-sandbox-x86_64-unknown-linux-musl      # the assembler (NEW under D21)
│   ├── pixi-x86_64-unknown-linux-musl
│   └── pixi-unpack-x86_64-unknown-linux-musl
├── README.md  AGENTS.md      # rendered by publish, travel with the branch
├── SHA256SUMS  dist-manifest.json  manifest.tsv
└── workspace/{pixi.toml,pixi.lock}
```

Payload branches (`…-envs[-N]`, `…-vendor[-N]`), the blob/branch budgets, split blobs, `manifest.tsv` and lazy
fetches are defined in [Git as the Artifact Registry §10.6](/spec/git-registry.md) — the assembler is a better
*reader* of that layout (native TSV/JSON parsing, parallel fetches), not the author of a new one.
