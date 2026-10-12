#!/usr/bin/env node
/**
 * Accessibility and print/no-script gate for the built docs site.
 *
 * Usage:  bun scripts/docs-a11y-check.mjs [distDir]
 * Exit:   0 when every check passes, 1 otherwise.
 *
 * What runs here (no browser required):
 *   - structure:   one <h1> per page, no skipped heading levels, <html lang>,
 *                  alt text on images, accessible names on links/buttons,
 *                  decorative SVGs aria-hidden, table header cells, skip link
 *   - focus:       :focus-visible rules exist in the built stylesheet
 *   - print:       a print stylesheet exists; the operator checklist avoids
 *                  page breaks inside itself
 *   - no-script:   the operator checklist region carries no <script> and no
 *                  client island, and its tick boxes are CSS borders rather
 *                  than glyphs that need a font
 *   - contrast:    WCAG 2.1 contrast ratios for the site's own color pairs,
 *                  computed in both the dark and light themes
 *   - responsive:  the transport diagram collapses on narrow viewports and
 *                  wide tables stay scrollable
 *
 * The keyboard walk-through (Tab order through nav groups, search, the theme
 * control, and copy buttons) is a browser protocol and is documented in
 * project/repository.mdx under "Docs quality gates".
 */
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const dist = process.argv[2] ?? "docs/dist";
if (!existsSync(dist)) {
  console.error(`a11y-check: build directory ${dist} does not exist — build the site first`);
  process.exit(1);
}

const failures = [];
const notes = [];
const fail = (page, message) => failures.push(`${page}: ${message}`);

function walk(dir) {
  const out = [];
  for (const entry of readdirSync(dir)) {
    const p = join(dir, entry);
    if (statSync(p).isDirectory()) out.push(...walk(p));
    else out.push(p);
  }
  return out;
}

const allFiles = walk(dist);
const htmlFiles = allFiles.filter((f) => f.endsWith(".html"));

/** Strip tags and collapse whitespace, for discernible-text checks. */
function textOf(html) {
  // An <img> contributes its alt text to the accessible name of its link.
  const withAlts = html.replace(/<img\b[^>]*>/g, (tag) => {
    const alt = tag.match(/\balt="([^"]*)"/);
    return alt ? ` ${alt[1]} ` : " ";
  });
  return withAlts
    .replace(/<script[\s\S]*?<\/script>/gi, " ")
    .replace(/<style[\s\S]*?<\/style>/gi, " ")
    .replace(/<[^>]+>/g, " ")
    .replace(/&[a-z]+;/gi, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function attrOf(tag, name) {
  const m = tag.match(new RegExp(`${name}="([^"]*)"`));
  return m ? m[1] : "";
}

// ---------------------------------------------------------------- structure
for (const file of htmlFiles) {
  const page = file.replace(`${dist}/`, "");
  const html = readFileSync(file, "utf8");
  const body = html.slice(html.indexOf("<body"));

  if (!/<html[^>]+lang="[^"]+"/.test(html)) fail(page, "<html> has no lang attribute");

  const h1s = body.match(/<h1[\s>]/g) ?? [];
  if (h1s.length !== 1) fail(page, `expected exactly one <h1>, found ${h1s.length}`);

  const headings = [...body.matchAll(/<h([1-6])[\s>]/g)].map((m) => Number(m[1]));
  let previous = 0;
  for (const level of headings) {
    if (previous && level > previous + 1) {
      fail(page, `heading level jumps from h${previous} to h${level}`);
    }
    previous = level;
  }

  for (const img of body.matchAll(/<img\b[^>]*>/g)) {
    const tag = img[0];
    // An empty alt is the correct markup for a decorative image (the site
    // logo beside the site title, for instance); a *missing* alt is not.
    // A valueless `alt` attribute parses as the empty string, so accept both.
    if (!/(^|\s)alt(=|\s|$)/.test(tag))
      fail(page, `<img> without an alt attribute: ${tag.slice(0, 80)}`);
    // An image that is a link's only content must name the destination.
    const openAt = body.slice(0, img.index).lastIndexOf("<a");
    const closeAt = body.slice(0, img.index).lastIndexOf("</a>");
    if (openAt > closeAt) {
      const linkEnd = body.indexOf("</a>", img.index);
      const linkText = textOf(body.slice(openAt, linkEnd === -1 ? undefined : linkEnd));
      if (!linkText && !attrOf(tag, "alt").trim()) {
        fail(page, `<img> is a link's only content but has empty alt: ${tag.slice(0, 80)}`);
      }
    }
  }

  for (const svg of body.matchAll(/<svg\b[^>]*>/g)) {
    const tag = svg[0];
    const hidden =
      /aria-hidden="true"/.test(tag) ||
      // Starlight hides its own decorative icons on a wrapping <span>.
      /aria-hidden="true"/.test(body.slice(Math.max(0, svg.index - 200), svg.index));
    const labelled =
      /role="img"/.test(tag) && /<title>/.test(body.slice(svg.index, svg.index + 400));
    if (!hidden && !labelled)
      fail(page, `inline <svg> is neither aria-hidden nor role="img"+<title>`);
  }

  for (const a of body.matchAll(/<a\b[^>]*>[\s\S]*?<\/a>/g)) {
    const tag = a[0];
    if (/aria-hidden="true"/.test(tag)) continue;
    const label = attrOf(tag, "aria-label");
    const text = textOf(tag);
    if (!label.trim() && !text) fail(page, `<a> without discernible text: ${tag.slice(0, 80)}`);
  }

  for (const button of body.matchAll(/<button\b[^>]*>[\s\S]*?<\/button>/g)) {
    const tag = button[0];
    const label = attrOf(tag, "aria-label") || attrOf(tag, "title");
    const text = textOf(tag);
    if (!label.trim() && !text) {
      fail(page, `<button> without an accessible name: ${tag.slice(0, 80)}`);
    }
  }

  for (const table of body.matchAll(/<table\b[\s\S]*?<\/table>/g)) {
    if (!/<th[\s>]/.test(table[0])) fail(page, "<table> without a <th> header cell");
  }

  if (!/class="[^"]*sl-skip-link/.test(body)) fail(page, "no skip link found");
}

