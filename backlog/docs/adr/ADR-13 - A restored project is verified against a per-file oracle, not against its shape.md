---
id: ADR-13
title: "A restored project is verified against a per-file oracle, not against its shape"
status: accepted
tags:
  - decision
---

# ADR-13 — A restored project is verified against a per-file oracle, not against its shape

**Decision.** Schema 2 manifests carry, per environment, a `files.json` oracle: the file list
of the *unpacked* prefix (relative path, sha256 of canonicalised content, exec bit, symlink
target; conda-meta records presence-only), recorded by `pack` from its own verification
unpack. `doctor --verify-restored <PROJECT>` compares a restored project against that list —
every entry, every extra file, the exec bits, the fingerprint marker — collects **all**
mismatches, and writes nothing. The airlock workflow's archived `ci`-feature e2e gate calls it
against the restored tree, and `restore.sh` calls it before cleaning up the worktree. A schema-1
env is reported `unverifiable`, never failed: published branches outlive the tool that packed
them.

**Why.** Everything the pipeline verified before task-10 was about the *branch*: blobs matched
the manifest, and the tree restore produced was checked only by shape — conda-meta present,
`pixi install` a no-op. A stub prefix with one hand-forged `conda-meta/*.json` record passed
all of it (measured against a real transport while designing this). Self-sufficiency and
integrity are different claims with different owners: the gate proves the sandbox needs no
network; only a per-file comparison of the restored tree against what the packer actually
unpacked can prove the tree is the one the manifest describes. The oracle closes exactly that
gap, and it travels in the branch — verified like every other blob — so the checker and the
digests it checks cannot disagree about what "correct" is.

**The canonicalisation rule.** Relocation rewrites the staging prefix into text files, but
NUL-padded binaries keep the staging path forever, and conda-meta records embed digests of
files relocation changed. So a digest is taken over a canonical form: collapse NUL runs, then
replace *this side's* path spellings with a sentinel. Pack's candidates are its scratch paths;
verify's candidates are the final prefix and the restore work dir — derived from the project
path, env name and work dir alone, never from files restore wrote (a candidate list taken from
restore output would let a malicious restore name its own scratch as "correct"). With that,
the same digest is stable across relocation without weakening the check: a prefix still
pointing into restore scratch fails it.

**Honest limits.** conda-meta records are presence-only (their bodies embed
`sha256_in_prefix` of relocated files — recording their content would fail every honest
restore); `conda-meta/pixi`, `conda-meta/history` and the two restore markers are exempt as
pixi/restore bookkeeping; and a malicious writer who rewrites payload *and* oracle and
manifest consistently is still outside the model (no signatures yet, §7). The oracle costs one
verification unpack per env at pack time (≈8.5 s and ~2 GiB of scratch, measured on the real
transport) and a 1895-byte fixture blob.
