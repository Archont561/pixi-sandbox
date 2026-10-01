# v1 evolution plan: standalone CLI, cross-platform workflow, and JS tooling

**Status:** proposed; derived from the 2026-10-01 design discussion and the existing `.knowledge/` evidence.

## Purpose

This is planning context, not an implementation decision. The backlog is the source of actionable work; this file preserves the broader rationale and alternatives.

## Target architecture

Connected machines and CI install `pixi-sandbox` from `https://prefix.dev/channels/@archont561/pixi-sandbox` and use its binary for `init`, `plan`, `pack`, `doctor`, and `publish`. `init` is provider-neutral: `pixi-sandbox init`, with `--github-workflow-path`, `--script-path`, and `--config` overrides. It generates one platform-appropriate restore launcher and a regenerable workflow that calls the binary directly, with no Archont561 composite actions.

The generated airlock launcher is deliberately thin: resolve `pixi-sandbox.toml` (preferred) or `.pixi-sandbox.toml`, select the platform branch, extract it, and execute the manifest-verified embedded `pixi-sandbox restore`. Restore owns verification, pack unpacking, relocation, Pixi markers, Cargo vendor restoration, and final verification.

## Packaging and deduplication options

The current measured design (D2/D3/D4/D12) embeds pixi-pack and pixi-unpack as pinned tools. A future standalone v1 may replace them with an internal transport implementation, but this is a transport-schema change and requires compatibility tests. Until then, do not duplicate `pixi-sandbox`: the bootstrap binary is the canonical copy; if a project declares it as a dependency, packing must either promote it to the bootstrap tool or explicitly exclude the environment copy. Git hardlinks are not a valid deduplication mechanism because Git archives do not preserve them.

## Platform distribution

The Rust release workflow already builds five native targets. The Pixi/Conda package recipe and package build matrix must also publish linux-64, linux-aarch64, osx-64, osx-arm64, and win-64. Native runners should build package variants; cargo-dist is optional and is not a runtime dependency. It could replace standalone GitHub binary release plumbing, but it does not replace Pixi package creation, transport embedding, or airlock verification.

## JavaScript tooling

Move npm-compatible development tools such as Biome, Astro, Starlight, and TypeScript to `package.json`/`bun.lock`, not Conda. Keep Bun in a separate Pixi web environment. Because Bun currently lacks the desired win-64 Conda package, Windows may use a Node.js feature and install the Bun npm distribution into the active Pixi prefix only if that distribution provides a working Windows executable. Do not use an uncontrolled `npm install -g`.

Turbo is not necessary while `docs` is the only Bun workspace. Adopt it once there are multiple independently buildable JS packages; then Turbo owns the JS task graph, Bun owns packages, and Pixi remains the outer environment/task facade.

## Evidence and conflicts to resolve

The current knowledge base records D2/D3 as a v1 requirement for pixi-pack/pixi-unpack and D8 as conda distribution. The newer standalone proposal must not silently overwrite those decisions. Create a measured spike/ADR before changing them, including binary size, restore disk usage, schema compatibility, and native platform proofs. Existing platform evidence is strongest for Linux and osx-arm64; Windows package and Bun paths require explicit CI proof.

## Backlog mapping

- `v1 standalone cross-platform workflow` milestone groups the implementation.
- `doc-1` is the actionable specification and links the relevant knowledge files.
- `decision-1` records the standalone orchestrator proposal as proposed, not accepted.
- Follow-up tasks separate CLI/config generation, workflow generation, transport/tool deduplication, package distribution, and JS environment orchestration.
