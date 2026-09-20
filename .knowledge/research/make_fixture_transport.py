#!/usr/bin/env python3
"""Generate the checked-in test payload: `crates/pixi-sandbox/tests/fixtures/transport`.

The fixture is a *synthetic* sandbox payload — no `.conda` is real, the tools are shell
stubs — but it is structurally exactly what `pack` produces: a manifest (schema 1) with
every file's size and sha256, environments, embedded tools, a vendored crate tree, generated
`README.md`/`AGENTS.md`, and one deliberately **split** blob so `doctor`/`restore` exercise
the `.partNNN` path with the fixture alone.

Why checked in rather than built at test time: the tests that use it (`doctor`, `publish`,
`restore`, verification) must be hermetic — no pixi, no packer, no network. The hashes are
computed here and committed, so a payload that changed by accident shows up as a diff.

    python3 .knowledge/research/make_fixture_transport.py     # idempotent

See `crates/pixi-sandbox/tests/fixtures/README.md` for the policy it serves.
"""

from __future__ import annotations

import hashlib
import json
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "crates" / "pixi-sandbox" / "tests" / "fixtures" / "transport"
MANIFEST_DIR = ".pixi-sandbox"
PLATFORM = "linux-64"
SHARD_LIMIT = 4096  # tiny on purpose: the fixture must stay small and still exercise splitting
CREATED_AT = "2026-09-20T00:00:00Z"


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def write(path: Path, data: bytes) -> dict:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    return {"path": path.relative_to(FIXTURE / MANIFEST_DIR).as_posix(), "size": len(data), "sha256": sha256(data)}


def record(path: Path) -> dict:
    """A blob for `path`, split into `.partNNN` siblings when it exceeds the limit."""
    blob = write(path, path.read_bytes())
    if blob["size"] <= SHARD_LIMIT:
        return blob

    data = path.read_bytes()
    parts = []
    for index, start in enumerate(range(0, len(data), SHARD_LIMIT)):
        chunk = data[start : start + SHARD_LIMIT]
        part = path.with_name(f"{path.name}.part{index:03d}")
        part.write_bytes(chunk)
        parts.append(
            {
                "path": part.relative_to(FIXTURE / MANIFEST_DIR).as_posix(),
                "size": len(chunk),
                "sha256": sha256(chunk),
            }
        )
    path.unlink()
    blob["parts"] = parts
    return blob


def tool_stub(name: str, version: str) -> bytes:
    """A shell stub: `pixi --version` prints something the fixture's tests can assert."""
    return (
        "#!/bin/sh\n"
        f"# fixture stub for {name} {version} (tests/fixtures/README.md)\n"
        f'if [ "$1" = "--version" ]; then echo "{name} {version}"; exit 0; fi\n'
        f'echo "{name} stub: $*"\n'
    ).encode()


