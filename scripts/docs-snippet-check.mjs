#!/usr/bin/env node
/**
 * Command-snippet test for the docs.
 *
 * Every `pixi-sandbox …` command the documentation tells a reader to run is
 * extracted from the MDX sources and executed against the real CLI's `--help`
 * output: the subcommand must exist, and every documented flag must be one the
 * binary actually accepts. A renamed or removed flag fails here instead of
 * failing on an airlock.
 *
 * Usage:  bun scripts/docs-snippet-check.mjs [--bin PATH] [--src DIR]
 * Exit:   0 when every snippet command checks out, 1 otherwise.
 *
 * `--bin` points at an existing binary. Without it the script builds one with
 * `cargo build -q -p pixi-sandbox` (the repo's default pixi environment carries
 * the Rust toolchain, which is what `pixi run docs-snippet-check` uses).
 */
import { spawnSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const args = process.argv.slice(2);
let bin = null;
let srcDir = "docs/src/content/docs";
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--bin") bin = args[++i];
  else if (args[i] === "--src") srcDir = args[++i];
}

const failures = [];
let commandsChecked = 0;
let flagsChecked = 0;

// ------------------------------------------------------------------ the CLI
if (!bin) {
  for (const candidate of ["target/release/pixi-sandbox", "target/debug/pixi-sandbox"]) {
    if (existsSync(candidate)) {
      bin = candidate;
      break;
    }
  }
}
if (!bin) {
  const build = spawnSync("cargo", ["build", "-q", "-p", "pixi-sandbox"], { stdio: "inherit" });
  if (build.status !== 0 || !existsSync("target/debug/pixi-sandbox")) {
    console.error(
      "snippet-check: no pixi-sandbox binary and `cargo build` failed — run this inside the " +
        "repo's default pixi environment (`pixi run docs-snippet-check`), or pass --bin <path>",
    );
    process.exit(1);
  }
  bin = "target/debug/pixi-sandbox";
}

/** `<bin> <path...> --help` → its combined output, or null when it fails. */
function helpFor(path) {
  const result = spawnSync(bin, [...path, "--help"], { encoding: "utf8" });
  if (result.status !== 0) return null;
  return `${result.stdout}\n${result.stderr}`;
}

/** The subcommand names clap lists under a "Commands:" section. */
function subcommandsOf(help) {
  const at = help.indexOf("Commands:");
  if (at === -1) return [];
  const section = help.slice(at).split(/\n\s*\n/)[0];
  const names = [];
  for (const line of section.split("\n").slice(1)) {
    const m = line.match(/^\s{2,}([a-z0-9][a-z0-9-]*)\s/);
    if (m) names.push(m[1]);
  }
  return names;
}

// ------------------------------------------------------------- extraction
function walk(dir) {
  const out = [];
  for (const entry of readdirSync(dir)) {
    const p = join(dir, entry);
    if (statSync(p).isDirectory()) out.push(...walk(p));
    else if (p.endsWith(".mdx")) out.push(p);
  }
  return out;
}

/** Logical commands from one bash fence, continuations joined. */
function commandsOf(block) {
  const logical = [];
  let current = "";
  for (const raw of block.split("\n")) {
    const line = raw.trim();
    if (line.startsWith("#") || line === "") continue;
    current += current ? ` ${line}` : line;
    if (!line.endsWith("\\")) {
      logical.push(current);
      current = "";
    }
  }
  if (current) logical.push(current);

  const commands = [];
  for (const line of logical) {
    for (const part of line.split(/&&|\|\||;|\|/)) {
      const stripped = part
        .replace(/^([A-Z_][A-Z0-9_]*=\S*\s+)+/, "") // env assignments
        .trim();
      if (stripped) commands.push(stripped);
    }
  }
  return commands;
}

const files = walk(srcDir);
for (const file of files) {
  const source = readFileSync(file, "utf8");
  const fences = [...source.matchAll(/```(?:bash|sh|shell)\n([\s\S]*?)```/g)];
  for (const fence of fences) {
    for (const command of commandsOf(fence[1])) {
      const tokens = command.split(/\s+/);
      if (tokens[0] !== "pixi-sandbox") continue;
      commandsChecked++;

      // Walk the token list, descending into subcommands while clap
      // reports them, collecting flags at the deepest level reached.
      const path = [];
      const flags = [];
      let help = helpFor(path);
      if (help === null) {
        failures.push(`${file}: \`${command}\` — \`pixi-sandbox --help\` failed`);
        continue;
      }
      for (const token of tokens.slice(1)) {
        if (token.startsWith("-")) {
          // Expand clustered short flags (`-vv` is `-v -v`) so each one
          // can be matched against the help text on its own.
          const clustered = /^-[a-zA-Z]{2,}$/.test(token)
            ? token
                .slice(1)
                .split("")
                .map((c) => `-${c}`)
            : [token];
          for (const flag of clustered) {
            // clap's builtins are not documented per subcommand.
            if (["--help", "--version", "-h", "-V"].includes(flag)) continue;
            flags.push(flag);
          }
          continue;
        }
        const subcommands = subcommandsOf(help);
        if (subcommands.includes(token)) {
          path.push(token);
          const next = helpFor(path);
          if (next === null) {
            failures.push(
              `${file}: \`${command}\` — \`pixi-sandbox ${path.join(" ")} --help\` failed`,
            );
            help = null;
            break;
          }
          help = next;
        }
        // Anything else is a positional argument value.
      }
      if (help === null) continue;

      for (const flag of flags) {
        flagsChecked++;
        if (!help.includes(flag)) {
          failures.push(
            `${file}: \`${command}\` — flag ${flag} is not in \`pixi-sandbox ${path.join(" ")} --help\``,
          );
        }
      }
    }
  }
}

// ----------------------------------------------------------------- report
for (const failure of failures) console.log(`FAIL  ${failure}`);
console.log(
  `snippet-check: ${files.length} page(s), ${commandsChecked} pixi-sandbox command(s), ` +
    `${flagsChecked} flag(s) verified, ${failures.length} failure(s)`,
);
process.exit(failures.length === 0 ? 0 : 1);
