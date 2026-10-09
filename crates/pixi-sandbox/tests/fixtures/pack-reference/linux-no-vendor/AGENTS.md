# AGENTS.md — machine instructions for this bundle

This is an **offline pixi sandbox**, not source code to merge.

- authoritative manifest: `.pixi-sandbox/manifest.json` (schema 2);
- environments: default (platform linux-64);
- restore with an installed `pixi-sandbox`; this transport does not contain a self-bootstrap binary;
- never download tools at restore time; bundled tools are: pixi, pixi-unpack;
- after restore, use pixi as the only entrypoint: `pixi install --frozen --offline` (or `<project>/.pixi/tools/linux-64/pixi` when user launchers were skipped) must be a no-op;
- no `.pixi/sandbox-env.sh` activation script is generated or supported.
