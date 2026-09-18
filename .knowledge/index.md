---
okf_version: "0.2"
---

# pixi-sandbox knowledge bundle

Design, workflows, evidence and measurements for **`pixi-sandbox`** - an ergonomic Rust tool that packs
pixi environments, `cargo vendor` trees and optional `node_modules` onto an orphan branch of this repo, so a
machine with nothing but git access can reconstruct a working, task-driven, `pixi run`-able environment.

> [!IMPORTANT]
> **Nothing in this bundle is implemented yet, on purpose.** This sandbox reaches GitHub, npm and PyPI but
> not crates.io, prefix.dev or conda channels, so any code written here could not be compiled or tested.
> Every concept states its own confidence; see [Bundle Conventions](/conventions.md).

# Areas

* [Why this exists](overview/) - The problem, the nine goals, the five requirements and the constraints that decide everything else. Start here. (4 concepts)
* [The design](spec/) - The design itself: modules, verbs, config, resolution rules, artifacts, CI, testing, decisions and risks. (15 concepts)
* [Operational playbooks](workflows/) - What a human or agent actually types on each side of the airlock, plus the honest register of what is still unproven. (9 concepts)
* [Evidence base](research/) - The external evidence every claim leans on: upstream docs and source read first-hand, registry queries, sandbox probes. (16 concepts)
* [The reference sandbox, measured](environment/) - Hardware, toolchain, egress matrix, execution semantics and budgets of the box this bundle was authored on - reproducible. (7 concepts)

# Reading paths

* **Decide whether to build this**: [Problem Statement](/overview/problem.md) → [Goals and Non-goals](/overview/goals.md) →
  [Requirement Traceability](/overview/requirements.md) → [Decision Log D1-D18](/spec/decisions.md).
* **Use it on a sealed machine**: [Creating and Packing an Environment](/workflows/create-environment.md) →
  [Reconstructing on an Airlocked Machine](/workflows/airlock-clone.md) → [What Is Still Unproven](/workflows/unproven.md).
* **Review a claim**: [Bibliography](/research/sources.md) → the area in [Evidence base](/research/) it belongs to;
  for environment facts, [Network Model and Egress Matrix](/environment/network-model.md) has the reproduction script.
* **Publish the docs**: [Docs Site Design](/spec/docs-site.md) and [Publishing the Docs](/workflows/publishing-docs.md).

# Inventory

| Area | Concepts | Character |
|---|---|---|
| [overview/](overview/) | 4 | framing: problem, goals, requirements, constraints |
| [spec/](spec/) | 15 | the design: architecture, verbs, config, resolution, artifacts, CI, testing, roadmap, decisions, risks |
| [workflows/](workflows/) | 9 | operational half, including the unproven-claims register |
| [research/](research/) | 16 | upstream evidence, measurements, corrections, bibliography |
| [environment/](environment/) | 7 | this host: inventory, egress, execution, budgets, safety, reproduction |
| root | 2 | [conventions.md](conventions.md), [log.md](log.md) |

Front matter on every concept records `generated`, `sources`, `status`, a `confidence` extension, and a
`legacy` key naming the pre-OKF file and section each concept was converted from, so older citations still
resolve. `51` concepts, all conformant with OKF v0.2 §11 as checked by the repo validator.
