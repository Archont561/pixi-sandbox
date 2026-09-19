---
type: Playbook
title: "Running the Action"
description: How to call the pixi-sandbox action from a repo, what the target types, and the nine outcomes measured in this sandbox with stub drivers.
resource: https://github.com/Archont561/pixi-sandbox
tags: [workflow, action, ci, dogfooding]
status: stable
confidence: mixed
generated: { by: arena-agent/agent-mode, at: 2026-09-20T00:15:00Z }
verified:
  - { by: process:sandbox-measurement, at: 2026-09-19T00:38:00Z }
sources:
  - { id: actions-learn-github-actions-workflow-syntax, resource: https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/use-actions, title: use-actions — docs.github.com }
  - { id: prefix-dev-setup-pixi, resource: https://github.com/prefix-dev/setup-pixi, title: prefix-dev/setup-pixi }
legacy: { files: [`WORKFLOWS.md`], sections: ["1", "2"] }
---

# Running the Action

## The caller's workflow (pin the action by SHA — same rule we apply to `withastro/action` ✅)

```yaml
name: kit
on:
  push: { branches: [main], paths: ["pixi.toml", "pixi.lock", "Cargo.lock", ".github/workflows/kit.yml"] }
permissions: { contents: write }
concurrency: { group: kit, cancel-in-progress: false }
jobs:
  publish:
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix: { platform: [linux-64] }        # one runner per platform — see the matrix rule
    steps:
      - uses: actions/checkout@<sha>
      - uses: archont561/pixi-sandbox@<sha>   # the public action
        with:
          branch: pixi-sandbox-dist
          environments: default rust
          platforms: ${{ matrix.platform }}
          bundle-crates: "true"               # ← the "develop locally with recompilation" switch
          ship-pixi: "true"
        id: kit
      - run: echo "pushed ${{ steps.kit.outputs.revision }} (${{ steps.kit.outputs.bytes }} bytes)"
```

For a fork PR or a "what would change" run, the same job with `push: "false"` stages and prints the manifest
instead of writing the branch ✅ — that is v1's substitute for `--dry-run`.

## What the target types

```bash
git clone --depth 1 --single-branch --branch pixi-sandbox-dist <url> kit
./kit/bin/pixi-sandbox-x86_64-unknown-linux-musl reconstruct \
    --from kit --env rust --with-vendor --workspace ~/work/myproj --print-rung
cd ~/work/myproj && . .pixi/assemble.env && pixi list && pixi run build
```

One clone, one command, no `curl`, no `sudo`, no root, and nothing to install: the reconstructor **is** the
mirrored `pixi-sandbox` binary in the kit's `bin/` (D21), git keeps its exec bit, and `--print-rung` makes the
*strength* of what you got machine-readable (`3` = installer-made prefix with `conda-meta`, `2` = offline
`pixi install --frozen` against the kit channel, `4` = a foreign `micromamba`/`conda` on the pack's
`environment.yml`, `5` = files only, and the assembler **refuses** — exit 5 — rather than tar a channel-shaped
pack when no rung above the floor is reachable). `pixi run cargo test` works the same way when the env carries
the conda-forge `rust` package and CI packed the vendor graph. The behaviour above is exactly the measured
`assemble.sh` oracle's — the binary reproduces it vector-for-vector ([Testing Strategy](/spec/testing-strategy.md)).

## The nine outcomes, measured in this sandbox

`pixi`, `pixi-unpack`, `cargo` and `micromamba` cannot be installed here, so they were **stubbed on `PATH`** —
which tests the script's decisions (its whole job) rather than rattler's. The kit fixtures are shaped exactly as
CI produces them (a pack tarball containing `envs/<env>/…`, `SHA256SUMS`, drivers in `bin/`). 2026-09-19,
`/home/user/proto`, `sh`:

| # | Fixture | Expected | Result |
|---|---|---|---|
| T1 | full kit, `pixi-unpack` + `cargo` stubs | verify → install drivers → unpack → vendor wired → exit 0 | ✅ exit 0, `rung=3`, `conda-meta/history` present, `.cargo/config.toml` written with `directory = "<ws>/.pixi/vendor/vendor"`, `assemble.env` written |
| T2 | kit without `pixi-unpack`, channel present | **descend to rung 2**, never a blind tar | ✅ exit 0, `rung=2`, `file://` kit channel registered, `pixi install --frozen` |
| T3 | rung 2 refused (stub pixi fails), `micromamba` on PATH | rung 4 on the pack's `environment.yml` | ✅ exit 0, `rung=4` |
| T4 | channel-shaped pack, no unpacker, no channel, no installer | **refuse**, not a broken-looking success | ✅ exit 5, `E-UNAVAILABLE` names the reason and points at this file |
| T5 | a raw prefix tar (`--pack-format raw`) | the floor rung, loudly | ✅ exit 0, `rung=5`, warning "prefix paths are NOT rewritten" printed |
| T6 | one payload byte perturbed | fail **before** executing anything | ✅ exit 4, `E-INTEGRITY` (`sha256sum -c` caught it, nothing unpacked) |
| T7 | `cargo metadata --locked --offline` → 101 | never a silent pass | ✅ exit 8, `E-VENDOR-INCOMPLETE` |
| T8 | `--env gpu` when only `demo` is packed · kit with no `bin/pixi-*` | named absence, not a guess | ✅ exit 7, `E-NOT-DETECTED: no pack for env 'gpu'` · exit 5, `E-UNAVAILABLE: no pixi binary in the kit` |
| T9 | `$GITHUB_PATH` set · `--promote-path` run twice | PATH exposure that outlives the process | ✅ `$GITHUB_PATH` gained the workspace-local `bin/`, `assemble.env` holds `export PATH=…` (and *nothing else* — no `PIXI_HOME` rewrite), rc files edited exactly once |

Two findings the prototype forced into the spec (both now in
[Error Taxonomy](/spec/error-taxonomy.md#what-a-shell-implementation-must-do-about-exit-codes)):

1. **`set -eu` leaks foreign exit codes.** The first draft exited **2** (tar's status) where the contract says
   **8**; every external command now ends in `|| fail <CLASS> … <code>`. A script cannot claim the exit-code API
   while letting raw tool statuses through.
2. **`tar -C <dir>` needs the directory to exist**, and only `pixi-unpack` creates it. The R5 path died with
   "Cannot open: No such file or directory" until `mkdir -p` was added — a two-line bug that would have looked
   like a broken pack on a real target.

Three more the second round of hardening added (all reproduced in T1/T2/T4/T9):

3. **Drivers must be resolved by provenance, not by PATH luck.** The first draft happily used *any* `pixi-unpack`
   on PATH, so a kit without one would "succeed" with a stale copy the last run left behind. Now the kit's own
   verified copies win, a foreign one earns a `W-PROVENANCE` warning, and `$BIN/.pixi-sandbox-kit` records which
   kit last wrote the bin dir.
4. **`--pack-format` (auto\|pack\|raw)** distinguishes a pixi-pack tarball from a raw prefix tar by listing the
   archive for `pixi-pack.json`/`channel/`. A channel-shaped pack is never unpacked to the floor rung; a raw one
   never pretends to be rung 3.
5. **PATH exposure is part of reconstruction.** `$GITHUB_PATH` gets the driver dir appended (so later CI steps and
   jobs see `pixi`), every workspace gets a sourceable `assemble.env`, and `--promote-path` appends to rc files
   idempotently — without touching the user's `~/.local/bin`, which is *their* pixi's home.

## Reproducing the harness

```bash
mkdir -p fakebin && cat > fakebin/pixi-unpack <<'EOF'
#!/bin/sh   # args: -o OUT -e envs/ENV PACK ; emulate Prefix::install by writing conda-meta
…
EOF
printf '#!/bin/sh\necho "cargo-stub: $*"; exit 0\n' > fakebin/cargo && chmod +x fakebin/*
PATH="$PWD/fakebin:$PATH" sh assemble.sh --from branch --env demo --with-vendor --workspace ws --print-rung
```

Stubbing is not a shortcut here: the reconstructor (and the oracle script it was measured as) contains **no**
solver, unpacker or compiler logic — it is selection, verification and wiring, which is precisely what a stub
exercises 🚧 *(the honest limit: real `pixi-unpack`/`pixi-pack` semantics still need one CI run to confirm)*.

## Publishing the action itself (the "public" half)

1. Repo `Archont561/pixi-sandbox-action` (or a `/action` subdirectory of this repo — Actions must be referenced
   by repo+ref, so **a public repo of its own is friendlier**: consumers pin `@<sha>` without cloning the docs).
2. `action.yml` at the root ✅ (the validated contract in [The Action Shape](/spec/action-shape.md)), whose
   steps call the **Rust binary's CI verbs** (`pixi-sandbox pack` / `publish`) — no `scripts/` of shell files.
   The kit's reconstructor is that *same binary* mirrored into `bin/`
   ([The Assembler Binary](/spec/assembler-binary.md)), so kit copy == builder copy by construction.
3. Tags `v1`, `v1.0.0` pointing at the same commit, and README's first block is the `uses:` snippet — plus the
   rule this repo already lives by: **callers pin by SHA, not by tag** ✅.
4. A `ci.yml` in the action repo that runs the L1/L2 `cargo test` suite plus `js-yaml` on every push, and
   `actionlint` + `zizmor` on a runner ⚠️ *(those cannot be installed in this sandbox — their npm wrappers fetch
   native binaries, measured ❌)*, and one **real** job that builds a fixture kit and reconstructs it in an
   `alpine` container — that job *is* the L3 gate in [Testing Strategy](/spec/testing-strategy.md) (M1.5).
