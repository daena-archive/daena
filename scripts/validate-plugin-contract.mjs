import { readFile, stat } from "node:fs/promises";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const readJson = async (path) => JSON.parse(await readFile(resolve(root, path), "utf8"));
const manifestSchema = await readJson("schemas/plugin-manifest-v1.json");
const rpcSchema = await readJson("schemas/plugin-rpc-v1.json");
const errorSchema = await readJson("schemas/plugin-error-v1.json");
const capabilityRegistry = await readJson("schemas/capability-registry-v1.json");

if (manifestSchema.$id !== "https://github.com/daena-archive/daena/schemas/plugin-manifest-v1.json")
  throw new Error("manifest schema id mismatch");
if (rpcSchema.$id !== "https://github.com/daena-archive/daena/schemas/plugin-rpc-v1.json")
  throw new Error("RPC schema id mismatch");
if (errorSchema.$id !== "https://github.com/daena-archive/daena/schemas/plugin-error-v1.json")
  throw new Error("error schema id mismatch");
if (capabilityRegistry.version !== 1 || capabilityRegistry.deniedByDefault.length === 0)
  throw new Error("capability registry is incomplete");
const rpcMethods = Object.entries(rpcSchema["x-methods"] ?? {});
const requestSchema = rpcSchema.$defs?.request;
if (rpcMethods.length < 20 || !requestSchema?.properties?.method?.enum || !Array.isArray(requestSchema.allOf)) {
  throw new Error("RPC method schema is incomplete");
}
for (const [method, contract] of rpcMethods) {
  if (!contract.payload || !rpcSchema.$defs[contract.payload]) throw new Error(`${method}: missing payload definition`);
  if (!requestSchema.properties.method.enum.includes(method)) throw new Error(`${method}: missing method enum entry`);
  if (!requestSchema.allOf.some((condition) => condition.if?.properties?.method?.const === method)) {
    throw new Error(`${method}: missing request payload condition`);
  }
}

for (const name of ["lore", "timeline", "writing", "maps"]) {
  const manifest = await readJson(`packages/modules/${name}/manifest.json`);
  const required = [
    "manifestVersion",
    "id",
    "name",
    "version",
    "publisher",
    "hostApi",
    "kind",
    "entrypoints",
    "capabilities",
    "dependencies",
    "namespaces",
    "schemas",
    "templates",
    "views",
    "commands",
    "services",
    "events",
    "migrations",
  ];
  for (const key of required) if (!(key in manifest)) throw new Error(`${name}: missing ${key}`);
  if (manifest.manifestVersion !== 1 || manifest.id !== `daena.${name}`) throw new Error(`${name}: identity mismatch`);
  if (manifest.migrations.length !== 1 || manifest.migrations[0].from !== 0 || manifest.migrations[0].to !== 1)
    throw new Error(`${name}: migration chain mismatch`);
  if (!manifest.namespaces.includes(manifest.schemas[0].namespace))
    throw new Error(`${name}: schema namespace is not owned`);
}

{
  const maps = await readJson("packages/modules/maps/manifest.json");
  const relationshipTypes = maps.schemas[0].fields
    .filter((field) => field.type === "relationship")
    .map((field) => field.relationshipType)
    .sort();
  const expected = ["daena.maps:detail-map", "daena.maps:overview-map", "daena.maps:related-map"];
  if (JSON.stringify(relationshipTypes) !== JSON.stringify(expected)) {
    throw new Error(
      `maps: expected hierarchy relationships ${expected.join(", ")}, got ${relationshipTypes.join(", ")}`,
    );
  }
  const fixtures = await readJson("docs/maps/phase-1-fixtures.json");
  if (!Array.isArray(fixtures.fixtures) || fixtures.fixtures.length < 3) {
    throw new Error("maps phase-1 fixtures are incomplete");
  }
  if (JSON.stringify([...(fixtures.relationships ?? [])].sort()) !== JSON.stringify(expected)) {
    throw new Error("maps phase-1 fixture relationships drift from the manifest");
  }
  for (const fixture of fixtures.fixtures) {
    if (fixture.value?.schemaVersion !== 1) throw new Error(`${fixture.id}: schemaVersion must be 1`);
  }
}

for (const name of ["declarative", "ui", "wasm-service"]) {
  const manifest = await readJson(`examples/plugins/${name}/manifest.json`);
  if (!manifest.id.startsWith("com.example.")) throw new Error(`${name}: example identity mismatch`);
  const entrypoint = manifest.entrypoints.ui ?? manifest.entrypoints.wasm;
  if (!entrypoint) throw new Error(`${name}: example entrypoint missing`);
  await stat(resolve(root, `examples/plugins/${name}`, entrypoint));
}

console.log("plugin contract fixtures are structurally valid");
