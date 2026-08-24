import assert from "node:assert/strict";

import {
  isStructuredFieldValue,
  restoreStructuredFieldValue,
  shouldPersistFieldValue,
} from "../src/lib/fields/persistence.ts";

const physicalDescriptor = {
  schemaVersion: 1,
  provider: {
    id: "daena-physical",
    adapterVersion: 2,
    sourceFormat: "physical-world-v2",
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

console.log("structured map field persistence checks passed");
