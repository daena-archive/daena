import assert from "node:assert/strict";
import {
  allocationRemaining,
  allocationSpent,
  componentHasValue,
  emptyProfile,
  evaluateProfile,
  formatProfileValue,
  canEditLoreProfile,
  parseProfile,
  profileCardShows,
  profileValidationErrors,
  withDerivedDependencies,
} from "../src/lib/lore/profile.ts";
import { parseFormula } from "../src/lib/lore/profileFormula.ts";
import {
  PROFILE_PRESETS,
  applyDndAncestry,
  matchingDndAncestry,
  profileFromPreset,
} from "../src/lib/lore/profilePresets.ts";

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
assert.equal(
  formatProfileValue({
    id: "unit-only-hp",
    kind: "resource",
    name: "Hit Points",
    value: { type: "resource", current: null, max: null, unit: "hp" },
  }),
  "",
);
assert.equal(
  componentHasValue({
    id: "unit-only-hp",
    kind: "resource",
    name: "Hit Points",
    value: { type: "resource", current: null, max: null, unit: "hp" },
  }),
  false,
);
assert.equal(
  profileCardShows({
    id: "brave",
    kind: "trait",
    name: "Brave",
    value: { type: "text", value: null },
  }),
  true,
);
assert.equal(
  profileCardShows({
    id: "class",
    kind: "tag",
    name: "Class",
    value: { type: "text", value: null },
  }),
  false,
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
        kind: "foo",
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
assert.equal(emptyProfile().presetOrigin, "custom");
assert.deepEqual(profileValidationErrors({ schemaVersion: 1, presetOrigin: "dnd", components: [] }), []);
assert.ok(profileValidationErrors({ schemaVersion: 1, presetOrigin: "modern", components: [] }).length);
assert.ok(profileValidationErrors({ schemaVersion: 1, allocation: { pool: -1 }, components: [] }).length);

for (const preset of PROFILE_PRESETS) {
  const copied = profileFromPreset(preset.id);
  const ids = copied.components.map((component) => component.id);
  assert.equal(new Set(ids).size, ids.length, preset.id);
  assert.deepEqual(profileValidationErrors(copied), []);
  assert.equal(copied.presetOrigin, preset.id);
}

const mutated = profileFromPreset("dnd");
mutated.components[0].name = "Renamed";
if (mutated.components[0].value.type === "number") mutated.components[0].value.value = 18;
const fresh = profileFromPreset("dnd");
assert.equal(fresh.components[0].name, "Strength");
assert.equal(fresh.components[0].value.type === "number" ? fresh.components[0].value.value : null, 10);
assert.equal(parseProfile(fresh).presetOrigin, "dnd");

const fantasy = profileFromPreset("fantasy");
assert.equal(fantasy.allocation?.pool, 40);
assert.equal(allocationSpent(fantasy), 40);
assert.equal(allocationRemaining(fantasy), 0);
assert.equal(profileFromPreset("custom").components.length, 0);
assert.equal(profileFromPreset("scifi").allocation, undefined);
assert.ok(profileFromPreset("dnd").components.some((component) => component.kind === "proficiency"));

function profileNumber(profile, id) {
  const component = profile.components.find((entry) => entry.id === id);
  return component?.value.type === "number" ? component.value.value : undefined;
}
function profileText(profile, id) {
  const component = profile.components.find((entry) => entry.id === id);
  return component?.value.type === "text" ? component.value.value : undefined;
}
const hill = applyDndAncestry(profileFromPreset("dnd"), "hill-dwarf");
assert.equal(profileText(hill, "tag-species"), "Hill Dwarf");
assert.equal(profileNumber(hill, "attribute-constitution"), 12);
assert.equal(profileNumber(hill, "attribute-wisdom"), 11);
assert.equal(profileNumber(hill, "attribute-strength"), 10);
assert.equal(profileNumber(hill, "attribute-speed"), 25);
assert.equal(matchingDndAncestry(hill)?.id, "hill-dwarf");
const wood = applyDndAncestry(hill, "wood-elf");
assert.equal(profileText(wood, "tag-species"), "Wood Elf");
assert.equal(profileNumber(wood, "attribute-constitution"), 10);
assert.equal(profileNumber(wood, "attribute-wisdom"), 11);
assert.equal(profileNumber(wood, "attribute-dexterity"), 12);
assert.equal(profileNumber(wood, "attribute-speed"), 35);
const cleared = applyDndAncestry(wood, "");
assert.equal(profileText(cleared, "tag-species"), null);
assert.equal(profileNumber(cleared, "attribute-dexterity"), 10);
assert.equal(profileNumber(cleared, "attribute-speed"), null);
const human = applyDndAncestry(profileFromPreset("dnd"), "human");
assert.equal(profileNumber(human, "attribute-strength"), 11);
assert.equal(profileNumber(human, "attribute-charisma"), 11);
assert.equal(profileNumber(human, "attribute-speed"), 30);
assert.deepEqual(applyDndAncestry(profileFromPreset("dnd"), "beholder"), profileFromPreset("dnd"));
assert.equal(
  profileCardShows(profileFromPreset("dnd").components.find((component) => component.name === "Hit Points")),
  false,
);

const parsedFormula = parseFormula("floor(({attribute-strength} - 10) / 2)");
assert.equal("error" in parsedFormula, false);
if (!("error" in parsedFormula)) {
  assert.deepEqual(parsedFormula.dependencies, ["attribute-strength"]);
}

const dnd = parseProfile(profileFromPreset("dnd"));
const dndValues = evaluateProfile(dnd);
assert.equal(dndValues.get("derived-strength-modifier"), 0);
assert.equal(
  formatProfileValue(
    dnd.components.find((component) => component.id === "derived-strength-modifier"),
    dndValues.get("derived-strength-modifier") ?? null,
  ),
  "0",
);
assert.equal(
  profileCardShows(
    dnd.components.find((component) => component.id === "derived-strength-modifier"),
    dndValues.get("derived-strength-modifier") ?? null,
  ),
  true,
);
const strong = structuredClone(dnd);
const strength = strong.components.find((component) => component.id === "attribute-strength");
if (strength?.value.type === "number") strength.value.value = 14;
assert.equal(evaluateProfile(strong).get("derived-strength-modifier"), 2);
assert.equal(strong.components.find((component) => component.id === "derived-strength-modifier")?.value.value, null);

assert.ok(
  profileValidationErrors({
    schemaVersion: 1,
    components: [
      {
        id: "mod",
        kind: "derived",
        name: "Modifier",
        formula: "floor(({missing} - 10) / 2)",
        value: { type: "number", value: null },
      },
    ],
  }).some((error) => error.includes("unknown")),
);
assert.ok(
  profileValidationErrors({
    schemaVersion: 1,
    components: [
      {
        id: "a",
        kind: "derived",
        name: "A",
        formula: "{b}",
        value: { type: "number", value: null },
      },
      {
        id: "b",
        kind: "derived",
        name: "B",
        formula: "{a}",
        value: { type: "number", value: null },
      },
    ],
  }).some((error) => error.includes("cycle")),
);

const divided = parseProfile({
  schemaVersion: 1,
  components: [
    { id: "zero", kind: "attribute", name: "Zero", value: { type: "number", value: 0 } },
    {
      id: "ratio",
      kind: "derived",
      name: "Ratio",
      formula: "10 / {zero}",
      value: { type: "number", value: null },
    },
  ],
});
assert.equal(evaluateProfile(divided).get("ratio"), null);
assert.equal(divided.components.find((component) => component.id === "zero")?.value.value, 0);

const overridden = parseProfile({
  schemaVersion: 1,
  components: [
    { id: "str", kind: "attribute", name: "Strength", value: { type: "number", value: 14 } },
    {
      id: "mod",
      kind: "derived",
      name: "Modifier",
      formula: "floor(({str} - 10) / 2)",
      override: 9,
      value: { type: "number", value: 99 },
    },
  ],
});
assert.equal(overridden.components[1].value.value, null);
assert.equal(overridden.components[1].override, 9);
assert.equal(evaluateProfile(overridden).get("mod"), 9);
assert.deepEqual(parseProfile(overridden).components[1].dependencies, ["str"]);

const copyA = profileFromPreset("dnd");
const copyB = profileFromPreset("dnd");
const depsA = copyA.components.find((component) => component.id === "derived-strength-modifier")?.dependencies;
const depsB = copyB.components.find((component) => component.id === "derived-strength-modifier")?.dependencies;
assert.ok(depsA);
assert.ok(depsB);
assert.notEqual(depsA, depsB);
depsA.push("x");
assert.equal(depsB.includes("x"), false);

const patched = withDerivedDependencies({
  id: "mod",
  kind: "derived",
  name: "Modifier",
  formula: "floor(({str} - 10) / 2)",
  dependencies: ["stale"],
  value: { type: "number", value: null },
});
assert.deepEqual(patched.dependencies, ["str"]);

assert.equal(
  formatProfileValue(
    {
      id: "mod",
      kind: "derived",
      name: "Modifier",
      unit: "hp",
      decimals: 1,
      value: { type: "number", value: null },
    },
    2,
  ),
  "2.0 hp",
);
assert.equal(
  componentHasValue({
    id: "mod",
    kind: "derived",
    name: "Modifier",
    formula: "{str}",
    value: { type: "number", value: null },
  }),
  true,
);
assert.equal(
  profileCardShows(
    {
      id: "ratio",
      kind: "derived",
      name: "Ratio",
      formula: "10 / {zero}",
      value: { type: "number", value: null },
    },
    null,
  ),
  true,
);

assert.ok(
  profileValidationErrors({
    schemaVersion: 1,
    components: [
      { id: "note", kind: "tag", name: "Note", value: { type: "text", value: "hi" } },
      {
        id: "mod",
        kind: "derived",
        name: "Modifier",
        formula: "{note}",
        value: { type: "number", value: null },
      },
    ],
  }).some((error) => error.includes("numeric")),
);
assert.ok(
  profileValidationErrors({
    schemaVersion: 1,
    components: [
      { id: "str", kind: "attribute", name: "Strength", value: { type: "number", value: 18 } },
      {
        id: "mod",
        kind: "derived",
        name: "Modifier",
        formula: "{str}",
        max: 10,
        value: { type: "number", value: null },
      },
    ],
  }).some((error) => error.includes("above max")),
);

const ranked = parseProfile({
  schemaVersion: 1,
  components: [
    {
      id: "skill",
      kind: "skill",
      name: "Swordsmanship",
      scale: ["Untrained", "Trained", "Expert"],
      value: { type: "rank", value: "Expert" },
    },
    {
      id: "rating",
      kind: "derived",
      name: "Rating",
      formula: "{skill}",
      value: { type: "number", value: null },
    },
  ],
});
assert.equal(evaluateProfile(ranked).get("rating"), 2);

const pooled = parseProfile({
  schemaVersion: 1,
  components: [
    {
      id: "hp",
      kind: "resource",
      name: "Hit Points",
      value: { type: "resource", current: null, max: 12, unit: null },
    },
    {
      id: "cap",
      kind: "derived",
      name: "Cap",
      formula: "{hp}",
      value: { type: "number", value: null },
    },
  ],
});
assert.equal(evaluateProfile(pooled).get("cap"), 12);

const literals = parseFormula("round(.5 + 5.)");
assert.equal("error" in literals, false);
if (!("error" in literals)) {
  assert.equal(
    evaluateProfile(
      parseProfile({
        schemaVersion: 1,
        components: [
          {
            id: "n",
            kind: "derived",
            name: "N",
            formula: "round(.5 + 5.)",
            value: { type: "number", value: null },
          },
        ],
      }),
    ).get("n"),
    6,
  );
}

console.log("lore profiles passed");
