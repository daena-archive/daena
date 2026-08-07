import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { validatePluginManifest } from "../packages/plugin-sdk/dist/index.js";

const root = resolve(import.meta.dirname, "..");
const readJson = async (path) => JSON.parse(await readFile(resolve(root, path), "utf8"));

const index = await readJson("schemas/fixtures/manifest/index.json");
const cases = index.fixtures;
if (cases.length < 18) throw new Error(`expected at least 18 fixtures, found ${cases.length}`);

let checked = 0;
for (const { rule, file, expected } of cases) {
  const manifest = await readJson(`schemas/fixtures/manifest/${file}`);
  const errors = validatePluginManifest(manifest);
  const outcome = errors.length ? "rejected" : "accepted";
  if (outcome !== expected) {
    throw new Error(`fixture ${rule} (${file}): expected ${expected}, got ${outcome}: ${errors.join("; ")}`);
  }
  checked += 1;
}

for (const name of ["lore", "timeline", "writing"]) {
  const manifest = await readJson(`packages/modules/${name}/manifest.json`);
  const errors = validatePluginManifest(manifest);
  if (errors.length) throw new Error(`${name} manifest should be valid: ${errors.join("; ")}`);
}

if (checked !== cases.length) throw new Error("parity script must exercise every indexed fixture");
console.log(`TS validator agrees with ${checked} manifest fixtures`);
