# Reproduce transcript

The whole flow, run cold, verbatim (pixi-pack's package-by-package log lines elided — they are
one line per conda package). Recorded on 2026-09-20, linux-64, pixi 0.81.0,
pixi-pack/pixi-unpack 0.7.11, cargo/rustc 1.98.1, network severed for the restore phase with
`unshare -rn`.

Re-run it with:

```bash
bash .knowledge/research/reproduce.sh .sandbox-proof          # or: pixi run -e dev sandbox-proof
```

The run below is the second consecutive run of the same script (the first, cold one, took
62 s; this one 52 s). The **numbers are identical** — fingerprints, packed bytes and vendor
size all match — which is what makes this script usable as a CI gate.

---

```
== 0. preflight
   repo    /home/user/pixi-sandbox
   output  /home/user/.cache/scaffold-proof
   envs    dev,docs (linux-64)
   pixi    pixi 0.81.0
   network: the restore phase runs in a namespace with no interfaces (unshare -rn)

== 1. install the project's environments
   (already installed: .pixi/envs/dev, .pixi/envs/docs — no solver work)

== 2. pack (envs + cargo vendor + pinned tools)

=== 1. pack environments (dev,docs · linux-64) ===
  $ …/pixi-pack /home/user/pixi-sandbox -e dev -p linux-64 -o …/transport/.pixi-sandbox/envs/dev/pack --directory-only
  dev: 40 files, 64.2 MiB packed (unpacked 507.5 MiB)
  $ …/pixi-pack /home/user/pixi-sandbox -e docs -p linux-64 -o …/transport/.pixi-sandbox/envs/docs/pack --directory-only
  docs: 28 files, 62.9 MiB packed (unpacked 501.9 MiB)

=== 2. embed the pinned tools ===
  fetch pixi 0.81.0 (x86_64-unknown-linux-musl)
  fetch pixi-pack 0.7.11 (x86_64-unknown-linux-musl)
  fetch pixi-unpack 0.7.11 (x86_64-unknown-linux-musl)
  pixi 0.81.0 · static · 42.1 MiB
  pixi-pack 0.7.11 · static · 14.0 MiB
  pixi-unpack 0.7.11 · static · 15.0 MiB
  pixi-sandbox 0.1.0-prototype · script · 0.0 MiB

=== 3. vendor the cargo dependencies ===
  33 crates, 31.0 MiB (1441 files, loose tree)

=== 4. record the manifest (shard anything above the limit) ===
  payload 262.0 MiB · 1512 blobs recorded · 1512 files on disk
  manifest …/transport/.pixi-sandbox/manifest.json (schema 1)

== 3. publish to an orphan branch
  pushed 1512 files (262.0 MiB) as one commit — force-push, no history to merge
  airlock: git fetch … remote.git sandbox/dev+docs-linux-64:sandbox/dev+docs-linux-64
   branch extracted: 251 MB on disk

== 4. airlock: a fresh project copy + the branch, then restore

=== 1. verify (nothing is written yet) ===
  1512 blob(s), 249.8 MiB — every sha256 matches the manifest

=== 2. tools ===
  pixi 0.81.0 -> …/.pixi/tools/linux-64/pixi
  pixi-unpack 0.7.11 -> …/.pixi/tools/linux-64/pixi-unpack
  pixi-sandbox 0.1.0-prototype -> …/.pixi/tools/linux-64/pixi-sandbox
  work dir …/.pixi/.restore-work · need ~1044 MiB, free 17903 MiB

=== 3. environments ===
  dev: 40 files materialised into …/.pixi/.restore-work/pack-dev
  dev -> …/.pixi/envs/dev (507.5 MiB)
  docs: 28 files materialised into …/.pixi/.restore-work/pack-docs
  docs -> …/.pixi/envs/docs (501.9 MiB)

=== 4. vendored cargo crates ===
  vendor -> …/.pixi-sandbox/vendor (33 crates, 31.0 MiB)
  wrote …/.cargo/config.toml — directory is relative to the project root

=== restore complete ===
  source …/.pixi/sandbox-env.sh
  pixi install --frozen --offline   # must be a no-op
  cargo build --offline             # must succeed with the vendored crates

  pixi install --frozen --offline
    ✔ The default environment has been installed.        (0.9 s, no network)
  cargo build --offline
    Compiling pixi-sandbox-core v0.1.0
    Compiling pixi-sandbox v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.99 s

== done in 52s
   transport 262 MB → branch 251 MB on disk → restored envs + vendor in …/airlock/project
```

## Numbers from this run

| item | value |
| --- | --- |
| payload | 262.0 MB — envs 133.3 (51 %), tools 96.1 (37 %), vendor 32.5 (12 %) |
| packed per env | dev 64.2 MiB / 40 files, docs 62.9 MiB / 28 files |
| unpacked per env | dev 507.5 MiB, docs 501.9 MiB |
| blobs in the manifest | 1512 (envs + tools + 1441 vendored files) |
| pinned tools fetched | pixi 0.81.0, pixi-pack 0.7.11, pixi-unpack 0.7.11 (all static musl) |
| environment fingerprints | `eaca22bcfed5ef20` (dev), `2c79f41f9afdf07c` (docs) |
| packed bytes, dev / docs | 67 346 020 / 65 995 586 |
| vendor tree | 32 514 233 B, 33 crates, loose |
| verify phase | 1512 blobs, 249.8 MiB, all digests match |
| assertions | `pixi install --frozen --offline` a no-op; `cargo build --offline` in 17.99 s |
| total | 52 s (62 s cold), identical numbers between runs |

## When you re-run it

* **Byte-identical numbers** require the same `pixi.lock`/`Cargo.lock` and the same tool pins;
  a lockfile change shows up as different fingerprints and packed sizes, which is the point.
* **Branch size** can drift by a MiB or so between git versions (delta encoding, compression
  level); the payload sizes above are the stable signal.
* **Timings** depend on the machine; the assertion timings (0.9 s / 17.99 s) are the useful
  ones — if `pixi install --frozen --offline` starts doing real work, a marker is missing
  (D5), and if `cargo build --offline` reaches for the network, the vendor wiring is wrong
  (D6).
* The restore phase is only meaningful under `unshare -rn` (or a genuinely unplugged machine);
  the script says which one it used in the preflight block.
