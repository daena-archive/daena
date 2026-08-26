import assert from "node:assert/strict";

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
        entityTypes: [
          {
            id: entityType,
            name: entityType,
            icon: { kind: "catalog", id: entityType } as any,
            iconColor: { kind: "preset", id: "violet" } as any,
          },
        ],
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
  } as unknown as ProjectModuleManifest;
}

const catalog = buildExternalImportMappingCatalog([
  manifest("enabled.module", true, "person", "summary"),
  manifest("disabled.module", false, "place", "coordinates"),
]);
assert.deepEqual(
  catalog.entityTypes.map((choice) => choice.id),
  ["person"],
  "disabled entity types stay out of the mapping catalog",
);
assert.deepEqual(
  catalog.fields.map((choice) => choice.key),
  ["summary"],
  "disabled fields stay out of the mapping catalog",
);
assert.doesNotMatch(catalog.fingerprint, /disabled\.module/);

assert.equal(importFolderFor("People/Heroes/Alice.md"), "People/Heroes");
assert.equal(importFolderFor("Root.md"), "");

const enabled = manifest("enabled.module", true, "person", "summary");
enabled.schemas[0].fields.push({
  key: "references",
  label: "References",
  type: "relationship",
  relationshipType: "references",
  targetEntityTypes: ["person"],
});
const relationships = buildExternalImportMappingCatalog([enabled]);
assert.equal(
  relationships.fields.some((choice) => choice.key === "references"),
  false,
  "relationship fields stay out of ordinary field mappings",
);
assert.deepEqual(
  relationships.relationships.map((choice) => choice.id),
  ["references"],
  "relationship mappings use the manifest relationship type",
);

console.log("external import mapping checks passed");
