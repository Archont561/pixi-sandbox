# Evidence base

## Upstream tools
* [pixi: manifest, lockfile, subcommands](/research/pixi.md) - What pixi 0.81.0 actually promises: manifest fields, tasks, environments, and the external-subcommand contract a pixi-sandbox binary rides on. _(✅)_
* [pixi-pack: format, flags, limits](/research/pixi-pack.md) - pixi-pack 0.7.11 in detail: pack layout, pixi-unpack, inject, and the boundaries this design must respect. _(✅)_
* [cargo vendor and offline builds](/research/cargo-offline.md) - Vendoring mechanics, source replacement, and cargo's own rules for dependency sources - what an offline Rust build really needs. _(✅)_
* [node_modules and bun in a pixi workspace](/research/node-bun.md) - How JS dependencies can (and cannot) travel with a pixi environment: bun on conda-forge, node_modules packaging, lockfile formats. _(✅)_
* [Rust CLI engineering with clap and thiserror](/research/rust-cli.md) - The dependency set and error-design patterns chosen for the tool, grounded in upstream docs and issue tracker. _(✅)_

## Platform limits
* [GitHub Actions limits and branch rules](/research/github-actions.md) - Quotas, pinning, permissions, caching and the branch/ref constraints that bound the CI design. _(✅)_
* [GitHub enhanced Markdown](/research/github-markdown.md) - The alert/admonition/table/mermaid subset GitHub renders natively, and what it does with front matter. _(✅)_
* [AGENTS.md and CONTEXT.md conventions](/research/agent-instruction-files.md) - How agent-facing instruction files are laid out, sized and scoped, and how they relate to an OKF bundle. _(✅)_
* [Open Knowledge Format](/research/okf.md) - The spec this bundle implements: v0.2 bundle shape, required front matter, trust families, reserved names, conformance rules. _(✅)_

## Measurements in this sandbox
* [What a Real Airlock Can Fetch](/research/airlock-fetching.md) - First-hand egress measurements for the dogfood loop: git throughput, blob:none clones, asset digests, dead hosts. _(✅)_
* [The Rust Toolchain Is a Conda Package](/research/conda-forge-rust.md) - Evidence that conda-forge rust is rustc+cargo in package form, what that fixes in the design, and what it does not. _(✅)_
* [Docs Pipeline, Measured in the Airlock](/research/docs-pipeline.md) - Probe log for the Starlight build: install size and timing, offline reproducibility, the four silent failures, link-validator counts. _(✅)_

## Reasoning about the corpus
* [How the research collapses into this design](/research/synthesis.md) - The flowchart and five concrete constraints that the research produced, kept next to the design they justify. _(⚠️ reasoned)_
* [Corrections and Method Notes](/research/corrections.md) - Claims that were retracted mid-research, and the rules of thumb that emerged for working in this sandbox. _(✅)_
* [Manifest discovery, platform validation, offline reconstruction](/research/rev2-discovery.md) - The second-round research that answered the five requirements: inventory tiers, platform existence, and the reconstruction rungs. _(✅)_
* [Bibliography](/research/sources.md) - Every external source read for this design, with what was verified first-hand versus quoted from a snippet. _(✅)_

## Group notes

This directory is one area of the `pixi-sandbox` knowledge bundle; the bundle root index is
[here](/index.md) and the conventions (labels, extensions, maintenance rules) are in
[Bundle Conventions](/conventions.md).
