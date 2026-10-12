#!/usr/bin/env node
/**
 * Internal link integrity check for the built docs site.
 *
 * Walks every generated HTML file under the build directory and verifies that
 * each internal link — and each fragment on it — resolves inside the same
 * build. External URLs, `mailto:`, `data:`, and `#_top` are out of scope.
 *
 * Usage:  bun scripts/docs-link-check.mjs [distDir] [--base /pixi-sandbox]
 * Exit:   0 when every internal link resolves, 1 otherwise.
 *
 * This is the gate that keeps the docs IA re-homes honest: splitting or moving
 * a page without retargeting its inbound links fails here, in CI, before the
 * site is published.
 */
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, normalize } from "node:path";

const args = process.argv.slice(2);
let dist = "docs/dist";
let base = "/pixi-sandbox";

for (let i = 0; i < args.length; i++) {
  if (args[i] === "--base") {
    base = args[++i];
  } else if (!args[i].startsWith("--")) {
    dist = args[i];
  }
}

if (!existsSync(dist)) {
  console.error(`link-check: build directory ${dist} does not exist — build the site first`);
  process.exit(1);
}

function walk(dir) {
  const out = [];
  for (const entry of readdirSync(dir)) {
    const p = join(dir, entry);
    if (statSync(p).isDirectory()) out.push(...walk(p));
    else if (p.endsWith(".html")) out.push(p);
  }
  return out;
}

const htmlFiles = walk(dist);
const attrRe = /(?:href|src)="([^"]+)"/g;
let failures = 0;
let checked = 0;

/** Does the target HTML carry the fragment as an id or a legacy anchor name? */
function hasFragment(file, frag) {
  const html = readFileSync(file, "utf8");
  if (html.includes(`id="${frag}"`) || html.includes(`id='${frag}'`)) return true;
  if (html.includes(`name="${frag}"`) || html.includes(`name='${frag}'`)) return true;
  // Tolerate ids that embed the fragment (Starlight's generated heading ids).
  const escaped = frag.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return new RegExp(`id="[^"]*\\b${escaped}[^"]*"`).test(html);
}

function resolveTarget(root, path) {
  const clean = normalize(path || "/").replace(/\/$/, "");
  const relative = join(root, clean);
  return [
    relative.endsWith(".html") ? relative : join(relative, "index.html"),
    `${relative}.html`,
    relative,
  ].find((c) => existsSync(c) && statSync(c).isFile());
}

for (const file of htmlFiles) {
  const html = readFileSync(file, "utf8");
  const here = normalize(file).replace(/\/[^/]*$/, "");
  for (const m of html.matchAll(attrRe)) {
    const url = m[1];
    if (/^(https?:|mailto:|data:|javascript:)/.test(url)) continue;

    let targetFile;
    let frag;
    if (url.startsWith("#")) {
      // Same-page fragment link.
      if (url === "#_top") continue;
      targetFile = file;
      frag = url.slice(1);
    } else if (url === base || url.startsWith(`${base}/`)) {
      // Site-absolute URL: resolve against the build root.
      const [path, fragment] = url.slice(base.length).split("#");
      targetFile = resolveTarget(dist, path);
      frag = fragment;
    } else if (!url.startsWith("/")) {
      // Relative URL: resolve against the linking page.
      const [path, fragment] = url.split("#");
      targetFile = resolveTarget(here, path);
      frag = fragment;
    } else {
      // Root-relative outside the site base: not ours to check.
      continue;
    }

    checked++;
    if (!targetFile) {
      failures++;
      console.log(`MISSING  ${file.replace(`${dist}/`, "")}  ->  ${url}`);
      continue;
    }
    if (frag && frag !== "_top" && !hasFragment(targetFile, frag)) {
      failures++;
      console.log(`FRAGMENT ${file.replace(`${dist}/`, "")}  ->  ${url}`);
    }
  }
}

const pageCount = htmlFiles.length;
console.log(
  `link-check: ${checked} internal links across ${pageCount} page(s) checked, ${failures} failure(s)`,
);
process.exit(failures === 0 ? 0 : 1);
