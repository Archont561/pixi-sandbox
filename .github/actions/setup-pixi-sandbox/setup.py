#!/usr/bin/env python3
"""Install one verified standalone pixi-sandbox release asset.

The action deliberately uses only the Python standard library. Hosted GitHub runners provide
Python, and keeping release download/verification dependency-free makes this action usable before
a project's Pixi environment exists.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import tempfile
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import quote
from urllib.request import Request, urlopen

API_DEFAULT = "https://api.github.com"
HEX_SHA256 = re.compile(r"^[0-9a-fA-F]{64}$")
# GitHub owner/repository components and Rust target triples are path-adjacent inputs below.
# Keep them as single, printable components rather than letting an Action input change where a
# verified binary is written or which API route is requested.
REPOSITORY_COMPONENT = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*$")
RUST_TARGET = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*$")
TARGETS = {
    ("Linux", "X64"): ("x86_64-unknown-linux-musl", ""),
    ("Linux", "ARM64"): ("aarch64-unknown-linux-musl", ""),
    ("macOS", "X64"): ("x86_64-apple-darwin", ""),
    ("macOS", "ARM64"): ("aarch64-apple-darwin", ""),
    ("Windows", "X64"): ("x86_64-pc-windows-msvc", ".exe"),
    ("Windows", "ARM64"): ("aarch64-pc-windows-msvc", ".exe"),
}


class SetupError(RuntimeError):
    """An actionable configuration, release, or integrity failure."""


def setting(name: str, default: str = "") -> str:
    return os.environ.get(f"SETUP_PIXI_SANDBOX_{name}", default).strip()


def repository_name(value: str) -> str:
    """Validate an OWNER/REPO API coordinate without permitting route traversal."""
    parts = value.split("/")
    if len(parts) != 2 or not all(REPOSITORY_COMPONENT.fullmatch(part) for part in parts):
        raise SetupError("repository must be an explicit OWNER/REPO value")
    return value


def rust_target(value: str) -> str:
    """Validate an explicit target before it becomes part of an asset name or temp path."""
    if not RUST_TARGET.fullmatch(value):
        raise SetupError(
            "target must be a single Rust target triple using letters, digits, dots, underscores, and hyphens"
        )
    return value


def release_asset_name(value: str, label: str) -> str:
    """Release assets are names, never paths or multi-line GitHub command data."""
    if (
        not value
        or any(character.isspace() or not character.isprintable() for character in value)
        or any(character in value for character in ("/", "\\", "\x00"))
    ):
        raise SetupError(
            f"{label} must be one non-empty asset filename without path separators, whitespace, or control characters"
        )
    return value


def github_file_value(value: str, label: str) -> str:
    """Reject output/path-file injection before appending caller-controlled text."""
    if any(character in value for character in ("\r", "\n", "\x00")):
        raise SetupError(f"{label} must not contain newline or NUL characters")
    return value


def release_tag(value: str) -> str:
    """Keep the tag usable as a directory below the runner's temporary area."""
    value = github_file_value(value, "release tag")
    parts = value.split("/")
    windows_unsafe = "\\<>:\"|?*"
    if (
        not value
        or any(character in value for character in windows_unsafe)
        or any(part in {"", ".", ".."} for part in parts)
    ):
        raise SetupError("release tag must be a non-empty safe relative name")
    return value


def github_headers(token: str) -> dict[str, str]:
    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "pixi-sandbox-setup-action",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    if token:
        headers["Authorization"] = f"Bearer {token}"
    return headers


def request_bytes(url: str, token: str) -> bytes:
    try:
        with urlopen(Request(url, headers=github_headers(token)), timeout=60) as response:
            return response.read()
    except HTTPError as error:
        raise SetupError(f"requesting {url} failed with HTTP {error.code}") from error
    except URLError as error:
        raise SetupError(f"requesting {url} failed: {error.reason}") from error


