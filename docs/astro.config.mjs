import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";
import icon from "astro-icon";

// Deployed to GitHub Pages by .github/workflows/docs.yml.
// Remember the Pages limits: 1 GB published site, ~100 GB/month bandwidth.
// The sandbox payload never belongs here — it lives on the sandbox branch.
export default defineConfig({
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
      description: "Pack pixi environments and restore them on an air-gapped machine — 100% offline, bit-for-bit verified",
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
