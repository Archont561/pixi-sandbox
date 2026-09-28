# Offline sandbox (orphan branch)

This is the **test fixture** for pixi-sandbox: a structurally complete transport, committed
so the tool's tests run with no pixi, no packer and no network. It is a static payload with
real digests, maintained directly.

Built 2026-09-20T00:00:00Z for platform `linux-64`.

The branch root contains Markdown only. Its verified bootstrap lives exclusively at
`.pixi-sandbox/tools/linux-64/pixi-sandbox`, matching the v0.3 transport layout. Project-side
launchers archive this branch and invoke that nested binary.

| env | platform | packed | files |
| --- | --- | --- | --- |
| `demo` | linux-64 | 87302 B | 6 |

## The prefix archive is real

`.pixi-sandbox/envs/demo/pack/prefix/prefix.tar.gz` is a **real conda prefix**, not a
hand-written stand-in: a `conda-forge` `linux-64` environment, reduced to the files that
matter for prefix relocation (#18) and compressed to keep the fixture small. The placeholder
`@PREFIX@` stands in for the install path that `pixi-pack` records, exactly as a real pack
carries conda's `_h_env_placehold_…` token.

| in the archive | why it is there |
| --- | --- |
| `lib/pkgconfig/{zlib,expat,freetype2}.pc` | pkg-config metadata: the most common baked-in prefix |
| `lib/cmake/libjpeg-turbo/libjpeg-turboTargets-release.cmake` | a CMake package config with absolute `IMPORTED_LOCATION`s |
| `bin/freetype-config` | a script under `bin`, the kind an airlock user runs |
| `x86_64-conda-linux-gnu/lib/ldscripts/elf_i386.xser` | a linker script naming the library search path |
| `lib/-gdb.py`, `lib/gcc/…/plugin/include/configargs.h` | a gdb helper and a generated header |
| `conda-meta/{zlib,expat}-*.json` | real package records, which carry **no** prefix: a false-positive check |
| `etc/conda/activate.d/activate-gxx_linux-64.sh` | a real activation script, which uses `$PREFIX` and is left alone |
| `lib/libz.so.1.3.2` | a real shared library, byte-identical after restore |
| `lib/libz.so` | a relative symlink, which relocation does not follow |
| `bin/lzmainfo` | a real **binary that embeds its build path**: it must keep that path, because rewriting a NUL-containing file would corrupt it |

The fixture's `pixi-unpack` extracts the archive and substitutes the staging prefix for
`@PREFIX@` in text files only — the same rule `restore.rs` applies afterwards. Restore then
relocates the staging prefix to the final one, so the tests assert the whole chain: placeholder
→ staging → final.

### Rebuilding the archive

```bash
# 1. a real environment to harvest (any small one with a native library works)
mkdir -p /tmp/prefix-source && cd /tmp/prefix-source
cat > pixi.toml <<'TOML'
[workspace]
name = "prefix-source"
channels = ["conda-forge"]
platforms = ["linux-64"]
[dependencies]
zlib = "*"
expat = "*"
freetype = "*"
libjpeg-turbo = "*"
xz = "*"
gxx_linux-64 = "*"
TOML
pixi install

# 2. copy the files in the table above, then replace the install path with the placeholder
prefix=$(pwd)/.pixi/envs/default
grep -rlI -- "$prefix" . | xargs sed -i "s|$prefix|@PREFIX@|g"

# 3. the binary's embedded build path, byte-exactly and the same length (bun, or any tool that
#    will not mangle NUL bytes or add a trailing newline)
bun -e 'const fs=require("fs");const p=process.argv[1];const b=fs.readFileSync(p);
const n=Buffer.from(process.argv[2]);const at=b.indexOf(n);
if(at<0) throw new Error("no build path in the binary");
fs.writeFileSync(p,Buffer.concat([b.subarray(0,at),Buffer.from(process.argv[3]),b.subarray(at+n.length)]));' \
  bin/lzmainfo "$prefix" /opt/conda/envs/demo-build-fixture-00000

# 4. deterministic tarball: sorted, zeroed mtime/owner, no gzip header timestamp
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cf prefix.tar prefix
gzip -n -9 -c prefix.tar > prefix.tar.gz
# then record its size and sha256 in .pixi-sandbox/manifest.json
```

Bumping package versions means bumping the versioned `conda-meta/*.json` names in the archive and
in the manifest, and the file count in the table above.

## Restore

```bash
./.pixi-sandbox/tools/linux-64/pixi-sandbox doctor --branch-location . --verify
./.pixi-sandbox/tools/linux-64/pixi-sandbox restore \
  --branch-location . --output-path <project> --force
```
