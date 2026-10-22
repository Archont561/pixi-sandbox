---
id: ADR-31
title: Native format-1 transports replace helper acquisition
date: '2026-10-22'
status: accepted
---

## Decision

TASK-125 supersedes ADR-2/3/4/24's requirement that *new* producers acquire and execute
pixi-pack/pixi-unpack. The directory-only format-1 channel and wheel layout remains; new
transport manifests use schema 3 to make the changed implementation/tool semantics explicit.
Existing manifest fields stay readable rather than silently reinterpreting older provenance.

- Read version-7 Pixi locks, including compact URL-derived Conda identities. Preflight selected
  environments, artifact kind, identity and SHA256 before creating transport output. Copy original
  archive/wheel bytes unchanged; install them using rattler/UV to generate the per-file oracle.
- Default new format-1 pack/restore/unpack never discovers, fetches, or executes pack/unpack helpers.
  Restore verifies transported blobs, installs into destination-filesystem scratch, verifies the
  staged tree, writes final-prefix markers, then promotes with same-parent rollback protection.
- Format-1 payloads under schemas 1/2 remain natively readable. Older opaque, non-format-1
  transports retain their already-recorded unpacker compatibility reader. An explicit raw-unpack
  override also remains available; neither path causes new packers to acquire helpers.
- Remove the helper catalogue, updater, tools verb and helper acquisition/override/cache flags.
  New manifests have no implicit tool entries. `--pixi-bin` and `--self-bin` are explicit local
  executable requests, retaining size/linkage/hash checks and the existing 95 MiB size ceiling.
- Generated publishing and candidate consumer proofs explicitly bundle the caller's Pixi and
  standalone bootstrap. The airlock release adapter capability-probes immutable older releases
  and speaks their old CLI only when needed; that adapter is not a current-product option.
- Keep ADR-28's independent on-demand rattler-build acquisition. Its cache flag is now
  `--build-cache`, with a caller-selected offline `--build-tool`; no environment helper catalogue
  or tools-update API is retained for it.

## Dependencies and approved exceptions

Use rattler 0.54.2 / conda types 0.57.1 / streaming 0.27.15 with defaults/TLS disabled, and
UV's low-level wheel installer 0.0.57 (the same installer generation as the former producer).
This avoids solving/building wheels and external Python execution. Packing retains ureq for
HTTPS; reqwest remains unavoidable transitively and is documented honestly. No host-normal
AWS-LC dependency is introduced. Rust 1.94 is the approved and CI-validated source minimum.

Only the exact YAML *unmaintained* advisories and bzip2/mailparse/quoted_printable license
exceptions approved by the owner are allowed in deny.toml. Vulnerability advisories remain gated.

## Evidence and limitations

Native public-CLI/library fixture coverage uses genuine Conda archives and wheels, empty PATH,
compact locks, empty environments, transport/bare roundtrips, schemas 1/2, markers, corruption,
unsafe members and refused replacement when a hash-valid oracle disagrees. Obsolete helper-CLI
and synthetic producer golden tests are retired, not counted as native compatibility evidence.
Historical baseline was 1,203 passes / 2 skipped; final evidence must report the actual new count.
Full lint, payload roundtrip, built-artifact consumer and airlock results remain completion gates;
this decision does not treat unit success as proof those connected/native-platform gates ran.
