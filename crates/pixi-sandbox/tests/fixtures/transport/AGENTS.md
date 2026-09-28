# AGENTS.md — machine instructions for this bundle

This branch is an **offline pixi sandbox** (the pixi-sandbox test fixture), not source code.

- manifest: `.pixi-sandbox/manifest.json` (schema 1) — every file, digest, split part and tool;
- envs: demo (platform linux-64). One blob is split into `.partNNN` pieces on purpose;
- branch root: Markdown documentation only;
- verified bootstrap: `.pixi-sandbox/tools/linux-64/pixi-sandbox`;
- restore through the project-side launcher or invoke the nested bootstrap explicitly;
- tools are shell fixtures; `pixi-unpack` materialises a tiny prefix for integration tests.
