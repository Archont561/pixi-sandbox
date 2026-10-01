import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { satteri } from "@astrojs/markdown-satteri";
import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";
import icon from "astro-icon";

// ---------------------------------------------------------------------------
// The version these docs describe. Two sources, in order:
//
// 1. SANDBOX_VERSION from the environment — an explicit caller override
//    (`cargo run -p xtask -- version` prints the same value). The version of
//    what you are reading about is decided by the tree you built from, never
//    restated in the site.
// 2. The root Cargo.toml — the same single source of truth, read directly. This
//    is the ordinary path for `pixi run docs-build` and a bare `bun run build`
//    in docs/ alike: the `web` environment carries no cargo, so the config
//    reads the manifest itself instead of leaning on a wrapper script.
//
// If neither resolves, the build throws. A docs site that quietly documents
// `v__VERSION__` is worse than one that refuses to build.
//
// Walking up from cwd rather than `new URL("../Cargo.toml", import.meta.url)`:
// Astro loads this config through a bundler, at which point import.meta.url can
// point at a compiled copy and the relative path resolves to a Cargo.toml that
// does not exist. The working directory survives that trip, whether the caller
// is a pixi task (cwd = docs/), CI, or a build from the repository root. The
// `[workspace.package]` test makes "nearest ancestor" safe: a vendored crate's
// Cargo.toml carries no such table and is skipped.
// ---------------------------------------------------------------------------
function workspaceVersion(startDir = process.cwd()) {
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
          "Run the docs through `pixi run docs-build`.",
      );
    }
    dir = parent;
  }
}

const version = process.env.SANDBOX_VERSION?.trim() || workspaceVersion();

// ---------------------------------------------------------------------------
// Replace `__VERSION__` with the derived version, everywhere in the content —
// fenced code blocks, inline code, prose, link targets. The docs never carry a
// literal release tag of ours (release-refs.sh scan enforces it), so a release
// commit rewrites zero documentation lines and a new page cannot ship a stale
// pin because it never contains one. Same placeholder convention as
// templates/install.sh, which is rendered the same way at release time.
//
// A Sätteri mdast plugin, not `markdown.remarkPlugins`: Astro 7's default
// Markdown processor is Sätteri, and the remark options are a legacy path that
// demands installing `@astrojs/markdown-remark` and swapping the whole pipeline
// back to unified — a heavy trade for a string replace. Sätteri visitors get a
// readonly node and adopt a returned replacement, hence the spread-copies.
// ---------------------------------------------------------------------------
const TOKEN = "__VERSION__";
const sub = (s) => s.replaceAll(TOKEN, version);
const subValue = (node) =>
  node.value.includes(TOKEN) ? { ...node, value: sub(node.value) } : undefined;
const subUrl = (node) => (node.url?.includes(TOKEN) ? { ...node, url: sub(node.url) } : undefined);
const versionPlugin = {
  name: "pixi-sandbox-version",
  text: subValue,
  inlineCode: subValue,
  code: subValue,
  html: subValue,
  link: subUrl,
  definition: subUrl,
};

// Deployed to GitHub Pages by .github/workflows/docs.yml.
// Remember the Pages limits: 1 GB published site, ~100 GB/month bandwidth.
// The sandbox payload never belongs here — it lives on the sandbox branch.
export default defineConfig({
  markdown: {
    processor: satteri({ mdastPlugins: [versionPlugin] }),
  },
  site: "https://archont561.github.io",
  base: "/pixi-sandbox",
  integrations: [
    icon({
      include: {
        lucide: ["*"],
        mdi: ["*"],
        "simple-icons": ["*"],
        tabler: ["*"],
      },
    }),
    starlight({
      title: "pixi-sandbox",
      description:
        "Pack pixi environments and restore them on an air-gapped machine — 100% offline, bit-for-bit verified",
      logo: {
        light: "./src/assets/logo-light.svg",
        dark: "./src/assets/logo-dark.svg",
        replacesTitle: false,
      },
      social: [
        { icon: "github", label: "GitHub", href: "https://github.com/Archont561/pixi-sandbox" },
      ],
      editLink: {
        baseUrl: "https://github.com/Archont561/pixi-sandbox/edit/main/docs/",
      },
      customCss: ["./src/styles/custom.css"],
      sidebar: [
        {
          label: "Start here",
          items: [
            { label: "Introduction", slug: "index" },
            { label: "Installation", slug: "installation" },
            { label: "Quickstart", slug: "quickstart" },
          ],
        },
        {
          label: "Guides",
          items: [
            { label: "Using in your project", slug: "guides/using-in-your-project" },
            { label: "CI Publishing", slug: "guides/ci-publishing" },
            { label: "Airlock Restore", slug: "restore" },
            { label: "GitHub Actions", slug: "guides/actions" },
          ],
        },
        {
          label: "Reference",
          items: [
            { label: "Configuration", slug: "reference/configuration" },
            { label: "CLI", slug: "reference/cli" },
            { label: "Actions API", slug: "reference/actions" },
          ],
        },
        {
          label: "Design",
          items: [
            { label: "Architecture", slug: "design" },
            { label: "Repository", slug: "repository" },
          ],
        },
      ],
      expressiveCode: {
        themes: ["github-dark", "github-light"],
      },
    }),
  ],
});
