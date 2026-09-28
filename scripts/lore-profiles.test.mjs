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
import { evaluateFormula, parseFormula } from "../src/lib/lore/profileFormula.ts";
import {
  applyPatches,
  changesForEvent,
  dateAfter,
  foldProfile,
  latestUnlinkedChange,
  parseProfileChange,
  patchedProfileErrors,
  matchingProfileChangeRels,
  PROFILE_CHANGE_MAX_BYTES,
  PROFILE_CHANGE_RELATIONSHIP,
  profileChangeValidationErrors,
  unlinkedChangeAt,
  valuePatches,
} from "../src/lib/lore/profileHistory.ts";
import {
  BUNDLED_PROFILE_PRESET_SEEDS,
  CUSTOM_PRESET_ID,
  DND_PRESET_ID,
  FANTASY_PRESET_ID,
  canonicalPresetOrigin,
} from "../src/lib/lore/profilePresetSeed.ts";
import {
  applyDndAncestry,
  bundledPresetDocument,
  defaultProfilePresetId,
  applyPresetFields,
  blankProfilePreset,
  bundledPresetNeedsMigration,
  deleteProfilePreset,
  duplicatedProfilePreset,
  loadProfilePresets,
  presetDocumentMatchesSeed,
  presetFromProfile,
  ensureBundledProfilePresets,
  groupProfilePresets,
  matchingDndAncestry,
  migrateLegacyPresetOrigins,
  profileForPresetSelection,
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
assert.equal(parseProfile({ schemaVersion: 1, presetOrigin: "custom", components: [] }).presetOrigin, CUSTOM_PRESET_ID);
assert.equal(emptyProfile().presetOrigin, undefined);
assert.deepEqual(profileValidationErrors({ schemaVersion: 1, presetOrigin: "dnd", components: [] }), []);
assert.deepEqual(profileValidationErrors({ schemaVersion: 1, presetOrigin: DND_PRESET_ID, components: [] }), []);
assert.ok(profileValidationErrors({ schemaVersion: 1, presetOrigin: "modern", components: [] }).length);
assert.ok(profileValidationErrors({ schemaVersion: 1, allocation: { pool: -1 }, components: [] }).length);

assert.deepEqual(
  BUNDLED_PROFILE_PRESET_SEEDS.slice(0, 4).map((preset) => preset.id),
  [
    "ef2bce76-ca39-59f5-ab9d-28a0b665eb15",
    "9771b70b-04c4-506e-949b-b26ea5b61235",
    "1b2c999f-65b5-57b6-9d02-6ce2c40cc977",
    "d9ff8d59-1cd3-52ae-8adb-5700fbff6ba1",
  ],
);
assert.equal(BUNDLED_PROFILE_PRESET_SEEDS.length, 15);
assert.equal(new Set(BUNDLED_PROFILE_PRESET_SEEDS.map((preset) => preset.id)).size, 15);
for (const preset of BUNDLED_PROFILE_PRESET_SEEDS) {
  const copied = profileFromPreset(preset.id);
  const ids = copied.components.map((component) => component.id);
  assert.equal(new Set(ids).size, ids.length, preset.name);
  assert.deepEqual(profileValidationErrors(copied), [], preset.name);
  assert.equal(copied.presetOrigin, preset.id);
  assert.equal(
    copied.components.some((component) => component.kind === "trait"),
    false,
    preset.name,
  );
  if (preset.legacyKey) assert.equal(canonicalPresetOrigin(preset.legacyKey), preset.id);
}
const customSeed = BUNDLED_PROFILE_PRESET_SEEDS.find((preset) => preset.id === CUSTOM_PRESET_ID);
assert.equal(customSeed.entityTypes, undefined);
assert.equal(customSeed.genre, undefined);
assert.equal(BUNDLED_PROFILE_PRESET_SEEDS.find((preset) => preset.id === DND_PRESET_ID).name, "D&D Character");
assert.equal(
  profileFromPreset("scifi").components.some(
    (component) => component.name === "Population" || component.name === "Fleet Strength",
  ),
  false,
);

const mutated = profileFromPreset("dnd");
mutated.components[0].name = "Renamed";
if (mutated.components[0].value.type === "number") mutated.components[0].value.value = 18;
const fresh = profileFromPreset("dnd");
assert.equal(fresh.components[0].name, "Strength");
assert.equal(fresh.components[0].value.type === "number" ? fresh.components[0].value.value : null, 10);
assert.equal(parseProfile(fresh).presetOrigin, DND_PRESET_ID);

const fantasy = profileFromPreset("fantasy");
assert.equal(fantasy.allocation?.pool, 40);
assert.equal(allocationSpent(fantasy), 40);
assert.equal(allocationRemaining(fantasy), 0);
const loadedPresets = BUNDLED_PROFILE_PRESET_SEEDS.map((seed) => ({
  id: seed.id,
  revision: "1",
  document: bundledPresetDocument(seed),
}));
for (const preset of loadedPresets) {
  const fromRecord = profileForPresetSelection(loadedPresets, preset.id);
  assert.deepEqual(fromRecord, profileFromPreset(preset.id));
  fromRecord.components.push({
    id: "extra",
    kind: "tag",
    name: "Extra",
    value: { type: "text", value: null },
  });
  assert.equal(
    profileForPresetSelection(loadedPresets, preset.id).components.length,
    profileFromPreset(preset.id).components.length,
  );
}
const emptySelection = profileForPresetSelection([], CUSTOM_PRESET_ID);
assert.equal(emptySelection.presetOrigin, undefined);
assert.deepEqual(emptySelection.components, []);
assert.equal(profileForPresetSelection(loadedPresets, "missing").presetOrigin, undefined);
const proxiedPresets = loadedPresets.map((preset) => new Proxy(preset, {}));
const fromProxy = profileForPresetSelection(proxiedPresets, DND_PRESET_ID);
assert.equal(fromProxy.presetOrigin, DND_PRESET_ID);
assert.equal(fromProxy.components[0]?.name, "Strength");
fromProxy.components[0].name = "Changed";
assert.equal(proxiedPresets.find((preset) => preset.id === DND_PRESET_ID).document.components[0].name, "Strength");

const seeded = [];
const updated = [];
const phase1Dnd = bundledPresetDocument(BUNDLED_PROFILE_PRESET_SEEDS.find((preset) => preset.legacyKey === "dnd"));
delete phase1Dnd.entityTypes;
delete phase1Dnd.genre;
phase1Dnd.name = "D&D";
await ensureBundledProfilePresets({
  projectRecords: {
    async list() {
      return [
        { id: CUSTOM_PRESET_ID, revision: "c1", value: bundledPresetDocument(customSeed) },
        { id: DND_PRESET_ID, revision: "d1", value: phase1Dnd },
      ];
    },
    async create(_collection, value, options) {
      seeded.push({ id: options.id, value });
      return { id: options.id, revision: "1", value };
    },
    async update(_collection, id, value, options) {
      updated.push({ id, value, options });
      return { id, revision: "2", value };
    },
  },
});
assert.equal(seeded.length, 13);
assert.equal(
  seeded.some((row) => row.id === CUSTOM_PRESET_ID || row.id === DND_PRESET_ID),
  false,
);
assert.equal(updated.length, 1);
assert.equal(updated[0].id, DND_PRESET_ID);
assert.equal(updated[0].options.expectedRevision, "d1");
assert.equal(updated[0].value.name, "D&D Character");
assert.deepEqual(updated[0].value.entityTypes, ["daena.lore:person"]);
assert.equal(
  seeded.every((row) => row.value.builtin === true),
  true,
);

function reorderKeys(value) {
  if (Array.isArray(value)) return value.map(reorderKeys);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .reverse()
        .map((key) => [key, reorderKeys(value[key])]),
    );
  }
  return value;
}
const currentDnd = bundledPresetDocument(BUNDLED_PROFILE_PRESET_SEEDS.find((preset) => preset.id === DND_PRESET_ID));
assert.equal(presetDocumentMatchesSeed(reorderKeys(currentDnd), currentDnd), true);
const reorderedUpdates = [];
await ensureBundledProfilePresets({
  projectRecords: {
    async list() {
      return BUNDLED_PROFILE_PRESET_SEEDS.map((seed) => ({
        id: seed.id,
        revision: "1",
        value: reorderKeys(bundledPresetDocument(seed)),
      }));
    },
    async create() {
      throw new Error("should not create");
    },
    async update(_collection, id) {
      reorderedUpdates.push(id);
    },
  },
});
assert.equal(reorderedUpdates.length, 0);

