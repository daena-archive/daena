import assert from "node:assert/strict";
import { hasMeaningfulDateValue, isEmptyFieldValue, isSentinelDateValue } from "../src/lib/fields/persistence.ts";
import {
  compareCalendarDates,
  parseCalendarDate,
  serializeCalendarDate,
  signedGregorianYear,
} from "../src/lib/date.ts";
import { patchCalendarDate } from "../src/lib/date/dateField.ts";
import { calendarDateToParts } from "../packages/modules/timeline/src/calendar.ts";

assert.equal(isSentinelDateValue("1-1"), true);
assert.equal(isSentinelDateValue("1-1-1"), true);
assert.equal(isSentinelDateValue("1"), false);
assert.equal(isSentinelDateValue("842"), false);
assert.equal(isSentinelDateValue("842-3-17"), false);
assert.equal(isSentinelDateValue({ year: 1, month: 1, day: 1, precision: "day" }), true);
assert.equal(isSentinelDateValue({ year: 1, precision: "year" }), false);
assert.equal(hasMeaningfulDateValue("1"), true);
assert.equal(hasMeaningfulDateValue("1-1-1"), false);
assert.equal(hasMeaningfulDateValue("842-3"), true);
assert.equal(hasMeaningfulDateValue({ year: 842, precision: "year", calendar: "gregorian" }), true);

assert.equal(isEmptyFieldValue(""), true);
assert.equal(isEmptyFieldValue("  "), true);
assert.equal(isEmptyFieldValue("842"), false);

const bceParsed = parseCalendarDate("-44-3-15");
assert.equal(bceParsed?.era, "BCE");
assert.equal(bceParsed?.year, 44);
assert.equal(serializeCalendarDate(bceParsed), "-44-3-15");
assert.equal(signedGregorianYear(bceParsed), -44);
assert.equal(Math.sign(compareCalendarDates("-44", "44")), -1);
assert.equal(Math.sign(compareCalendarDates("44", "-44")), 1);

// BCE dates are meaningful and editable end to end: not sentinels, keep their sign in parts,
// and editing another part must not flip the era.
assert.equal(isSentinelDateValue("-1-1-1"), false);
assert.equal(hasMeaningfulDateValue("-1-1-1"), true);
assert.equal(calendarDateToParts(parseCalendarDate("-44-3-15"), null)?.year, -44);
assert.equal(patchCalendarDate("-44-3-15", { month: 5 }, null, "gregorian"), "-44-5-15");
assert.equal(patchCalendarDate("-44", { year: -50 }, null, "gregorian"), "-50");

assert.equal(patchCalendarDate("", { year: 842 }, null, "gregorian"), "842");
assert.equal(patchCalendarDate("842", { month: 3 }, null, "gregorian"), "842-3");
assert.equal(patchCalendarDate("", { month: 3 }, null, "gregorian"), null);

console.log("date field persistence checks passed");
