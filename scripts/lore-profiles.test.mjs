import assert from "node:assert/strict";
import {
  componentHasValue,
  emptyProfile,
  formatProfileValue,
  canEditLoreProfile,
  parseProfile,
  profileValidationErrors,
} from "../src/lib/lore/profile.ts";

const liveLoreTypes = ["daena.lore:person", "daena.lore:faction", "daena.lore:species"];
assert.equal(canEditLoreProfile("daena.lore:person", liveLoreTypes), true);
assert.equal(canEditLoreProfile("daena.lore:species", liveLoreTypes), true);
assert.equal(canEditLoreProfile("daena.maps:world-map", liveLoreTypes), false);
assert.equal(canEditLoreProfile(null, liveLoreTypes), false);

const empty = emptyProfile();
assert.deepEqual(profileValidationErrors(empty), []);
assert.deepEqual(parseProfile(empty).components, []);

const authored = {
  schemaVersion: 1,
  presetOrigin: "custom",
  components: [
    {
      id: "str",
      kind: "attribute",
      name: "Strength",
      min: 0,
      max: 20,
      value: { type: "number", value: 14 },
    },
    {
      id: "hp",
      kind: "resource",
      name: "Hit Points",
      value: { type: "resource", current: 8, max: 12, unit: "hp" },
    },
    {
      id: "brave",
      kind: "trait",
      name: "Brave",
      value: { type: "text", value: "Stands first" },
    },
    {
      id: "rank",
      kind: "skill",
      name: "Swordsmanship",
      scale: ["untrained", "trained", "expert"],
      value: { type: "rank", value: "trained" },
    },
  ],
};
const parsed = parseProfile(authored);
assert.equal(parsed.components[0].value.type, "number");
assert.equal(parsed.components[0].value.value, 14);
assert.equal(formatProfileValue(parsed.components[1]), "8 / 12 hp");
assert.equal(
  formatProfileValue({
    id: "empty-hp",
    kind: "resource",
    name: "Hit Points",
    value: { type: "resource", current: null, max: null, unit: null },
  }),
  "",
);
assert.deepEqual(
  parseProfile({
    schemaVersion: 1,
    components: [
      {
        id: "rank",
        kind: "skill",
        name: " Swordsmanship ",
        scale: [" untrained ", "trained"],
        value: { type: "rank", value: "trained" },
      },
    ],
  }).components[0],
  {
    id: "rank",
    kind: "skill",
    name: "Swordsmanship",
    scale: ["untrained", "trained"],
    min: undefined,
    max: undefined,
    unit: undefined,
    value: { type: "rank", value: "trained" },
  },
);
assert.equal(componentHasValue(parsed.components[2]), true);
assert.equal(componentHasValue({ ...parsed.components[0], value: { type: "number", value: null } }), false);

assert.ok(profileValidationErrors({ ...authored, schemaVersion: 2 }).length);
assert.ok(profileValidationErrors({ ...authored, changes: [] }).length);
assert.ok(
  profileValidationErrors({
    ...authored,
    components: [authored.components[0], { ...authored.components[0], name: "Copy" }],
  }).some((error) => error.includes("Duplicate")),
);
assert.ok(
  profileValidationErrors({
    schemaVersion: 1,
    components: [
      {
        id: "rank",
        kind: "skill",
        name: "Swordsmanship",
        scale: ["untrained", "trained"],
        value: { type: "rank", value: "expert" },
      },
    ],
  }).some((error) => error.includes("not on its scale")),
);
assert.ok(
  profileValidationErrors({
    schemaVersion: 1,
    components: [
      {
        id: "derived",
        kind: "derived",
        name: "Modifier",
        value: { type: "number", value: 2 },
      },
    ],
  }).some((error) => error.includes("unknown kind")),
);
assert.ok(
  profileValidationErrors({
    schemaVersion: 1,
    components: [
      {
        id: "str",
        kind: "attribute",
        name: "Strength",
        min: 10,
        max: 5,
        value: { type: "number", value: 8 },
      },
    ],
  }).some((error) => error.includes("min cannot exceed max")),
);
assert.ok(
  profileValidationErrors({
    schemaVersion: 1,
    components: [
      {
        id: "hp",
        kind: "resource",
        name: "Hit Points",
        value: { type: "resource", current: 12, max: 8, unit: "hp" },
      },
    ],
  }).some((error) => error.includes("current cannot exceed max")),
);
assert.ok(
  profileValidationErrors({
    schemaVersion: 1,
    components: [
      {
        id: "rank",
        kind: "skill",
        name: "Swordsmanship",
        scale: ["trained", "trained"],
        value: { type: "rank", value: "trained" },
      },
    ],
  }).some((error) => error.includes("unique")),
);
assert.equal(
  formatProfileValue({
    id: "brave",
    kind: "trait",
    name: "Brave",
    value: { type: "text", value: null },
  }),
  "",
);
assert.equal(parseProfile({ schemaVersion: 1, components: [] }).presetOrigin, undefined);
assert.equal(parseProfile({ schemaVersion: 1, presetOrigin: "custom", components: [] }).presetOrigin, "custom");

console.log("lore profiles passed");
