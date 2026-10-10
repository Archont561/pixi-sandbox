# Offline sandbox (orphan branch)

Built 2026-10-10T15:02:06Z from commit `a79f82d` for platform `linux-64`.
`pixi.lock` sha256 `7ab9bb32768d185b180e92c9f5a1b03d3a33db1cb037dd60a9e0a15821b98d1a`.

The verified self-bootstrap binary is stored at `.pixi-sandbox/tools/linux-64/pixi-sandbox`. The branch root intentionally contains documentation only.

| env | platform | packed | unpacked | files |
| --- | --- | ---: | ---: | ---: |
| `default` | linux-64 | 436.0 MiB | 1801.5 MiB | 56 |

Cargo dependencies: **268 crates**, 334.2 MiB (loose) from `Cargo.lock` sha256 `07297d846a56…`; restore materialises them to `.pixi-sandbox/vendor/`. Built with cargo 1.99.0 (5f94df478 2026-08-27); rustc 1.99.0 (b940084d7 2026-09-28).


## Restore on the disconnected machine

```bash
./.pixi-sandbox/tools/linux-64/pixi-sandbox doctor --branch-location . --verify
./.pixi-sandbox/tools/linux-64/pixi-sandbox restore --branch-location . --output-path <project> --force
# then, from <project> with no network, use pixi as the sole entrypoint:
pixi install --frozen --offline
pixi run --frozen -- cargo build --offline
```

Every manifest blob is verified before it is written into the working tree.
