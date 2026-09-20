#!/usr/bin/env python3
"""Run actionlint against the repository's GitHub Actions workflows."""

import os
import shutil
import subprocess
import sys


def main() -> int:
    actionlint = shutil.which("actionlint")
    if not actionlint:
        print("actionlint not found on PATH; skipping action linting", file=sys.stderr)
        return 0

    cmd = [actionlint]
    if len(sys.argv) > 1:
        cmd.extend(sys.argv[1:])
    return subprocess.run(cmd).returncode


if __name__ == "__main__":
    sys.exit(main())