const createdAfterFailure = [];
await assert.rejects(
  () =>
    ensureBundledProfilePresets({
      projectRecords: {
        async list() {
          return [{ id: DND_PRESET_ID, revision: "d1", value: phase1Dnd }];
        },
        async create(_collection, _value, options) {
          createdAfterFailure.push(options.id);
          return { id: options.id, revision: "1", value: {} };
        },
        async update() {
          throw new Error("revision conflict");
        },
      },
    }),
  /revision conflict/,
);
assert.ok(createdAfterFailure.includes(CUSTOM_PRESET_ID));
assert.equal(createdAfterFailure.includes(DND_PRESET_ID), false);

const loadedPresetsForGroups = BUNDLED_PROFILE_PRESET_SEEDS.map((seed) => ({
  id: seed.id,
  revision: "1",
  document: bundledPresetDocument(seed),
}));
const factionGroups = groupProfilePresets(loadedPresetsForGroups, "daena.lore:faction");
const factionSuggested = factionGroups.suggested.map((preset) => preset.document.name);
assert.ok(factionSuggested.includes("Custom"));
assert.ok(factionSuggested.includes("D&D Faction"));
assert.ok(factionSuggested.includes("Fantasy Kingdom"));
assert.equal(factionSuggested.includes("D&D Character"), false);
const placeSuggested = groupProfilePresets(loadedPresetsForGroups, "place").suggested.map(
  (preset) => preset.document.name,
);
assert.ok(placeSuggested.includes("Fantasy Kingdom"));
assert.ok(placeSuggested.includes("Fantasy Place"));
assert.equal(placeSuggested.includes("D&D Character"), false);
for (const type of ["person", "faction", "place", "artifact", "culture", "concept"]) {
  const names = groupProfilePresets(loadedPresetsForGroups, `daena.lore:${type}`).suggested.map(
    (preset) => preset.document.name,
  );
  assert.ok(names.includes("Custom"), type);
  assert.ok(
    names.some((name) => name !== "Custom"),
    type,
  );
}
const overlaySuggested = groupProfilePresets(loadedPresetsForGroups, "daena.lore:knightly-order").suggested.map(
  (preset) => preset.document.name,
);
assert.deepEqual(overlaySuggested, ["Custom"]);
assert.equal(
  factionGroups.others.some((group) => group.genre === "Other"),
  false,
);
assert.equal(
  factionGroups.others.some((group) => group.presets.some((preset) => preset.document.name === "Culture")),
  true,
);
assert.equal(defaultProfilePresetId(loadedPresetsForGroups, "daena.lore:faction") === CUSTOM_PRESET_ID, false);

