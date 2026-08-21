import assert from "node:assert/strict";
import { filterWorkspaceEntities, TIMELINE_VIEW_TYPES } from "../src/lib/modules/workspace.ts";

const ENTITY_TYPES = new Set(["event", "encounter", "era", "calendar", "person"]);
const entities = [
  {
    id: "1",
    name: "Battle of Iradan",
    entity_type: "event",
    deleted: false,
    created_at: "",
    updated_at: "",
    revision: "a",
  },
  { id: "2", name: "Imperial Era", entity_type: "era", deleted: false, created_at: "", updated_at: "", revision: "b" },
  {
    id: "3",
    name: "Temple Calendar",
    entity_type: "calendar",
    deleted: false,
    created_at: "",
    updated_at: "",
    revision: "c",
  },
  { id: "4", name: "Council", entity_type: "encounter", deleted: false, created_at: "", updated_at: "", revision: "d" },
  { id: "5", name: "Aldric", entity_type: "person", deleted: false, created_at: "", updated_at: "", revision: "e" },
];

assert.deepEqual(
  filterWorkspaceEntities({ entityTypes: ENTITY_TYPES, entities, query: "", timelineView: "events" }).map((e) => e.id),
  ["1", "4"],
  "events tab shows events and encounters",
);
assert.deepEqual(
  filterWorkspaceEntities({ entityTypes: ENTITY_TYPES, entities, query: "", timelineView: "eras" }).map((e) => e.id),
  ["2"],
  "eras tab shows only eras",
);
assert.deepEqual(
  filterWorkspaceEntities({ entityTypes: ENTITY_TYPES, entities, query: "", timelineView: "calendars" }).map(
    (e) => e.id,
  ),
  ["3"],
  "calendars tab shows only calendars",
);
assert.deepEqual(
  filterWorkspaceEntities({ entityTypes: ENTITY_TYPES, entities, query: "temple", timelineView: "calendars" }).map(
    (e) => e.id,
  ),
  ["3"],
  "search matches within the active timeline tab",
);
assert.equal(
  filterWorkspaceEntities({ entityTypes: ENTITY_TYPES, entities, query: "temple", timelineView: "events" }).length,
  0,
  "search cannot cross into the inactive timeline tab",
);
assert.deepEqual(TIMELINE_VIEW_TYPES.eras, ["era"]);
assert.deepEqual(TIMELINE_VIEW_TYPES.calendars, ["calendar"]);
console.log("timeline tabs fixtures passed");
