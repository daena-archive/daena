import assert from "node:assert/strict";
import { collectionEntityTypes, TIMELINE_VIEW_TYPES } from "../src/lib/modules/workspace.ts";

const ENTITY_TYPES = new Set(["event", "encounter", "era", "calendar", "person"]);

assert.deepEqual(
  collectionEntityTypes({ entityTypes: ENTITY_TYPES, timelineView: "events" }),
  ["encounter", "event"],
  "events tab sends event and encounter types to the backend",
);
assert.deepEqual(
  collectionEntityTypes({ entityTypes: ENTITY_TYPES, timelineView: "eras" }),
  ["era"],
  "eras tab sends only era types to the backend",
);
assert.deepEqual(
  collectionEntityTypes({ entityTypes: ENTITY_TYPES, timelineView: "calendars" }),
  ["calendar"],
  "calendars tab sends only calendar types to the backend",
);
assert.deepEqual(TIMELINE_VIEW_TYPES.eras, ["era"]);
assert.deepEqual(TIMELINE_VIEW_TYPES.calendars, ["calendar"]);
console.log("timeline tabs fixtures passed");
