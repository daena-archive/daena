import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  isStructuredFieldValue,
  restoreStructuredFieldValue,
  shouldPersistFieldValue,
} from "../src/lib/fields/persistence.ts";
import { isMapsProviderField, MAPS_PROVIDER_FIELD_KEYS } from "../src/lib/maps/provider-fields.ts";

const physicalDescriptor = {
  schemaVersion: 1,
  provider: {
    id: "daena-physical",
    adapterVersion: 1,
    sourceFormat: "physical-world-v1",
  },
  sourceAssetId: "caa3c40c-480a-4d85-952f-b2dfdab9d289",
  authoredSourceAssetId: "11468005-3cea-4c47-a072-59ed65bf5017",
  previewAssetId: null,
  defaultView: { center: [0.5, 0.5], zoom: 1 },
};

assert.equal(isStructuredFieldValue(physicalDescriptor), true);
assert.equal(shouldPersistFieldValue("", false), false, "absent optional map fields must stay absent");
assert.equal(shouldPersistFieldValue("", true), true, "an existing field may be explicitly cleared");
assert.equal(shouldPersistFieldValue(JSON.stringify(physicalDescriptor), true), true);
assert.deepEqual(
  restoreStructuredFieldValue(JSON.stringify(physicalDescriptor), true, "Map descriptor"),
  physicalDescriptor,
  "an object-valued map descriptor must not be persisted as a JSON string",
);
assert.equal(
  restoreStructuredFieldValue(JSON.stringify(physicalDescriptor), false, "Notes"),
  JSON.stringify(physicalDescriptor),
  "ordinary text fields must retain JSON-looking text",
);
assert.throws(
  () => restoreStructuredFieldValue("{invalid", true, "Map descriptor"),
  /Map descriptor must contain valid JSON\./,
);
assert.throws(
  () => restoreStructuredFieldValue('"scalar"', true, "Map descriptor"),
  /Map descriptor must contain a JSON object or array\./,
);

assert.equal(isMapsProviderField({ key: "map", type: "text" }), true);
assert.equal(isMapsProviderField({ key: "layers", type: "text" }), true);
assert.equal(isMapsProviderField({ key: "detailMap", type: "relationship" }), false);
assert.equal(isMapsProviderField({ key: "genre", type: "text" }), false);

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const mapsManifest = JSON.parse(readFileSync(join(root, "packages/modules/maps/manifest.json"), "utf8"));
const mapsTextKeys = mapsManifest.schemas[0].fields
  .filter((field) => field.type !== "relationship")
  .map((field) => field.key)
  .sort();
assert.deepEqual(mapsTextKeys, [...MAPS_PROVIDER_FIELD_KEYS].sort());

const shell = readFileSync(join(root, "src/routes/+page.svelte"), "utf8");
assert.match(shell, /isMapsProviderField/);
assert.match(shell, /propertyDefinitions\(\)\.length > 0/);
assert.match(shell, /hasDetailsSection/);
assert.match(shell, /\{#if hasDetailsSection\(\)\}/);
assert.match(
  shell,
  /function emptyInspectorDefinitions\(\)[\s\S]*?isMapsProviderField\(definition\)/,
  "AI fill must not treat Maps provider JSON as empty Properties",
);

console.log("structured map field persistence checks passed");
