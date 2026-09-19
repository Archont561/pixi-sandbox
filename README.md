# Compressed Environments, Pixi Binary, and Vendored Dependencies

This branch contains standalone portable environments, the `pixi` executable, and vendored dependencies.
Large archives are split into `<= 45MB` chunks (`.part_*`) to stay within GitHub limits without Git LFS.

### Included Assets:
- `pixi-bin.tar.gz`: Standalone `pixi` CLI binary.
- `cargo-vendor.tar.gz`: Vendored Rust crates (compressed `.cargo/vendor`).
- `env-dev.tar.gz*`: Standalone `dev` Pixi environment (multi-part).
- `env-docs.tar.gz*`: Standalone `docs` Pixi environment (multi-part).
- `env-utils.tar.gz`: Standalone `utils` Pixi environment.

---

### Unpacking & Restoring

```bash
# 1. Reassemble multi-part archives (if split):
for f in *.part_00; do
  [ -e "$f" ] || continue
  base="${f%.part_00}"
  echo "Reassembling $base..."
  cat "${base}.part_"* > "$base"
done

# 2. Extract Pixi CLI binary:
tar -xzf pixi-bin.tar.gz -C /usr/local/bin/

# 3. Extract Cargo vendor directory:
mkdir -p .cargo && tar -xzf cargo-vendor.tar.gz -C .cargo/

# 4. Unpack Pixi environments:
pixi-unpack unpack env-dev.tar.gz --output-dir .pixi/envs/dev
pixi-unpack unpack env-docs.tar.gz --output-dir .pixi/envs/docs
pixi-unpack unpack env-utils.tar.gz --output-dir .pixi/envs/utils
```