def main() -> None:
    if FIXTURE.exists():
        shutil.rmtree(FIXTURE)
    payload = FIXTURE / MANIFEST_DIR

    # --- environments ---------------------------------------------------------------
    env_blobs = []
    env_root = payload / "envs" / "demo" / "pack"
    env_blobs.append(write(
        env_root / "channel" / "linux-64" / "demo-tool-0.1.0-0.conda",
        b"# fixture .conda (not a real package)\nname: demo-tool\nversion: 0.1.0\n" + b"x" * 512,
    ))
    env_blobs.append(write(
        env_root / "channel" / "noarch" / "demo-pure-0.1.0-0.conda",
        b"# fixture .conda (not a real package)\nname: demo-pure\nversion: 0.1.0\n",
    ))
    # deliberately above the shard limit: 9 000 bytes -> three parts
    big = env_root / "channel" / "noarch" / "demo-big-0.1.0-0.conda"
    big.parent.mkdir(parents=True, exist_ok=True)
    big.write_bytes(b"# fixture .conda sized to be split\n" + bytes(range(256)) * 36)
    env_blobs.append(record(big))
    env_blobs.append(write(
        env_root / "environment.yml",
        b"name: demo\nchannels:\n- ./.pixi-sandbox/envs/demo/pack/channel\n- conda-forge\ndependencies:\n- demo-tool 0.1.0\n",
    ))
    env_blobs.append(write(
        env_root / "pixi-pack.json",
        b'{"channels": ["./.pixi-sandbox/envs/demo/pack/channel"], "environment": "demo"}\n',
    ))
    packed = sum(b["size"] for b in env_blobs)

    # --- tools ----------------------------------------------------------------------
    tools = {}
    for name, version, pinned in (
        ("pixi", "0.81.0", None),
        ("pixi-unpack", "0.7.11", None),
        ("pixi-sandbox", "0.1.0", None),
    ):
        path = payload / "tools" / PLATFORM / name
        data = tool_stub(name, version)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        path.chmod(0o755)
        tools[name] = {
            "version": version,
            "url": None,
            "pinned_sha256": pinned,
            "linkage": "script",  # a stub script is allowed; a *dynamic* binary would not be
            "size_bytes": len(data),
            "path": f"tools/{PLATFORM}/{name}",
        }

    # --- vendored crates ------------------------------------------------------------
    vendor_blobs = [
        write(payload / "vendor" / "demo-dep-1.0.0" / "Cargo.toml",
              b'[package]\nname = "demo-dep"\nversion = "1.0.0"\n'),
        write(payload / "vendor" / "demo-dep-1.0.0" / "src" / "lib.rs",
              b'pub fn offline() -> &\'static str {\n    "vendored"\n}\n'),
    ]

    # --- generated branch docs (what pack writes for humans and agents) --------------
    (FIXTURE / "README.md").write_text(
        f"""# Offline sandbox (orphan branch)

This is the **test fixture** for pixi-sandbox: a structurally complete transport, committed
so the tool's tests run with no pixi, no packer and no network. It is generated by
`.knowledge/research/make_fixture_transport.py` — edit that, not this.

Built {CREATED_AT} for platform `{PLATFORM}`.

| env | platform | packed | files |
| --- | --- | --- | --- |
| `demo` | {PLATFORM} | {packed} B | {len(env_blobs)} |

## Restore

```bash
.pixi-sandbox/tools/{PLATFORM}/pixi-sandbox doctor  --branch-location . --verify
.pixi-sandbox/tools/{PLATFORM}/pixi-sandbox restore --branch-location . --path-to-main-repo-code .
```
"""
    )
    (FIXTURE / "AGENTS.md").write_text(
        f"""# AGENTS.md — machine instructions for this bundle

This branch is an **offline pixi sandbox** (the pixi-sandbox test fixture), not source code.

- manifest: `{MANIFEST_DIR}/manifest.json` (schema 1) — every file, digest, split part and tool.
- envs: demo (platform {PLATFORM}). One blob is split into `.partNNN` pieces on purpose, so
  the split path is covered without a 95 MiB payload.
- To restore: `{MANIFEST_DIR}/tools/{PLATFORM}/pixi-sandbox restore --branch-location <dir> --path-to-main-repo-code <project>`.
- The tools here are shell stubs: they report their version and nothing else.
"""
    )

    manifest = {
        "schema": 1,
        "tool": {"name": "pixi-sandbox", "version": "0.1.0"},
        "created_at": CREATED_AT,
        "platform": PLATFORM,
        "shard_limit_bytes": SHARD_LIMIT,
        "source": {"commit": "0000000", "lock_sha256": sha256((ROOT / "pixi.lock").read_bytes())},
        "tools": tools,
        "envs": {
            "demo": {
                "platform": PLATFORM,
                "pack_path": f"{MANIFEST_DIR}/envs/demo/pack",
                "packed_size_bytes": packed,
                "unpacked_size_bytes": packed * 4,
                "pixi_environment_fingerprint": "0123456789abcdef",
                "blobs": env_blobs,
            }
        },
        "vendor": {
            "mode": "loose",
            "crates": 1,
            "size_bytes": sum(b["size"] for b in vendor_blobs),
            "cargo_lock_sha256": sha256(
                (ROOT / "crates/pixi-sandbox/tests/fixtures/demo-project/Cargo.lock").read_bytes()
            ),
            "directory": f"{MANIFEST_DIR}/vendor",
            "blobs": vendor_blobs,
        },
    }
    (payload / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")

    blobs = len(env_blobs) + len(tools) + len(vendor_blobs)
    size = sum(p.stat().st_size for p in FIXTURE.rglob("*") if p.is_file())
    split = sum(1 for b in env_blobs if b.get("parts"))
    print(f"fixture transport written: {FIXTURE}")
    print(f"  {blobs} blobs declared, {split} split blob, {size} bytes on disk")


if __name__ == "__main__":
    main()
