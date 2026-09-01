import assert from "node:assert/strict";
import {
  belongsToEraScope,
  chronologyWarnings,
  dateOutsideEraBounds,
  firstEraCalendarId,
  isChronologyPropertyField,
  isEraRelationshipField,
} from "../src/lib/modules/chronology.ts";
import { normalizeCalendarDefinition, partsToCalendarDate } from "../packages/modules/timeline/src/calendar.ts";

const customCalendar = normalizeCalendarDefinition({
  months: Array.from({ length: 12 }, (_, index) => ({ name: `Month ${index + 1}`, days: 30 })),
  startingYear: 1,
});
// Stored dates keep Gregorian-anchored digits, exactly like the date editor writes them;
// era bounds and event dates must go through the same conversion pipeline here.
const storedCustom = (parts) => partsToCalendarDate(parts, customCalendar);

assert.equal(isEraRelationshipField({ key: "era", type: "relationship", relationshipType: "during" }), true);
assert.equal(isEraRelationshipField({ key: "location", type: "relationship", relationshipType: "occurred_at" }), false);
assert.equal(isChronologyPropertyField({ key: "startsAt", type: "date" }), true);
assert.equal(isChronologyPropertyField({ key: "description", type: "text" }), false);

assert.equal(dateOutsideEraBounds("412-3-17", "400-1-1", "420-1-1"), false);
assert.equal(dateOutsideEraBounds("390-1-1", "400-1-1", "420-1-1"), true);
assert.equal(dateOutsideEraBounds("421-1-1", "400-1-1", "420-1-1"), true);
assert.equal(dateOutsideEraBounds("412-3-17", null, null), false);
assert.equal(dateOutsideEraBounds(null, "400-1-1", "420-1-1"), false);

// BCE events anchor before the common era, so era bounds compare correctly.
assert.equal(dateOutsideEraBounds("-44-1-1", "1-1-1", "200-1-1"), true);
assert.equal(dateOutsideEraBounds("-44-1-1", "-100-1-1", "-1-1-1"), false);

const customEraStart = storedCustom({ year: 2, month: 1, day: 1, precision: "day" });
const customEraEnd = storedCustom({ year: 2, month: 3, day: 1, precision: "day" });
assert.equal(
  dateOutsideEraBounds(
    storedCustom({ year: 2, month: 2, day: 1, precision: "day" }),
    customEraStart,
    customEraEnd,
    customCalendar,
  ),
  false,
);
assert.equal(
  dateOutsideEraBounds(
    storedCustom({ year: 2, month: 4, day: 1, precision: "day" }),
    customEraStart,
    customEraEnd,
    customCalendar,
  ),
  true,
);
// A mid-era date must not be flagged: comparing raw stored (Gregorian) digits as if they
// were custom month/day parts produced false "outside era" warnings.
assert.equal(
  dateOutsideEraBounds(
    storedCustom({ year: 2, month: 2, day: 15, precision: "day" }),
    customEraStart,
    customEraEnd,
    customCalendar,
  ),
  false,
);
// Plain Gregorian dates compare against custom-calendar era bounds through the same mapping.
assert.equal(dateOutsideEraBounds("2-1-15", customEraStart, customEraEnd, customCalendar), false);
assert.equal(dateOutsideEraBounds("1-6-1", customEraStart, customEraEnd, customCalendar), true);

assert.deepEqual(
  chronologyWarnings(
    [{ label: "Starts", value: "390-1-1" }],
    [{ id: "era-1", name: "Third Age", start: "400-1-1", end: "420-1-1", calendarIds: [] }],
  ),
  ["Starts falls outside Third Age."],
);

assert.deepEqual(
  chronologyWarnings(
    [{ label: "Starts", value: storedCustom({ year: 2, month: 2, day: 15, precision: "day" }) }],
    [{ id: "era-cal", name: "Custom Age", start: customEraStart, end: customEraEnd, calendarIds: ["cal-1"] }],
    { "cal-1": customCalendar },
  ),
  [],
  "mid-era custom-calendar dates must not warn",
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
