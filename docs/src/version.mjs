// The version these docs describe. Two sources, in order:
//
// 1. SANDBOX_VERSION from the environment — an explicit caller override
//    (`cargo run -p xtask -- version` prints the same value). The version of
//    what you are reading about is decided by the tree you built from, never
//    restated in the site.
// 2. The root Cargo.toml — the same single source of truth, read directly. This
//    is the ordinary path for `pixi run docs build` and a bare `bun run build`
//    in docs/ alike: the `web` environment carries no cargo, so the resolver
//    reads the manifest itself instead of leaning on a wrapper script.
//
// If neither resolves, the caller throws. A docs site that quietly documents
// `v__VERSION__` is worse than one that refuses to build.
//
// Walking up from cwd rather than `new URL("../Cargo.toml", import.meta.url)`:
// Astro loads this config through a bundler, at which point import.meta.url can
// point at a compiled copy and the relative path resolves to a Cargo.toml that
// does not exist. The working directory survives that trip, whether the caller
// is a pixi task (cwd = docs/), CI, or a build from the repository root. The
// `[workspace.package]` test makes "nearest ancestor" safe: a vendored crate's
// Cargo.toml carries no such table and is skipped.
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

export function resolveVersion(startDir = process.cwd()) {
  let dir = resolve(startDir);
  for (;;) {
    const candidate = join(dir, "Cargo.toml");
    if (existsSync(candidate)) {
      const manifest = readFileSync(candidate, "utf8");
      if (/^\[workspace\.package\]$/m.test(manifest)) {
        // Scoped to the table, not the first `version` key in the file: the
        // workspace dependencies and member manifests carry their own.
        const table = manifest.match(/^\[workspace\.package\]$([\s\S]*?)^\[/m)?.[1];
        const version = table?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
        if (version) return version;
        throw new Error(`no [workspace.package] version in ${candidate}`);
      }
    }
    const parent = dirname(dir);
    if (parent === dir) {
      throw new Error(
        "pixi-sandbox version unresolved: SANDBOX_VERSION is unset and no Cargo.toml " +
          "with a [workspace.package] table exists above the working directory. " +
          "Run the docs through `pixi run docs build`.",
      );
    }
    dir = parent;
  }
}
