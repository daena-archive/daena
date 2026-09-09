#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const workspace = resolve(import.meta.dirname, "..");
const packDir = mkdtempSync(join(tmpdir(), "daena-plugin-pack-"));
const consumer = mkdtempSync(join(tmpdir(), "daena-plugin-consumer-"));
const packedSchema = join(workspace, "packages/plugin-cli/schema");

function run(command, args, cwd) {
  return execFileSync(command, args, { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
}

try {
  for (const name of ["@daena-archive/plugin-sdk", "@daena-archive/plugin-cli", "@daena-archive/plugin-test-host"]) {
    run("npm", ["pack", "-w", name, "--pack-destination", packDir], workspace);
  }
  const tarballs = readdirSync(packDir)
    .filter((name) => name.endsWith(".tgz"))
    .sort();
  assert.equal(tarballs.length, 3);
  const cliTarball = tarballs.find((name) => name.includes("plugin-cli"));
  assert.ok(cliTarball);
  const listing = run("tar", ["tzf", join(packDir, cliTarball)], workspace);
  assert.match(listing, /schema\/plugin-manifest-v1\.json/);
  run("tar", ["xzf", join(packDir, cliTarball), "-C", packDir, "package/schema/plugin-manifest-v1.json"], workspace);
  assert.equal(
    readFileSync(join(packDir, "package/schema/plugin-manifest-v1.json"), "utf8"),
    readFileSync(join(workspace, "schemas/plugin-manifest-v1.json"), "utf8"),
  );

  writeFileSync(
    join(consumer, "package.json"),
    `${JSON.stringify({ name: "outside-plugin", private: true, type: "module" }, null, 2)}\n`,
  );
  run(
    "npm",
    ["install", "--prefer-offline", "--no-fund", "--no-audit", ...tarballs.map((name) => join(packDir, name))],
    consumer,
  );

  const cli = join(consumer, "node_modules/@daena-archive/plugin-cli/bin/daena-plugin.mjs");
  const pluginDir = join(consumer, "my-plugin");
  run(process.execPath, [cli, "init", pluginDir, "--id", "com.example.outside"], consumer);
  const validated = JSON.parse(run(process.execPath, [cli, "validate", pluginDir], consumer));
  assert.equal(validated.ok, true);
  assert.equal(validated.id, "com.example.outside");

  writeFileSync(
    join(consumer, "smoke.mjs"),
    `import { readFileSync } from "node:fs";
import { FakePluginHost } from "@daena-archive/plugin-test-host";

const manifest = JSON.parse(readFileSync("my-plugin/manifest.json", "utf8"));
const host = new FakePluginHost({ manifest, grants: manifest.capabilities });
const bootstrap = await host.client().bootstrap();
if (bootstrap.pluginId !== manifest.id) throw new Error("bootstrap identity mismatch");
console.log("ok");
`,
  );
  assert.match(run(process.execPath, ["smoke.mjs"], consumer), /^ok$/m);
  console.log("plugin pack-outside-monorepo checks passed");
} finally {
  rmSync(packDir, { recursive: true, force: true });
  rmSync(consumer, { recursive: true, force: true });
  rmSync(packedSchema, { recursive: true, force: true });
}