def release(repository: str, requested: str, token: str) -> dict[str, Any]:
    api = os.environ.get("GITHUB_API_URL", API_DEFAULT).rstrip("/")
    if requested.lower() == "latest":
        endpoint = f"{api}/repos/{repository}/releases/latest"
        print("::warning::setup-pixi-sandbox resolved a moving `latest` release; pin a tag in production.")
    else:
        endpoint = f"{api}/repos/{repository}/releases/tags/{quote(requested, safe='')}"
    try:
        payload = json.loads(request_bytes(endpoint, token))
    except json.JSONDecodeError as error:
        raise SetupError(f"GitHub release API returned invalid JSON for {repository}@{requested}") from error
    if not isinstance(payload, dict) or not isinstance(payload.get("tag_name"), str):
        raise SetupError(f"GitHub release API returned no usable release for {repository}@{requested}")
    return payload


def select_asset(release_data: dict[str, Any], name: str) -> dict[str, Any]:
    assets = release_data.get("assets")
    if not isinstance(assets, list):
        raise SetupError("release has no downloadable assets")
    for asset in assets:
        if isinstance(asset, dict) and asset.get("name") == name:
            return asset
    available = ", ".join(
        str(asset.get("name")) for asset in assets if isinstance(asset, dict) and asset.get("name")
    )
    raise SetupError(f"release asset {name!r} was not found (available: {available or 'none'})")


def checksum_from_manifest(text: str, asset_name: str) -> str:
    # Accept both standard `sha256sum` lines and BSD `SHA256 (file) = hash` lines. A release
    # manifest is a security boundary, so choosing the first of multiple lines for one asset
    # would make the verifier order-dependent; reject that ambiguity instead.
    bsd = re.compile(r"^SHA256 \((?P<name>.+)\) = (?P<hash>[0-9a-fA-F]{64})$")
    matches: list[str] = []
    for raw_line in text.splitlines():
        line = raw_line.strip()
        match = bsd.match(line)
        if match and match.group("name") == asset_name:
            matches.append(match.group("hash").lower())
            continue
        fields = line.split()
        if len(fields) >= 2 and HEX_SHA256.fullmatch(fields[0]):
            candidate = fields[-1].lstrip("*")
            if candidate == asset_name:
                matches.append(fields[0].lower())
    if not matches:
        raise SetupError(f"checksum manifest has no SHA-256 entry for {asset_name!r}")
    if len(matches) != 1:
        raise SetupError(f"checksum manifest has multiple SHA-256 entries for {asset_name!r}")
    return matches[0]


def resolve_target(requested: str) -> tuple[str, str]:
    if requested and requested.lower() != "auto":
        # Explicit targets are intentionally not guessed: callers control the release naming,
        # but not the directory structure used for the verified download.
        target = rust_target(requested)
        return target, ".exe" if target.endswith("windows-msvc") else ""
    runner = (os.environ.get("RUNNER_OS", ""), os.environ.get("RUNNER_ARCH", ""))
    try:
        return TARGETS[runner]
    except KeyError as error:
        raise SetupError(
            "cannot infer a pixi-sandbox target for "
            f"RUNNER_OS={runner[0]!r}, RUNNER_ARCH={runner[1]!r}; pass target explicitly"
        ) from error


def download_verified(url: str, destination: Path, expected: str, token: str) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{destination.name}.", suffix=".download", dir=destination.parent
    )
    actual = hashlib.sha256()
    try:
        with os.fdopen(descriptor, "wb") as output:
            try:
                with urlopen(Request(url, headers=github_headers(token)), timeout=120) as response:
                    while chunk := response.read(1024 * 1024):
                        actual.update(chunk)
                        output.write(chunk)
            except HTTPError as error:
                raise SetupError(f"downloading {url} failed with HTTP {error.code}") from error
            except URLError as error:
                raise SetupError(f"downloading {url} failed: {error.reason}") from error
        found = actual.hexdigest()
        if found != expected:
            raise SetupError(
                f"release asset integrity mismatch: expected SHA-256 {expected}, got {found}"
            )
        os.replace(temporary_name, destination)
    except Exception:
        Path(temporary_name).unlink(missing_ok=True)
        raise