const editedBundled = bundledPresetDocument(BUNDLED_PROFILE_PRESET_SEEDS.find((preset) => preset.id === DND_PRESET_ID));
editedBundled.name = "House rules";
assert.equal(
  bundledPresetNeedsMigration(
    editedBundled,
    BUNDLED_PROFILE_PRESET_SEEDS.find((preset) => preset.id === DND_PRESET_ID),
  ),
  false,
);
const editedUpdates = [];
await ensureBundledProfilePresets({
  projectRecords: {
    async list() {
      return [{ id: DND_PRESET_ID, revision: "9", value: editedBundled }];
    },
    async create() {
      return { id: "created", revision: "1", value: {} };
    },
    async update(_collection, id) {
      editedUpdates.push(id);
    },
  },
});
assert.equal(editedUpdates.includes(DND_PRESET_ID), false);

const customizedLegacy = bundledPresetDocument(
  BUNDLED_PROFILE_PRESET_SEEDS.find((preset) => preset.id === DND_PRESET_ID),
);
delete customizedLegacy.entityTypes;
delete customizedLegacy.genre;
customizedLegacy.name = "D&D";
customizedLegacy.customized = true;
const customizedUpdates = [];
await ensureBundledProfilePresets({
  projectRecords: {
    async list() {
      return BUNDLED_PROFILE_PRESET_SEEDS.map((seed) => ({
        id: seed.id,
        revision: "1",
        value: seed.id === DND_PRESET_ID ? customizedLegacy : bundledPresetDocument(seed),
      }));
    },
    async create() {
      throw new Error("should not create");
    },
    async update(_collection, id) {
      customizedUpdates.push(id);
    },
  },
});
assert.equal(customizedUpdates.length, 0);

const withUnit = profileFromPreset(DND_PRESET_ID);
const hitPoints = withUnit.components.find((component) => component.name === "Hit Points");
hitPoints.value = { type: "resource", current: 3, max: 9, unit: "points" };
const savedUnit = presetFromProfile(
  withUnit,
  { name: "Kept unit" },
  bundledPresetDocument(BUNDLED_PROFILE_PRESET_SEEDS.find((preset) => preset.id === DND_PRESET_ID)),
);
assert.equal(savedUnit.components.find((component) => component.name === "Hit Points").value.unit, "points");
assert.equal(savedUnit.components.find((component) => component.name === "Hit Points").value.current, null);

