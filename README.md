# Offline sandbox (orphan branch)

Built 2026-10-03T18:10:24Z from commit `b13a7fd` for platform `linux-64`.
`pixi.lock` sha256 `3d03e24150cf636336b346e706ccda409118ed890ad31714d227b55007996ae4`.

The verified self-bootstrap binary is stored at `.pixi-sandbox/tools/linux-64/pixi-sandbox`. The branch root intentionally contains documentation only.

| env | platform | packed | unpacked | files |
| --- | --- | ---: | ---: | ---: |
| `default` | linux-64 | 398.9 MiB | 1658.4 MiB | 34 |
| `web` | linux-64 | 47.0 MiB | 171.4 MiB | 34 |

Cargo dependencies: **169 crates**, 280.6 MiB (loose) from `Cargo.lock` sha256 `d0b31d83f24a…`; restore materialises them to `.pixi-sandbox/vendor/`. Built with cargo 1.98.1 (797e8a9bc 2026-08-05); rustc 1.98.1 (48a229cea 2026-09-01).

## Restore on the disconnected machine

```bash
./.pixi-sandbox/tools/linux-64/pixi-sandbox doctor --branch-location . --verify
./.pixi-sandbox/tools/linux-64/pixi-sandbox restore --branch-location . --output-path <project> --force
# then, from <project> with no network, use pixi as the sole entrypoint:
pixi install --frozen --offline
pixi run --frozen -- cargo build --offline
```

Every manifest blob is verified before it is written into the working tree.
