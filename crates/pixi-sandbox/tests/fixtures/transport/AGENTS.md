# AGENTS.md — machine instructions for this bundle

This branch is an **offline pixi sandbox** (the pixi-sandbox test fixture), not source code.

- manifest: `.pixi-sandbox/manifest.json` (schema 1) — every file, digest, split part and tool.
- envs: demo (platform linux-64). One blob is split into `.partNNN` pieces on purpose, so
  the split path is covered without a 95 MiB payload.
- To restore: `.pixi-sandbox/tools/linux-64/pixi-sandbox restore --branch-location <dir> --output-path <project>`.
- The tools here are shell stubs: they report their version and nothing else.
