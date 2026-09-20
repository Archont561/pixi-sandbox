# Test fixtures

Two things live here, and the rule that ties them together:

| fixture | what it is | used for |
| --- | --- | --- |
| [`demo-project/`](demo-project) | a **complete pixi project definition** — `pixi.toml`, `pixi.lock`, `Cargo.toml`, `Cargo.lock`, `src/` | anything that needs a project to pack: the pack input, the `sha256` a manifest would record, the crate tree a vendor step would carry |
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

`transport/` is checked in so the tests are hermetic. It was originally generated deterministically
via Python; now it is maintained as a static synthetic payload with real digests (edit files directly,
or re-pack via Rust CLI and copy manifest structure). The fixture intentionally includes a split blob
`.partNNN` case.

`demo-project/` is a real project: its `pixi.lock` comes from `pixi lock`, its `Cargo.lock`
from `cargo generate-lockfile` (both were run once and committed). Nothing in the test suite
runs a solver or a download.

## Rules for new tests

1. Project paths come from `fixtures::demo_project()`; payload paths from
   `fixtures::transport()`. Never `..` out of the crate to the repository root.
2. Anything that writes copies the fixture into a `tempfile::TempDir` first — fixtures are
   read-only, and a test that mutates one breaks every other test.
3. A test that needs the network (a real `pixi install`, a real `pixi-pack` download) is
   `#[ignore]`d with the reason in the attribute, so `cargo test` stays offline and fast.
