# Project context

## Airlock bootstrap executable

The Rust CLI implements pack, publish, unpack, and restore, but the default executable embedded
in a sandbox branch is currently the stdlib-only Python bootstrap script. This is intentional:
the available experimental static Rust release artifact is not usable yet, while the Python
script is portable wherever `python3` is present.

A validated standalone Rust release binary can replace it immediately through `--self-bin` or
`PIXI_SANDBOX_SELF_BIN`; do not use the known-bad experimental `target/release/pixi-sandbox`
artifact.

Read **[Rust bootstrap release artifact](.knowledge/rust-bootstrap.md)** before changing the
embedded bootstrap, release workflow, or airlock proof. It defines the integration commands,
platform requirements, validation checklist, and the current provenance caveat.

## Release-driven publishing

`crates/pixi-sandbox-core/assets/tools.lock.json` is compiled into released binaries; source
consumers do not need a root-level tool-pin sidecar. `--tools-lock PATH` remains an explicit
override for a reviewed internal mirror or emergency pin.

Projects opt into published branches through `.pixi-sandbox.toml`. `pixi-sandbox plan --json`
turns that file into native bundle/platform jobs. The publishable custom Action
[`setup-pixi-sandbox`](.github/actions/setup-pixi-sandbox/) checksum-verifies a standalone
GitHub release before execution; the reusable
[`publish-sandboxes.yml`](.github/workflows/publish-sandboxes.yml) workflow orchestrates one
native job and one `sandbox/<bundle>-<platform>` branch per plan entry. Generate `SHA256SUMS`
only from proof-accepted release bytes with
[`scripts/write_release_checksums.py`](scripts/write_release_checksums.py). Do not use `latest`
or cross-platform publication without an explicit reviewed decision. Third-party workflow
Actions and the custom Action source use full commit-SHA pins; mutable tags are rejected by the
Action lint policy.

## Source of truth

- Design and architecture: [`.knowledge/design.md`](.knowledge/design.md)
- Decisions and rationale: [`.knowledge/decisions.md`](.knowledge/decisions.md)
- Measured proof results: [`.knowledge/research/EVIDENCE.md`](.knowledge/research/EVIDENCE.md)
- Knowledge-base index: [`.knowledge/README.md`](.knowledge/README.md)
