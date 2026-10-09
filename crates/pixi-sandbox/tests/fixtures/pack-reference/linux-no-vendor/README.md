# Offline sandbox (orphan branch)

Built 2000-01-01T00:00:00Z from commit `unknown` for platform `linux-64`.
`pixi.lock` sha256 `1e38522bd51f873697d48718560433040b9c2371fc22cd6ea8d95b5fe1db9111`.

This transport has no embedded self-bootstrap binary; use an installed `pixi-sandbox` to restore it.

| env | platform | packed | unpacked | files |
| --- | --- | ---: | ---: | ---: |
| `default` | linux-64 | 0.0 MiB | 0.0 MiB | 2 |


## Restore on the disconnected machine

```bash
pixi-sandbox restore --branch-location <extracted-branch> --output-path <project>
# then, from <project> with no network, use pixi as the sole entrypoint:
pixi install --frozen --offline
pixi run --frozen -- cargo build --offline
```

Every manifest blob is verified before it is written into the working tree.
