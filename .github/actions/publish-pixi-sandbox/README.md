# `publish-pixi-sandbox`

Internal companion action for the reusable `publish-sandboxes.yml` workflow. It deliberately
publishes **one** `(bundle, platform)` target on the runner selected by `pixi-sandbox plan`:

1. installs the selected Pixi environments with `--frozen`;
2. invokes the exact `self-bin` path from the verified setup action for `pack --fetch-tools` (never an arbitrary `pixi-sandbox` found on `PATH`);
3. verifies the transport with `doctor --verify`;
4. force-pushes the requested orphan branch.

The action refuses a runner whose native platform does not match the requested platform. GitHub
matrix orchestration belongs in the reusable workflow so native runner selection remains visible
and reviewable. A push token is passed to Git through an in-memory `http.*.extraheader`, never
placed in the remote URL or command arguments.

The hermetic action integration test also places a malicious `pixi-sandbox` earlier on `PATH` and
asserts that this action still invokes only the supplied verified `self-bin` path. Branch and
output-path values are rejected if they contain line breaks, before the action performs an install,
pack, or push; they cannot forge additional GitHub Action outputs.
