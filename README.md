# Offline sandbox (orphan branch)

Built 2026-10-24T19:29:44Z from commit `d48ea1c` for platform `linux-64`.
`pixi.lock` sha256 `920cde52de77b4f39bdf6320e2a02edb96f49a80e94889d1ae2e1c1c0b017906`.

The verified self-bootstrap binary is stored at `.pixi-sandbox/tools/linux-64/pixi-sandbox`. The branch root intentionally contains documentation only.

| env | platform | packed | unpacked | files |
| --- | --- | ---: | ---: | ---: |
| `default` | linux-64 | 425.2 MiB | 1772.2 MiB | 55 |

Cargo dependencies: **590 crates**, 734.0 MiB (loose) from `Cargo.lock` sha256 `ee2940c56e26…`; restore materialises them to `.pixi-sandbox/vendor/`. Built with cargo 1.99.0 (5f94df478 2026-08-27); rustc 1.99.0 (b940084d7 2026-09-28).


## Restore on the disconnected machine

```bash
./.pixi-sandbox/tools/linux-64/pixi-sandbox doctor --branch-location . --verify
./.pixi-sandbox/tools/linux-64/pixi-sandbox restore --branch-location . --output-path <project> --force
# then, from <project> with no network, use pixi as the sole entrypoint:
pixi install --frozen --offline
pixi run --frozen -- cargo build --offline
```

Every manifest blob is verified before it is written into the working tree.
