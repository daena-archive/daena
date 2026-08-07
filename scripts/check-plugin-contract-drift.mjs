#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");

const schemaNames = [
  "plugin-manifest-v1.json",
  "plugin-rpc-v1.json",
  "plugin-error-v1.json",
  "capability-registry-v1.json",
];
const distGenerated = ["generated.js", "generated.d.ts", "generated.js.map", "generated.d.ts.map"];

function normalizeSourceMap(bytes) {
  const map = JSON.parse(bytes);
  delete map.sources;
  return JSON.stringify(map);
}

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

const temp = mkdtempSync(join(tmpdir(), "daena-contract-drift-"));
try {
  const schemaOut = join(temp, "schemas");
  mkdirSync(schemaOut, { recursive: true });

  run([
    "cargo",
    "run",
    "--quiet",
    "--manifest-path",
    "crates/daena-plugin-api/Cargo.toml",
    "--features",
    "gen",
    "--locked",
    "--offline",
    "--bin",
    "gen-contract",
  ], { DAENA_SCHEMA_OUT_DIR: schemaOut });

  const tsOut = join(temp, "generated.ts");
  run(["node", "scripts/gen-plugin-contract.mjs"], {
    DAENA_SCHEMA_DIR: schemaOut,
    DAENA_GENERATED_TS: tsOut,
  });

  for (const name of schemaNames) {
    const fresh = readFileSync(join(schemaOut, name));
    const committed = readFileSync(resolve(root, "schemas", name));
    if (!fresh.equals(committed)) {
      throw new Error(`schemas/${name} is stale — run \`npm run gen:plugin-contract\``);
    }
  }

  const freshTs = readFileSync(tsOut);
  const committedTs = readFileSync(resolve(root, "packages/plugin-sdk/src/generated.ts"));
  if (!freshTs.equals(committedTs)) {
    throw new Error("packages/plugin-sdk/src/generated.ts is stale — run `npm run gen:plugin-contract`");
  }

  const distOut = join(temp, "sdk-dist");
  mkdirSync(distOut, { recursive: true });
  run([
    process.execPath,
    resolve(root, "node_modules/typescript/bin/tsc"),
    "-p",
    "packages/plugin-sdk/tsconfig.json",
    "--outDir",
    distOut,
  ]);

  for (const name of distGenerated) {
    const fresh = readFileSync(join(distOut, name));
    const committed = readFileSync(resolve(root, "packages/plugin-sdk/dist", name));
    const same = name.endsWith(".map")
      ? normalizeSourceMap(fresh) === normalizeSourceMap(committed)
      : fresh.equals(committed);
    if (!same) {
      throw new Error(`packages/plugin-sdk/dist/${name} is stale — run \`npm run build:plugin-sdk\``);
    }
  }

  console.log("plugin contract artifacts are in sync with the Rust sources");
} finally {
  rmSync(temp, { recursive: true, force: true });
}
