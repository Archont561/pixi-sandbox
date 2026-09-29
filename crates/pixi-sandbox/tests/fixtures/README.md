# Test fixtures

Two things live here, and the rule that ties them together:

| fixture | what it is | used for |
| --- | --- | --- |
| [`demo-project/`](demo-project) | a **complete pixi project definition** — `pixi.toml`, `pixi.lock`, `Cargo.toml`, `Cargo.lock`, `src/` | anything that needs a project to pack: the pack input, the `sha256` a manifest would record, the crate tree a vendor step would carry |
| [`duplicate-source-project/`](duplicate-source-project) | a project whose `Cargo.lock` has **one crate+version from two sources** (crates.io and a git source) — the input `cargo vendor` cannot represent (design.md §11) | the pack-time duplicate-source check: it must fail with a remedy, before anything is written. The lockfile is the only variable against `demo-project` |
| [`transport/`](transport) | a tiny **synthetic sandbox payload** (`.pixi-sandbox/manifest.json` + fake envs, tools, vendor, and one deliberately split blob) | anything that needs a payload that is already packed: `doctor`, `publish`, `restore`, hash verification — no packer, no pixi, no network |

## Why a fixture and not this repository

**Tests must not sandbox this repository.** The repository is this tool's own development
environment, so its `dev` environment contains `pixi-pack`, `pixi-unpack` and a lockfile that
resolves whatever the last `pixi lock` said. A test pointed at the repository root therefore
exercises the developer's machine, not the code:

* it silently succeeds where a fresh clone would fail (pixi-pack came from the hook tooling);
* it is slow (hundreds of MiB) and mutates state (`pixi.lock` churn, `.pixi/` caches);
* it makes a test's result depend on GitHub connectivity, because packing reads the registry;
* and worst of all, a bug that only shows up on a *plain* project — one with no packer in its
  environment — would never be seen.

The fixture is the opposite of all four: `demo-project/pixi.toml` deliberately has **no**
`pixi-pack`/`pixi-unpack` dependency, and `tests/fixtures.rs` asserts that stays true. A test
that wants a packer has to ask for one explicitly (`--fetch-tools` uses the CLI's embedded,
sha256-verified pins and works on a machine with nothing installed), or be marked `#[ignore]`.

## Regenerating

`transport/` is checked in so the tests are hermetic. It is maintained as a static payload with
real digests (edit files directly,
or re-pack via Rust CLI and copy manifest structure). The fixture intentionally includes a split blob
`.partNNN` case and one verified `pixi-sandbox` stub under `.pixi-sandbox/tools/linux-64/`.
Its root contains only `README.md` and `AGENTS.md`, matching the v0.3 transport layout.

One part of it is **not** synthetic: `.pixi-sandbox/envs/demo/pack/prefix/prefix.tar.gz` is a real
conda prefix, captured from a `conda-forge` environment and reduced to the files that matter for
prefix relocation. `transport/README.md` has the recipe.

`demo-project/` is a real project: its `pixi.lock` comes from `pixi lock`, its `Cargo.lock`
from `cargo generate-lockfile` (both were run once and committed). Nothing in the test suite
runs a solver or a download — except the `#[ignore]`d live test, which packs it for real.

`duplicate-source-project/` is real in the same way: its `Cargo.lock` was generated once against
a genuine crates.io `itoa 1.0.15` and a `file://` git repository carrying the same crate at the
same version. The git URL is frozen at the path used to create it, because nothing ever fetches
it — the check reads the lockfile, so the test needs no network and no git remote. It also
carries a `source`-less package so the check is exercised against a lockfile that is not
uniformly shaped.

## Rules for new tests

1. Project paths come from `fixtures::demo_project()`; payload paths from
   `fixtures::transport()`. Never `..` out of the crate to the repository root — the one
   sanctioned exception is `e2e.rs`'s gate test, which reaches the real
   `scripts/airlock-gate.sh` because the script itself is the artifact under test (task-10);
   `fixtures.rs::no_test_targets_the_repository_root` allows exactly that path and nothing else.
2. Anything that writes copies the fixture into a `tempfile::TempDir` first — fixtures are
   read-only, and a test that mutates one breaks every other test.
3. A test that needs the network (a real `pixi install`, a real `pixi-pack` download) is
   `#[ignore]`d with the reason in the attribute, so `cargo test` stays offline and fast.
