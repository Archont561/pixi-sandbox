import { defineCollection } from "astro:content";
import { docsLoader, i18nLoader } from "@astrojs/starlight/loaders";
import { docsSchema, i18nSchema } from "@astrojs/starlight/schema";

// The i18n collection is where UI-string overrides live (ours: `page.editLink` in
// src/content/i18n/en.json). Starlight's runtime reads it on every page render, so leaving
// it undeclared made every build warn "The collection 'i18n' does not exist or is empty".
export const collections = {
  docs: defineCollection({ loader: docsLoader(), schema: docsSchema() }),
  i18n: defineCollection({ loader: i18nLoader(), schema: i18nSchema() }),
};
