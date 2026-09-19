---
type: Configuration Schema
title: "Every key pixi.toml and sandbox.lock.json may carry"
description: Full annotated config schema: [kit], [vendor], [policy], [doctor], [docs] plus the lockfile schema (the former [node] block is retired — D20).
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, configuration]
status: stable
confidence: reasoned
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`DESIGN.md`], sections: ["6"] }
sources:
  - { id: pixiprefixdev-latest-reference, resource: https://pixi.prefix.dev/latest/reference/pixi_configuration/, title: pixi.prefix.dev/latest/reference/pixi_configuration/ }
  - { id: bunsh-docs-installation, resource: https://bun.sh/docs/installation, title: bun.sh/docs/installation }
---

# Every key pixi.toml and sandbox.lock.json may carry

## 6. Configuration schema

`pixi-sandbox.toml` at the workspace root (searched upward; every key optional). It is deliberately
small: **it describes the repo, not the run** (run-time choices are flags).

```toml
# pixi-sandbox.toml — how this workspace becomes portable
[meta]
name = "pixi-sandbox"
manifest = "pixi.toml"        # or "pyproject.toml"; default: auto-detect
artifacts-dir = "sandbox"     # gitignored; every artifact lands under here

# ── what to pack ────────────────────────────────────────────────────────────
[pack]
environments = "auto"                    # "auto" = every environment the Inventory detects (§4.4);
                                         #   a list here is an OVERRIDE for CI, not a gate; names are
                                         #   arbitrary — they need not exist in any config to be packed
target-platforms = "auto"                # "auto" = (workspace ∪ per-feature `platforms`) ∩ locked platforms;
                                         #   explicit entries are validated by P0/P1/P2 (§7.4)
create-executable = false                # also emit pixi-pack --create-executable env.sh
use-cache = "~/.cache/rattler"           # forwarded to --use-cache; also the source for kit `cache/pkgs/`
ignore-pypi-non-wheel = true             # tolerate PyPI sdists (they can't be packed)
format = "tar"                           # tar (pixi-pack) | environment-yml (`pixi project export
                                         #   conda-environment` ✅) | both — "both" is the R4-friendly default
inject = []                              # extra --inject packages
# runner: "auto" => prefer a local pixi-pack, else `pixi exec --spec pixi-pack@<pin> --`
[pack.runner]
program = "auto"
pin = "0.7.11"                           # ✅ the version this design was verified against
args = []

# ── cargo source deps ───────────────────────────────────────────────────────
[vendor]
dir = "vendor"
strategy = "branch"                      # "commit" | "branch" | "skip"  (see D9)
branch = "pixi-sandbox-vendor"
format = "directory"                     # how the crates themselves travel (/workflows/pixi-cargo-interop.md §4.5):
                                         #   "directory"      = `cargo vendor` unpacked trees ✅ (no --target
                                         #                    flag ⇒ every platform's crates, all dev/build deps)
                                         #   "local-registry" = `*.crate` + crates.io-format index ✅ (~¼ the
                                         #                    bytes; needs the cargo-local-registry subcommand ✅)
                                         #   "cargo-home"     = `cargo fetch --locked --target <t>` then copy
                                         #                    $CARGO_HOME ✅ — the ONLY target-scoped carrier,
                                         #                    layout is an implementation detail ⚠️ never parse it
wiring = "config-file"                   # who tells cargo about the replacement:
                                         #   "config-file" → write [source.*] into cargo-config below ✅
                                         #   "cli-flags"   → per-invocation `cargo --config …` ✅ (nothing
                                         #                   written into a foreign repo — what `reconstruct`
                                         #                   uses; `cargo vendor` prints this config for us ✅)
                                         #   "cargo-home"  → $CARGO_HOME/config.toml (works when task text
                                         #                   hard-codes `cargo build`)
cargo-config = ".cargo/config.toml"
write-config = true                      # emit the [source.*] replacement block (format+wire allow "none")
all-features = false
features = ["native-tls"]
respect-source-config = false            # needed when vendoring from a mirror ✅
sync-manifests = ["crates/pixi-sandbox-core/Cargo.toml"]   # `-s <manifest>` per extra workspace member ✅ (renamed from sandbox-core D22)
no-delete = false                        # true ⇒ keep foreign files in vendor/ across re-syncs ✅
prune-unverified = false                 # 🚧 shrinking A/B to the host triple: cargo resolves against the
                                         #   replacement source, so a pruned tree is expected to fail with
                                         #   "no matching package named …" — gated behind a flag + probe
# `check` = `cargo metadata --locked --offline` + config/directory consistency; cargo's documented exit
# codes are 0/101 ✅, and 101 with empty stderr is reported as E-VENDOR-INCOMPLETE, never as a pass

# ── node: NOT a v1 component (D20) ─────────────────────────────────────────
# `node_modules` is no longer packed as a kit component. What stays supported:
#   • JS *runtimes* travel as conda packages (`nodejs`, `bun` ✅ see [toolchain.bun] below);
#   • a JS lockfile is still **validated** by CI (`bun ci` ≡ `--frozen-lockfile` ✅), and a binary
#     `bun.lockb` is still a hard failure whose remediation is the migration command;
#   • the docs pipeline installs its 227 MB of `node_modules` with npm in whatever box builds it ✅.
# What is gone: the pack, the `node/` artifact dir, the `node` verbs, the platform-keyed JS tarball.
# `[node] pack = true` remains **reserved** and returns `not-supported` (exit 5) rather than silently
# doing nothing — a config key that lies is worse than one that never existed.
# Evidence behind the decision: §8.3 of [The Three Packagers](/spec/packagers.md) and
# [node_modules and bun in a pixi workspace §5](/research/node-bun.md#5-node_modules--bun-in-a-pixi-workspace).

# ── the offline kit ─────────────────────────────────────────────────────────
[kit]
output-dir = "kit"
components = "auto"                      # "auto" = derive from Inventory; or an explicit list:
                                         #   ["env", "vendor", "self"]   (node → D20)
checksums = true                         # sha256sums.txt
bootstrap = true                         # embed apply.sh / apply.ps1
tarball = false                          # also produce kit.tar.gz (for a USB-stick hop)
cache-source = "use-cache"               # "use-cache": ship pixi-pack's --use-cache dir (channel-shaped, documented ✅)
                                         # "rattler": copy $PIXI_CACHE_DIR/pkgs (rung R1, undocumented combo ⚠️)
                                         # "none": force the R2/R3 path (smallest kit)
ship-pixi = true                         # include bin/pixi-<triple> so the target can run pixi at all
targets-compile = "auto"                 # true ⇒ `vendor` is a real component and `.cargo/config.toml` is
                                         #   written; false ⇒ the kit ships the *built* binary via
                                         #   `pixi-pack --inject <own .conda>` ✅ and skips ~400 crates.
                                         #   "auto" = Cargo.lock exists AND some task runs cargo build/test
only-changed = true                       # skip any component whose digest already exists on the dist branch
allowed-git-origins = ["https://github.com/Archont561/pixi-sandbox.git"]
                                         # G9: git is NOT repo-scoped in an airlock (any public repo clones
                                         #   fine ✅, measured in `inventory.md §4.3`.1), so reachability is
                                         #   not authorization: consuming a vendor/pack branch from an unlisted
                                         #   origin is refused as E-POLICY rather than silently trusted
emit-pixi-config = true                   # write workspace/.pixi/config.toml (priority 10 ✅): offline,
                                         #   [cache] absolute paths, pinning-strategy, netfs-redirect=never

# ── git as the registry ─────────────────────────────────────────────────────
[transport]
branch = "pixi-sandbox-dist"
remote = "origin"
bundle = true                            # also emit a `git bundle` for air-gapped hops
lfs = false
keep-last = 3                            # gc policy for the dist branch (see §10.4)

# ── what to bundle: detection unless overridden (§4.4) ─────────────────────
[discovery]
max-tier = "auto"            # auto = L0..L3 where the tool exists; "L1" = no sub-tool may run
require-lock-for-selection = true   # component auto-derivation trusts lockfiles, not intentions
ignore = ["sandbox/**", "target/**", ".pixi/**", "**/node_modules/**"]

# ── target platforms: request them, then validate them (§7.4) ──────────────
[platforms]
default = ["linux-64"]       # host platform is always implied on top of this
on-missing = "fail"          # fail | warn | skip   (per artifact, never per run)
per-env = { gpu = ["linux-64"], docs = ["linux-64", "osx-arm64"] }
---
refresh-days = 30            # age after which a cached P2 answer must be re-probed in CI
# Gaps are DATA, not folklore. This entry is a *cache* of a verified probe, dated:
known-gaps = [
  { package = "bun", platform = "win-64", checked = "2026-09-18",
    reason = "conda-forge publishes linux-64, linux-aarch64, osx-64, osx-arm64 only (1.3.11) ✅" },
]

# ── self-hosting: the branch carries the drivers, not just your payload (§10.5)
[selfhost]
include-self = true          # bin/pixi-sandbox-<triple>
include-pixi = true           # bin/pixi-<triple> — without it there is no `pixi run` on the target
include-pack-tools = ["pixi-pack", "pixi-unpack"]
pixi-version = "0.81.0"      # ✅ current release; pin whatever you actually mirrored
branch = "pixi-sandbox-mirror"   # keep mirrored upstream blobs discardable, separate from kit payloads
emit-conda-file = true       # also ship .conda so `pixi add <abs path>` works offline ✅ (§9.3)
notice = false               # MUST be true for `--with-pixi`; no NOTICE.md ⇒ refuse (§10.5)

# ── how `reconstruct` yields a WORKING workspace, not a dead prefix (§9.4) ──
[reconstruct]
mode = "auto"                     # auto | cache(R1) | file-channel(R2) | unpack(R3) | env-yml(R4) | tar(R5)
guaranteed-rung = "R3"            # what we *promise*; R1/R2 are the workspace-grade rungs, tried first
remote-mirrors = "off"            # write [mirrors] only when an internal exact-copy mirror exists ✅
ship-activation-cache = "if-lock-matches"   # .pixi/activation-env-v0/*.json is hash-keyed ✅
cache-subset = "pack"             # R1 only: full | pack | none — "pack" copies only what this kit's
                                 #   channel repodata names, so the cache never dwarfs the packs for nothing
pixi-install-args = ["install", "--frozen"]
keep-env-names = true             # reproduce .pixi/envs/<env>, not a flat ./env
verify = ["pixi run __selftest__", "pixi list", "pixi shell -e {env}"]   # the trio, per env
allow-partial = false             # true ⇒ exit 8 downgrades to 0 (prefix without workspace)

# ── per-tool provenance & availability (see §7) ─────────────────────────────
[toolchain.rust]
kind = "conda"
package = "rust"
spec = "1.95.*"          # ✅ conda-forge ships rust; pixi-pack pins ==1.95.0
verify = "cargo --version"

[toolchain.python]
kind = "conda"
package = "python"
spec = "3.12.*"

[toolchain.bun]
kind = "conda"
package = "bun"
spec = "1.3.*"           # ✅ conda-forge has bun (1.3.11, updated 2026-03-18)
platforms = ["linux-64", "linux-aarch64", "osx-64", "osx-arm64"]   # ⚠ NO win-64
fallback = [             # tried in order; each one recorded in provenance
  { kind = "conda", channel = "conda-forge" },
  { kind = "npm", package = "bun" },          # ✅ works even in a GitHub-only sandbox (C2)
  { kind = "manual", hint = "https://bun.sh/docs/installation" },
]

[toolchain.pixi-pack]
kind = "conda"
package = "pixi-pack"
spec = "==0.7.11"

# ── docs site (optional; the same transport as everything else, §9.6) ───────
[docs]
enabled = false                          # "GitHub renders our markdown fine" is a complete answer
source = [".knowledge/**/*.md"]          # the OKF bundle is canonical; sync COPIES concepts into the site
skip = ["**/index.md", "**/log.md"]         # OKF reserved names are navigation, not concepts
site-dir = "docs"                        # Astro project (astro.config.mjs + src/content/docs)
package-manager = "bun"                  # withastro/action auto-detects from the lockfile ✅
site = "https://archont561.github.io"    # both site+base are required, or every asset 404s ✅
base = "/pixi-sandbox/"
publish-branch = "pixi-sandbox-docs"     # orphan branch holding the built site ✅ readable in the airlock
codec = "gzip"                           # "zstd" only where the box has it — ⚠️ absent on this one 🚧
check-links = true                       # starlight-links-validator: a broken link fails the build ✅

# ── the sandbox-aware self-check ────────────────────────────────────────────
[doctor]
require = ["git", "pixi"]
probe = ["github.com", "index.crates.io", "prefix.dev", "conda.anaconda.org"]
probe-timeout-secs = 8
expect-unreachable = ["index.crates.io", "prefix.dev", "conda.anaconda.org"]   # G9: a *reachable* blocked
                                       #   host is reported too — an environment that got more open is not
                                       #   automatically safe to rely on, and a plan keyed on it will not work
                                       #   on the real target
egress-fixture = "fixtures/egress.json"      # the measured matrix, as data (see /workflows/dogfooding.md §7.3)
```

Notes: `deny_unknown_fields` on every section (a typo must fail loudly — `meta.typo = 1` should be a hard
error, not silently ignored); kebab-case TOML keys ↔ snake_case Rust fields via `rename_all`; all paths
**relative to the workspace root**, and any path containing `~` is expanded exactly once at load time (a
🚧 recurring footgun in config handling — pixi itself requires absolute paths in `[cache]` and expands
`~` once ✅ [docs](https://pixi.prefix.dev/latest/reference/pixi_configuration/)).

---
