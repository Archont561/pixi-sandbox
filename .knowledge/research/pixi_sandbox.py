#!/usr/bin/env python3
"""pixi_sandbox.py — reference implementation of the pixi-sandbox transport.

This is the executable specification the Rust port in `crates/` is written against
(`.knowledge/design.md` §9). It is deliberately one stdlib-only file so that it runs on a
bare machine:

    pack     connected side: pixi envs + cargo vendor + pinned tools -> one directory
    publish  put that directory on an orphan branch (one commit, force-push)
    restore  airlock side:    verify everything, then install it into a project
    unpack   one packed environment -> one prefix (the primitive restore drives)
    doctor   read-only inspection (+ --verify: every sha256, nothing written)

Everything it writes is described by `.pixi-sandbox/manifest.json` (schema 1), which both
this script and `pixi-sandbox-core` read and validate. Rules that are not negotiable:

  * nothing is written before its sha256 is verified (environments, tools *and* the
    vendored crate tree — see `blobs()` in the core crate);
  * shards are whole files; a file above the shard limit becomes `.partNNN` because
    GitHub hard-blocks git blobs over 100 MiB;
  * the airlock never fetches anything: tools are embedded in the branch and are static;
  * the work dir is on the target filesystem, never a small `/tmp` (pixi-unpack stages
    into `$TMPDIR`).

Measured behaviour lives in `.knowledge/research/EVIDENCE.md`.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import time
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

MANIFEST_DIR = ".pixi-sandbox"
MANIFEST_FILE = "manifest.json"
SCHEMA = 1
DEFAULT_SHARD_LIMIT_MIB = 95.0  # GitHub blocks git blobs above 100 MiB
TOOL_NAME = "pixi-sandbox"
TOOL_VERSION = "0.1.0-prototype"
COPY_CHUNK = 1 << 20
# The Rust CLI compiles this catalogue into its release binary. The reference implementation
# reads the same canonical source asset when it is used on the connected side; a shipped branch
# only restores and therefore never needs this path.
DEFAULT_TOOLS_LOCK = Path(__file__).resolve().parents[2] / "crates/pixi-sandbox-core/assets/tools.lock.json"


# --------------------------------------------------------------------------- helpers


def say(msg: str) -> None:
    print(f"\n=== {msg} ===", flush=True)


def sub(msg: str) -> None:
    print(f"  {msg}", flush=True)


def mib(n: float) -> str:
    return f"{n / (1024 * 1024):.1f} MiB"


def now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        while chunk := fh.read(COPY_CHUNK):
            h.update(chunk)
    return h.hexdigest()


def verify_file(path: Path, sha256: str, size: int) -> None:
    """Raise unless `path` is exactly the declared file. Never writes."""
    try:
        actual_size = path.stat().st_size
    except OSError as exc:
        raise SystemExit(f"missing {path}: {exc}")
    if actual_size != size:
        raise SystemExit(f"integrity: {path} is {actual_size} bytes, expected {size}")
    actual = sha256_file(path)
    if actual != sha256:
        raise SystemExit(f"integrity: {path} does not match the manifest (expected {sha256}, got {actual})")


def run(cmd: list, *, cwd: Path | None = None, env: dict | None = None, capture: bool = False) -> str:
    sub("$ " + " ".join(str(c) for c in cmd))
    try:
        proc = subprocess.run(
            [str(c) for c in cmd],
            cwd=str(cwd) if cwd else None,
            env=env,
            stdout=subprocess.PIPE if capture else None,
            stderr=subprocess.STDOUT if capture else None,
            text=True,
        )
    except OSError as exc:  # a path that is not executable, a missing interpreter, …
        raise SystemExit(f"cannot run {cmd[0]}: {exc}")
    if proc.returncode != 0:
        if capture and proc.stdout:
            print(proc.stdout, file=sys.stderr)
        raise SystemExit(f"command failed ({proc.returncode}): {' '.join(str(c) for c in cmd)}")
    return proc.stdout or ""


def which(name: str) -> Path | None:
    found = shutil.which(name)
    return Path(found) if found else None


def cargo_version() -> str:
    cargo = which("cargo")
    if not cargo:
        return "cargo (unknown)"
    return run([cargo, "--version"], capture=True).strip()


def rustc_version() -> str:
    rustc = which("rustc")
    if not rustc:
        return "rustc (unknown)"
    return run([rustc, "--version"], capture=True).strip()


def human_bytes(n: int) -> str:
    return f"{n / (1024 * 1024):.1f} MiB"


# --------------------------------------------------------------------------- linkage


def linkage_of(path: Path) -> str:
    """static | dynamic | script | system | unknown.

    A *shipped* dynamic tool is a bug: it works on the machine that built it and dies on
    the airlock (decision D4). `pixi global install`'s `~/.pixi/bin/*` are trampolines that
    exec a dynamic binary inside their own prefix — never embed one of those.
    """
    try:
        with path.open("rb") as fh:
            head = fh.read(64)
    except OSError:
        return "unknown"
    if len(head) < 20:
        return "unknown"
    if head[:2] == b"#!":
        return "script"
    macho = [b"\xfe\xed\xfa\xce", b"\xce\xfa\xed\xfe", b"\xfe\xed\xfa\xcf", b"\xcf\xfa\xed\xfe"]
    if head[:4] in macho or head[:2] == b"MZ":
        return "system"
    if head[:4] != b"\x7fELF":
        return "unknown"

    little = head[5] == 1
    elf64 = head[4] == 2
    order = "little" if little else "big"

    def u16(b: bytes) -> int:
        return int.from_bytes(b, order)

    def u32(b: bytes) -> int:
        return int.from_bytes(b, order)

    def u64(b: bytes) -> int:
        return int.from_bytes(b, order)

    if elf64:
        if len(head) < 64:
            return "unknown"
        phoff, phentsize, phnum = u64(head[32:40]), u16(head[54:56]), u16(head[56:58])
    else:
        phoff, phentsize, phnum = u32(head[28:32]), u16(head[42:44]), u16(head[44:46])

    if not phoff or not phentsize or not phnum:
        return "unknown"
    with path.open("rb") as fh:
        fh.seek(phoff)
        for _ in range(min(phnum, 64)):
            entry = fh.read(phentsize)
            if len(entry) < 4:
                return "unknown"
            if u32(entry[:4]) == 3:  # PT_INTERP
                return "dynamic"
    return "static"


# --------------------------------------------------------------------------- sharding


def files_under(root: Path) -> list[Path]:
    return sorted(p for p in root.rglob("*") if p.is_file())


def record_file(path: Path, rel: str, limit: int) -> dict:
    """Describe `path` as a manifest blob, splitting it in place when it exceeds `limit`."""
    size = path.stat().st_size
    blob = {"path": rel, "size": size, "sha256": sha256_file(path)}
    if size <= limit:
        return blob

    parts = []
    index = 0
    with path.open("rb") as fh:
        while True:
            data = fh.read(limit)
            if not data:
                break
            part_rel = f"{rel}.part{index:03d}"
            part_abs = path.parent / Path(part_rel).name
            part_abs.write_bytes(data)
            parts.append({"path": part_rel, "size": len(data), "sha256": sha256_file(part_abs)})
            index += 1
    if len(parts) < 2:
        raise SystemExit(f"split_file({path}) produced {len(parts)} part(s)")
    path.unlink()
    blob["parts"] = parts
    return blob


def materialise(src_root: Path, blob: dict, dst: Path) -> None:
    """Copy (joining parts) a blob into `dst`, verifying before the final rename.

    `src_root` is the directory the blob's path is relative to (`<branch>/.pixi-sandbox`).
    Never writes into `src_root`; never leaves a partial file behind on failure.
    """
    dst.parent.mkdir(parents=True, exist_ok=True)
    parts = blob.get("parts") or []
    if not parts:
        src = src_root / blob["path"]
        if src == dst or (src.exists() and dst.exists() and src.samefile(dst)):
            verify_file(dst, blob["sha256"], blob["size"])  # copying onto itself is a verify
            return
        shutil.copyfile(src, dst)
        verify_file(dst, blob["sha256"], blob["size"])
        return

    parts_root = (src_root / blob["path"]).parent
    tmp = dst.with_name(f".{dst.name}.join{os.getpid()}")
    h = hashlib.sha256()
    total = 0
    try:
        with tmp.open("wb") as out:
            for part in parts:
                src = parts_root / Path(part["path"]).name
                if not src.exists():
                    raise SystemExit(f"missing split part {src}")
                verify_file(src, part["sha256"], part["size"])
                with src.open("rb") as fh:
                    while chunk := fh.read(COPY_CHUNK):
                        out.write(chunk)
                        h.update(chunk)
                        total += len(chunk)
        actual = h.hexdigest()
        if actual != blob["sha256"] or total != blob["size"]:
            raise SystemExit(f"integrity: {blob['path']} does not match the manifest")
    except BaseException:
        tmp.unlink(missing_ok=True)
        raise
    dst.unlink(missing_ok=True)
    tmp.rename(dst)


# --------------------------------------------------------------------------- tools


def executable_filename(name: str, platform: str) -> str:
    """Keep manifest identities extension-free while preserving Windows executable paths."""
    if platform.startswith("win-") and not name.lower().endswith(".exe"):
        return f"{name}.exe"
    return name


def tool_path_in_manifest(manifest: dict, name: str) -> str:
    tool = manifest.get("tools", {}).get(name, {})
    return (tool.get("path") or f"tools/{manifest['platform']}/{executable_filename(name, manifest['platform'])}").removeprefix(
        f"{MANIFEST_DIR}/"
    )


def tools_cache_dir(explicit: str | None) -> Path:
    if explicit:
        return Path(explicit).expanduser()
    env = os.environ.get("PIXI_SANDBOX_TOOLS_CACHE")
    if env:
        return Path(env).expanduser()
    return Path.home() / ".cache" / "pixi-sandbox" / "tools"


def load_tools_lock(path: Path) -> dict:
    if not path.exists():
        raise SystemExit(f"no tools lock file at {path}")
    lock = json.loads(path.read_text())
    if lock.get("schema") != 1:
        raise SystemExit(f"{path}: unsupported schema {lock.get('schema')}")
    return lock


def fetch_tool(lock: dict, name: str, platform: str, cache: Path) -> tuple[Path, dict, str]:
    """Download -> verify sha256 -> cache -> execute. Returns (binary, pin, url)."""
    entry = lock["tools"].get(name)
    if not entry:
        raise SystemExit(f"{name} is not pinned in the tools lock file")
    pin = entry["platforms"].get(platform)
    if not pin:
        raise SystemExit(f"{name} has no pin for {platform}")
    url = entry["url_template"].replace("{version}", entry["version"]).replace("{target}", pin["target"])

    cache.mkdir(parents=True, exist_ok=True)
    target = cache / executable_filename(f"{name}-{entry['version']}-{platform}", platform)
    if not target.exists() or sha256_file(target) != pin["sha256"]:
        sub(f"fetch {name} {entry['version']} ({pin['target']})")
        tmp = target.with_name(target.name + ".part")
        with urllib.request.urlopen(url) as response, tmp.open("wb") as fh:  # noqa: S310 (https pin)
            shutil.copyfileobj(response, fh)
        actual = sha256_file(tmp)
        if actual != pin["sha256"]:
            tmp.unlink(missing_ok=True)
            raise SystemExit(f"integrity: {name} from {url} does not match the pin")
        tmp.rename(target)
    target.chmod(target.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)

    reported = run([target, "--version"], capture=True).strip()
    if entry["version"] not in reported:
        raise SystemExit(f"{name}: pinned {entry['version']} but the binary reports {reported!r}")
    return target, pin, url


# --------------------------------------------------------------------------- manifest


def load_manifest(branch: Path) -> dict:
    path = branch / MANIFEST_DIR / MANIFEST_FILE
    if not path.exists():
        raise SystemExit(f"no manifest at {path} (is that an extracted branch?)")
    manifest = json.loads(path.read_text())
    if manifest.get("schema") != SCHEMA:
        raise SystemExit(f"unsupported manifest schema {manifest.get('schema')} (this build reads {SCHEMA})")
    if not manifest.get("envs") and not manifest.get("tools"):
        raise SystemExit(f"{path}: declares no environments and no tools")
    for name, env in manifest.get("envs", {}).items():
        if not env.get("blobs"):
            raise SystemExit(f"{path}: env {name} declares no blobs")
        for blob in env["blobs"]:
            check_blob(blob, f"env {name}")
    for blob in (manifest.get("vendor") or {}).get("blobs", []):
        check_blob(blob, "vendor")
    return manifest


def check_blob(blob: dict, where: str) -> None:
    path = blob.get("path", "")
    if not path or path.startswith("/") or ":" in path or ".." in Path(path).parts:
        raise SystemExit(f"{where}: blob path must stay inside the transport: {path!r}")
    if len(blob.get("sha256", "")) != 64:
        raise SystemExit(f"{where}: {path}: not a sha256 digest")
    parts = blob.get("parts") or []
    if parts and sum(p["size"] for p in parts) != blob["size"]:
        raise SystemExit(f"{where}: {path}: parts do not sum to the blob size")


def blobs_of(manifest: dict, only: list[str] | None = None) -> list[tuple[str, dict]]:
    out: list[tuple[str, dict]] = []
    for name, env in manifest.get("envs", {}).items():
        if only and name not in only:
            continue
        out.extend((name, blob) for blob in env["blobs"])
    # the vendor tree is not filtered by --envs: any restore needs the crates
    out.extend(("vendor", blob) for blob in (manifest.get("vendor") or {}).get("blobs", []))
    return out


def write_branch_docs(out: Path, manifest: dict, vendor_info: dict | None) -> None:
    """Generate the two files a human (README.md) and an agent (AGENTS.md) read on the branch."""
    envs = manifest["envs"]
    platform = manifest["platform"]
    pixi_file = executable_filename("pixi", platform)
    rows = "\n".join(
        f"| `{name}` | {env['platform']} | {human_bytes(env['packed_size_bytes'])} "
        f"| {human_bytes(env['unpacked_size_bytes'])} | {len(env['blobs'])} |"
        for name, env in sorted(envs.items())
    )
    lock = manifest["source"].get("lock_sha256") or "unknown"
    commit = manifest["source"].get("commit")
    vendor = manifest.get("vendor")
    vendor_line = (
        f"Cargo dependencies: **{vendor['crates']} crates**, {human_bytes(vendor['size_bytes'])} "
        f"({vendor['mode']}), vendored from `Cargo.lock` sha256 `{str(vendor.get('cargo_lock_sha256'))[:12]}…` "
        f"with {vendor_info['cargo']}. Restored to `.pixi-sandbox/vendor`."
        if vendor and vendor_info
        else ""
    )

    (out / "README.md").write_text(
        f"""# Offline sandbox (orphan branch)

