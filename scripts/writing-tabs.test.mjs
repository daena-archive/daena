import assert from "node:assert/strict";
import { collectionEntityTypes, WRITING_VIEW_TYPES } from "../src/lib/modules/workspace.ts";

const ENTITY_TYPES = new Set(["manuscript", "reference-page", "person"]);

assert.deepEqual(
  collectionEntityTypes({ entityTypes: ENTITY_TYPES, writingView: "manuscripts" }),
  ["manuscript"],
  "manuscripts tab sends only manuscript types to the backend",
);
assert.deepEqual(
  collectionEntityTypes({ entityTypes: ENTITY_TYPES, writingView: "reference" }),
  ["reference-page"],
  "reference tab sends only reference-page types to the backend",
);
assert.deepEqual(
  collectionEntityTypes({ entityTypes: new Set(), writingView: "manuscripts" }),
  [],
  "an empty manifest declares no entity types",
);
assert.deepEqual(
  collectionEntityTypes({ entityTypes: new Set(["person"]), writingView: "reference" }),
  [],
  "writing tabs cannot request types the manifest does not declare",
);
assert.deepEqual(WRITING_VIEW_TYPES.reference, ["reference-page"]);
console.log("writing tabs fixtures passed");
