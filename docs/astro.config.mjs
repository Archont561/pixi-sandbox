import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";

// Deployed to GitHub Pages by .github/workflows/docs.yml.
// Remember the Pages limits: 1 GB published site, ~100 GB/month bandwidth.
// The sandbox payload never belongs here — it lives on the sandbox branch.
export default defineConfig({
  site: "https://OWNER.github.io",
  base: "/pixi-sandbox",
  integrations: [
    starlight({
      title: "pixi-sandbox",
      description: "Pack pixi environments and restore them on an air-gapped machine",
      social: [{ icon: "github", label: "GitHub", href: "https://github.com/OWNER/pixi-sandbox" }],
      sidebar: [
        { label: "Start here", items: ["index", "quickstart"] },
        { label: "For the airlock operator", items: ["restore"] },
        { label: "Design", items: ["design"] },
      ],
      editLink: { baseUrl: "https://github.com/OWNER/pixi-sandbox/edit/main/docs/" },
    }),
  ],
});
