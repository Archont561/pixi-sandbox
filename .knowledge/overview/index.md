# Why this exists

* [The one idea: a git ref is an artifact registry](/overview/problem.md) - The problem statement and the single mechanism that solves it: pin portable artifacts on orphan branches of this repo so a sealed box can reconstruct everything it needs. _(⚠️ reasoned)_
* [Goals G1-G9 and what this project refuses to do](/overview/goals.md) - Nine goals (G1 zero-config packing, G2 detect-first, G3 lockfile-derived bundles, G4 platform existence checks, G9 dogfooding) plus the explicitly rejected approaches. _(⚠️ reasoned)_
* [Where each of the five stated requirements landed](/overview/requirements.md) - Traceability table mapping the five requirements (a)-(e) to the mechanisms that answer them, and an honest note on which answer is still a probe. _(✅ ⚠️)_
* [The sandbox facts that shape every design choice](/overview/constraints.md) - The measured environment constraints (no crates.io, no prefix.dev, no release assets, git only) restated as design forcing functions, with links to the measurements. _(✅)_

## Group notes

This directory is one area of the `pixi-sandbox` knowledge bundle; the bundle root index is
[here](/index.md) and the conventions (labels, extensions, maintenance rules) are in
[Bundle Conventions](/conventions.md).
