import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseCalendarDate, serializeCalendarDate, GREGORIAN_CALENDAR_ID } from "../src/lib/date.ts";
import { fieldAppliesToEntity } from "../src/lib/modules/fields.ts";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const lore = JSON.parse(readFileSync(join(root, "packages/modules/lore/manifest.json"), "utf8"));
const timeline = JSON.parse(readFileSync(join(root, "packages/modules/timeline/manifest.json"), "utf8"));

const loreFields = lore.schemas[0].fields;
const byKey = Object.fromEntries(loreFields.map((field) => [field.key, field]));
const timelineTypes = new Set(timeline.schemas[0].entityTypes);
const loreTypes = new Set(lore.schemas[0].entityTypes);
const withTimeline = new Set([...loreTypes, ...timelineTypes]);

assert.equal(byKey.birth.type, "date");
assert.deepEqual(byKey.birth.entityTypes, ["person"]);
assert.equal(byKey.death.type, "date");
assert.equal(byKey.createdAt.type, "date");
assert.equal(byKey.endedAt.type, "date");
assert.equal(byKey.birth.relationshipType, undefined);
assert.equal(
  timeline.schemas[0].fields.some((field) => field.key === "calendar"),
  false,
  "calendar choice lives on the date, not an event relationship",
);

assert.equal(fieldAppliesToEntity(byKey.birth, "person", loreTypes), false, "born hides without Timeline");
assert.equal(fieldAppliesToEntity(byKey.createdAt, "artifact", loreTypes), false, "created hides without Timeline");
assert.equal(fieldAppliesToEntity(byKey.endedAt, "faction", loreTypes), false, "ended hides without Timeline");
assert.equal(fieldAppliesToEntity(byKey.origin, "person", loreTypes), true, "origin stays available without Timeline");

assert.equal(fieldAppliesToEntity(byKey.birth, "person", withTimeline), true);
assert.equal(fieldAppliesToEntity(byKey.death, "person", withTimeline), true);
assert.equal(fieldAppliesToEntity(byKey.createdAt, "artifact", withTimeline), true);
assert.equal(fieldAppliesToEntity(byKey.createdAt, "person", withTimeline), false);
assert.equal(fieldAppliesToEntity(byKey.endedAt, "concept", withTimeline), false);
assert.equal(fieldAppliesToEntity(byKey.createdAt, "concept", withTimeline), true);

const startsAt = timeline.schemas[0].fields.find((field) => field.key === "startsAt");
assert.equal(fieldAppliesToEntity(startsAt, "event", timelineTypes), true);

const gregorian = parseCalendarDate("412-3-17");
assert.equal(gregorian?.calendar, GREGORIAN_CALENDAR_ID);
assert.equal(serializeCalendarDate(gregorian), "412-3-17");

const world = parseCalendarDate({ calendar: "cal-1", year: 412, month: 3, day: 17, precision: "day" });
assert.equal(world?.calendar, "cal-1");
const stored = serializeCalendarDate(world);
assert.equal(typeof stored, "object");
assert.equal(stored.calendar, "cal-1");
assert.equal(parseCalendarDate(stored)?.year, 412);

console.log("lore–timeline date fields fixtures passed");
