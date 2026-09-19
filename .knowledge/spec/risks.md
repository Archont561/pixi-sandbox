---
type: Risk Register
title: "Risks and Open Questions"
description: Named risks with likelihood, blast radius and mitigation, including docs-pipeline churn and toolchain-migration risk.
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, risks]
status: stable
confidence: mixed
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`DESIGN.md`], sections: ["17"] }
sources:
  - { id: pixiprefixdev-latest-reference, resource: https://pixi.prefix.dev/latest/reference/pixi_configuration/, title: pixi.prefix.dev/latest/reference/pixi_configuration/ }
---

# Risks and Open Questions

## 17. Risks and open questions

| Risk | Severity | Mitigation |
|---|---|---|
| **Repo bloat**: every pack is 15–500 MB, and git blobs are forever | 🔴 high | orphan branch + `keep-last` + `bundle` for bulk transfer + **documented "dist branch is disposable: `git push --force` is the gc"** (that force-push is why D1 keeps dist off `main` protection) |
| pixi-pack CLI drift (its flags already changed once: `pack --manifest-file` → positional) | 🟠 | `pin = "0.7.11"` in config + capability probe (§5.2.5) + CI contract job (§14.5) |
| Conda channel lag / missing subdirs (`bun` on win-64) | 🟠 | §7 fallback chain, `omitted` records (§9.1), `explain unavailable-platform`, and the P0/P1/P2 layering + `[platforms].known-gaps` cache in [§7.4](/spec/toolchain-resolution.md#74-platform-validation-does-this-platform-actually-exist) |
| **R1 (copied `$PIXI_CACHE_DIR`) is undocumented territory** — the pieces are documented, the combination is not | 🟠 (was 🔴) | **the risk shrank when R2 turned out to be documented** ✅: if R1's probe fails we lose a size optimisation, not the capability. R1 is tried first only because it avoids touching the manifest; `reconstruct` prints the rung and `--mode` pins it |
| **whether pixi will `--frozen`-install from a *pack-produced* `channel/`** (file:// channels ✅, the pack's layout ✅, the composition ⚠️) | 🔴 | this is the one probe that gates the guarantee; M4.5 runs it in a container with `git`+`tar` only. If it fails: R3 becomes the promise, offline `pixi add` is documented as unsupported, and `pixi add <abs path>.conda` ✅ is the only manifest-editing escape |
| **`--offline` is not a complete barrier** ✅: PyPI solving is delegated to `uv` ("a solve that succeeds may still require network access for PyPI" ✅) and build backends for source deps run with their own clients, **offline not enforced** ✅ | 🟠 | kits for airlocked targets set `[kit] targets-compile = false`, prefer conda packages over `[pypi-dependencies]` in packed envs, and `doctor --strict` warns when a *packed* env has pypi deps at all |
| **post-link scripts do not run** ✅ (`run-post-link-scripts` defaults to `false`, "we deem these scripts insecure", sandbox mode only "planned") | 🟠 | a package that needs post-link work installs half-complete and looks like a missing file, not an error → `reconstruct` runs a `__selftest__` task after every rung before it reports success; `run-post-link-scripts = "insecure"` is the documented opt-in, and the tool prints that exact line rather than setting it silently |
| **cache/auto-redirect surprises on shared or netfs homes** ✅ (`[cache]` needs absolute paths, `~` expands, **`$VAR` is not substituted** ✅; `netfs-redirect = "auto"` may move kinds to `$SLURM_TMPDIR`/`$PBS_JOBFS`/`$SCRATCH`/`$TMPDIR` ✅) | 🟡 | generated `.pixi/config.toml` always writes an absolute `conda-packages` path and `netfs-redirect = "never"`, because a cache that silently moves is a reconstruction that fails only on some nodes; `PIXI_CACHE_CONDA_PACKAGES_DIR` ✅ is the env-var form the bootstrap script uses instead of editing config |
| **Mirroring upstream binaries into the repo** (licence, attribution, blob permanence, "who is accountable for this `pixi`?") | 🟠 | `NOTICE.md` + `[selfhost].notice` gate + `mirror`-vs-`built` provenance ([§10.5](/spec/git-registry.md#105-self-hosting-the-branch-ships-pixi-and-the-tool)); keep a separate `pixi-sandbox-mirror` orphan branch so the mirror can be force-pushed/discarded independently of the kits |
| **Detection heuristics wrong on exotic manifests** (`[tool.pixi.environments]`, inline `[environments.<e>.dependencies]` ✅, `solve-group` coupling) | 🟠 | every fact carries its tier; L3 (ask pixi) overrides L2 (scan); `plan --strict` fails on heuristic-sourced decisions; the `detect-golden` CI job keeps foreign fixtures honest |
| **docs pipeline drift: Astro 7 deprecations silently produce a one-page site** ✅ measured (empty collection + exit 0) | 🟡 | `docs check` asserts page count ≥ `ls *.md \| wc -l` and runs the site's own link validator, which fails the build; if the JS toolchain churns more than it earns, `enabled = false` costs nothing — the mirrored markdown in the kit is the durable artifact, the site is a render |
| **a `rust`-bearing pack is far bigger than a "just run it" kit, and users will ask for it** | 🟡 | `plan`/`kit build` print the toolchain's byte share separately; `targets-compile = "auto"` stays false unless a task really invokes cargo; `rust-std-<triple>` documented as the opt-in for cross-targets ✅ |
| **the dogfood loop stalls because D2's seed is manual** (a human must carry `pixi` in once) | 🟡 | one-time, and *one-way*: the branch keeps it forever, `verify-upstream` closes the trust gap ✅, `mirror-refresh` detects staleness; document the seed as an install step, not a ritual. If it is judged unacceptable, the alternative is "no pixi on the target, ever" ⇒ R5-only kits, which contradicts G8 |
| **CI's airlock simulation may not be expressible on hosted runners** 🚧 (egress allowlists need a privileged container or a self-hosted runner) | 🟠 | fallback is what D1 already is: this sandbox as a *real* airlock, run by hand on a schedule, with results recorded in the PR body; `doctor --check-egress` is the automated half of that and works everywhere |
| **`cache/pkgs/` dwarfs the packs** (a copied rattler cache can be GBs) | 🟡 | `[reconstruct] cache-subset = "pack"` ships only packages named by the pack's own repodata; `"none"` forces R3. `plan` prints the byte estimate for each subset so it's a visible trade, and the dist branch's force-push policy bounds git cost |
| pixi `--offline` **not** enforced for build backends, and uv's PyPI solve can still reach out ✅ [docs](https://pixi.prefix.dev/latest/reference/pixi_configuration/) | 🟠 | the tool never claims "fully offline" from `--offline` alone; `kit verify` asserts *its own* files, and §11's default path makes no claim about source builds |
| Partial clone / `--filter=blob:none` unsupported on a private git host | 🟡 | `dist pull --full` fallback (measured: plain `--depth 1` of a 73 MB repo took ~3.5 s here) |
| GNU-tar-only flags (`--sort`, `--mtime`) break on bsdtar/Windows | 🟡 | detect `tar --version`, degrade determinism with a recorded `note`, never fail the pack for it |
| ~~Windows: symlinks/`chmod`/path length in `node_modules`~~ | ✅ closed | **moot since D20** — no `node_modules` payload exists in v1, so JS-tree unpack semantics on Windows are nobody's problem until `[node] pack` returns |
| A CI-side packager pushes **huge tarballs to a branch** (227 MB of `node_modules` would have been a typical payload ✅ measured) | 🟠 | `keep-last = 3` + a force-pushed orphan tip prune *references*, but **git blobs are forever** ⚠️; per-artifact `git-oid` fetch keeps the size off a consumer's clone ✅; documented escapes: LFS opt-in, `git bundle` for the air-gap hop — [Action Shape](/spec/action-shape.md) |
| **"tar of `.pixi/envs`" instead of a `pixi-pack`** ⇒ prefixes with the build box's absolute paths baked in | 🟠 | ship `pixi-pack` output (channel-shaped ⇒ R2/R3 ✅); a raw env tree may travel only as an *extra* artifact and `assemble.sh` must refuse to relocate it ⚠️ — [Action Shape](/spec/action-shape.md) |
| CI minutes (private repo: 2 000/mo) with a 7-job matrix | 🟡 | `paths:` filters, `cache-write` on main only, `pack` nightly not per-PR |
| **The design's premises are all measured in *this* sandbox** — a different host changes them | 🔴 (process) | `doctor` must be run at install time, not assumed; §14.5 keeps the premises under test |

**🚧 For you to decide**

Requirements (a)–(e) are now in the design (detection [§4.4](/spec/architecture.md#44-discovery-the-inventory), component
derivation [§8](/spec/packagers.md#8-the-three-packagers), platform validation
[§7.4](/spec/toolchain-resolution.md#74-platform-validation-does-this-platform-actually-exist), self-hosted pixi
[§10.5](/spec/git-registry.md#105-self-hosting-the-branch-ships-pixi-and-the-tool), reconstruction ladder
[§9.4](/spec/artifacts.md#94-reconstruction-making-pixi-run-work-offline)), which retired my old "which environments?"
question. What's left genuinely needs your input:

1. **How much pixi must survive on the target?** *"I just need to run the tasks"* ⇒ R3 is enough and the kit
   stays small. *"I need to `pixi add`/edit/re-solve offline"* ⇒ R1/R2, and R2 is now **documented** (a
   `file://` channel whose packages `--offline` counts as available ✅), so the choice is really *"do we ship
   `channel/` (≈ the pack, already there) plus a 2-line manifest edit, or not"*. I defaulted to **"R1
   attempted, R2 as the advertised guarantee, R3 as the floor"** — [Reconstructing on an Airlocked Machine §2.2](/workflows/airlock-clone.md#22-reconstruct-the-ladder-now-with-the-rungs-the-docs-actually-support)
   has the table this decision came out of.
2. Do you accept **mirroring upstream `pixi`/`pixi-pack` binaries into the repo's dist branch** (with
   `NOTICE.md` + digests), or must the target build them from source? Mirroring is what makes requirement
   (e) work on a machine with *nothing*; source-building is what a licence review might demand. ⚠️ Their
   current licence terms are not yet verified by me.
3. Is **`win-64`** a target for the *tool binary* (musl/static-glibc covers Linux cleanly; `bun` has no
   win-64 on conda-forge ✅, so win-64 kits are degraded by construction)?
4. **`vendor` default**: commit-into-`main` (simplest here — ours is 4 crates) or the `pixi-sandbox-vendor`
   branch? I defaulted to branch; "commit" makes M3 simpler.
5. Should `kit` include a **Dockerfile/OCI** recipe (pixi `oci://` mirrors ✅) or stay tar-only? Relevant to
   R2 — an OCI mirror is the one *verified* non-git channel format.
6. **Licence** for a tool that will be `cargo install`ed and vendored into other repos: `MIT OR
   Apache-2.0` (assumed in the tree sketch) — needs your call, and it interacts with #2.
7. Optional but my recommendation: let `pixi sandbox init` **emit** the `[tool.pixi]` blocks your repo
   lacks (`scaffold`). I left it out of [§5](/spec/cli.md#5-command-surface) as scope creep, but "clone a branch and
   get a working workspace" is 3 `pixi add` lines away.

---

### Appendix — the whole bootstrap this design must survive

```bash
git clone --depth 1 --branch pixi-sandbox-dist <url> && cd pixi-sandbox-dist   # kits, pixi-sandbox AND pixi
sha256sum -c SHA256SUMS --ignore-missing
install -m755 bin/pixi-$(uname -m)-unknown-linux-musl ~/.pixi/bin/pixi
install -m755 bin/pixi-sandbox-$(uname -m)-unknown-linux-musl ~/.pixi/bin/pixi-sandbox  # not a glob: bin/pixi-* would match both
pixi sandbox reconstruct --from . --mode auto && pixi run test     # --from = the kit dir, not a manifest
# → .pixi/envs/* exist, `pixi run test` executes a real task, and nothing but git was contacted
```

No compiler, no `curl | sh`, no root, no release-asset host, **no pre-existing pixi**, and the last line is
the tool verifying the assumptions it just depended on ([§11.1](/spec/bootstrap.md#111-the-bootstrap-revised-for-pixi-must-actually-run)).
If that block ever stops working, the design has a bug — not the network.
