import assert from "node:assert/strict";
import { filterWorkspaceEntities, WRITING_VIEW_TYPES } from "../src/lib/modules/workspace.ts";

const ENTITY_TYPES = new Set(["manuscript", "reference-page", "person"]);
const entities = [
  {
    id: "1",
    name: "The Long Road",
    entity_type: "manuscript",
    deleted: false,
    created_at: "",
    updated_at: "",
    revision: "a",
  },
  {
    id: "2",
    name: "Kerby the Cat",
    entity_type: "reference-page",
    deleted: false,
    created_at: "",
    updated_at: "",
    revision: "b",
  },
  { id: "3", name: "Marrow", entity_type: "manuscript", deleted: false, created_at: "", updated_at: "", revision: "c" },
  { id: "4", name: "Aldric", entity_type: "person", deleted: false, created_at: "", updated_at: "", revision: "d" },
];

assert.deepEqual(
  filterWorkspaceEntities({ entityTypes: ENTITY_TYPES, entities, query: "", writingView: "manuscripts" }).map(
    (e) => e.id,
  ),
  ["1", "3"],
  "manuscripts tab shows only manuscripts",
);
assert.deepEqual(
  filterWorkspaceEntities({ entityTypes: ENTITY_TYPES, entities, query: "", writingView: "reference" }).map(
    (e) => e.id,
  ),
  ["2"],
  "reference tab shows only reference pages",
);
assert.deepEqual(
  filterWorkspaceEntities({ entityTypes: ENTITY_TYPES, entities, query: "cat", writingView: "reference" }).map(
    (e) => e.id,
  ),
  ["2"],
  "search matches within the active tab",
);
assert.deepEqual(
  filterWorkspaceEntities({ entityTypes: ENTITY_TYPES, entities, query: "cat", writingView: "manuscripts" }).length,
  0,
  "search cannot cross into the inactive tab",
);
assert.deepEqual(
  filterWorkspaceEntities({ entityTypes: new Set(), entities, query: "", writingView: "manuscripts" }).length,
  0,
  "an empty manifest declares no entity types even when a writing tab is active",
);
assert.deepEqual(
  filterWorkspaceEntities({ entityTypes: new Set(["person"]), entities, query: "", writingView: "reference" }).length,
  0,
  "writing tabs cannot surface types the manifest does not declare",
);
assert.deepEqual(
  WRITING_VIEW_TYPES.reference,
  ["reference-page"],
  "writing view mapping matches the module manifest entity types",
);
console.log("writing tabs fixtures passed");
