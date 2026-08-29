import assert from "node:assert/strict";
import {
  collectionEntityTypes,
  collectionTabForEntityType,
  workspaceCollectionTabs,
  workspaceSectionViewNav,
} from "../src/lib/modules/workspace.ts";

const TYPES = [
  { id: "daena.timeline:event", name: "Timeline event" },
  { id: "daena.timeline:encounter", name: "Encounter" },
  { id: "daena.timeline:era", name: "Era" },
  { id: "daena.timeline:calendar", name: "Calendar" },
  { id: "daena.timeline:war", name: "War" },
];

const tabs = workspaceCollectionTabs("timeline", TYPES);
assert.deepEqual(
  tabs.map((tab) => tab.id),
  ["events", "calendars", "daena.timeline:war"],
  "custom timeline types get their own tab after builtins",
);
assert.deepEqual(tabs.find((tab) => tab.id === "events")?.entityTypes, [
  "daena.timeline:event",
  "daena.timeline:encounter",
  "daena.timeline:era",
]);
assert.deepEqual(tabs.find((tab) => tab.id === "daena.timeline:war")?.entityTypes, ["daena.timeline:war"]);

const entityIds = new Set(TYPES.map((type) => type.id));
assert.deepEqual(
  collectionEntityTypes({
    entityTypes: entityIds,
    tabEntityTypes: tabs.find((tab) => tab.id === "events")?.entityTypes,
  }),
  ["daena.timeline:encounter", "daena.timeline:era", "daena.timeline:event"],
  "events tab sends event, encounter, and era types to the backend",
);
assert.deepEqual(
  collectionEntityTypes({
    entityTypes: entityIds,
    tabEntityTypes: tabs.find((tab) => tab.id === "daena.timeline:war")?.entityTypes,
  }),
  ["daena.timeline:war"],
  "custom tabs query only that type",
);

const withoutEra = TYPES.filter((type) => type.id !== "daena.timeline:era");
assert.deepEqual(
  workspaceCollectionTabs("timeline", withoutEra).find((tab) => tab.id === "events")?.entityTypes,
  ["daena.timeline:event", "daena.timeline:encounter"],
  "disabled era types drop out of the events tab",
);

assert.equal(collectionTabForEntityType(tabs, "daena.timeline:encounter")?.id, "events");
assert.equal(collectionTabForEntityType(tabs, "daena.timeline:era")?.id, "events");
assert.equal(collectionTabForEntityType(tabs, "daena.timeline:war")?.id, "daena.timeline:war");

assert.deepEqual(
  workspaceSectionViewNav("timeline", TYPES).map((view) => view.id),
  ["timeline", "events", "calendars", "daena.timeline:war"],
);

console.log("timeline tabs fixtures passed");
