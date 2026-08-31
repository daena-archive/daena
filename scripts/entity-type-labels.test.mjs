import assert from "node:assert/strict";
import {
  buildTypeLabelMap,
  createEntityTypeLabelResolver,
  humanizeTypeId,
  resolveEntityTypeLabel,
} from "../packages/module-api/src/entityTypeLabels.ts";

assert.equal(humanizeTypeId("daena.timeline:event"), "Event");
assert.equal(humanizeTypeId("relative_year"), "Relative Year");
assert.equal(humanizeTypeId("camelCaseType"), "Camel Case Type");

const timeline = {
  id: "daena.timeline",
  schemas: [
    {
      entityTypes: [
        { id: "event", name: "Timeline event" },
        { id: "era", name: "Era" },
      ],
    },
  ],
};
const lore = {
  id: "daena.lore",
  schemas: [{ entityTypes: [{ id: "person", name: "Person" }] }],
};
const overlay = {
  id: "daena.timeline",
  schemas: [
    {
      entityTypes: [
        { id: "event", name: "Chronicle" },
        { id: "daena.timeline:war", name: "War" },
      ],
    },
  ],
};

const labels = buildTypeLabelMap([timeline, lore, overlay]);
assert.equal(resolveEntityTypeLabel("daena.timeline:event", labels), "Chronicle");
assert.equal(resolveEntityTypeLabel("event", labels), "Chronicle");
assert.equal(resolveEntityTypeLabel("daena.timeline:war", labels), "War");
assert.equal(resolveEntityTypeLabel("daena.lore:person", labels), "Person");
assert.equal(resolveEntityTypeLabel("daena.lore:faction", labels), "Faction");
assert.equal(resolveEntityTypeLabel(null, labels), "Unknown type");

const typeLabel = createEntityTypeLabelResolver([timeline]);
assert.equal(typeLabel("daena.timeline:era"), "Era");

console.log("entity type labels passed");
