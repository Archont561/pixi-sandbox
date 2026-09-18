---
type: Design Spec
title: "Command Surface"
description: Every verb the tool exposes, its flags, defaults and dry-run/JSON behaviour, including docs and kit subcommands.
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, cli]
status: stable
confidence: reasoned
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`DESIGN.md`], sections: ["5"] }
---

# Command Surface

## 5. Command surface

Design rule: **verb = artifact you get, not internal module you poke.** Sub-nouns only where a verb has
genuinely different modes.

```bash
pixi sandbox inventory           [--json] [--tier L0|L1|L2|L3|auto]   # what is in this repo, and what we could prove
pixi sandbox environments        [--json]                             # detected envs + their platforms/features
pixi sandbox platforms           [--env E] [--target P…] [--explain] # does P exist for E? why not?
pixi sandbox plan                [--env E…] [--target P…] [--components auto] [--json] [--dry-run]
pixi sandbox build               [--env E] [--target P] [--release] [--all-targets]

pixi sandbox pack          [ENV…] [--target P…] [--out DIR] [--all-envs] [--executable] [--cache DIR]
pixi sandbox unpack        <tar>  [--into DIR] [--fallback pixi|conda|micromamba|tar|auto]

pixi sandbox vendor         sync [--features F…] [--all-features] [--strategy commit|branch|skip]
pixi sandbox vendor        check [--offline] [--strict]
pixi sandbox vendor        prune

pixi sandbox node           sync [--pkg auto|bun|npm|pnpm|yarn] [--frozen]
pixi sandbox node          check
pixi sandbox node           pack [--out DIR] / unpack <tar>

pixi sandbox kit           build  [--components auto|env,vendor,node,self] [--only-changed [--against REF]] [--tar] [--cache]
pixi sandbox kit          verify  [DIR]
pixi sandbox kit          apply   <dir|url> [--prefix DIR]          # alias of reconstruct (below)

pixi sandbox reconstruct          [--from DIR|git-url] [--env E…] [--tasks] [--mode auto|cache|file-channel|unpack|env-yml|tar] [--print-rung]
pixi sandbox docs          build / pack [--out F] / publish [--branch B] / check [--base B]   # §9.6: Starlight site,
                                         #   mirrored into the repo so the airlock can read it (docs are OFF by default)
pixi sandbox mirror        binary [--path P | --url U] [--as pixi] [--version V]   # ingest pixi/pixi-pack/… into dist staging
pixi sandbox dist           push [FILES…] [--branch B] [--note MSG] [--no-push] [--with-self] [--with-pixi]
pixi sandbox dist            tag / pull [--triple T] [--dest DIR] / bundle [--out F.bundle]

pixi sandbox doctor                [--json] [--strict]
pixi sandbox completions     <shell>
pixi sandbox explain   <ACTION-ID | error-code | gap-id>
```

