---
type: Measurement
title: "What a Real Airlock Can Fetch"
description: First-hand egress measurements for the dogfood loop: git throughput, blob:none clones, asset digests, dead hosts.
resource: https://github.com/Archont561/pixi-sandbox
tags: [research, egress, measurement]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`WEB_SEARCH_RESULTS.md`], sections: ["18"] }
stale_after: 2026-12-19T00:00:00Z
sources:
  - { id: githubblog-changelog-2025-06-03-releases-now-expose-, resource: https://github.blog/changelog/2025-06-03-releases-now-expose-digests-for-release-assets/, title: github.blog/changelog/2025-06-03-releases-now-expose-digests }
  - { id: githubcom-github-roadmap, resource: https://github.com/github/roadmap/issues/1136, title: github/roadmap — issues/1136 }
  - { id: docsgithubcom-en-enterprise-cloudlatest, resource: https://docs.github.com/en/enterprise-cloud@latest/rest/releases/assets, title: docs.github.com/en/enterprise-cloud@latest/rest/releases/ass }
  - { id: githubcom-scoopinstaller-scoop, resource: https://github.com/ScoopInstaller/Scoop/issues/6381, title: ScoopInstaller/Scoop — issues/6381 }
  - { id: githubcom-rust-lang-cratesio-index, resource: https://github.com/rust-lang/crates.io-index, title: rust-lang/crates.io-index }
---

# What a Real Airlock Can Fetch

## 18. What a real airlock can and cannot fetch — measurements for the dogfood loop

Prompt for this round: *"how do I dogfood this project, i.e. develop it in an airlocked environment like the
current host machine?"* Since the current host *is* an airlock (measured in
[Host Inventory and Verdict](/environment/inventory.md)), the honest method was to probe it rather than reason about
it. Everything below was run on 2026-09-19 from inside the sandbox.

### 18.1 The mechanism that changes a design decision: release-asset digests are readable offline-ishly

GitHub **computes and publishes a SHA-256 digest for every release asset**, in the UI and the API, immutable at
upload time — [GitHub changelog, 2025-06-03](https://github.blog/changelog/2025-06-03-releases-now-expose-digests-for-release-assets/)
✅, [roadmap #1136](https://github.com/github/roadmap/issues/1136) ✅, and it is a documented response field
`assets[].digest` in the
[REST docs](https://docs.github.com/en/enterprise-cloud@latest/rest/releases/assets) ✅
(`"digest": "sha256:2151b604…"`, third parties already hash-check installers with it —
[Scoop request #6381](https://github.com/ScoopInstaller/Scoop/issues/6381) ✅).

Measured here, where `objects.githubusercontent.com` (the asset *bytes*) is blocked but `api.github.com` is
not:

```
$ gh api repos/prefix-dev/pixi/releases/latest --jq '.assets[] | select(.digest != null) | {name, digest}'
{"name":"install.ps1","digest":"sha256:3c68fe92e4a20b3c0ac3bec398463d6497279b8b10c6d1a3add235f9e746212c"}
{"name":"pixi-aarch64-apple-darwin","digest":"sha256:f389557fc5de929cc33a042f6ae5abbaeb087ffa1affd55aa6ab8da3222d1122"}
---
```

⇒ **Consequence:** a binary that enters an airlock *by any route* (branch blob, USB, a colleague) can be
checked **inside** the airlock against upstream's own published digest. That demotes the old
"manual mirror = no provenance" objection to "manual transport, verified integrity", and it is why
`dist-manifest.json`'s `upstream-sha256` is now *re-fetchable* rather than inherited
(`pixi-sandbox verify-upstream`, `problem.md §10.5`). What it does **not** buy: trust in the publisher, or
coverage of anything published outside GitHub releases — `api.github.com` says nothing about crates.io or
conda-forge, both ❌ here.

### 18.2 Transport measurements (this box → GitHub)

| Route | Result |
|---|---|
| `git clone --depth 1 --single-branch Quantco/pixi-pack` | ✅ 754 ms, `size-pack: 2.10 MiB` ⇒ **~2.8 MB/s** through `https` git |
| `git fetch --depth 1` (second call, same repo) | ✅ 997 ms |
| `codeload.github.com/Quantco/pixi-pack/tar.gz/refs/heads/main` | ✅ 2 266 228 B in 410 ms ⇒ **~5.4 MB/s** |
| `git ls-remote https://github.com/rust-lang/crates.io-index` | ✅ `HEAD = e7f5a6d6…` (and its codeload tarball `200`) — index metadata reachable, **payload not** (`static.crates.io` ❌) |
| `git clone --filter=blob:none` of a **foreign** repo + `git checkout .` | ✅ blobs fetched on demand ⇒ the git door is **not** repo-scoped |
| `raw.githubusercontent.com/<our repo>/main/README.md` | ❌ `000` — raw is unreliable here; read your own branches with git, not with URLs |
| `git push --dry-run` (nothing mutated) | ✅ accepted ⇒ the airlocked box can *publish* over git, so W3's republish loop does not require a networked machine |

### 18.3 The compiler question, settled

| Route tried | Verdict |
|---|---|
| `static.rust-lang.org`, `sh.rustup.rs` | ❌ blocked (reset, `000`) |
| `rust-lang/rust` GitHub releases (metadata) | ✅ reachable, **no assets** ⇒ nothing to mirror even in principle |
| npm `rustup` | `1.0.10` ✅ installable *package*, description: *"Unofficial wrapper of rustup installer"* — it downloads from the blocked host |
| PyPI `rustup` | `1.29.0.1` ✅ — same trick, same blocked dependency |
| `apt` / `deb.debian.org` | ❌ (port 80 closed to the internet, proxy answers `403`) |
| `pypi.org/legacy/` (twine endpoint) | `404` to GET ⇒ host writable-ish ✅ (policy question, not capability) |

⇒ **`rustc` cannot reach this box from any allowlisted *registry*.** (Scope that: conda-forge's
`rust` *is* a toolchain in package form, so a **pack** carries `rustc`+`cargo` to any target that has conda —
see §19, which corrects the over-generalisation this line originally made.) It is
the strongest available argument for `problem.md` D11 (ship the tool as a verified binary in a branch) and for
D17's tier ladder (D3 compilation is delegated to CI by construction, not by preference). Note the asymmetry
it exposes: PyPI/npm **can** deliver arbitrary static binaries (that is exactly how `bun` arrived ✅ §14/C1),
so "the airlock can install a toolchain" is a *packaging* question — a wheel/sdist that embeds `rustc` would
walk straight in, and no such official artifact exists for Rust. That is also why conda-forge is the
toolchain carrier *for users* (they have it) while a git branch is the only carrier *for us*.

### 18.4 Design deltas from this round

| Finding | Change |
|---|---|
| Asset digests verifiable in-airlock ✅ | `problem.md §10.5` gains `verify-upstream`; §8.1/D11's "manual mirror, no provenance" caveat downgraded; `mirror-refresh` CI job now *compares* digests instead of merely listing versions |
| git transport is repo-agnostic ✅ | new `[kit] allowed-git-origins` allowlist + `doctor` refuses an origin not listed — "reachability ≠ authorization" |
| `raw.` dead, `codeload`/git alive ✅ | the tool must never build `raw.githubusercontent.com` URLs; artifact reads go through `git cat-file`/sparse checkout |
| Throughput ~3–5 MB/s ✅ | kit-size guidance re-anchored: fetch time is ~seconds for any kit ≤ 500 MB, so **size pressure is about `pixi install`/disk, not transport** (previously implied otherwise) |
| No compiler obtainable ✅ | G9 added with an explicit D0–D4 tier ladder ([Dogfooding on a Box Like This One §7](/workflows/dogfooding.md#7-dogfooding-developing-pixi-sandbox-on-a-box-like-this-one)); `dogfood` + `airlock-sim` CI jobs added (§14/`problem.md` job table); egress matrix becomes **data** (`fixtures/egress.json`) consumed by `doctor --check-egress` |

---
