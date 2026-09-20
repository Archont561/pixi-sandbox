# pixi-sandbox

Offline sandboxes for [pixi](https://pixi.sh) projects: pack environments (and optionally your
vendored cargo crates) on a connected machine, publish them as a git orphan branch, restore them
on a machine with no network.

```bash
pixi-sandbox pack    --repo-root . --envs dev,docs --output-dir .sandbox-transport --fetch-tools --cargo-vendor
pixi-sandbox doctor  --branch-location .sandbox-transport --verify
pixi-sandbox publish --input-dir .sandbox-transport --branch-name sandbox/dev-linux-64
# on the disconnected machine:
pixi-sandbox restore --branch-location /tmp/branch --path-to-main-repo-code .
```

pixi discovers `pixi-<command>` binaries on `PATH`, so once this is installed the same
verbs work as `pixi sandbox pack|doctor|publish|restore`.

**Status.** The Rust CLI implements `pack`, `publish`, `restore`, `unpack`, `doctor`, `plan`,
and `tools list`; its integration suite exercises a fully verified synthetic pack → unpack →
restore flow, including sharding and vendored cargo sources. Helper-tool pins are compiled into
the release binary, and `.pixi-sandbox.toml` produces a reviewed native publish matrix. The
stdlib-only reference implementation in `.knowledge/research/` remains the portable bootstrap
until a validated static Rust release artifact is completed for every supported airlock platform.

Source-driven CI commands are one line (`pixi run -e ci ci-pack`, `pixi run -e dev lint`, …);
the checksum-verified release publisher is a deliberate composite-action exception. The rule and
task list are in `.knowledge/design.md` §6.

Latest source validation (2026-09-20; pixi 0.81.0, rust 1.98.1, linux-64): 67 Rust tests plus
doc tests and 13 hermetic release-Action/checksum/pin-policy tests pass; `cargo fmt --check`, `cargo clippy
-- -D warnings`, `cargo deny check`, source-archive-safe `actionlint`, `taplo fmt --check`,
Python compilation, and `biome check docs` pass;
`pixi lock --check` confirms the lockfile is current, and the docs site builds. `cargo deny`
reports its existing transitive duplicate-version warnings but exits successfully. The full
network-severed proof remains the separate release gate: it packs, verifies, publishes, restores,
then passes both `pixi install --frozen --offline` and `cargo build --offline`; the portable
bootstrap and Rust restore path were measured against the real 931.7 MiB transport. Measurements
and command evidence: `.knowledge/research/EVIDENCE.md` §12.

## Layout

| path | what it is |
| --- | --- |
| `crates/pixi-sandbox-core` | manifest format, sharding, embedded tool pins, publish-plan schema, verification (library) |
| `crates/pixi-sandbox-git` | git behind a trait: real `git` **and** an in-memory mock, so publish/fetch are testable without a remote |
| `crates/pixi-sandbox` | the CLI (`pack`, `publish`, `restore`, `unpack`, `doctor`, `plan`, `tools`) |
| `crates/pixi-sandbox/tests/fixtures` | a complete pixi project + a synthetic transport — what the tests operate on, *never* this repository |
| `docs/` | Astro + Starlight site → GitHub Pages |
| `.knowledge/` | why everything is the way it is, plus the research artifacts |
| `.github/` | CI workflows plus checksum-verifying setup and native publish composite Actions |
| `.pixi-sandbox.toml` | reviewed environment bundles / native runners for release-driven publishing |
| `scripts/write_release_checksums.py` | deterministic `SHA256SUMS` generator for already proof-accepted native release assets |
| `crates/pixi-sandbox-core/assets/tools.lock.json` | helper-tool pins compiled into the release binary; `--tools-lock` remains an explicit override |

Not in the repository, by design: `.pixi/`, `target/`, `vendor/` and `.pixi-sandbox/` are
generated. The payload lives on the sandbox branch; the crate sources are vendored on demand.

## Working on it

```bash
pixi run -e dev lint      # fmt + clippy + actionlint + taplo
pixi run -e dev test      # cargo nextest
pixi run -e dev coverage  # cargo llvm-cov → lcov.info
pixi run -e dev sandbox-plan # validate .pixi-sandbox.toml / native branches
pixi run -e dev sandbox-proof   # Rust pack/doctor/publish + portable airlock-bootstrap proof
```

Environments: `default` (rust), `dev` (rust + docs + utils + sandbox), `ci` (rust + sandbox, no
local hook tooling), `docs` (bun + biome).

## Publish from a verified release

For consumers, the recommended CI entry point is the reusable
[`.github/workflows/publish-sandboxes.yml`](.github/workflows/publish-sandboxes.yml). It reads a
reviewed `.pixi-sandbox.toml`, creates one native job per bundle/platform, downloads a
checksum-verified standalone release binary through
[`setup-pixi-sandbox`](.github/actions/setup-pixi-sandbox/), and publishes branches such as
`sandbox/developer-linux-64`. Pin the action/workflow commit and binary release tag; see the
[quickstart](docs/src/content/docs/quickstart.mdx) and
[bootstrap release guide](.knowledge/rust-bootstrap.md) for the release contract. Workflow
`uses:` dependencies are full commit-SHA pins; the visible release label comments are informational
rather than executable references.

## The transport, in one picture

```text
<branch>/
├── .pixi-sandbox/
│   ├── manifest.json       every file, its sha256, split parts, tool versions, source commit
│   ├── envs/<env>/pack/    a local conda channel (the original .conda files)
│   ├── tools/<platform>/   pixi · pixi-unpack · pixi-sandbox (static, sha256-pinned, verified)
│   └── vendor/             cargo vendor output (loose tree: git dedup does the heavy lifting)
├── README.md               human guide, generated
└── AGENTS.md               machine guide, generated
```

Rules that the tooling enforces: nothing is written before its hash is verified; whole files
are the shards (a file above 95 MiB is split into `.partNNN` because GitHub blocks git blobs
over 100 MiB); a restore never leaves a half-populated environment.

## License

MIT (see `LICENSE`). Vendored crates and packed conda packages keep their own licenses — generate
a third-party notice from the lockfiles for anything you redistribute.
