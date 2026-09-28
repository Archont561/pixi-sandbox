# Offline sandbox (orphan branch)

This is the **test fixture** for pixi-sandbox: a structurally complete transport, committed
so the tool's tests run with no pixi, no packer and no network. It is a static synthetic
payload with real digests, maintained directly.

Built 2026-09-20T00:00:00Z for platform `linux-64`.

The branch root contains Markdown only. Its verified bootstrap lives exclusively at
`.pixi-sandbox/tools/linux-64/pixi-sandbox`, matching the v0.3 transport layout. Project-side
launchers archive this branch and invoke that nested binary.

| env | platform | packed | files |
| --- | --- | --- | --- |
| `demo` | linux-64 | 10089 B | 5 |

## Restore

```bash
./.pixi-sandbox/tools/linux-64/pixi-sandbox doctor --branch-location . --verify
./.pixi-sandbox/tools/linux-64/pixi-sandbox restore \
  --branch-location . --output-path <project> --force
```
