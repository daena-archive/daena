import assert from "node:assert/strict";
import {
  collectionEntityTypes,
  collectionTabForEntityType,
  workspaceCollectionTabs,
  workspaceSectionViewNav,
} from "../src/lib/modules/workspace.ts";

const TYPES = [
  { id: "daena.writing:manuscript", name: "Manuscript" },
  { id: "daena.writing:reference-page", name: "Reference page" },
  { id: "daena.writing:codex", name: "Codex" },
];

const tabs = workspaceCollectionTabs("writing", TYPES);
assert.deepEqual(
  tabs.map((tab) => tab.id),
  ["manuscripts", "reference", "daena.writing:codex"],
  "custom writing types get their own tab after builtins",
);

const entityIds = new Set(TYPES.map((type) => type.id));
assert.deepEqual(
  collectionEntityTypes({
    entityTypes: entityIds,
    tabEntityTypes: tabs.find((tab) => tab.id === "manuscripts")?.entityTypes,
  }),
  ["daena.writing:manuscript"],
  "manuscripts tab sends only manuscript types to the backend",
);
assert.deepEqual(
  collectionEntityTypes({
    entityTypes: entityIds,
    tabEntityTypes: tabs.find((tab) => tab.id === "reference")?.entityTypes,
  }),
  ["daena.writing:reference-page"],
  "reference tab sends only reference-page types to the backend",
);
assert.deepEqual(
  collectionEntityTypes({ entityTypes: new Set(), tabEntityTypes: ["daena.writing:manuscript"] }),
  [],
  "an empty manifest declares no entity types",
);
assert.deepEqual(
  collectionEntityTypes({
    entityTypes: new Set(["daena.lore:person"]),
    tabEntityTypes: ["daena.writing:reference-page"],
  }),
  [],
  "writing tabs cannot request types the manifest does not declare",
);

assert.equal(collectionTabForEntityType(tabs, "daena.writing:codex")?.id, "daena.writing:codex");
assert.deepEqual(
  workspaceSectionViewNav("writing", TYPES).map((view) => view.id),
  ["manuscripts", "reference", "daena.writing:codex"],
);
assert.deepEqual(
  workspaceSectionViewNav("lore", TYPES).map((view) => view.id),
  ["library", "wiki", "graph"],
);
assert.deepEqual(
  workspaceSectionViewNav("houses", TYPES).map((view) => view.id),
  ["houses", "tree"],
);

console.log("writing tabs fixtures passed");