const progressed = profileFromPreset(DND_PRESET_ID);
const progressedStrength = progressed.components.find((component) => component.id === "attribute-strength");
progressedStrength.value = { type: "number", value: 18 };
progressed.components.push({
  id: "attribute-guild-rank",
  kind: "attribute",
  name: "Guild Rank",
  min: 0,
  max: 5,
  value: { type: "number", value: 4 },
});
const tradeGuild = presetFromProfile(
  progressed,
  { name: "Trade Guild", description: "A faction sheet", entityTypes: ["faction"], genre: "Local" },
  bundledPresetDocument(BUNDLED_PROFILE_PRESET_SEEDS.find((preset) => preset.id === DND_PRESET_ID)),
);
assert.equal(tradeGuild.name, "Trade Guild");
assert.deepEqual(tradeGuild.entityTypes, ["daena.lore:faction"]);
assert.equal(tradeGuild.genre, "Local");
assert.equal(tradeGuild.builtin, undefined);
assert.equal(tradeGuild.components.find((component) => component.id === "attribute-strength").value.value, 10);
assert.equal(tradeGuild.components.find((component) => component.id === "attribute-guild-rank").value.value, 0);
assert.equal(
  "override" in (tradeGuild.components.find((component) => component.id === "attribute-strength") ?? {}),
  false,
);

const hiddenBundled = { ...editedBundled, hidden: true };
const visible = await loadProfilePresets({
  projectRecords: {
    async list() {
      return [
        { id: DND_PRESET_ID, revision: "1", value: hiddenBundled },
        { id: "user-1", revision: "1", value: tradeGuild },
      ];
    },
  },
});
assert.equal(
  visible.some((preset) => preset.id === DND_PRESET_ID),
  false,
);
assert.equal(
  visible.some((preset) => preset.document.name === "Trade Guild"),
  true,
);
const withUser = groupProfilePresets(
  [
    ...loadedPresetsForGroups.filter((preset) => preset.id !== DND_PRESET_ID),
    { id: "user-1", revision: "1", document: tradeGuild },
  ],
  "daena.lore:faction",
);
assert.ok(withUser.suggested.some((preset) => preset.document.name === "Trade Guild"));

const profileBeforeDelete = profileForPresetSelection(
  [{ id: "user-1", revision: "1", document: tradeGuild }],
  "user-1",
);
const deleted = [];
await deleteProfilePreset(
  {
    projectRecords: {
      async delete(_collection, id) {
        deleted.push(id);
      },
    },
  },
  { id: "user-1", revision: "1", document: tradeGuild },
);
assert.deepEqual(deleted, ["user-1"]);
assert.equal(profileBeforeDelete.presetOrigin, "user-1");
assert.equal(profileBeforeDelete.components.length, tradeGuild.components.length);
await assert.rejects(
  () =>
    deleteProfilePreset(
      { projectRecords: { async delete() {} } },
      { id: DND_PRESET_ID, revision: "1", document: editedBundled },
    ),
  /cannot be deleted/,
);
const reset = duplicatedProfilePreset({ ...editedBundled, builtin: true, hidden: true });
assert.equal(reset.builtin, undefined);
assert.equal(reset.hidden, undefined);
assert.equal(reset.name, "House rules copy");
const blank = blankProfilePreset({ name: "Blank", entityTypes: ["daena.lore:place"] });
assert.deepEqual(blank.components, []);
assert.deepEqual(blank.entityTypes, ["daena.lore:place"]);
const renamed = applyPresetFields(tradeGuild, { name: "Noble House", entityTypes: ["daena.lore:faction"], genre: "" });
assert.equal(renamed.name, "Noble House");
assert.equal(renamed.genre, undefined);
assert.equal(renamed.components.length, tradeGuild.components.length);