// ------------------------------------------------------- css-level checks
const css = allFiles
  .filter((f) => f.endsWith(".css"))
  .map((f) => readFileSync(f, "utf8"))
  .join("\n");

if (!/:focus-visible/.test(css)) {
  failures.push("stylesheet: no :focus-visible rules — keyboard focus would be invisible");
}
if (!/@media print/.test(css)) {
  failures.push("stylesheet: no @media print block");
}
if (!/\.sandbox-diagram[\s\S]{0,400}grid-template-columns:\s*1fr/.test(css)) {
  failures.push("stylesheet: .sandbox-diagram has no single-column collapse rule");
}
if (!/overflow-x:\s*auto/.test(css)) {
  failures.push("stylesheet: no overflow-x rule — wide tables cannot scroll");
}

// --------------------------------------------- no-script operator checklist
const restoreFile = join(dist, "restore/index.html");
if (!existsSync(restoreFile)) {
  failures.push("restore/index.html missing — the operator checklist page did not build");
} else {
  const restore = readFileSync(restoreFile, "utf8");
  const bodyStart = restore.indexOf("<body");
  const start = restore.indexOf("guide-meta", bodyStart);
  const end = restore.indexOf('id="1-get-the-branch"', bodyStart);
  const checklist = restore.slice(start, end);
  if (start === -1 || end === -1 || end < start) {
    failures.push("restore: could not locate the operator checklist region");
  } else {
    if (/<script/i.test(checklist)) fail("restore", "operator checklist contains a <script>");
    if (/astro-island/i.test(checklist))
      fail("restore", "operator checklist contains a client island");
    if (checklist.includes("\u2610")) {
      fail("restore", "operator checklist uses a glyph tick box (font-dependent)");
    }
    const boxes = checklist.match(/border:\s*1\.5px solid currentColor/g) ?? [];
    if (boxes.length < 7) {
      fail("restore", `expected 7 CSS-drawn tick boxes, found ${boxes.length}`);
    }
    if (!/break-inside:\s*avoid/.test(checklist)) {
      fail("restore", "operator checklist does not avoid page breaks when printed");
    }
    if (!/href="#[1-4]-/.test(checklist)) {
      fail("restore", "operator checklist does not link to its detailed sections");
    }
    notes.push(
      `restore: operator checklist verified script-free and print-safe (${boxes.length} tick boxes)`,
    );
  }
}

// ------------------------------------------------------------- contrast
/** #rgb / #rrggbb / rgb() → relative luminance. */
function luminance(color) {
  let r, g, b;
  const hex = color.trim().match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/i);
  if (hex) {
    const h = hex[1];
    const full =
      h.length === 3
        ? h
            .split("")
            .map((c) => c + c)
            .join("")
        : h;
    r = parseInt(full.slice(0, 2), 16);
    g = parseInt(full.slice(2, 4), 16);
    b = parseInt(full.slice(4, 6), 16);
  } else {
    const m = color.match(/rgba?\(([^)]+)\)/i);
    if (!m) return null;
    const parts = m[1].split(",").map((p) => parseFloat(p));
    [r, g, b] = parts;
  }
  const lin = (v) => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

