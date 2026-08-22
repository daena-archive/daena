import { buildExternalImportMappingCatalog, importFolderFor } from "../src/lib/externalImport.ts";
import type { ProjectModuleManifest } from "../src/lib/project/client.ts";

function manifest(id: string, enabled: boolean, entityType: string, fieldKey: string): ProjectModuleManifest {
  return {
    id,
    enabled,
    manifestVersion: 1,
    name: id,
    version: "1.0.0",
    publisher: "test",
    kind: "sandboxed",
    hostApi: "1",
    enabledByDefault: false,
    capabilities: [],
    namespaces: [id],
    dependencies: {},
    entrypoints: {},
    schemas: [
      {
        namespace: id,
        entityTypes: [entityType],
        fields: [{ key: fieldKey, label: fieldKey, type: "text" }],
      },
    ],
    templates: [
      {
        id: `${id}.default`,
        name: `${entityType} template`,
        entityType,
        fields: {},
      },
    ],
    migrations: [],
    views: [],
    commands: [],
    services: { provides: [], consumes: [] },
    events: { publishes: [], subscribes: [] },
  } as ProjectModuleManifest;
}

Deno.test("external import mapping catalog excludes disabled contributions", () => {
  const catalog = buildExternalImportMappingCatalog([
    manifest("enabled.module", true, "person", "summary"),
    manifest("disabled.module", false, "place", "coordinates"),
  ]);

  if (catalog.entityTypes.map((choice) => choice.id).join(",") !== "person") {
    throw new Error("disabled entity types leaked into the mapping catalog");
  }
  if (catalog.fields.map((choice) => choice.key).join(",") !== "summary") {
    throw new Error("disabled fields leaked into the mapping catalog");
  }
  if (catalog.fingerprint.includes("disabled.module")) {
    throw new Error("disabled manifests leaked into the mapping fingerprint");
  }
});

Deno.test("external import folder scope uses portable parent paths", () => {
  if (importFolderFor("People/Heroes/Alice.md") !== "People/Heroes") {
    throw new Error("nested portable folder was not preserved");
  }
  if (importFolderFor("Root.md") !== "") {
    throw new Error("root-level files must use the global scope");
  }
});

Deno.test("external import keeps relationship fields out of ordinary field mappings", () => {
  const enabled = manifest("enabled.module", true, "person", "summary");
  enabled.schemas[0].fields.push({
    key: "references",
    label: "References",
    type: "relationship",
    relationshipType: "references",
    targetEntityTypes: ["person"],
  });
  const catalog = buildExternalImportMappingCatalog([enabled]);

  if (catalog.fields.some((choice) => choice.key === "references")) {
    throw new Error("relationship fields leaked into ordinary import field mappings");
  }
  if (catalog.relationships.map((choice) => choice.id).join(",") !== "references") {
    throw new Error("relationship mapping must use the manifest relationship type");
  }
});
