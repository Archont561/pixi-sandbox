# The design

## Shape of the tool
* [Architecture](/spec/architecture.md) - Module map, the two-crate layout, and the tiered Inventory that decides what can be packed. _(⚠️ reasoned)_
* [Command Surface](/spec/cli.md) - Every verb the tool exposes, its flags, defaults and dry-run/JSON behaviour, including docs and kit subcommands. _(⚠️ reasoned)_
* [Every key pixi.toml and sandbox.lock.json may carry](/spec/configuration.md) - Full annotated config schema: [kit], [vendor], [policy], [doctor], [docs], plus the lockfile schema and defaults ([node] is reserved and returns not-supported since D20). _(⚠️ reasoned)_

* [The Action Shape (v1)](/spec/action-shape.md) - The GitHub Action: inputs, outputs, the branch layout it pushes, the relocation trap, the embedded `action.yml`, and `assemble.sh` — the measured oracle of the assembler binary. _(✅ ⚠️)_
* [The Assembler Binary (D21)](/spec/assembler-binary.md) - The v1 artifact: one Rust binary, two surfaces — the CI verbs the action runs and the musl-static assembler the kit ships, mirrored like the pixi drivers and installed by nobody. _(⚠️ reasoned)_
## Mechanisms
* [Toolchain Resolution](/spec/toolchain-resolution.md) - How a target platform is resolved and validated: P0 declared / P1 locked / P2 published, omitted lists, and compile-capability checks. _(✅ ⚠️)_
* [The Three Packagers](/spec/packagers.md) - What pixi, pixi-pack and cargo vendor each do and cannot do, and how the tool composes them. _(✅)_
* [Artifact Formats and Integrity](/spec/artifacts.md) - Artifact layout on the dist branch, manifests, channel mirroring, and the reconstruction rung ladder R1-R5. _(✅ ⚠️)_
* [Docs Site Design](/spec/docs-site.md) - The Starlight docs site as a design object: [docs] config, four verbs, the publish-twice rule, and the measured failure modes. _(✅)_
* [Git as the Artifact Registry](/spec/git-registry.md) - Branch layout, publishing without touching the working tree, cheap consumption, and the repo-bloat budget. _(✅)_
* [Bootstrap: Installing the Tool](/spec/bootstrap.md) - How the tool arrives on a machine that can reach nothing but git, including the pixi seed ceremony and digest verification. _(✅ ⚠️)_

## Delivery and trust
* [CI Design](/spec/ci.md) - The workflow set, triggers, pins and permissions that build, verify and publish kits. _(✅)_
* [Error Taxonomy and UX Contract](/spec/error-taxonomy.md) - Exit codes, message voice, and the promise that every refusal names the check that failed. _(⚠️ reasoned)_
* [Testing Without a Compiler](/spec/testing.md) - The strategy that keeps a Rust CLI trustworthy when the authoring box cannot compile it: fixtures, snapshot plans, and CI-only proofs. _(✅ ⚠️)_
* [Testing Strategy (for the Rust assembler + action)](/spec/testing-strategy.md) - The five-layer plan for the D21 binary: what each layer proves, where it runs, the oracle vector table, and which layer this sandbox is locked out of. _(⚠️ reasoned)_
* [Roadmap and Acceptance Gates](/spec/roadmap.md) - Milestones M0-M5 with the gate that closes each one, including the docs and dogfood gates. _(⚠️ reasoned)_
* [Decision Log D1-D21](/spec/decisions.md) - Every design decision, the alternative rejected, why, and its status (decided / open / corrected). _(✅ ⚠️)_
* [Risks and Open Questions](/spec/risks.md) - Named risks with likelihood, blast radius and mitigation, including docs-pipeline churn and toolchain-migration risk. _(✅ ⚠️)_

## Group notes

This directory is one area of the `pixi-sandbox` knowledge bundle; the bundle root index is
[here](/index.md) and the conventions (labels, extensions, maintenance rules) are in
[Bundle Conventions](/conventions.md).