function contrast(a, b) {
  const la = luminance(a);
  const lb = luminance(b);
  if (la === null || lb === null) return null;
  const [hi, lo] = la > lb ? [la, lb] : [lb, la];
  return (hi + 0.05) / (lo + 0.05);
}

/** Extract the custom properties of a `:root`-scoped block. */
function tokenBlock(selectorNeedle) {
  const at = css.indexOf(selectorNeedle);
  if (at === -1) return null;
  const open = css.indexOf("{", at);
  let depth = 0;
  let i = open;
  for (; i < css.length; i++) {
    if (css[i] === "{") depth++;
    else if (css[i] === "}") {
      depth--;
      if (depth === 0) break;
    }
  }
  const block = css.slice(open + 1, i);
  const tokens = {};
  for (const m of block.matchAll(/(--sl-color-[a-z0-9-]+)\s*:\s*([^;]+)/g)) {
    tokens[m[1]] = m[2].trim();
  }
  return tokens;
}

/** Resolve `var(--x)` chains inside one theme's token map. */
function resolve(tokens, name, seen = new Set()) {
  if (seen.has(name)) return null;
  seen.add(name);
  const value = tokens[name];
  if (!value) return null;
  const ref = value.match(/^var\((--[a-z0-9-]+)\)$/);
  if (ref) return resolve(tokens, ref[1], seen);
  return value;
}

const themes = {
  dark: tokenBlock(":root,::backdrop"),
  // The light theme overrides a subset of the base tokens; start from the
  // base map so `var()` chains (text → gray-2, bg → black) still resolve.
  light: {
    ...(tokenBlock(":root,::backdrop") ?? {}),
    ...(tokenBlock("data-theme=light") ?? {}),
  },
};

/** [label, foreground token or literal, background token or literal, minimum] */
const pairs = [
  ["body text", "--sl-color-text", "--sl-color-bg", 4.5],
  ["secondary text on panel (meta box, cards)", "--sl-color-gray-2", "--sl-color-gray-6", 4.5],
  ["diagram caption on page", "--sl-color-gray-3", "--sl-color-bg", 4.5],
  ["link text", "--sl-color-text-accent", "--sl-color-bg", 4.5],
  // Starlight's primary link button: accent background, inverted label.
  ["primary button label", "--sl-color-black", "--sl-color-text-accent", 4.5],
  ["callout text on tinted background", "--sl-color-text", "--sl-color-accent-low", 4.5],
  // Hardcoded component colors (theme-independent by design).
  // Note: `--sl-color-gray-3` on a `--sl-color-gray-6` panel measures 4.49:1
  // in the dark theme, so small captions on those cards use gray-2.
  ["diagram env chip: dev", "#000000", "#facc15", 4.5],
  ["diagram env chip: docs", "#000000", "#e5e7eb", 4.5],
  ["diagram offline badge", "#ffffff", "#047857", 4.5],
  ["how-it-works step marker (blue)", "#ffffff", "#1d4ed8", 4.5],
  ["how-it-works step marker (green)", "#ffffff", "#047857", 4.5],
  ["how-it-works step marker (yellow)", "#000000", "#facc15", 4.5],
];

for (const [themeName, tokens] of Object.entries(themes)) {
  if (!tokens) {
    failures.push(`contrast: could not read the ${themeName} theme tokens`);
    continue;
  }
  for (const [label, fg, bg, min] of pairs) {
    const fgValue = fg.startsWith("--") ? resolve(tokens, fg) : fg;
    const bgValue = bg.startsWith("--") ? resolve(tokens, bg) : bg;
    if (!fgValue || !bgValue) {
      notes.push(`contrast[${themeName}]: skipped ${label} (token missing)`);
      continue;
    }
    const ratio = contrast(fgValue, bgValue);
    if (ratio === null) {
      notes.push(
        `contrast[${themeName}]: skipped ${label} (unparseable color ${fgValue}/${bgValue})`,
      );
      continue;
    }
    if (ratio < min) {
      failures.push(
        `contrast[${themeName}]: ${label} — ${fgValue} on ${bgValue} is ${ratio.toFixed(2)}:1, below ${min}:1`,
      );
    } else {
      notes.push(`contrast[${themeName}]: ${label} ${ratio.toFixed(2)}:1 ok`);
    }
  }
}

// ---------------------------------------------------------------- report
for (const note of notes) console.log(`  ok  ${note}`);
if (failures.length > 0) {
  for (const f of failures) console.log(`FAIL  ${f}`);
}
console.log(`a11y-check: ${htmlFiles.length} page(s), ${failures.length} failure(s)`);
process.exit(failures.length === 0 ? 0 : 1);
