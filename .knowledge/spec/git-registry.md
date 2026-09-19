---
type: Design Spec
title: "Git as the Artifact Registry"
description: Branch layout, publishing without touching the working tree, cheap consumption, and the repo-bloat budget.
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, git, transport]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T23:40:00Z }
legacy: { files: [`DESIGN.md`], sections: ["10"] }
sources:
  - { id: pixish-install, resource: https://pixi.sh/install, title: pixi.sh/install }
---

# Git as the Artifact Registry

## 10. Git as the artifact registry

### 10.1 Layout

* **working branch (`main`)**: source + the two JSON manifests + `sandbox/` in `.gitignore`.
* **`pixi-sandbox-dist` (orphan, no history link)**: `dist-manifest.json` + blobs. Rebuilt
  *append-only* by `dist push` (see §10.4 for retention).
* **`pixi-sandbox-vendor` (optional orphan)**: the full `vendor/` tree, when `strategy = "branch"`, so
  `cargo build` can be made offline by a 1-line fetch instead of a 40 MB commit on `main`.
* **`pixi-sandbox-docs` (optional orphan)**: the built docs site (`dist/` of the Starlight project, 5.7 MB for
  this repo's 5 files ✅) — because a Pages URL is unreachable from an airlock (`*.github.io` ❌ measured ✅),
  so `docs publish` is what makes the documentation exist *on the target*. Same append-only, force-pushable,
  never-tagged discipline as the vendor branch, and `reconstruct --with-docs` unpacks it to `.pixi/docs/`.
* **tags `dist/0.1.0`** on the dist commit → immutable, human-namable, and `git clone --branch dist/0.1.0`
  works (tags are refs; this is why we don't need release assets).

### 10.2 Publishing without touching the working tree

`dist push` must not `checkout` anything (your repo is dirty; you have uncommitted work; agents hate
surprise checkouts). So use plumbing only:

```bash
git --work-tree=<staging> --git-dir=.git hash-object -w --path=<rel> <file>   # -> blob oid
GIT_INDEX_FILE=<tmp-idx> git update-index --add --cacheinfo 100755,<oid>,<rel>
git write-tree                                                                # -> tree oid
git commit-tree <tree> -p <dist-tip-oid> -m "dist: …"                         # -> commit oid
git update-ref refs/heads/<dist-branch> <commit>
git push <remote> <dist-branch>
```

The staging tree is `sandbox/dist-staging/`, generated from `sandbox.lock.json`; `--cacheinfo 100755`
preserves the executable bit for `bin/*` (this is why the binary works after a plain `git clone` with no
build step). ⚠️ `git worktree add --orphan` is the modern alternative but `--work-tree`+temp index is
portable to older git (our git is **2.39.5**, measured).

### 10.3 Consuming cheaply

```bash
git clone --depth 1 --branch <dist-branch> --single-branch --filter=blob:none <url> <tmp>
# manifest only, no blobs:
git cat-file blob HEAD:dist-manifest.json
# then fetch exactly the one artifact you need:
git fetch --depth 1 origin <git-oid-of-artifact> && git checkout FETCH_HEAD -- <path>
```

`--filter=blob:none` + partial clone is the trick that keeps "download one 8 MB binary" from becoming
"clone 400 MB of history". ⚠️ Requires server-side `uploadfilter`/partial clone support — GitHub supports
it; a self-hosted Forgejo/Gitea may not, so `dist pull` must fall back to a plain `--depth 1` clone. Also
offer `codeload.github.com/<org>/<repo>/tar.gz/refs/heads/<branch>` (measured `200`, ~20 MB in seconds) as
the no-`.git` path for the *source* half of a kit.

### 10.4 Repo bloat is the real cost — so budget it explicitly

| Policy | Default | Why |
|---|---|---|
| `keep-last = 3` per artifact name on the dist branch | ✅ | bounds dist history; `dist push` rewrites the dist branch as a *fresh* commit whose parent is the previous tip, pruning superseded blobs, then `git gc` is left to the host |
| `.gitattributes`: `* binary -merge -text`, `*.tar binary` | ✅ | prevents git trying to diff blobs (measured: our repo patchset cap is ~128 MB) |
| Never write artifacts under a tracked path | ✅ enforced by `pack`/`vendor` | `sandbox/` is gitignored; if you want a *committed* kit (tiny repos), `dist push --from-commit` is the explicit opt-in |
| LFS opt-in (`transport.lfs = true`) | ❌ off | for >50 MB artifacts, where a blob is a permanent liability; documented as "switch when your dist branch exceeds ~200 MB" |
| `git bundle` export | optional | a single file that crosses a truly air-gapped hop; `git bundle verify` is the integrity check, `git fetch <bundle> <branch>` restores it |

### 10.5 Self-hosting: the branch ships pixi and the tool

The premise of this whole project is that **git is the only reliable channel**. That cuts both ways: it
also means a fresh sandbox cannot `curl -fsSL https://pixi.sh/install | bash` (blocked) ✅. So the dist
branch must carry *the control plane*, not just your payload — otherwise the kit is a pile of prefixes
nothing can manage.

```
pixi-sandbox-dist (orphan branch, one commit per release)
├── dist-manifest.json
├── bin/pixi-sandbox-<triple>          role: "tool"       provenance: "built"
├── bin/pixi-<triple>                  role: "driver"     provenance: "mirror" + upstream URL + sha256
├── bin/pixi-pack-<triple>             role: "driver"     provenance: "mirror"
├── conda/pixi-sandbox-0.1.0-<plat>.conda   role: "tool"  → `pixi add <abs path>` offline ✅ (§9.3)
├── conda/pixi-0.81.0-<plat>.conda          role: "driver" (when available as a conda pkg ✅)
└── NOTICE.md                          attribution for every mirrored upstream binary
```

* **`pixi sandbox dist push --with-self --with-pixi`** is what makes the branch self-sufficient; `mirror
  binary --path $(command -v pixi) --as pixi --version 0.81.0` ingests a binary you already have (measured
  here: `pixi 0.81.0` ✅ is the current release, so a networked dev box can prime the mirror in one command).
* **Two provenance classes, always distinguished:** `mirror` = byte-identical copy of an upstream release
  asset, with `upstream:` URL + sha256; `built` = produced by our CI from a recorded git `rev`. Never blur
  them: a reviewer deciding whether to trust a binary cares exactly about this. `dist verify` fails on a
  `mirror` entry whose sha256 doesn't match the recorded upstream digest.
* **Mirrored bytes can be re-verified *inside* the airlock** ✅ — GitHub's API publishes an authoritative
  digest per release asset (`assets[].digest`, e.g. `pixi@v0.81.0` → `sha256:3c68fe92…` ✅ measured from this
  blocked sandbox, where `api.github.com` answers even though `objects.githubusercontent.com` refuses). So
  `pixi-sandbox verify-upstream <file> --asset prefix-dev/pixi@v0.81.0/pixi-x86_64-unknown-linux-musl`
  fetches the digest itself and compares; `upstream-sha256` in `sandbox.lock.json` stops being a claim
  inherited from whoever made the branch, and the seed step in [Dogfooding on a Box Like This One §7.4](/workflows/dogfooding.md#74-d0d1-today-the-parts-that-need-no-compiler)
  is "copy, then verify" rather than "copy, then trust". ⚠️ This verifies *upstream published these bytes*,
  not *upstream is trustworthy* — those are different claims and the tool should never conflate them.
* **Licensing is a shipping decision, not a legal afterthought.** Mirroring upstream *binaries* into a
  repo is normal air-gap practice, but the tool emits `NOTICE.md` (upstream project, licence, version,
  digest, source URL) and refuses `--with-pixi` unless `[selfhost].notice = true` is set — a cheap nudge
  so the decision is made deliberately. ⚠️ Verify pixi's and pixi-pack's current licence terms before the
  first mirror.
* **`pixi exec --spec pixi-pack -- pixi-pack …`** is used *only* where pixi already exists ✅; on a bare
  target the `bin/` entries are what make the same verbs work, which is the point of shipping them.
* **The closure property to test in CI (M7):** the tool must be able to install *itself* from a `.conda`
  file it produced, into a workspace whose env it produced, on a machine with only `git` + `tar`. That is
  the whole design in one assertion.

### 10.6 Branch sharding: one branch until it hurts

A single `pixi-sandbox-dist` branch holds the **kit** — the small, always-needed index: the reconstructor
(`assemble.sh` under D19, `bin/pixi-sandbox-<triple>` under D21), `README.md` + `AGENTS.md`, `SHA256SUMS`,
`dist-manifest.json` + `manifest.tsv`, `workspace/{pixi.toml,pixi.lock}` and the `bin/` drivers. Payloads live
on sibling branches of the same remote: `<dist>-envs`, `<dist>-vendor` — and a class gets a numbered shard
(`<dist>-envs-2`, `-3`, …) only when its byte budget is exceeded. Greedy first-fit assignment in listing order
keeps the plan readable; nothing about consumption changes, because the consumer reads the manifest, not the
branch names.

Two budgets, both derived from measured GitHub behaviour:

* **Blob budget — 99 MB** (default `max-blob-bytes`). GitHub refuses files over **100 MB** and warns over
  50 MB ✅ (the `GH006` class of push failure), so any payload above the budget is split with `split -b` into
  `<name>.part-aa…` pieces at publish time; the reconstructor concatenates the parts in scratch space and
  verifies the **reassembled** digest from the manifest before using it. No LFS: LFS needs an extra server
  conversation an airlock cannot have, and splitting keeps every byte a plain blob.
* **Branch budget — 1 GB** (default `max-branch-bytes`). Blobs are forever (§10.4), so each branch stays lean
  enough that keep-last rotation and a shallow clone remain cheap.

`manifest.tsv` is the index that makes this one command on the far side — tab-separated, parseable by POSIX
`cut`/`awk` and trivially by Rust:

```
# component  env  platform  branch                    path                          sha256  bytes  parts
env          demo linux-64  pixi-sandbox-dist-envs    packs/ws-demo-linux-64.tar    <sha>   1234   1
vendor       -    -         pixi-sandbox-dist-vendor  vendor/vendor.tar.gz          <sha>   9999   3
```

Consumption is **lazy**: `git clone --depth 1 --branch <dist>` fetches only the kit branch (small by
construction); the reconstructor then shallow-clones only the payload branches that contain the requested
`--env`s (and the vendor branch only with `--with-vendor`), each into its own scratch dir, verifying each
branch's `SHA256SUMS` on arrival and the reassembled digest before unpacking. A kit whose manifest lists six
branches therefore still costs one clone for a single-env target. Both halves of this design were prototyped
against a local bare remote in the authoring sandbox (publish → clone → lazy-fetch → digest-verify) ✅ as shell;
under [D21](/spec/decisions.md) the same layout is read by the assembler binary instead.
