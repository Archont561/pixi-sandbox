---
type: Measurement
title: "Network Model and Egress Matrix"
description: Reachable hosts, blocked hosts with the failure mode each shows, port rules, and the one-line reproduction script.
resource: https://github.com/Archont561/pixi-sandbox
tags: [environment, egress, measurement]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
verified:
  - { by: process:sandbox-measurement, at: 2026-09-19T21:00:00Z }
  - { by: process:sandbox-measurement, at: 2026-09-19T10:45:00Z }   # §4.4 re-measurement on the current (Codespace-style) host
legacy: { files: [`SANDBOX_CONSTRAINTS.md`], sections: ["4"] }
sources:
  - { id: indexcratesio-configjson, resource: https://index.crates.io/config.json, title: index.crates.io/config.json }
  - { id: pypiorg-simple-, resource: http://pypi.org/simple/, title: pypi.org/simple/ }
  - { id: debdebianorg--inrelease, resource: http://deb.debian.org/.../InRelease, title: deb.debian.org/.../InRelease }
  - { id: githubcom-rust-lang-cratesio-index, resource: https://github.com/rust-lang/crates.io-index, title: rust-lang/crates.io-index }
  - { id: registrynpmjsorg-astrojs2fmermaid, resource: https://registry.npmjs.org/@astrojs%2fmermaid, title: registry.npmjs.org/@astrojs%2fmermaid }
  - { id: registrynpmjsorg-astrojs2fmermaid, resource: https://registry.npmjs.org/@astrojs%2fmermaid, title: registry.npmjs.org/@astrojs%2fmermaid }
---

# Network Model and Egress Matrix

## 4. Network model

### 4.1 Interfaces and addresses you can actually use

```
lo     UNKNOWN  127.0.0.1/8      ::1/128
eth0   UP       169.254.0.21/30  fe80::fc:ff:fe00:5/64

default via 169.254.0.22 dev eth0
169.254.0.20/30 dev eth0 proto kernel scope link src 169.254.0.21
```

| Address | Role | Reachable |
|---|---|---|
| `127.0.0.1` / `::1` | loopback — bind servers here for self-tests | ✅ local only (never from the user's browser) |
| `169.254.0.21/30` on `eth0` | the **only** routable IPv4; a link-local (APIPA-style) address in a /30 | ✅ locally bindable; verified a `0.0.0.0` HTTP server answered `200` on both `127.0.0.1` and `169.254.0.21` |
| `169.254.0.22` | default gateway (the proxy/firewall) | it *is* the egress path |
| `192.0.2.1` (`events.e2b.local`, TEST-NET-1) | E2B control/event sink, `E2B_EVENTS_ADDRESS=http://192.0.2.1` | telemetry only |
| `fe80::…` | IPv6 link-local; **no global IPv6, no `::/0` route** | ❌ outbound IPv6 unusable |
| `10.x`, `172.16.x`, `192.168.x` | — | ❌ **no private LAN, no peer sandboxes, no RFC1918 routing** |
| `0.0.0.0:22` | sshd listening | inbound only inside the sandbox; no external SSH into it |
| `0.0.0.0:49983` | `envd` (E2B agent that serves the exec/filesystem API) | internal control plane |
| `0.0.0.0:111` | `rpcbind` | irrelevant |

> [!IMPORTANT]
> **The user's browser is not this machine.** There is no path from the preview browser to any of the
> above addresses. Any HTTP endpoint you expose must be reached through the platform's reverse proxy:
> `https://{port}-{sandboxId}.e2b.app`, with the server bound to `0.0.0.0`, and the frontend calling
> **relative** URLs (the dev server then proxies to the backend). Hardcoding `localhost:PORT` in
> browser-facing code is guaranteed to fail.

### 4.2 Egress is a TLS-intercepting allowlist proxy

DNS resolves (resolver: `nameserver 8.8.8.8`, and most names return Fastly/Cloudflare addresses,
often AAAA-first), but the **TLS handshake is what is filtered**. Proof of interception:

```
$ openssl s_client -connect github.com:443 -servername github.com
subject=O = E2B, CN = github.com
issuer=O = E2B, CN = E2B Proxy CA
```

A blocked host produces a characteristic **immediate reset**, not a timeout:

```
$ curl https://index.crates.io/config.json
curl: (35) OpenSSL SSL_connect: SSL_ERROR_SYSCALL in connection to index.crates.io:443
   → http=000 tcp=0.000000s
```

### 4.3 Measured allowlist (2026-09-18)

**Reachable**

| Host | Evidence | Usefulness |
|---|---|---|
| `github.com` | `200`; `git clone --depth 1 prefix-dev/pixi` succeeded (73 MB) | source access, push/pull, PR/issue workflows |
| `api.github.com` | `200`; `rate_limit` returned `limit: 5000, used: 0` | full REST via `gh`/`curl` |
| `codeload.github.com` | root `301`; `/tar.gz/refs/heads/main` → `200`, 19.9 MB | tarball fetch without git |
| `registry.npmjs.org` | `200`; `npm view bun version` → `1.4.2`; `npm install ms@2.1.3` → "added 1 package in 464ms" | **Node/npm dependency installs work** |
| `pypi.org` + `files.pythonhosted.org` | `200` / reachable; `pip download requests` succeeded; `pip download pixi` succeeded | Python deps work |
| `github.com/.../releases` (HTML/API metadata) | `302` → `/releases/tag/v0.81.0`; API asset list readable | you can *inspect* releases, not download them |

**Blocked** (TLS reset ⇒ `http=000`)

| Host | Consequence |
|---|---|
| `index.crates.io`, `static.crates.io`, `crates.io` | no `cargo build`/`cargo add`/`cargo install`/`cargo vendor` |
| `prefix.dev`, `conda.anaconda.org`, `repo.anaconda.com`, `micro.mamba.pm`, `docs.anaconda.com`, `anaconda.com` | no conda channel repodata, no `pixi add`, no micromamba bootstrap |
| `pixi.sh` | no pixi docs *and* no `curl pixi.sh/install` |
| `sh.rustup.rs` | no rustup |
| `raw.githubusercontent.com`, `objects.githubusercontent.com`, **`release-assets.githubusercontent.com`** | no release binary download, no raw-file fetch, no pixi badge assets |
| `docs.astro.build`, `starlight.astro.build` | https=000 — Astro/Starlight documentation is unreachable, so version constraints must come from npm metadata and measurement, not from reading the guide |
| `*.github.io` (e.g. `quantco.github.io/pixi-pack`) | https=000 — GitHub **Pages** is unreadable from inside an airlock, so a published docs site cannot be the only copy (`workflows/publishing-docs.md §8.5`) |
| `nodejs.org`, `nodejs.org/dist` | https=000 — no Node provisioning; the box's own `v22.22.3` is what a target is allowed to assume ✅ |
| `ghcr.io` | no OCI channel mirrors (`oci://ghcr.io/channel-mirrors/conda-forge` — the documented air-gap escape hatch — is **also** closed here), no container pulls |
| `gitlab.com`, `bun.sh`, `get.helm.sh`, `download.pyodide.org`, `mirror.spack.io`, `registry.yarnpkg.com`(v6-only route), `example.com:80` | misc. |
| plain **HTTP/80** generally | `http://pypi.org/simple/` → `403` (proxy refuses), `http://deb.debian.org/.../InRelease` → "Connection failed [IP: 151.101.2.132 80]" ⇒ **`apt-get update` cannot work even with sudo**; `apt-cache policy` shows only `/var/lib/dpkg/status` |

#### 4.3.1 Re-measurement (2026-09-19), done specifically for the dogfood loop

| Probe | Result | Why it matters |
|---|---|---|
| `git ls-remote https://github.com/rust-lang/crates.io-index` | ✅ `HEAD = e7f5a6d6…` | the **crates.io index** is fetchable, and `codeload.github.com/rust-lang/crates.io-index/tar.gz/refs/heads/master` → `200` ✅ — metadata *without payload*: the `.crate` blobs still live on `static.crates.io` ❌, so this buys resolution/diffing, not building |
| `git clone --depth 1 --single-branch Quantco/pixi-pack` | ✅ **2.10 MiB of objects in 754 ms**; re-`fetch --depth 1` 997 ms | real git throughput on this box ≈ **3 MB/s**, so a 60 MB kit/vendor snapshot is ~20 s and the "publish artifacts into a branch" transport is not the bottleneck |
| `codeload.github.com/Quantco/pixi-pack/tar.gz/refs/tags/v0.7.11` | ✅ 2 266 228 B in 410 ms (~5.4 MB/s) | a repo snapshot is a legitimate second transport route when you only need *files*, not history |
| `git clone --depth 1 --filter=blob:none smol-rs/async-channel` + `git checkout .` | ✅ blobs materialised on demand | **git transport is not repo-scoped** — any public GitHub repo's content (and any branch) transits this airlock. Which is why a kit must pin `[kit] allowed-git-origins`: reachability ≠ authorization |
| `raw.githubusercontent.com/Archont561/pixi-sandbox/main/README.md` | ❌ `000` (was `403`/`200`-ish before) | treat `raw.` as **unusable**; use `git fetch` or `codeload` |
| `api.github.com/repos/prefix-dev/pixi/releases/latest` → `.assets[].digest` | ✅ e.g. `pixi-aarch64-apple-darwin` → `sha256:f389557f…`, `install.sh` → `sha256:958fba15…` | **the authoritative upstream sha256 of every release asset is readable from inside the airlock**, even though the asset bytes are blocked ⇒ a binary carried in via a branch can be verified *here*, no networked witness needed |
| `api.github.com/repos/rust-lang/rust/releases/latest` | ✅ `1.98.1`, **no assets** | even the metadata route offers no compiler |
| `registry.npmjs.org/rustup` | ✅ `1.0.10`, *"Unofficial wrapper of rustup installer"* | **no Rust toolchain enters via npm**; it is a wrapper around `sh.rustup.rs` ❌ |
| `pypi.org/pypi/rustup/json` | ✅ `1.29.0.1` (empty summary) | same story via pip: wrapper, not toolchain. Conclusion, **corrected**: no route from an *allowlisted registry* installs `rustc` here — while conda-forge's `rust` package **is** `rustc`+`cargo` ✅, and this box simply has no conda route to it (`research/conda-forge-rust.md §19`) |
| `npm install astro@latest @astrojs/starlight@latest` (in a scratch project) | ✅ 227 MB, 33 s → `astro 7.3.3` + `starlight 0.42.2` | **a real JS toolchain is installable here**, so a docs pipeline is fully dogfoodable even though a Rust one isn't |
| `npx astro build` with `HTTPS_PROXY=:9` (no egress) | ✅ exit 0, byte-identical `dist/` (8 pages, 5.7 MB, 4.6 s) | the build is genuinely offline; Pagefind's binary came from npm (`@pagefind/linux-x64`), **not** a blocked release-asset URL |
| `curl https://registry.npmjs.org/@astrojs%2fmermaid` | ❌ `404` | no such package (search-engine result); `astro-mermaid@2.1.0` and `rehype-mermaid@3.0.0` ✅ exist — verify names against the registry, not against snippets |
| inventory of *this* box for build assumptions | ✅ node `v22.22.3`, npm `10.9.8`, GNU tar `1.34` (gzip, **no `zstd`** CLI); `bun` **not preinstalled** (only via `npm i bun` ✅, which is what the earlier row records) | kit/docs artifacts must default to **gzip**, and "we have bun" is never a safe assumption about a target — so a docs pipeline pins the npm toolchain and `bun.lock` support stays a *detected* feature, not a requirement |
| `git push --dry-run` to this repo | ✅ `* [new branch] HEAD -> df-probe` (nothing mutated) | the airlocked box *can* publish over git — the W3 republish loop does not require a networked machine |
| `pypi.org/legacy/` / `test.pypi.org/legacy/` | `404` (reachable) / ❌ `000` | an upload endpoint answers for the real index; whether publishing *from* a sandbox is desirable is a policy question, not a capability one |

> [!IMPORTANT]
> Three consequences for **developing here** (the tiers are in
> [Dogfooding on a Box Like This One §7](/workflows/dogfooding.md#7-dogfooding-developing-pixi-sandbox-on-a-box-like-this-one)):
> 1. **Compile outside, consume inside — for this box.** No route *from an allowlisted registry* delivers
>    `rustc` (or `pixi`) here — `rustc` is only ever a conda package install, and there is no conda route ✅ — so
>    the tool is *built by CI and used here*. That is not a workaround, it is the design's own D11 in the
>    limit case, and it is what makes the loop measurable: push a `wip/*` branch, get a kit back.
> 2. **Everything else about the airlock can be rehearsed for real**: clone-by-branch, `--filter=blob:none`,
>    `sha256sum -c`, `tar -xf`, config overlays, and now *digest verification against upstream's own API* ✅.
> 3. **`raw.` being dead and `codeload`/git being alive** changes the tool's implementation detail: never
>    construct a "download this file from the repo" URL from `raw.githubusercontent.com`; `git cat-file`
>    or a sparse checkout is the only reliable way to read one's own branch here ✅.

**Port rules observed:** TCP `443` to allowlisted hostnames ✅; TCP `22` to `github.com` ✅ **open**
(`bash /dev/tcp/github.com/22` succeeded) — but note our git remote is HTTPS and the sandbox has no
SSH key provisioned for user repos; TCP `80` ❌ for general internet, ✅ only insofar as the proxy
answers itself (403).

### 4.4 Re-measurement (2026-09-19): the authoring host is no longer the E2B airlock

The rows above describe the **E2B box this bundle was authored on** (§4.1–4.3). That box is closed down;
the current working host is a **GitHub Codespace-style box** (`/workspaces/pixi-sandbox`, hostname
`codespaces-…`, OS Debian bookworm, `conda 26.7.2` at `/opt/conda`, node `v24.21.0` via nvm, docker
present). Its egress was re-probed on **2026-09-19**: the allowlist is **wide open** — every host the
E2B box blocked now returns real HTTP answers.

| Host | Today (2026-09-19) | E2B box (2026-09-18) |
|---|---|---|
| `index.crates.io` / `static.crates.io` | ✅ `200`; a real `.crate` (65 KB) downloaded | ❌ TLS reset |
| `crates.io` API | ✅ `200` with a `User-Agent` header (`403` without one — UA is required, not blocked) | ❌ TLS reset |
| `prefix.dev` | ✅ `200` | ❌ TLS reset |
| `conda.anaconda.org` | ✅ `200`; `conda create` installs work end-to-end | ❌ TLS reset |
| `api.anaconda.org` | ✅ `200` (package/subdir queries) | ❌ `000` |
| `pixi.sh` | ✅ `301` → `pixi.prefix.dev` | ❌ TLS reset |
| `sh.rustup.rs` / `static.rust-lang.org` | ✅ `200`; `rustup` script reachable, dist tarball URL `206` | ❌ TLS reset |
| `raw.githubusercontent.com` | ✅ `200` | ❌ `000` |
| `release-assets.githubusercontent.com` | ✅ `200` via 302 from the releases download URL (34 MB pixi tarball pulled) | ❌ `000` |
| `ghcr.io` | ✅ `401` = reachable, auth challenge (no anonymous crate pull) | ❌ `000` |
| `gitlab.com` / `bun.sh` / `nodejs.org` / `*.github.io` | ✅ reachable | ❌ mostly `000` |
| unauthenticated `api.github.com` | ⚠️ `200` until the **unauthenticated rate limit** (60/h from this host IP) — use `gh`/a token for bursts | ✅ full 5000/h |

**Consequences for this corpus:**

1. **The airlock rows above remain the *design target*.** The tool is specified for a machine whose only
   egress is git ([Constraints](/overview/constraints.md#3-constraints-that-shape-the-design)); that
   specification did not come from this host's width, it is the user's target. Nothing in the design
   changed because the *authoring* box got wider.
2. **But "measured here" belongs to the E2B box, dated.** Every "can't install X here" claim in the old
   matrix was true of *that* box; the current host can install and run `pixi`, `rustc`/`cargo`,
   `pixi-pack`/`pixi-unpack` and `convco` from conda-forge — several unproven 🚧/⚠️ rows are now
   **measured** (see [inventory](/environment/inventory.md), [unproven](/workflows/unproven.md), and
   the §16.8 probe in [pixi-pack](/research/pixi-pack.md)).
3. **Tool-flight honesty survives the wider net.** "Verified" still means *measured on a real machine or
   read from upstream source* — a reachable website and a *verified claim* are different things.

---
