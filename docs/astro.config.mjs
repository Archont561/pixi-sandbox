import { satteri } from "@astrojs/markdown-satteri";
import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";
import icon from "astro-icon";

import { resolveVersion } from "./src/version.mjs";

// The version these docs describe: SANDBOX_VERSION when the caller overrides it
// explicitly (`cargo run -p xtask -- version` prints the same value), otherwise
// the root Cargo.toml read directly. The resolver, and the reasoning behind
// walking up from cwd, live in src/version.mjs so the custom Footer component
// resolves the same value the page content substitutes.
const version = process.env.SANDBOX_VERSION?.trim() || resolveVersion();

// ---------------------------------------------------------------------------
// Replace `__VERSION__` with the derived version, everywhere in the content —
// fenced code blocks, inline code, prose, link targets. The docs never carry a
// literal release tag of ours (release-refs.sh scan enforces it), so a release
// commit rewrites zero documentation lines and a new page cannot ship a stale
// pin because it never contains one.
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
  vite: {
    build: {
      rollupOptions: {
        onwarn(warning, warn) {
          if (
            warning.code === "MODULE_LEVEL_DIRECTIVE" &&
            warning.message.includes('"use astro:head-inject"')
          ) {
            return;
          }
          warn(warning);
        },
      },
    },
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
      // Starlight's footer plus the documented-release line (audit doc-18): the
      // version every page describes, a link to the compatibility page, and the
      // releases feed. The component reuses Starlight's own EditLink,
      // LastUpdated, and Pagination.
      components: {
        Footer: "./src/components/Footer.astro",
        ContentPanel: "./src/components/ContentPanel.astro",
      },
      sidebar: [
        {
          // The audit's eight-group IA (doc-18): roles and lifecycle phases, not
          // artifact types. Groups gain their remaining audit-listed pages as the
          // refactor tasks land (task-97, 99, 100, 102, 103, 104 wire their own
          // sidebar entries), so every commit leaves a working, link-clean site.
          label: "Overview",
          items: [
            { label: "What is pixi-sandbox?", slug: "index" },
            { label: "How it works", slug: "how-it-works" },
            { label: "Glossary and security boundaries", slug: "glossary" },
            { label: "Compatibility and releases", slug: "compatibility" },
          ],
        },
        {
          label: "Start here",
          items: [
            { label: "Install and verify the CLI", slug: "installation" },
            { label: "First verified offline bundle", slug: "first-offline-bundle" },
            { label: "Choose your role", slug: "choose-your-role" },
          ],
        },
        {
          label: "For project owners",
          items: [
            { label: "Configure bundles", slug: "guides/configure-bundles" },
            { label: "Cargo vendoring", slug: "guides/cargo-vendoring" },
            { label: "Transport sizes, sharding, and budgets", slug: "guides/sizes-and-budgets" },
          ],
        },
        {
          label: "For CI maintainers",
          items: [
            { label: "CI publishing", slug: "guides/ci-publishing" },
            { label: "Generated workflow policy", slug: "guides/workflow-policy" },
            { label: "Migration from composite Actions", slug: "guides/migration" },
          ],
        },
        {
          label: "For airlock operators",
          items: [
            { label: "Restore on the airlock", slug: "restore" },
            { label: "Troubleshooting and incident response", slug: "troubleshooting" },
          ],
        },
        {
          label: "Security and operations",
          items: [
            { label: "Verification guarantees", slug: "security/verification-guarantees" },
            { label: "Host and GUI requirements", slug: "security/host-requirements" },
            { label: "Diagnostics and evidence logs", slug: "security/diagnostics" },
          ],
        },
        {
          label: "Reference",
          items: [
            { label: "CLI overview and shared diagnostics", slug: "reference/cli/overview" },
            { label: "Create and verify a transport", slug: "reference/cli/create-verify" },
            { label: "Publish transport branches", slug: "reference/cli/publish" },
            { label: "Restore and inspect a transport", slug: "reference/cli/restore" },
            { label: "Tools and self-update", slug: "reference/cli/tools" },
            { label: "Configuration overview", slug: "reference/configuration/overview" },
            { label: "Bundle, CI, and host policy", slug: "reference/configuration/policy" },
          ],
        },
        {
          label: "Project",
          items: [
            { label: "Architecture", slug: "project/architecture" },
            { label: "Repository and contributing", slug: "project/repository" },
          ],
        },
      ],
      expressiveCode: {
        themes: ["github-dark", "github-light"],
      },
    }),
  ],
});
