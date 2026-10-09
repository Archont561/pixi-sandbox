# Offline sandbox (orphan branch)

Built 2000-01-01T00:00:00Z from commit `unknown` for platform `linux-64`.
`pixi.lock` sha256 `1e38522bd51f873697d48718560433040b9c2371fc22cd6ea8d95b5fe1db9111`.

The verified self-bootstrap binary is stored at `.pixi-sandbox/tools/linux-64/pixi-sandbox`. The branch root intentionally contains documentation only.

| env | platform | packed | unpacked | files |
| --- | --- | ---: | ---: | ---: |
| `default` | linux-64 | 0.0 MiB | 0.0 MiB | 2 |

Cargo dependencies: **1 crates**, 0.0 MiB (loose) from `Cargo.lock` sha256 `f894d0c873a3…`; restore materialises them to `.pixi-sandbox/vendor/`. Built with cargo 1.90.0 (fixture); rustc 1.90.0 (fixture).


Host requirements (provided by the host, never installed by this transport): libc 2.34 · packages fontconfig · capabilities display.

## Restore on the disconnected machine

```bash
./.pixi-sandbox/tools/linux-64/pixi-sandbox doctor --branch-location . --verify
./.pixi-sandbox/tools/linux-64/pixi-sandbox restore --branch-location . --output-path <project> --force
# then, from <project> with no network, use pixi as the sole entrypoint:
pixi install --frozen --offline
pixi run --frozen -- cargo build --offline
```

Every manifest blob is verified before it is written into the working tree.
