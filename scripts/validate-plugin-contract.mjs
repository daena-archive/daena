import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const readJson = async (path) => JSON.parse(await readFile(resolve(root, path), "utf8"));
const manifestSchema = await readJson("schemas/plugin-manifest-v1.json");
const rpcSchema = await readJson("schemas/plugin-rpc-v1.json");
const errorSchema = await readJson("schemas/plugin-error-v1.json");
const capabilityRegistry = await readJson("schemas/capability-registry-v1.json");

if (manifestSchema.$id !== "https://worldbuilder.app/schemas/plugin-manifest-v1.json") throw new Error("manifest schema id mismatch");
if (rpcSchema.$id !== "https://worldbuilder.app/schemas/plugin-rpc-v1.json") throw new Error("RPC schema id mismatch");
if (errorSchema.$id !== "https://worldbuilder.app/schemas/plugin-error-v1.json") throw new Error("error schema id mismatch");
if (capabilityRegistry.version !== 1 || capabilityRegistry.deniedByDefault.length === 0) throw new Error("capability registry is incomplete");

for (const name of ["lore", "timeline", "writing"]) {
  const manifest = await readJson(`packages/modules/${name}/manifest.json`);
  const required = ["manifestVersion", "id", "name", "version", "publisher", "hostApi", "kind", "entrypoints", "capabilities", "dependencies", "namespaces", "schemas", "templates", "views", "commands", "services", "events", "migrations"];
  for (const key of required) if (!(key in manifest)) throw new Error(`${name}: missing ${key}`);
  if (manifest.manifestVersion !== 1 || manifest.id !== `worldbuilder.${name}`) throw new Error(`${name}: identity mismatch`);
  if (manifest.migrations.length !== 1 || manifest.migrations[0].from !== 0 || manifest.migrations[0].to !== 1) throw new Error(`${name}: migration chain mismatch`);
  if (!manifest.namespaces.includes(manifest.schemas[0].namespace)) throw new Error(`${name}: schema namespace is not owned`);
}

console.log("plugin contract fixtures are structurally valid");
