#!/usr/bin/env python3
"""Native publish implementation used by the reusable matrix workflow.

The script runs without shell interpolation so environment names, paths, and the Git token never
become shell syntax. It intentionally publishes one bundle/platform pair; matrix orchestration
belongs to the reusable workflow, where each job can use a matching native runner.
"""

from __future__ import annotations

import base64
import os
from pathlib import Path
import shutil
import subprocess
import sys
from typing import Iterable
from urllib.parse import urlparse


class PublishError(RuntimeError):
    pass


def setting(name: str, default: str = "") -> str:
    return os.environ.get(f"PUBLISH_PIXI_SANDBOX_{name}", default).strip()


def github_file_value(value: str, label: str) -> str:
    """Prevent Action output-file injection through a caller-supplied path or branch."""
    if any(character in value for character in ("\r", "\n", "\x00")):
        raise PublishError(f"{label} must not contain newline or NUL characters")
    return value


def host_platform() -> str:
    runner = (os.environ.get("RUNNER_OS", ""), os.environ.get("RUNNER_ARCH", ""))
    mapping = {
        ("Linux", "X64"): "linux-64",
        ("Linux", "ARM64"): "linux-aarch64",
        ("macOS", "X64"): "osx-64",
        ("macOS", "ARM64"): "osx-arm64",
        ("Windows", "X64"): "win-64",
    }
    try:
        return mapping[runner]
    except KeyError as error:
        raise PublishError(
            "cannot establish a native Pixi platform for "
            f"RUNNER_OS={runner[0]!r}, RUNNER_ARCH={runner[1]!r}"
        ) from error


def run(command: Iterable[str], *, cwd: Path, env: dict[str, str] | None = None) -> None:
    command = [str(argument) for argument in command]
    print("+ " + " ".join(command))
    try:
        subprocess.run(command, cwd=cwd, env=env, check=True)
    except (OSError, subprocess.CalledProcessError) as error:
        raise PublishError(f"command failed: {command[0]}") from error


def git_auth_environment(token: str, remote: str) -> dict[str, str]:
    env = dict(os.environ)
    if not token:
        return env
    parsed = urlparse(remote)
    if parsed.scheme != "https" or not parsed.netloc:
        # SSH and file remotes deliberately retain the caller's normal Git credentials.
        return env
    try:
        count = int(env.get("GIT_CONFIG_COUNT", "0"))
    except ValueError as error:
        raise PublishError("GIT_CONFIG_COUNT is not an integer") from error
    basic = base64.b64encode(f"x-access-token:{token}".encode("utf-8")).decode("ascii")
    origin = f"{parsed.scheme}://{parsed.netloc}/"
    env["GIT_CONFIG_COUNT"] = str(count + 1)
    env[f"GIT_CONFIG_KEY_{count}"] = f"http.{origin}.extraheader"
    env[f"GIT_CONFIG_VALUE_{count}"] = f"AUTHORIZATION: basic {basic}"
    return env


def write_output(key: str, value: str) -> None:
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        value = github_file_value(value, f"GitHub output {key}")
        with Path(output).open("a", encoding="utf-8") as handle:
            handle.write(f"{key}={value}\n")


def main() -> None:
    project = Path(setting("PROJECT")).expanduser().resolve()
    self_bin = Path(setting("SELF_BIN")).expanduser().resolve()
    output = Path(setting("OUTPUT_DIR")).expanduser().resolve()
    platform = setting("PLATFORM")
    environments = [item.strip() for item in setting("ENVIRONMENTS").split(",") if item.strip()]
    branch = setting("BRANCH")
    remote = setting("REMOTE")
    cargo_vendor = setting("CARGO_VENDOR", "true").lower() not in {"false", "0", "no"}

    # These values become GitHub outputs after publication. Reject line-oriented workflow-file
    # injection before any install, pack, or push side effect occurs.
    github_file_value(str(output), "output-dir")
    github_file_value(branch, "branch")

    if not project.is_dir():
        raise PublishError(f"project is not a directory: {project}")
    if not self_bin.is_file():
        raise PublishError(f"self-bin is not a file: {self_bin}")
    if not environments:
        raise PublishError("environments is empty")
    if not branch or not remote or not platform:
        raise PublishError("branch, remote, and platform are required")
    actual_platform = host_platform()
    if actual_platform != platform:
        raise PublishError(
            f"native runner mismatch: requested {platform}, but this runner is {actual_platform}; "
            "use the matching native runner from the plan"
        )
    if output.exists():
        raise PublishError(f"output-dir already exists: {output}")

    for environment in environments:
        run(["pixi", "install", "--frozen", "-e", environment], cwd=project)

    # Invoke the exact checksum-verified binary passed by setup-pixi-sandbox, not an arbitrary
    # same-named executable that happened to be earlier on PATH.
    pack = [
        self_bin,
        "pack",
        "--repo-root",
        project,
        "--envs",
        ",".join(environments),
        "--output-dir",
        output,
        "--platform",
        platform,
        "--fetch-tools",
        "--self-bin",
        self_bin,
    ]
    if cargo_vendor:
        pack.append("--cargo-vendor")
    run(pack, cwd=project)
    run(
        [
            self_bin,
            "doctor",
            "--branch-location",
            output,
            "--verify",
        ],
        cwd=project,
    )
    run(
        [
            self_bin,
            "publish",
            "--input-dir",
            output,
            "--branch-name",
            branch,
            "--remote",
            remote,
        ],
        cwd=project,
        env=git_auth_environment(setting("PUSH_TOKEN"), remote),
    )
    write_output("transport", str(output))
    write_output("manifest", str(output / ".pixi-sandbox" / "manifest.json"))
    write_output("branch", branch)


if __name__ == "__main__":
    try:
        main()
    except PublishError as error:
        print(f"::error::{error}", file=sys.stderr)
        raise SystemExit(1)