Built {manifest['created_at']} from commit `{commit}` for platform `{platform}`.
`pixi.lock` sha256 `{lock}`.

| env | platform | packed | unpacked | files |
| --- | --- | --- | --- | --- |
{rows}

{vendor_line}

## Restore (on the disconnected machine)

```bash
# 1. fetch this orphan branch into your clone of the project
git fetch origin pixi-sandbox:pixi-sandbox      # branch name used at publish time

# 2. materialise the branch content anywhere, e.g.
git archive pixi-sandbox | tar -x -C .pixi/branch

# 3. unpack into your working copy
pixi-sandbox restore --branch-location .pixi/branch \\
                     --path-to-main-repo-code .

# 4. sanity check (must be a no-op, and must work with no network)
.pixi/tools/{platform}/{pixi_file} install --frozen --offline
source .pixi/sandbox-env.sh   # puts the bundled toolchain on PATH
```

Integrity: every file listed in `.pixi-sandbox/manifest.json` is verified by sha256
before anything is written into your working tree.
"""
    )

    vendor_block = ""
    if vendor and vendor_info:
        vendor_block = f"""
- vendored cargo crates: `.pixi-sandbox/vendor` ({vendor['crates']} crates, {vendor['mode']}), restore_target
  `.pixi-sandbox/vendor`. They were vendored from `Cargo.lock` sha256 `{vendor.get('cargo_lock_sha256')}` with
  {vendor_info['cargo']} / {vendor_info['rustc']}.
  `restore` wires `.cargo/config.toml` (relative `directory`) and exports `CARGO_NET_OFFLINE=true`.
  After a restore, `cargo build --offline` must succeed with an empty `CARGO_HOME` and no network;
  vendoring contains **no toolchain and no build artifacts** — rustc/cargo must come from the env
  (or the machine) and the airlock pays the compile time.
