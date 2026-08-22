import assert from "node:assert/strict";
import {
  buildFieldContributions,
  discoverTimelineFieldSpecs,
  timelineDateAnchor,
} from "../packages/modules/timeline/src/projection.ts";

const loreManifest = {
  id: "daena.lore",
  schemas: [
    {
      namespace: "lore",
      entityTypes: ["person", "artifact"],
      fields: [
        {
          key: "birth",
          label: "Birth",
          type: "date",
          entityTypes: ["person"],
          shared: true,
          timeline: { role: "start", group: "life", label: "Born", layer: "lifelines" },
        },
        {
          key: "death",
          label: "Death",
          type: "date",
          entityTypes: ["person"],
          shared: true,
          timeline: { role: "end", group: "life", label: "Died", layer: "lifelines" },
        },
        { key: "discovered", label: "Discovered", type: "date", shared: true },
        { key: "privateDate", label: "Private", type: "date", shared: false },
      ],
    },
  ],
};

const specs = discoverTimelineFieldSpecs([loreManifest], "daena.timeline");
assert.deepEqual(
  specs.map(({ key, role, layer }) => ({ key, role, layer })),
  [
    { key: "discovered", role: "point", layer: "dates" },
    { key: "birth", role: "start", layer: "lifelines" },
    { key: "death", role: "end", layer: "lifelines" },
  ],
);

const person = { id: "person-1", name: "Aven", type: "person" };
const record = (key, value) => ({ entityId: person.id, namespace: "lore", key, value, revision: "1" });

const birthOnly = buildFieldContributions(person, [record("birth", "42")], specs);
assert.equal(birthOnly.length, 1);
assert.equal(birthOnly[0].layer, "lifelines");
assert.equal(birthOnly[0].pointRole, "start");
assert.equal(birthOnly[0].startLabel, "Born");
assert.equal(birthOnly[0].endValue, undefined);

const deathOnly = buildFieldContributions(person, [record("death", "81-4")], specs);
assert.equal(deathOnly.length, 1);
assert.equal(deathOnly[0].pointRole, "end");
assert.equal(deathOnly[0].startLabel, "Died");
assert.equal(deathOnly[0].endValue, undefined);

const fullLife = buildFieldContributions(person, [record("birth", "42"), record("death", "81-4")], specs);
assert.equal(fullLife.length, 1);
assert.equal(fullLife[0].pointRole, undefined);
assert.equal(fullLife[0].startValue, "42");
assert.equal(fullLife[0].endValue, "81-4");
assert.equal(fullLife[0].startLabel, "Born");
assert.equal(fullLife[0].endLabel, "Died");

const withProjectDate = buildFieldContributions(
  person,
  [record("birth", "42"), record("discovered", "60-2-3"), record("privateDate", "70")],
  specs,
);
assert.equal(
  withProjectDate.some((item) => item.layer === "dates" && item.startLabel === "Discovered"),
  true,
);
assert.equal(
  withProjectDate.some((item) => item.startLabel === "Private"),
  false,
);

const yearOnly = timelineDateAnchor({ calendar: "calendar-1", year: 42, precision: "year" });
assert.equal(yearOnly?.date.getUTCFullYear(), 42);
assert.equal(yearOnly?.date.getUTCMonth(), 0);
assert.equal(yearOnly?.date.getUTCDate(), 1);
assert.equal(yearOnly?.source.calendar, "calendar-1");
assert.equal(yearOnly?.source.precision, "year");

console.log("timeline field projection fixtures passed");
