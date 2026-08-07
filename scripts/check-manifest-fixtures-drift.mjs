#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const committedDir = join(root, "schemas", "fixtures", "manifest");

function run(args, env = {}) {
  const result = spawnSync(args[0], args.slice(1), {
    cwd: root,
    encoding: "utf8",
    env: { ...process.env, ...env },
  });
  if (result.status !== 0) {
    throw new Error(`${args.join(" ")} failed:\n${result.stderr ?? result.stdout}`);
  }
  return result.stdout ?? "";
}

const temp = mkdtempSync(join(tmpdir(), "daena-fixtures-drift-"));
try {
  run(["node", "scripts/gen-plugin-manifest-fixtures.mjs"], { DAENA_FIXTURES_OUT_DIR: temp });

  const freshIndex = readFileSync(join(temp, "index.json"));
  const committedIndex = readFileSync(join(committedDir, "index.json"));
  if (!freshIndex.equals(committedIndex)) {
    throw new Error("schemas/fixtures/manifest/index.json is stale — run `npm run gen:manifest-fixtures`");
  }

  const index = JSON.parse(freshIndex);
  for (const { file } of index.fixtures) {
    const fresh = readFileSync(join(temp, file));
    const committed = readFileSync(join(committedDir, file));
    if (!fresh.equals(committed)) {
      throw new Error(`schemas/fixtures/manifest/${file} is stale — run \`npm run gen:manifest-fixtures\``);
    }
  }

  console.log(`manifest fixtures are in sync with the generator (${index.fixtures.length} fixtures)`);
} finally {
  rmSync(temp, { recursive: true, force: true });
}
