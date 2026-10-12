#!/usr/bin/env node
/**
 * Frontmatter guard for the docs MDX sources.
 *
 * Starlight parses each page's YAML frontmatter at build time, and an unquoted
 * `title:`/`description:` value that contains `: ` (or ends with `:`) is a YAML
 * mapping — the build fails with a confusing parse error far from the cause.
 * This check fails first, naming the page and the line.
 *
 * Usage:  bun scripts/docs-frontmatter-check.mjs [srcDir]
 * Exit:   0 when every page's frontmatter is safe, 1 otherwise.
 */
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const srcDir = process.argv[2] ?? "docs/src/content/docs";
if (!existsSync(srcDir)) {
  console.error(`frontmatter-check: source directory ${srcDir} does not exist`);
  process.exit(1);
}

const problems = [];

function walk(dir) {
  const out = [];
  for (const entry of readdirSync(dir)) {
    const p = join(dir, entry);
    if (statSync(p).isDirectory()) out.push(...walk(p));
    else if (p.endsWith(".mdx")) out.push(p);
  }
  return out;
}

for (const file of walk(srcDir)) {
  const text = readFileSync(file, "utf8");
  const match = text.match(/^---\n([\s\S]*?)\n---\n/);
  if (!match) {
    problems.push(`${relative(".", file)}: no frontmatter block`);
    continue;
  }
  for (const line of match[1].split("\n")) {
    const keyMatch = line.match(/^(title|description):\s+(.*)$/);
    if (!keyMatch) continue;
    const value = keyMatch[2];
    if (
      value.startsWith('"') ||
      value.startsWith("'") ||
      value.startsWith(">") ||
      value.startsWith("|") ||
      value.startsWith("[")
    ) {
      continue; // already a quoted / folded / flow scalar
    }
    if (value.includes(": ") || value.endsWith(":") || value.includes(" #")) {
      problems.push(
        `${relative(".", file)}: unquoted ${keyMatch[1]} with a YAML-breaking sequence: ${line.slice(0, 70)}`,
      );
    }
  }
}

for (const problem of problems) console.log(`FAIL  ${problem}`);
console.log(`frontmatter-check: ${problems.length} problem(s)`);
process.exit(problems.length === 0 ? 0 : 1);