"""

    (out / "AGENTS.md").write_text(
        f"""# AGENTS.md — machine instructions for this bundle

This branch is an **offline pixi sandbox**, not source code. Do not treat it as a branch to merge.

- manifest: `.pixi-sandbox/manifest.json` (schema {manifest['schema']}) — authoritative list of files,
  sha256 digests, split parts, tool versions, source commit, pixi.lock hash and the vendored crate set.
- envs: {', '.join(sorted(envs))} (platform {platform}).
- To restore: `pixi-sandbox restore --branch-location <dir> --path-to-main-repo-code <project>`.
  The tool verifies every sha256, reassembles split `.partNNN` files, unpacks with
  `pixi-unpack`, installs envs into `<project>/.pixi/envs/<env>` and materialises the vendor tree.
- After restore, `pixi install --frozen --offline` must be a no-op. If it is not, stop:
  do not let pixi try to reach the network.
- Tools shipped here: {', '.join(sorted(manifest['tools']))}. Never download tooling at restore time.
{vendor_block}"""
    )


# --------------------------------------------------------------------------- pack


def cmd_pack(args: argparse.Namespace) -> None:
    root = Path(args.repo_root).resolve()
    out = Path(args.output_dir).resolve()
    payload = out / MANIFEST_DIR
    limit = int(args.shard_limit_mib * 1024 * 1024)
    started = time.time()

    if not (root / "pixi.lock").exists():
        raise SystemExit(f"{root} has no pixi.lock — run `pixi install` first")
    if out.exists():
        raise SystemExit(f"{out} already exists — remove it first (`rm -rf`), never pack into a stale dir")

    lock = load_tools_lock(root / args.tools_lock) if args.fetch_tools else None
    cache = tools_cache_dir(args.tools_cache)

    say(f"1. pack environments ({', '.join(args.envs)} · {args.platform})")
    if args.fetch_tools:
        packer, _, _ = fetch_tool(lock, "pixi-pack", args.platform, cache)
    else:
        packer = which("pixi-pack")
        if not packer:
            raise SystemExit("pixi-pack is not on PATH (use --fetch-tools, or `pixi global install pixi-pack`)")

    envs: dict[str, dict] = {}
    for env in args.envs:
        target = payload / "envs" / env / "pack"
        target.parent.mkdir(parents=True, exist_ok=True)
        output = run(
            [packer, root, "-e", env, "-p", args.platform, "-o", target, "--directory-only"],
            cwd=root,
            capture=True,
        )
        unpacked = 0
        for line in output.splitlines():
            # pixi-pack prints the unpacked size while packing; keep it for the report
            marker = line.lower()
            if "unpacked" in marker:
                number = ""
                unit = ""
                for token, scale in (("GiB", 1024**3), ("MiB", 1024**2), ("KiB", 1024), ("B", 1)):
                    if token.lower() in marker:
                        unit = token
                        tail = line.split(token, 1)[0].strip().split()
                        number = tail[-1] if tail else ""
                        unpacked = int(float(number) * scale) if number else 0
                        break
                if unit:
                    break
        files = files_under(target)
        packed = sum(p.stat().st_size for p in files)
        sub(f"{env}: {len(files)} files, {mib(packed)} packed (unpacked {mib(unpacked)})")
        envs[env] = {
            "platform": args.platform,
            "pack_path": f"{MANIFEST_DIR}/envs/{env}/pack",
            "packed_size_bytes": packed,
            "unpacked_size_bytes": unpacked,
            "pixi_environment_fingerprint": fingerprint_of(root, env),
            "blobs": [],
        }

    say("2. embed the pinned tools")
    tools: dict[str, dict] = {}

    def embed(name: str, src: Path, version: str, url: str | None, pinned: str | None, expect: str | None):
        rel = f"tools/{args.platform}/{executable_filename(name, args.platform)}"
        dst = payload / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(src, dst)
        dst.chmod(dst.stat().st_mode | stat.S_IXUSR)
        actual = sha256_file(dst)
        if expect and actual != expect:
            raise SystemExit(f"integrity: embedded {name} does not match its pin")
        linkage = linkage_of(dst)
        if linkage == "dynamic":
            raise SystemExit(
                f"{name} is dynamically linked — it would work here and fail on the airlock. "
                f"Ship the static release asset (decision D4)."
            )
        tools[name] = {
            "version": version,
            "url": url,
            "pinned_sha256": pinned,
            "linkage": linkage,
            "size_bytes": dst.stat().st_size,
            "path": rel,  # relative to MANIFEST_DIR, like every other path in the manifest
        }
        sub(f"{name} {version} · {linkage} · {mib(dst.stat().st_size)}")

    if args.fetch_tools:
        for name in ("pixi", "pixi-unpack"):
            binary, pin, url = fetch_tool(lock, name, args.platform, cache)
            embed(name, binary, lock["tools"][name]["version"], url, pin["sha256"], pin["sha256"])
    else:
        for name in ("pixi", "pixi-unpack"):
            found = which(name)
            if not found:
                raise SystemExit(f"{name} is not on PATH (use --fetch-tools)")
            embed(name, found, run([found, "--version"], capture=True).strip().split()[-1], None, None, None)

    if args.self_bin:
        embed("pixi-sandbox", Path(args.self_bin).resolve(), TOOL_VERSION, None, None, None)

    vendor_info = None
    vendor = None
    if args.cargo_vendor:
        say("3. vendor the cargo dependencies")
        if not (root / "Cargo.lock").exists():
            raise SystemExit("--cargo-vendor needs Cargo.lock")
        vendor_dir = payload / "vendor"
        shutil.rmtree(vendor_dir, ignore_errors=True)
        run(["cargo", "vendor", "--locked", "--versioned-dirs", vendor_dir], cwd=root, capture=True)
        crates = sorted(p for p in vendor_dir.iterdir() if p.is_dir())
        files = files_under(vendor_dir)
        size = sum(p.stat().st_size for p in files)
        vendor_info = {"cargo": cargo_version(), "rustc": rustc_version()}
        sub(f"{len(crates)} crates, {mib(size)} ({len(files)} files, loose tree)")
        vendor = {
            "mode": args.cargo_vendor_mode,
            "crates": len(crates),
            "size_bytes": size,
            "cargo_lock_sha256": sha256_file(root / "Cargo.lock"),
            "directory": f"{MANIFEST_DIR}/vendor",
            "blobs": [],
        }

    say("4. record the manifest (shard anything above the limit)")
    for scope in ("envs", "tools", "vendor"):
        base = payload / scope
        if not base.exists():
            continue
        for path in files_under(base):
            rel = path.relative_to(payload).as_posix()
            blob = record_file(path, rel, limit)
            if scope == "envs":
                envs[rel.split("/")[1]]["blobs"].append(blob)
            elif scope == "vendor":
                vendor["blobs"].append(blob)

    manifest = {
        "schema": SCHEMA,
        "tool": {"name": TOOL_NAME, "version": TOOL_VERSION},
        "created_at": now(),
        "platform": args.platform,
        "shard_limit_bytes": limit,
        "source": {"commit": git_commit(root), "lock_sha256": sha256_file(root / "pixi.lock")},
        "tools": tools,
        "envs": envs,
    }
    if vendor:
        manifest["vendor"] = vendor

    (payload / MANIFEST_FILE).write_text(json.dumps(manifest, indent=2) + "\n")
    write_branch_docs(out, manifest, vendor_info)

    recorded = sum(len(e["blobs"]) for e in envs.values()) + len(tools) + len(vendor["blobs"] if vendor else [])
    payload_bytes = (
        sum(e["packed_size_bytes"] for e in envs.values())
        + sum(t["size_bytes"] for t in tools.values())
        + (vendor["size_bytes"] if vendor else 0)
    )
    sub(f"payload {mib(payload_bytes)} · {recorded} blobs recorded · {len(files_under(payload))} files on disk")
    sub(f"manifest {payload / MANIFEST_FILE} (schema {SCHEMA})")
    sub(f"pack took {time.time() - started:.1f}s — next: publish --input-dir {out} --branch-name <name>")


def fingerprint_of(root: Path, env: str) -> str | None:
    marker = root / ".pixi" / "envs" / env / "conda-meta" / ".pixi-environment-fingerprint"
    return marker.read_text().strip() if marker.exists() else None


def git_commit(root: Path) -> str | None:
    try:
        out = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "--short", "HEAD"],
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
        )
    except OSError:
        return None
    return out.stdout.strip() or None


# --------------------------------------------------------------------------- publish


def cmd_publish(args: argparse.Namespace) -> None:
    say(f"publish -> {args.branch_name}")
    src = Path(args.input_dir).resolve()
    manifest = load_manifest(src)
    remote = args.remote or "origin"

    if args.dry_run:
        sub(f"would force-push {src} to {remote}:{args.branch_name} as a single commit")
        sub("nothing was written (--dry-run)")
        return

    with tempfile.TemporaryDirectory(prefix="pixi-sandbox-publish-") as tmp:
        repo = Path(tmp) / "repo"
        repo.mkdir()
        run(["git", "init", "-q", "-b", args.branch_name, repo])
        for item in src.iterdir():
            dst = repo / item.name
            if item.is_dir():
                shutil.copytree(item, dst, symlinks=True)
            else:
                shutil.copy2(item, dst)
        run(["git", "-C", repo, "add", "-A"])
        run(
            [
                "git", "-C", repo,
                "-c", "user.email=pixi-sandbox@invalid",
                "-c", "user.name=pixi-sandbox",
                "commit", "-q", "-m",
                f"sandbox snapshot {manifest['created_at']} ({', '.join(sorted(manifest['envs']))} · "
                f"{manifest['platform']} · schema {manifest['schema']})",
            ]
        )
        run(["git", "-C", repo, "push", "--force", remote, f"HEAD:refs/heads/{args.branch_name}"], capture=True)

    files = files_under(src)
    size = sum(p.stat().st_size for p in files)
    sub(f"pushed {len(files)} files ({mib(size)}) as one commit — force-push, no history to merge")
    sub(f"airlock: git fetch {remote} {args.branch_name}:{args.branch_name}")


# --------------------------------------------------------------------------- restore / unpack


def verify_transport(branch: Path, manifest: dict, only: list[str] | None) -> tuple[int, int]:
    """Check every declared byte before anything is written. Returns (files, bytes)."""
    root = branch / MANIFEST_DIR
    files = 0
    total = 0
    for owner, blob in blobs_of(manifest, only):
        abs_path = root / blob["path"]
        parts = blob.get("parts") or []
        if parts:
            parts_root = abs_path.parent
            for part in parts:
                verify_file(parts_root / Path(part["path"]).name, part["sha256"], part["size"])
        else:
            verify_file(abs_path, blob["sha256"], blob["size"])
        files += 1
        total += blob["size"]
    for name, tool in manifest.get("tools", {}).items():
        rel = tool_path_in_manifest(manifest, name)
        abs_path = branch / MANIFEST_DIR / rel
        if not abs_path.exists():
            raise SystemExit(f"missing tool {name} at {abs_path}")
        if abs_path.stat().st_size != tool["size_bytes"]:
            raise SystemExit(
                f"integrity: tool {name} is {abs_path.stat().st_size} bytes, expected {tool['size_bytes']}"
            )
        pin = tool.get("pinned_sha256")
        if pin and sha256_file(abs_path) != pin:
            raise SystemExit(f"integrity: tool {name} does not match its pin")
        if linkage_of(abs_path) == "dynamic":
            raise SystemExit(f"integrity: tool {name} is dynamically linked — do not unpack this transport")
        files += 1
        total += tool["size_bytes"]
    return files, total


def select_envs(manifest: dict, wanted: list[str] | None) -> list[str]:
    names = sorted(manifest.get("envs", {}))
    if not wanted:
        return names
    missing = [w for w in wanted if w not in names]
    if missing:
        raise SystemExit(f"env {', '.join(missing)} is not in this branch (have: {', '.join(names)})")
    return wanted


def work_dir_for(project: Path, explicit: str | None) -> Path:
    work = Path(explicit).expanduser() if explicit else project / ".pixi" / ".restore-work"
    work.mkdir(parents=True, exist_ok=True)
    (work / "tmp").mkdir(exist_ok=True)
    return work


def preflight_space(work: Path, needed: int) -> None:
    free = shutil.disk_usage(work).free
    sub(f"work dir {work} · need ~{mib(needed)}, free {mib(free)}")
    if free < needed:
        raise SystemExit("not enough free space on the work dir's filesystem")


def child_env(work: Path) -> dict:
    env = dict(os.environ)
    # pixi-unpack stages the pack into $TMPDIR before extracting: keep it on the target
    # filesystem. A small tmpfs fails mid-unpack (measured — EVIDENCE §9).
    env["TMPDIR"] = env["TMP"] = env["TEMP"] = str(work / "tmp")
    return env


def materialise_tools(branch: Path, manifest: dict, project: Path, force: bool) -> Path:
    platform = manifest["platform"]
    tools_dir = project / ".pixi" / "tools" / platform
    root = branch / MANIFEST_DIR
    for name, tool in sorted(manifest.get("tools", {}).items()):
        rel = tool_path_in_manifest(manifest, name)
        dst = tools_dir / Path(rel).name
        if dst.exists() and not force and tool.get("pinned_sha256") and sha256_file(dst) == tool["pinned_sha256"]:
            sub(f"{name} {tool['version']}: already present, sha256 matches (skipped)")
            continue
        # Rust omits `pinned_sha256` for a supplied bootstrap script: its transport hash is
        # still authoritative, but it is not an externally pinned release asset.
        expected_sha256 = tool.get("pinned_sha256") or sha256_file(root / rel)
        materialise(root, {"path": rel, "size": tool["size_bytes"], "sha256": expected_sha256}, dst)
        dst.chmod(dst.stat().st_mode | stat.S_IXUSR)
        sub(f"{name} {tool['version']} -> {dst}")
    return tools_dir


def install_env(
    branch: Path, manifest: dict, env: str, project: Path, work: Path, unpacker: Path, force: bool
) -> Path:
    entry = manifest["envs"][env]
    root = branch / MANIFEST_DIR
    pack_dir = work / f"pack-{env}"
    shutil.rmtree(pack_dir, ignore_errors=True)
    pack_dir.mkdir(parents=True)
    for blob in entry["blobs"]:
        # Blob paths include `pack/`; pixi-unpack needs that directory's contents directly.
        rel = blob["path"].removeprefix(f"envs/{env}/pack/")
        materialise(root, blob, pack_dir / rel)
    sub(f"{env}: {len(entry['blobs'])} files materialised into {pack_dir}")

    stage = work / f"stage-{env}"
    shutil.rmtree(stage, ignore_errors=True)
    stage.mkdir(parents=True)
    run([unpacker, pack_dir, "-o", stage, "-e", env], env=child_env(work), capture=True)

    target = project / ".pixi" / "envs" / env
    if target.exists():
        if not force:
            raise SystemExit(f"{target} already exists — pass --force to replace it")
        shutil.rmtree(target)
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.move(str(stage / env), str(target))
    write_markers(target, manifest["envs"][env])
    sub(f"{env} -> {target} ({mib(entry['unpacked_size_bytes']) if entry['unpacked_size_bytes'] else 'size unknown'})")
    return target


def write_markers(prefix: Path, entry: dict) -> None:
    """A raw prefix is not a pixi environment until these exist (decision D5)."""
    conda_meta = prefix / "conda-meta"
    conda_meta.mkdir(parents=True, exist_ok=True)
    (conda_meta / "pixi_env_prefix").write_text(str(prefix.resolve()))
    fingerprint = entry.get("pixi_environment_fingerprint")
    if fingerprint:
        (conda_meta / ".pixi-environment-fingerprint").write_text(fingerprint)


def install_vendor(branch: Path, manifest: dict, project: Path, force: bool) -> None:
    root = branch / MANIFEST_DIR
    vendor_dir = project / MANIFEST_DIR / "vendor"
    if vendor_dir.exists():
        if not force:
            raise SystemExit(f"{vendor_dir} already exists — pass --force to replace it")
        shutil.rmtree(vendor_dir)
    vendor_dir.mkdir(parents=True)
    blobs = (manifest.get("vendor") or {}).get("blobs", [])
    for blob in blobs:
        materialise(root, blob, vendor_dir / blob["path"].removeprefix("vendor/"))
    sub(f"vendor -> {vendor_dir} ({manifest['vendor']['crates']} crates, {mib(manifest['vendor']['size_bytes'])})")


def write_cargo_config(project: Path, mode: str) -> None:
    snippet = (
        "[source.crates-io]\n"
        'replace-with = "vendored-sources"\n\n'
        "[source.vendored-sources]\n"
        f'directory = "{MANIFEST_DIR}/vendor"\n'
    )
    config = project / ".cargo" / "config.toml"
    if mode == "none":
        sub("cargo config: left alone (--cargo-config none)")
        return
    if mode == "print":
        print(snippet)
        return
    if config.exists() and mode == "auto":
        sub(f"{config} already exists — left alone (use --cargo-config write to replace it)")
        return
    config.parent.mkdir(parents=True, exist_ok=True)
    config.write_text(snippet)
    sub(f"wrote {config} — directory is relative to the project root")


def write_sandbox_env(project: Path, manifest: dict) -> None:
    platform = manifest["platform"]
    pixi_file = Path(tool_path_in_manifest(manifest, "pixi")).name
    path = project / ".pixi" / "sandbox-env.sh"
    path.write_text(
        f"""# generated by pixi-sandbox restore — `source .pixi/sandbox-env.sh`
