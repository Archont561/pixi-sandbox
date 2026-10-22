// Temporary connected-side bootstrap for TASK-125, removed after provisioning.
// Store original registry archives as unreferenced Git blobs, not commits/branches.
// This lets an airlocked developer retrieve them through the authenticated GitHub
// API, verify the registry checksums, and populate the ignored local vendor directory.
import { createHash } from "node:crypto";
import { readdir } from "node:fs/promises";
import { join, resolve } from "node:path";

const root = process.cwd();
const repository = process.env.GITHUB_REPOSITORY;
if (!repository || !process.env.GH_TOKEN || !process.env.RUNNER_TEMP) {
  throw new Error("Requires the explicitly authorized connected GitHub Actions job");
}
const manifest = resolve(root, "research/task125-dependencies/Cargo.toml");
const vendor = join(process.env.RUNNER_TEMP, "task125-vendor");
async function run(args, input) {
  const child = Bun.spawn(args, {
    cwd: root,
    stdin: input === undefined ? "ignore" : "pipe",
    stdout: "pipe",
    stderr: "inherit",
  });
  if (input !== undefined) {
    child.stdin.write(input);
    child.stdin.end();
  }
  const output = await new Response(child.stdout).text();
  if ((await child.exited) !== 0) throw new Error(`Failed: ${args.join(" ")}`);
  return output;
}
function packages(lock) {
  return lock.split("[[package]]").slice(1).flatMap((block) => {
    const name = /^name = "([^"]+)"$/m.exec(block)?.[1];
    const version = /^version = "([^"]+)"$/m.exec(block)?.[1];
    const checksum = /^checksum = "([a-f0-9]{64})"$/m.exec(block)?.[1];
    return checksum ? [{ name, version, checksum }] : [];
  });
}
async function blob(bytes) {
  const response = JSON.parse(await run([
    "gh", "api", "--method", "POST", `repos/${repository}/git/blobs`, "--input", "-",
  ], JSON.stringify({ content: Buffer.from(bytes).toString("base64"), encoding: "base64" })));
  if (!/^[a-f0-9]{40}$/.test(response.sha)) throw new Error("Missing Git blob ID");
  return response.sha;
}
await run(["cargo", "vendor", "--manifest-path", manifest, vendor]);
const baseline = new Set(packages(await Bun.file("Cargo.lock").text())
  .map(({ name, version }) => `${name}-${version}`));
const selected = packages(await Bun.file("research/task125-dependencies/Cargo.lock").text());
const cacheRoot = join(process.env.CARGO_HOME || join(process.env.HOME, ".cargo"), "registry/cache");
const registries = await readdir(cacheRoot);
const exported = [];
for (const entry of selected) {
  if (baseline.has(`${entry.name}-${entry.version}`)) continue;
  let bytes;
  for (const registry of registries) {
    const path = join(cacheRoot, registry, `${entry.name}-${entry.version}.crate`);
    if (await Bun.file(path).exists()) {
      bytes = await Bun.file(path).arrayBuffer();
      break;
    }
  }
  if (!bytes) throw new Error(`Missing original registry archive: ${entry.name} ${entry.version}`);
  if (createHash("sha256").update(Buffer.from(bytes)).digest("hex") !== entry.checksum) {
    throw new Error(`Registry checksum mismatch: ${entry.name} ${entry.version}`);
  }
  exported.push({ ...entry, blob: await blob(bytes) });
  console.log(`Verified original crate: ${entry.name} ${entry.version}`);
}
const exportManifest = {
  repository,
  sourceCommit: process.env.GITHUB_SHA,
  packages: exported,
};
const manifestBlob = await blob(Buffer.from(JSON.stringify(exportManifest, null, 2)));
console.log(`TASK125_SOURCE_MANIFEST_BLOB=${manifestBlob}`);
console.log(`Exported ${exported.length} original registry archives; no branch or commit created.`);