const listedOrigins = [];
const rewritten = [];
await assert.rejects(
  () =>
    migrateLegacyPresetOrigins(
      {
        records: {
          async list(_collection, entityId) {
            listedOrigins.push(entityId);
            if (entityId === "bad") throw new Error("list failed");
            if (entityId === "legacy") {
              return [
                { id: "p1", revision: "r1", value: { schemaVersion: 1, presetOrigin: "fantasy", components: [] } },
              ];
            }
            if (entityId === "current") {
              return [
                { id: "p2", revision: "r2", value: { schemaVersion: 1, presetOrigin: DND_PRESET_ID, components: [] } },
              ];
            }
            return [];
          },
          async update(_collection, id, owner, value, options) {
            rewritten.push({ id, owner, value, options });
          },
        },
      },
      ["bad", "legacy", "current", "none"],
    ),
  /list failed/,
);
assert.deepEqual(listedOrigins, ["bad", "legacy", "current", "none"]);
assert.equal(rewritten.length, 1);
assert.equal(rewritten[0].id, "p1");
assert.equal(rewritten[0].value.presetOrigin, FANTASY_PRESET_ID);
assert.equal(rewritten[0].options.expectedRevision, "r1");

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
const averaged = parseFormula("avg({military}, {political}, {economic})");
assert.equal("error" in averaged, false);
if (!("error" in averaged)) {
  assert.deepEqual(averaged.dependencies, ["military", "political", "economic"]);
  assert.equal(
    evaluateFormula(averaged.ast, (id) => ({ military: 10, political: 10, economic: 11 })[id] ?? null),
    (10 + 10 + 11) / 3,
  );
  assert.equal(
    formatProfileValue(
      {
        id: "derived-overall-power",
        kind: "derived",
        name: "Overall Power",
        formula: "avg({military}, {political}, {economic})",
        value: { type: "number", value: null },
      },
      (10 + 10 + 11) / 3,
    ),
    "10.33",
  );
}
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

