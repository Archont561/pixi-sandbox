# `publish` — short alias

Short reference alias for `publish-pixi-sandbox`. Use as:

```yaml
- uses: Archont561/pixi-sandbox/publish@v0.2.0
  with:
    project: .
    environments: dev,docs
    platform: linux-64
    branch: sandbox/dev-linux-64
    self-bin: ${{ steps.setup.outputs.path }}
    remote: https://github.com/OWNER/REPO.git
    output-dir: /tmp/transport
```

Implementation lives in `.github/actions/publish-pixi-sandbox/`. Pair with setup:

```yaml
- uses: Archont561/pixi-sandbox/setup@v0.2.0
  id: setup
  with:
    version: v0.2.0
- uses: Archont561/pixi-sandbox/publish@v0.2.0
  with:
    project: .
    environments: dev
    platform: linux-64
    branch: sandbox/linux-64
    self-bin: ${{ steps.setup.outputs.path }}
    remote: https://github.com/OWNER/REPO.git
    output-dir: /tmp/transport
```
