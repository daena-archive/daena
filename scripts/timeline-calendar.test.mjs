import assert from "node:assert/strict";
import {
  applyYearPreset,
  calendarDateToParts,
  calendarHasStructure,
  calendarSummary,
  computedYearLength,
  daysInCalendarMonth,
  DATE_FORMAT_GUIDE,
  DEFAULT_CALENDAR_ID,
  DEFAULT_DATE_FORMAT,
  emptyCalendarDefinition,
  formatCalendarParts,
  formatWithCalendar,
  gregorianPresetDefinition,
  isDefaultCalendarId,
  matchYearPreset,
  normalizeCalendarDefinition,
  partsToCalendarDate,
  validateCalendarDefinition,
} from "../packages/modules/timeline/src/calendar.ts";

assert.equal(isDefaultCalendarId(undefined), true);
assert.equal(isDefaultCalendarId(DEFAULT_CALENDAR_ID), true);
assert.equal(isDefaultCalendarId("abc"), false);

const empty = emptyCalendarDefinition();
assert.equal(calendarHasStructure(empty), false);
assert.equal(computedYearLength(empty), null);
assert.equal(formatWithCalendar("842-3-17", empty), "842-3-17");
assert.ok(validateCalendarDefinition(empty).some((issue) => issue.level === "warning"));

const preset = gregorianPresetDefinition();
assert.equal(computedYearLength(preset), 365);
assert.equal(preset.months.length, 12);
assert.equal(preset.weekdays.length, 7);
assert.equal(matchYearPreset(preset), "gregorian");
assert.equal(daysInCalendarMonth(preset, 842, 1), 31);
assert.equal(daysInCalendarMonth(null, 2024, 2), 29);

const equalMonths = applyYearPreset("twelve-30", empty);
assert.equal(matchYearPreset(equalMonths), "twelve-30");
assert.equal(computedYearLength(equalMonths), 360);

const named = normalizeCalendarDefinition({
  months: [
    { name: "Deepwinter", days: 38 },
    { name: "Thaw", days: 31 },
    { name: "Bloom", days: 34 },
  ],
  weekdays: [{ name: "Dawn" }, { name: "Flame" }, { name: "River" }, { name: "Stone" }, { name: "Night" }],
  seasons: [{ name: "Long Winter", startMonth: 1, startDay: 1, endMonth: 2, endDay: 10 }],
  startingYear: 1,
  epoch: { year: 1, month: 1, day: 1 },
  dateFormat: "YYYY/MM/DD",
});
assert.equal(computedYearLength(named), 103);
assert.equal(matchYearPreset(named), "custom");
assert.ok(
  validateCalendarDefinition({ ...named, months: [{ ...named.months[0], days: 0 }] }).some(
    (issue) => issue.message === "Deepwinter must contain at least one day.",
  ),
);

const stored = partsToCalendarDate({ year: 2, month: 2, day: 1, precision: "day" }, named);
assert.equal(stored.calendar, "gregorian");
const roundTrip = calendarDateToParts(stored, named);
assert.equal(roundTrip?.year, 2);
assert.equal(roundTrip?.month, 2);
assert.equal(roundTrip?.day, 1);
assert.equal(formatWithCalendar(stored, named), "2/02/01");
assert.equal(
  formatCalendarParts({ year: 842, month: 2, day: 3, precision: "day" }, { ...named, dateFormat: "D MMMM YYYY" }),
  "3 Thaw 842",
);
assert.equal(calendarSummary(named).includes("103 days"), true);
assert.equal(calendarSummary(named).includes("YYYY/MM/DD"), true);

const yearOnly = partsToCalendarDate({ year: 842, precision: "year" }, named);
assert.equal(yearOnly.precision, "year");
assert.equal(calendarDateToParts(yearOnly, named)?.precision, "year");

const shifted = normalizeCalendarDefinition({
  months: [
    { name: "Frostwane", days: 30 },
    { name: "Thaw", days: 30 },
  ],
  startingYear: 1,
  epoch: { year: 1000, month: 1, day: 1 },
  dateFormat: DEFAULT_DATE_FORMAT,
});
const afterEpoch = partsToCalendarDate({ year: 1, month: 1, day: 1, precision: "day" }, shifted);
assert.equal(afterEpoch.year, 1000);
assert.equal(afterEpoch.month, 1);
assert.equal(afterEpoch.day, 1);

assert.equal("primary" in normalizeCalendarDefinition({ primary: true, months: [] }), false);
assert.deepEqual(
  DATE_FORMAT_GUIDE.map((item) => item.token),
  ["YYYY", "YY", "MMMM", "MMM", "MM", "M", "DD", "D", "WWWW", "WWW", "SSSS"],
);

console.log("timeline calendar fixtures passed");
