# Pre-extraction pack reference

Captured with the pre-TASK-82 binary at `d8f14cf8` (pixi-sandbox 0.6.0), before any
production extraction. Binary sha256:
`a73cdd6a540e51f6755f9ef8d80b9a9c1a239650615685ff96d57e5bb9e68fa9`.

Inputs are a temporary copy of `demo-project`, the synthetic pack/unpack programs
in `tests/support`, and the committed transport fixture's vendor files supplied
by a synthetic `cargo vendor`. The fake compiler reports `cargo 1.90.0 (fixture)`
and `rustc 1.90.0 (fixture)`; no registry or real environment solve is involved.

Cases: Linux without vendor or self-bin; Linux loose vendor with a self-bin and
host requirements; Windows loose vendor with a self-bin. All payload files and
guides are retained, not just a selected subset of manifest fields.

Only `manifest.created_at` and the corresponding README `Built … from commit`
timestamp are replaced with `2000-01-01T00:00:00Z`. This exception was explicitly
approved for TASK-82; every other byte remains the pre-extraction binary's output.
The CLI comparison supplies the current package version as the reference manifest's
build-version input, without rewriting that field in actual output. Thus a release
bump remains valid but a wrong/hard-coded emitted build stamp fails. At extraction
time the input is 0.6.0, identical to these captured bytes.

Tarball mode is also compared before/after locally (raw archive bytes included),
with fixed fixture mtimes; portable vendor tests compare extracted contents
because tar headers carry the runner's native UID/GID.
