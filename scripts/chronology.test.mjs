import assert from "node:assert/strict";
import {
  belongsToEraScope,
  chronologyWarnings,
  dateOutsideEraBounds,
  firstEraCalendarId,
  isChronologyPropertyField,
  isEraRelationshipField,
} from "../src/lib/modules/chronology.ts";

assert.equal(isEraRelationshipField({ key: "era", type: "relationship", relationshipType: "during" }), true);
assert.equal(isEraRelationshipField({ key: "location", type: "relationship", relationshipType: "occurred_at" }), false);
assert.equal(isChronologyPropertyField({ key: "startsAt", type: "date" }), true);
assert.equal(isChronologyPropertyField({ key: "description", type: "text" }), false);

assert.equal(dateOutsideEraBounds("412-3-17", "400-1-1", "420-1-1"), false);
assert.equal(dateOutsideEraBounds("390-1-1", "400-1-1", "420-1-1"), true);
assert.equal(dateOutsideEraBounds("421-1-1", "400-1-1", "420-1-1"), true);
assert.equal(dateOutsideEraBounds("412-3-17", null, null), false);
assert.equal(dateOutsideEraBounds(null, "400-1-1", "420-1-1"), false);

assert.deepEqual(
  chronologyWarnings(
    [{ label: "Starts", value: "390-1-1" }],
    [{ id: "era-1", name: "Third Age", start: "400-1-1", end: "420-1-1", calendarIds: [] }],
  ),
  ["Starts falls outside Third Age."],
);

assert.equal(
  belongsToEraScope({
    eraIds: ["era-1"],
    eraId: "era-1",
    startValue: "390-1-1",
    eraStart: "400-1-1",
    eraEnd: "420-1-1",
  }),
  true,
  "explicit during membership wins even outside bounds",
);
assert.equal(
  belongsToEraScope({
    eraIds: [],
    eraId: "era-1",
    startValue: "412-3-17",
    eraStart: "400-1-1",
    eraEnd: "420-1-1",
  }),
  true,
  "dated events in bounds are in scope without a during link",
);
assert.equal(
  belongsToEraScope({ eraIds: [], eraId: "era-1", startValue: "390-1-1", eraStart: "400-1-1", eraEnd: "420-1-1" }),
  false,
);

assert.equal(
  firstEraCalendarId([
    { id: "a", name: "A", start: null, end: null, calendarIds: [] },
    { id: "b", name: "B", start: null, end: null, calendarIds: ["cal-1"] },
  ]),
  "cal-1",
);

console.log("chronology fixtures passed");
