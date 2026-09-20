# demo-project

The fixture project: a complete pixi project definition (conda environment + a small Rust
crate) that the test suite packs, hashes and restores **instead of this repository**.

It intentionally does not depend on `pixi-pack`/`pixi-unpack` — see
`tests/fixtures/README.md` for why that matters.