const baseline = parseProfile({
  schemaVersion: 1,
  components: [
    {
      id: "str",
      kind: "attribute",
      name: "Strength",
      value: { type: "number", value: 10 },
    },
    {
      id: "mod",
      kind: "derived",
      name: "Modifier",
      formula: "floor(({str} - 10) / 2)",
      value: { type: "number", value: null },
    },
  ],
});
assert.deepEqual(profileChangeValidationErrors({ schemaVersion: 1, date: { year: 247 }, patches: [] }), [
  "Profile change needs at least one patch",
]);
const early = parseProfileChange({
  schemaVersion: 1,
  date: { calendar: "gregorian", year: 240, era: "CE", precision: "year" },
  patches: [{ componentId: "str", value: { type: "number", value: 12 } }],
});
const linked = parseProfileChange({
  schemaVersion: 1,
  date: { calendar: "gregorian", year: 1, era: "CE", precision: "year" },
  eventId: "battle",
  patches: [{ componentId: "str", value: { type: "number", value: 16 } }],
});
const storedChanges = [
  { id: "c1", revision: "1", createdAt: "1", value: early },
  { id: "c2", revision: "1", createdAt: "2", value: linked },
];
const liveDates = new Map([["battle", { calendar: "gregorian", year: 247, era: "CE", precision: "year" }]]);
assert.equal(foldProfile(baseline, storedChanges, { year: 239 }, liveDates).components[0].value.value, 10);
assert.equal(foldProfile(baseline, storedChanges, { year: 240 }, liveDates).components[0].value.value, 12);
assert.equal(foldProfile(baseline, storedChanges, { year: 247 }, liveDates).components[0].value.value, 16);
assert.equal(evaluateProfile(foldProfile(baseline, storedChanges, { year: 247 }, liveDates)).get("mod"), 3);
assert.equal(foldProfile(baseline, storedChanges, { year: 247 }, new Map()).components[0].value.value, 12);
assert.equal(
  foldProfile(baseline, storedChanges, { year: 247 }, new Map([["battle", null]])).components[0].value.value,
  12,
);
const moved = new Map([["battle", { calendar: "gregorian", year: 230, era: "CE", precision: "year" }]]);
assert.equal(foldProfile(baseline, storedChanges, { year: 235 }, moved).components[0].value.value, 16);
assert.equal(foldProfile(baseline, storedChanges, { year: 240 }, moved).components[0].value.value, 12);
const otherBattle = parseProfileChange({
  schemaVersion: 1,
  date: { calendar: "gregorian", year: 1, era: "CE", precision: "year" },
  eventId: "duel",
  patches: [{ componentId: "str", value: { type: "number", value: 18 } }],
});
const twoEvents = [...storedChanges, { id: "c3", revision: "1", createdAt: "3", value: otherBattle }];
const bothDates = new Map([
  ["battle", { calendar: "gregorian", year: 247, era: "CE", precision: "year" }],
  ["duel", { calendar: "gregorian", year: 250, era: "CE", precision: "year" }],
]);
assert.equal(foldProfile(baseline, twoEvents, { year: 247 }, bothDates).components[0].value.value, 16);
assert.equal(foldProfile(baseline, twoEvents, { year: 250 }, bothDates).components[0].value.value, 18);
assert.equal(
  foldProfile(baseline, twoEvents, { year: 247 }, new Map([["duel", bothDates.get("duel")]])).components[0].value.value,
  12,
);
const current = foldProfile(baseline, storedChanges, undefined, liveDates);
assert.deepEqual(valuePatches(baseline, current), [{ componentId: "str", value: { type: "number", value: 16 } }]);
assert.equal(applyPatches(baseline, []).components[0].value.value, 10);
assert.deepEqual(dateAfter({ calendar: "gregorian", year: 247, era: "CE", precision: "year" }), {
  calendar: "gregorian",
  year: 248,
  era: "CE",
  precision: "year",
});
assert.equal(dateAfter({ calendar: "gregorian", year: 247, month: 3, day: 15, era: "CE", precision: "day" }).month, 3);
assert.equal(dateAfter({ calendar: "gregorian", year: 247, month: 3, day: 15, era: "CE", precision: "day" }).day, 15);
assert.equal(dateAfter({ calendar: "gregorian", year: 247, month: 3, day: 15, era: "CE", precision: "day" }).year, 248);
assert.deepEqual(dateAfter({ calendar: "gregorian", year: 5, era: "BCE", precision: "year" }).year, 4);
assert.equal(dateAfter({ calendar: "gregorian", year: 1, era: "BCE", precision: "year" }).era, "CE");
assert.equal(changesForEvent(twoEvents, "battle").length, 1);
assert.equal(unlinkedChangeAt(storedChanges, early.date)?.id, "c1");
assert.equal(latestUnlinkedChange([{ id: "c1", revision: "1", createdAt: "1", value: early }], liveDates)?.id, "c1");
assert.equal(latestUnlinkedChange(storedChanges, liveDates)?.id, undefined);
assert.equal(latestUnlinkedChange(twoEvents, bothDates)?.id, undefined);
assert.deepEqual(patchedProfileErrors(baseline, [{ componentId: "str", value: { type: "number", value: 99 } }]), []);
const bounded = parseProfile({
  schemaVersion: 1,
  components: [
    {
      id: "str",
      kind: "attribute",
      name: "Strength",
      min: 0,
      max: 20,
      value: { type: "number", value: 10 },
    },
  ],
});
assert.equal(
  patchedProfileErrors(bounded, [{ componentId: "str", value: { type: "number", value: 99 } }]).some((error) =>
    error.includes("above max"),
  ),
  true,
);
assert.equal(
  foldProfile(
    baseline,
    [
      {
        id: "bad",
        revision: "1",
        createdAt: "0",
        value: {
          schemaVersion: 1,
          date: { calendar: "gregorian", year: 1, era: "CE", precision: "year" },
          patches: [],
        },
        invalid: true,
        error: "bad",
      },
      ...storedChanges,
    ],
    { year: 240 },
    liveDates,
  ).components[0].value.value,
  12,
);
assert.equal(
  changesForEvent(
    [
      ...storedChanges,
      {
        id: "c2b",
        revision: "1",
        createdAt: "2b",
        value: parseProfileChange({
          schemaVersion: 1,
          date: { calendar: "gregorian", year: 1, era: "CE", precision: "year" },
          eventId: "battle",
          patches: [{ componentId: "str", value: { type: "number", value: 17 } }],
        }),
      },
    ],
    "battle",
  ).length,
  2,
);
assert.equal(
  matchingProfileChangeRels(
    [
      { type: PROFILE_CHANGE_RELATIONSHIP, sourceId: "battle", targetId: "person" },
      { type: PROFILE_CHANGE_RELATIONSHIP, sourceId: "duel", targetId: "person" },
    ],
    "battle",
    "person",
  ).length,
  1,
);
assert.equal(
  matchingProfileChangeRels(
    [{ type: PROFILE_CHANGE_RELATIONSHIP, sourceId: "battle", targetId: "person" }],
    "battle",
    "person",
  )[0].sourceId,
  "battle",
);
assert.equal(
  profileChangeValidationErrors({
    schemaVersion: 1,
    date: { calendar: "gregorian", year: 1, era: "CE", precision: "year" },
    patches: [{ componentId: "str", value: { type: "text", value: "x".repeat(PROFILE_CHANGE_MAX_BYTES) } }],
  }).some((error) => error.includes("64 KiB")),
  true,
);

console.log("lore profiles passed");