export PATH="{project}/.pixi/tools/{platform}:$PATH"
export CARGO_NET_OFFLINE=true
pixi() {{ command "{project}/.pixi/tools/{platform}/{pixi_file}" "$@"; }}
"""
    )


def cmd_restore(args: argparse.Namespace) -> None:
    branch = Path(args.branch_location).resolve()
    project = Path(args.path_to_main_repo_code).resolve()
    manifest = load_manifest(branch)
    envs = select_envs(manifest, args.envs)

    say("1. verify (nothing is written yet)")
    files, total = verify_transport(branch, manifest, envs)
    sub(f"{files} blob(s), {mib(total)} — every sha256 matches the manifest")
    if args.verify_only:
        sub("--verify-only: nothing was written")
        return

    work = work_dir_for(project, args.work_dir)
    needed = sum(manifest["envs"][e]["packed_size_bytes"] + manifest["envs"][e]["unpacked_size_bytes"] for e in envs)
    if manifest.get("vendor"):
        needed += manifest["vendor"]["size_bytes"] * 2
    preflight_space(work, needed)

    say("2. tools")
    tools_dir = materialise_tools(branch, manifest, project, args.force)
    unpacker = tools_dir / Path(tool_path_in_manifest(manifest, "pixi-unpack")).name
    if not unpacker.exists():
        unpacker = which("pixi-unpack") or Path("pixi-unpack")

    say("3. environments")
    for env in envs:
        install_env(branch, manifest, env, project, work, unpacker, args.force)

    if manifest.get("vendor") and not args.no_vendor:
        say("4. vendored cargo crates")
        install_vendor(branch, manifest, project, args.force)
        write_cargo_config(project, args.cargo_config)

    write_sandbox_env(project, manifest)
    shutil.rmtree(work / "tmp", ignore_errors=True)
    say("restore complete")
    sub(f"source {project / '.pixi' / 'sandbox-env.sh'}")
    sub("pixi install --frozen --offline   # must be a no-op")
    if manifest.get("vendor"):
        sub("cargo build --offline             # must succeed with the vendored crates")


def cmd_unpack(args: argparse.Namespace) -> None:
    src = Path(args.input_dir).resolve()
    out = Path(args.output_dir).resolve()

    # Accept a transport dir, `.pixi-sandbox/envs/<env>/pack`, or a bare pack dir.
    branch = src
    while branch != branch.parent and not (branch / MANIFEST_DIR / MANIFEST_FILE).exists():
        branch = branch.parent
    manifest = load_manifest(branch) if (branch / MANIFEST_DIR / MANIFEST_FILE).exists() else None

    env = args.env
    if manifest:
        if not env:
            names = sorted(manifest["envs"])
            if len(names) != 1:
                raise SystemExit(f"this transport holds {', '.join(names)} — pick one with --env")
            env = names[0]
        if env not in manifest["envs"]:
            raise SystemExit(f"env {env} is not in this transport")
        say(f"unpack {env} from {branch}")
        files, total = verify_transport(branch, manifest, [env])
        sub(f"{files} blob(s), {mib(total)} verified")
    else:
        say(f"unpack {src} (no manifest found — treating it as a bare pack directory)")
        if not env:
            env = "env"

    if args.verify_only:
        sub("--verify-only: nothing was written")
        return

    if out.exists() and any(out.iterdir()):
        if not args.force:
            raise SystemExit(f"{out} is not empty — pass --force to replace it")
        shutil.rmtree(out)
    out.mkdir(parents=True, exist_ok=True)

    work = work_dir_for(out.parent, args.work_dir)
    if args.unpacker:
        unpacker = Path(args.unpacker)
    elif manifest:
        bundled = branch / MANIFEST_DIR / tool_path_in_manifest(manifest, "pixi-unpack")
        unpacker = bundled if bundled.is_file() else (which("pixi-unpack") or Path("pixi-unpack"))
    else:
        unpacker = which("pixi-unpack") or Path("pixi-unpack")

    stage = work / f"unpack-{env}"
    shutil.rmtree(stage, ignore_errors=True)
    stage.mkdir(parents=True)
    if manifest:
        pack_dir = stage / "pack"
        pack_dir.mkdir()
        for blob in manifest["envs"][env]["blobs"]:
            # Blob paths include `pack/`; pixi-unpack needs that directory's contents directly.
            rel = blob["path"].removeprefix(f"envs/{env}/pack/")
            materialise(branch / MANIFEST_DIR, blob, pack_dir / rel)
        source = pack_dir
    else:
        source = src

    run([unpacker, source, "-o", stage, "-e", env], env=child_env(work), capture=True)
    shutil.move(str(stage / env), str(out / env))
    sub(f"{env} -> {out / env}")
    if manifest:
        sub("markers are restore's job — a raw prefix is not yet a pixi environment (D5)")


# --------------------------------------------------------------------------- doctor


def cmd_doctor(args: argparse.Namespace) -> None:
    branch = Path(args.branch_location).resolve()
    manifest = load_manifest(branch)
    envs = select_envs(manifest, args.envs)
    report: dict = {
        "manifest": str(branch / MANIFEST_DIR / MANIFEST_FILE),
        "schema": manifest["schema"],
        "platform": manifest["platform"],
        "created_at": manifest["created_at"],
        "source": manifest["source"],
        "envs": {
            name: {
                "files": len(manifest["envs"][name]["blobs"]),
                "packed": manifest["envs"][name]["packed_size_bytes"],
                "unpacked": manifest["envs"][name]["unpacked_size_bytes"],
                "fingerprint": manifest["envs"][name].get("pixi_environment_fingerprint"),
            }
            for name in envs
        },
        "tools": manifest.get("tools", {}),
        "vendor": manifest.get("vendor") and {
            k: v for k, v in manifest["vendor"].items() if k != "blobs"
        },
    }

    if not args.json:
        sub(f"platform {manifest['platform']} · schema {manifest['schema']} · created {manifest['created_at']}")
        for name in envs:
            entry = manifest["envs"][name]
            sub(f"env {name}: {len(entry['blobs'])} blobs · {mib(entry['packed_size_bytes'])} packed "
                f"-> {mib(entry['unpacked_size_bytes'])} unpacked · fingerprint {entry.get('pixi_environment_fingerprint')}")
        for name, tool in manifest.get("tools", {}).items():
            sub(f"tool {name} {tool['version']} · {tool['linkage']} · {mib(tool['size_bytes'])}")
        if manifest.get("vendor"):
            vendor = manifest["vendor"]
            sub(f"vendor {vendor['crates']} crates · {mib(vendor['size_bytes'])} · {vendor['mode']} · "
                f"{len(vendor.get('blobs', []))} files")
        payload = sum(e["packed_size_bytes"] for e in manifest["envs"].values()) + sum(
            t["size_bytes"] for t in manifest.get("tools", {}).values()
        ) + ((manifest.get("vendor") or {}).get("size_bytes") or 0)
        sub(f"payload {mib(payload)} total")

    if args.verify:
        files, total = verify_transport(branch, manifest, envs)
        report["verify"] = {"files": files, "bytes": total, "ok": True}
        if not args.json:
            sub(f"verify: {files} blob(s), {mib(total)} checked — every declared byte matches")
    elif not args.json:
        sub("hint: pass --verify to check every sha256 (nothing is written)")

    if args.json:
        print(json.dumps(report, indent=2))


# --------------------------------------------------------------------------- cli


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(prog="pixi-sandbox", description=__doc__.splitlines()[0])
    parser.add_argument("--version", action="version", version=f"{TOOL_NAME} {TOOL_VERSION}")
    subs = parser.add_subparsers(dest="command", required=True)

    pack = subs.add_parser("pack", help="pack environments + vendor + tools into a directory")
    pack.add_argument("--repo-root", default=".")
    pack.add_argument("--envs", required=True, help="comma-separated")
    pack.add_argument("--output-dir", required=True)
    pack.add_argument("--platform", default="linux-64")
    pack.add_argument("--shard-limit-mib", type=float, default=DEFAULT_SHARD_LIMIT_MIB)
    pack.add_argument("--cargo-vendor", action="store_true")
    pack.add_argument("--cargo-vendor-mode", default="loose", choices=["loose", "tarballs"])
    pack.add_argument("--fetch-tools", action="store_true")
    pack.add_argument("--tools-lock", default=str(DEFAULT_TOOLS_LOCK))
    pack.add_argument("--tools-cache")  # PIXI_SANDBOX_TOOLS_CACHE is honoured too (decision D4)
    pack.add_argument("--self-bin")
    pack.set_defaults(func=cmd_pack)

    publish = subs.add_parser("publish", help="force-push a transport as an orphan branch")
    publish.add_argument("--input-dir", required=True)
    publish.add_argument("--branch-name", required=True)
    publish.add_argument("--remote")
    publish.add_argument("--keep", type=int, default=0)
    publish.add_argument("--dry-run", action="store_true")
    publish.set_defaults(func=cmd_publish)

    restore = subs.add_parser("restore", help="verify + install a branch into a project")
    restore.add_argument("--branch-location", required=True)
    restore.add_argument("--path-to-main-repo-code", required=True)
    restore.add_argument("--envs", help="comma-separated subset")
    restore.add_argument("--verify-only", action="store_true")
    restore.add_argument("--force", action="store_true")
    restore.add_argument("--no-vendor", action="store_true")
    restore.add_argument("--work-dir")
    restore.add_argument("--cargo-config", default="auto", choices=["auto", "write", "print", "none"])
    restore.set_defaults(func=cmd_restore)

    unpack = subs.add_parser("unpack", help="unpack one packed environment into a prefix")
    unpack.add_argument("--input-dir", required=True)
    unpack.add_argument("--output-dir", required=True)
    unpack.add_argument("--env")
    unpack.add_argument("--unpacker")
    unpack.add_argument("--force", action="store_true")
    unpack.add_argument("--work-dir")
    unpack.add_argument("--verify-only", action="store_true")
    unpack.set_defaults(func=cmd_unpack)

    doctor = subs.add_parser("doctor", help="inspect a transport (read-only)")
    doctor.add_argument("--branch-location", required=True)
    doctor.add_argument("--envs", help="comma-separated subset")
    doctor.add_argument("--verify", action="store_true")
    doctor.add_argument("--json", action="store_true")
    doctor.set_defaults(func=cmd_doctor)

    args = parser.parse_args(argv)
    for field in ("envs",):
        value = getattr(args, field, None)
        if isinstance(value, str):
            setattr(args, field, [v for v in value.split(",") if v])
    args.func(args)


if __name__ == "__main__":
    main()