def write_output(key: str, value: str) -> None:
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        value = github_file_value(value, f"GitHub output {key}")
        with Path(output).open("a", encoding="utf-8") as handle:
            handle.write(f"{key}={value}\n")


def add_to_path(directory: Path) -> None:
    github_path = os.environ.get("GITHUB_PATH")
    if github_path:
        value = github_file_value(str(directory), "GitHub PATH entry")
        with Path(github_path).open("a", encoding="utf-8") as handle:
            handle.write(f"{value}\n")


def default_install_dir(temporary: str, tag: str, target: str) -> Path:
    """Place automatic installs below one controlled runner-temp root."""
    root = (Path(temporary) / "pixi-sandbox").resolve()
    candidate = (root / tag / target).resolve()
    try:
        candidate.relative_to(root)
    except ValueError as error:
        raise SetupError("resolved release tag or target escapes the runner temporary directory") from error
    return candidate


def main() -> None:
    repository = repository_name(setting("REPOSITORY"))
    requested_version = setting("VERSION")
    if not requested_version:
        raise SetupError("version is required; use an immutable tag or explicitly request latest")
    token = setting("TOKEN")
    target, executable_suffix = resolve_target(setting("TARGET", "auto"))
    release_data = release(repository, requested_version, token)
    tag = release_tag(str(release_data["tag_name"]))
    template = setting("ASSET_TEMPLATE", "pixi-sandbox-{target}{exe}")
    try:
        asset_name = release_asset_name(
            template.format(
                target=target,
                exe=executable_suffix,
                tag=tag,
                version=tag.removeprefix("v"),
            ),
            "asset-template result",
        )
    except (KeyError, ValueError) as error:
        raise SetupError(
            "asset-template may use only {target}, {exe}, {tag}, and {version}"
        ) from error
    asset = select_asset(release_data, asset_name)
    asset_url = asset.get("browser_download_url")
    if not isinstance(asset_url, str):
        raise SetupError(f"release asset {asset_name!r} has no browser download URL")

    expected = setting("SHA256").lower()
    if expected and not HEX_SHA256.fullmatch(expected):
        raise SetupError("sha256 must be exactly 64 hexadecimal characters")
    if not expected:
        checksums_name = release_asset_name(
            setting("CHECKSUMS_ASSET", "SHA256SUMS"), "checksums-asset"
        )
        checksums = select_asset(release_data, checksums_name)
        checksums_url = checksums.get("browser_download_url")
        if not isinstance(checksums_url, str):
            raise SetupError(f"checksum asset {checksums_name!r} has no browser download URL")
        expected = checksum_from_manifest(request_bytes(checksums_url, token).decode("utf-8"), asset_name)

    install = setting("INSTALL_DIR")
    if install:
        install_dir = Path(install).expanduser()
    else:
        temporary = os.environ.get("RUNNER_TEMP") or tempfile.gettempdir()
        install_dir = default_install_dir(temporary, tag, target)
    binary = install_dir / f"pixi-sandbox{executable_suffix}"
    download_verified(asset_url, binary, expected, token)
    if os.name != "nt":
        binary.chmod(binary.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)

    try:
        version_probe = subprocess.run(
            [str(binary), "--version"],
            check=True,
            capture_output=True,
            text=True,
            timeout=30,
        )
    except (OSError, subprocess.SubprocessError) as error:
        binary.unlink(missing_ok=True)
        raise SetupError(f"verified release asset cannot execute `--version`: {error}") from error
    reported = version_probe.stdout.strip() or version_probe.stderr.strip()
    if not reported:
        binary.unlink(missing_ok=True)
        raise SetupError("verified release asset produced no `--version` output")

    if setting("ADD_TO_PATH", "true").lower() not in {"false", "0", "no"}:
        add_to_path(install_dir)
    write_output("path", str(binary))
    write_output("version", tag)
    write_output("sha256", expected)
    write_output("target", target)
    print(f"pixi-sandbox {reported} ({tag}, {target}, sha256 {expected[:12]}…)")


if __name__ == "__main__":
    try:
        main()
    except SetupError as error:
        print(f"::error::{error}", file=sys.stderr)
        raise SystemExit(1)
