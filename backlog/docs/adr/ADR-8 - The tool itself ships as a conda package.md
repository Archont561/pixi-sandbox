---
id: ADR-8
title: "The tool itself ships as a conda package"
status: accepted
tags:
  - decision
---

# ADR-8 — The tool itself ships as a conda package

**Decision.** `crates/pixi-sandbox/pixi.toml` builds `pixi-sandbox` with the
`pixi-build-rust` backend; releases are cut with `pixi publish --path crates/pixi-sandbox
--target-channel <dir>` and installed with
`pixi global install -c <channel> -c conda-forge pixi-sandbox`, after which the verbs are
available as `pixi sandbox <verb>`.

**Why.** It is the same distribution channel the users already have (pixi), it works on an
airlock (the `.conda` is a file you can carry), and it needs no Rust toolchain on the machine
that installs it.

**Evidence ✅.** Verified end-to-end with pixi 0.81.0: `pixi publish` produced
`pixi-sandbox-0.1.0-ha35fb5c_0.conda` (607,957 B, 34 s) ✅; pointing
`--target-channel file://…` at a directory produced an indexed channel without
`rattler-index`; `pixi global install -c file://… -c conda-forge pixi-sandbox` installed 0.1.0
and `pixi sandbox doctor --verify` then checked 1512 blobs / 249.8 MiB in 1.28 s ✅. Gotchas
recorded in `research/EVIDENCE.md` §10: `[workspace] preview = ["pixi-build"]` is required,
`build.config.extra-args` must stay empty (the backend already passes `--locked`), and a
`file://` channel needs `-c conda-forge` alongside it or the solve fails on `libgcc` ✅.

**What would change it.** Publishing to conda-forge itself (needs a feedstock and a review),
or GitHub-Pages-hosted channels becoming the preferred distribution (that is an **open**
question in `research-results.md`, not a decision yet).
