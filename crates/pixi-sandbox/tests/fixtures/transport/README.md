# Offline sandbox (orphan branch)

This is the **test fixture** for pixi-sandbox: a structurally complete transport, committed
so the tool's tests run with no pixi, no packer and no network. It is a static synthetic
payload with real digests (historically generated via Python, now maintained directly).

Built 2026-09-20T00:00:00Z for platform `linux-64`.

The root `pixi-sandbox` is a convenience copy of the manifest's
`.pixi-sandbox/tools/linux-64/pixi-sandbox`. `restore.sh` and `restore.ps1` are thin launchers
that call the root binary; the root copy is intentionally outside the manifest because the
nested copy remains the integrity-checked payload.

| env | platform | packed | files |
| --- | --- | --- | --- |
| `demo` | linux-64 | 10089 B | 5 |

## Restore

```bash
./pixi-sandbox doctor --branch-location . --verify
./pixi-sandbox restore --branch-location . --output-path <project> --force
# or: ./restore.sh <project>
```