**Two invariants of this surface.** (a) *Nothing requires config*: with no `pixi-sandbox.toml`, `pack`
takes the env names you pass (arbitrary, including ones that exist only in `pixi.lock`), defaults to the
`default` env, and `--all-envs` means "every environment L1/L2 could enumerate"; `[pack] environments` is
a convenience for CI, not a gate. (b) *`--target` is a request, `platforms` is the answer*: `pack
--target win-64` for an environment that lockfile says has no `win-64` block fails **before** spawning
`pixi-pack`, with `explain` text — see [§7.4](/spec/toolchain-resolution.md#74-platform-validation-does-this-platform-actually-exist).
`reconstruct` and `mirror` are new in this revision because the kit is only finished when it can hand you
back a *working pixi workspace* and a *pixi binary to run it with* ([§9.4](/spec/artifacts.md#94-reconstruction-making-pixi-run-work-offline), [§10.5](/spec/git-registry.md#105-self-hosting-the-branch-ships-pixi-and-the-tool)).

`pixi sandbox …` **and** `./target/release/pixi-sandbox …` are the same program; the `pixi` prefix comes
from pixi's `pixi-<verb>` PATH lookup ✅ verified. Because that lookup only sees *global* PATH,
`doctor` reports whether `pixi-sandbox` is discoverable and offers the exact `pixi global install
--from-lockfile`/`pixi link` remediation. 🚧 (Verify the intended discovery path on your pixi version
before shipping the `pixi sandbox` sugar as documented behaviour.)

### 5.1 Global flags (uniform across verbs)

| Flag | Meaning |
|---|---|
| `-C, --cwd <DIR>` | start discovery here (default: current dir; walks up to the nearest `pixi-sandbox.toml` **or** `pixi.toml`/`pyproject.toml`) |
| `--manifest-path <P>` | point at `pixi.toml`/`pyproject.toml` explicitly — including a *foreign* repo's, which is what makes requirement (a) work. (`--from` is reserved for `reconstruct`/`kit apply`, where it means "artifact source", not "manifest" — two different nouns, so two different flags.) |
| `--config <P>` | explicit `pixi-sandbox.toml` |
| `-e, --env <NAME>` (repeatable) / `--all-envs` | environment selection; names need not appear in any config ([§8.1](/spec/packagers.md#81-pack--pixi-environments--environmenttar)) |
| `--target <PLAT>` (repeatable) | requested conda platform(s) — validated by P0/P1/P2 before anything runs ([§7.4](/spec/toolchain-resolution.md#74-platform-validation-does-this-platform-actually-exist)) |
| `--components <auto\|list>` | which artifacts to bundle; `auto` = derived from detected lockfiles ([D13](/spec/decisions.md#16-decision-log)) |
| `--tier <L0..L3\|auto>` | cap discovery at a tier (`--tier L1` = "no tool may run" — how you get a plan you can reproduce on a box with nothing installed) |
| `-n, --dry-run` | print the plan, execute only `mutates == false` actions |
| `--json` | machine-readable output on stdout; logs to stderr |
| `-v/-q` | verbosity (mirrors `clap-verbosity-flag`, tracing-gated) |
| `--strict` | any `warn` (heuristic-tier detection, degraded platform, sdist, `bun.lockb`) becomes a failure |
| `--offline` | refuse every action with `requires_hosts` non-empty; imply `--frozen`/`--locked` semantics everywhere |
| `--color <auto\|always\|never>` | `auto` **and** auto-disable when stderr is not a TTY (this sandbox has no TTY — colour must not corrupt `--json`) |
| `--yes` | no prompts; required for non-interactive/CI use, so the tool never hangs on a closed stdin |

### 5.2 Ergonomics decisions worth arguing about

1. **Zero arguments is the recommended usage.** `pixi sandbox pack` with no args = pack the `default`
   environment for the host platform, discovered from the manifest ([§4.4](/spec/architecture.md#44-discovery-the-inventory)).
   Passing `--env`/`--target` overrides for *this run*; editing `pixi-sandbox.toml` overrides for
   everybody and CI. Both routes converge on the same resolved `plan`, which is the property that makes
   them interchangeable — and it is why a config file is optional rather than a gate.
2. **Idempotent + convergent.** Running any verb twice does not change the tree: `vendor check`,
   `kit verify`, `node check` are *assertions*; `sync` verbs converge. This is what makes it usable as a
   CI gate and safe for agents to re-run.
3. **`plan` before everything.** `plan` is the only verb that must be perfect, because it is how you
   review a mutation before allowing it (and how the test suite asserts behaviour without a compiler —
   [§14](/spec/testing.md#14-testing-strategy-under-a-no-compiler-here-constraint)).
4. **`explain` instead of stack traces.** Errors carry a code (`E-VENDOR-STALE`), an artifact, and 1–3
   remediation commands; `pixi-sandbox explain E-VENDOR-STALE` prints the long form. Cheap to implement
   (a doc-comment table), disproportionately nice in a terminal.
5. **Never guess a flag.** Sub-tool invocations are assembled from the config's `runner` blocks with
   `auto` resolution + a runtime capability probe (`pixi-pack --help` grep), so a pixi-pack version bump
   produces a clear "flag `--manifest-file` was removed upstream" error instead of a wrong command. ⚠️
   Inferred as feasible; `--help` output parsing is brittle — bound it with an `override` in config.

### 5.3 What detection actually prints

The best argument for the tiered `Inventory` is that it makes the tool *useful on the first run in a
hostile machine*, with no config and nothing installed. `inventory` and `environments` are the same
structure rendered two ways; every line carries the tier that produced it:

```console
$ pixi sandbox inventory --tier L2
workspace   pixi.toml (digest 9f2c…)                                    tier L1
  channels  conda-forge            platforms  linux-64 osx-arm64       tasks  build test lint package
environments (4 detected)
  default    —              lock: 128 pkgs @ linux-64                ✅ L1+L2
  rust       build          lock:  96 pkgs @ linux-64                ✅ L1+L2
  training   cuda, py311    lock: 412 pkgs @ linux-64                ⚠ heuristic: platforms inherited (L2)
  docs       —              lock:  61 pkgs @ linux-64, osx-arm64    ✅ L1+L2
targets
  linux-64    ✅ P0 declared · P1 locked (4/4) · P2 skipped (--offline)
  osx-arm64   ⛔ P1 not-locked for [rust, training]  → add to `platforms`, then `pixi lock`
  win-64      ⛔ P0 not-declared (workspace) — and known-gap: bun has no win-64 on conda-forge ✅ (checked 2026-09-18)
components  auto-derived
  env     on    pixi.lock present (L0)          → pixi-pack per (env × platform)
  vendor  on    Cargo.lock present (L0)         → cargo vendor --locked --versioned-dirs
  node    off   no package.json / lockfile (L0) → not-applicable, not an error
  self    on    package "pixi-sandbox" (L1)     → bin/* into kit + dist branch
toolchains
  pixi 0.81.0 ✅   pixi-pack ⛔ not-found → `pixi exec --spec pixi-pack --` or ship `bin/pixi-pack-<triple>` from the branch (D14)
  cargo  ⛔ not-found, and index.crates.io is reset (L0) → build in CI or rely on vendor/ (D6)
hosts (measured, not assumed)
  open: github.com · api.github.com · codeload.github.com · registry.npmjs.org · pypi.org
  reset: prefix.dev · conda.anaconda.org · index.crates.io · pixi.sh · sh.rustup.rs · ghcr.io
```

Three properties worth naming, because they are what "detects what is available" has to mean to be worth
implementing: it prints **absences as findings** (`node off` is a result, not a shrug); it never claims a
tier it didn't reach (`--tier L2` is *how you test the tool on a box with nothing installed*); and the
platform answer is *layered*, so "win-64 is not declared" and "win-64 is not published by anyone" produce
different remediations — the first is a manifest edit, the second is a substitution or a degraded kit.

---
