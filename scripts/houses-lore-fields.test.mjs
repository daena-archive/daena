import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  contributedRelationshipFields,
  counterpartId,
  counterpartIds,
  coveredRelationshipIds,
  defaultRelationshipMetadata,
  endpointsForCreate,
  groupedRelationshipFields,
  groupRelationshipsByOwningModule,
  parseRelationshipMetadata,
  partitionPopulatedFields,
  relationshipAttributeRows,
  relationshipDirection,
  relationshipFieldForType,
  relationshipsForField,
} from "../src/lib/modules/contributed-fields.ts";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const lore = JSON.parse(readFileSync(join(root, "packages/modules/lore/manifest.json"), "utf8"));
const family = JSON.parse(readFileSync(join(root, "packages/modules/houses/manifest.json"), "utf8"));

const person = "daena.lore:person";
const enabledTypes = new Set([person, "daena.lore:place", "daena.lore:faction", "house", "daena.houses:house"]);
const familyEnabled = { ...family, enabled: true };

const parents = family.schemas[0].fields.find((field) => field.key === "parents");
const children = family.schemas[0].fields.find((field) => field.key === "children");
const partners = family.schemas[0].fields.find((field) => field.key === "partners");
const houses = family.schemas[0].fields.find((field) => field.key === "houses");
const members = family.schemas[0].fields.find((field) => field.key === "members");
const summary = family.schemas[0].fields.find((field) => field.key === "summary");
const aliases = family.schemas[0].fields.find((field) => field.key === "aliases");
const founded = family.schemas[0].fields.find((field) => field.key === "founded");
const allies = family.schemas[0].fields.find((field) => field.key === "allies");
const rivals = family.schemas[0].fields.find((field) => field.key === "rivals");

assert.equal(relationshipDirection(parents), "incoming");
assert.equal(relationshipDirection(children), "outgoing");
assert.equal(relationshipDirection(partners), "undirected");

const merged = contributedRelationshipFields(lore, person, [lore, familyEnabled], enabledTypes);
const keys = merged.filter((field) => field.type === "relationship").map((field) => field.key);
assert.equal(keys.includes("parents"), true);
assert.equal(keys.includes("children"), true);
assert.equal(keys.includes("partners"), true);
assert.equal(keys.includes("houses"), true);
assert.equal(relationshipDirection(houses), "outgoing");
assert.equal(relationshipDirection(members), "incoming");
assert.equal(houses.relationshipType, "family_member_of");
assert.equal(summary?.type, "text");
assert.equal(aliases?.type, "text");
assert.equal(founded?.type, "date");
assert.equal(relationshipDirection(allies), "undirected");
assert.equal(relationshipDirection(rivals), "undirected");
assert.equal(allies.relationshipType, "house_allied_with");
assert.equal(rivals.relationshipType, "house_rival_of");
assert.equal(houses.targetEntityTypes.includes("house"), true);
assert.equal(members.entityTypes.includes("house"), true);

const houseInspector = contributedRelationshipFields(familyEnabled, "house", [familyEnabled, lore], enabledTypes);
const houseInspectorKeys = houseInspector.map((field) => field.key);
assert.equal(houseInspectorKeys.includes("summary"), true);
assert.equal(houseInspectorKeys.includes("aliases"), true);
assert.equal(houseInspectorKeys.includes("allies"), true);
assert.equal(houseInspectorKeys.includes("rivals"), true);
assert.equal(houseInspectorKeys.includes("members"), true);
assert.equal(houseInspectorKeys.includes("founded"), false);
assert.equal(houseInspectorKeys.includes("parents"), false);
const typesWithTimeline = new Set([...enabledTypes, "daena.timeline:event"]);
const houseWithTimeline = contributedRelationshipFields(
  familyEnabled,
  "house",
  [familyEnabled, lore],
  typesWithTimeline,
);
assert.equal(
  houseWithTimeline.some((field) => field.key === "founded"),
  true,
);
assert.equal(defaultRelationshipMetadata(houses).role, "member");
assert.equal(family.capabilities.includes("schema.overlay"), true);
assert.equal(
  family.schemas[0].entityTypes.some((type) => type.id === "house"),
  true,
);

const hidden = contributedRelationshipFields(lore, person, [lore, { ...family, enabled: false }], enabledTypes);
assert.equal(
  hidden.some((field) => field.relationshipType === "family_parent_of"),
  false,
);

const parentId = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
const childId = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
const otherId = "cccccccc-cccc-cccc-cccc-cccccccccccc";
const parentEdge = {
  id: "rel-parent",
  source_id: parentId,
  target_id: childId,
  relationship_type: "family_parent_of",
};
const partnerEdge = {
  id: "rel-partner",
  source_id: childId,
  target_id: otherId,
  relationship_type: "family_partner_with",
};

assert.equal(counterpartId(childId, parentEdge, parents), parentId);
assert.equal(counterpartId(parentId, parentEdge, parents), null);
assert.equal(counterpartId(parentId, parentEdge, children), childId);
assert.equal(counterpartId(childId, parentEdge, children), null);
assert.equal(counterpartId(childId, partnerEdge, partners), otherId);
assert.equal(counterpartId(otherId, partnerEdge, partners), childId);

assert.deepEqual(counterpartIds(childId, [parentEdge, partnerEdge], parents), [parentId]);
assert.equal(relationshipsForField(childId, [parentEdge, partnerEdge], children).length, 0);

assert.deepEqual(endpointsForCreate(childId, parentId, parents), { sourceId: parentId, targetId: childId });
assert.deepEqual(endpointsForCreate(parentId, childId, children), { sourceId: parentId, targetId: childId });
assert.deepEqual(endpointsForCreate(otherId, childId, partners), { sourceId: childId, targetId: otherId });

assert.equal(defaultRelationshipMetadata(parents).kind, "biological");
assert.equal(defaultRelationshipMetadata(partners).kind, "marriage");

const houseId = "dddddddd-dddd-dddd-dddd-dddddddddddd";
const memberEdge = {
  id: "rel-house",
  source_id: childId,
  target_id: houseId,
  relationship_type: "family_member_of",
};
assert.equal(counterpartId(childId, memberEdge, houses), houseId);
assert.equal(counterpartId(houseId, memberEdge, members), childId);
assert.deepEqual(endpointsForCreate(childId, houseId, houses), { sourceId: childId, targetId: houseId });
assert.deepEqual(endpointsForCreate(houseId, childId, members), { sourceId: childId, targetId: houseId });

const named = [parents, children, partners, houses, members];
const childCovered = coveredRelationshipIds(childId, [parentEdge, partnerEdge, memberEdge], named);
assert.equal(childCovered.has("rel-parent"), true);
assert.equal(childCovered.has("rel-partner"), true);
assert.equal(childCovered.has("rel-house"), true);
assert.equal(coveredRelationshipIds(parentId, [parentEdge, partnerEdge], named).has("rel-parent"), true);
assert.equal(coveredRelationshipIds(parentId, [parentEdge, partnerEdge], named).has("rel-partner"), false);

const maps = JSON.parse(readFileSync(join(root, "packages/modules/maps/manifest.json"), "utf8"));
const mapsEnabled = { ...maps, enabled: true };
const typesWithMaps = new Set([...enabledTypes, "world-map", "daena.maps:world-map"]);
const personFields = contributedRelationshipFields(lore, person, [lore, familyEnabled, mapsEnabled], typesWithMaps);
assert.equal(
  personFields.some((field) => field.key === "detailMap" || field.key === "overviewMap" || field.key === "relatedMap"),
  false,
  "map-to-map fields must not appear on Person",
);
const mapFields = contributedRelationshipFields(mapsEnabled, "world-map", [mapsEnabled], typesWithMaps);
assert.equal(
  mapFields
    .filter((field) => field.type === "relationship")
    .map((field) => field.key)
    .sort()
    .join(","),
  "detailMap,overviewMap,relatedMap",
);

const lorePersonLinks = lore.schemas[0].fields.filter(
  (field) => field.type === "relationship" && field.entityTypes?.includes("person"),
);
const groupingFields = [...lorePersonLinks, parents, children, partners, houses];
const emptyGroups = groupedRelationshipFields(groupingFields, [lore, familyEnabled, mapsEnabled], () => 0);
assert.deepEqual(
  emptyGroups.map((group) => group.moduleName),
  ["Houses", "Lore"],
);

const populated = (field) => (field.key === "parents" ? 1 : 0);
const filledGroups = groupedRelationshipFields(groupingFields, [lore, familyEnabled], populated);
assert.equal(filledGroups[0].moduleName, "Houses");
assert.equal(filledGroups[0].fields[0].key, "parents");
assert.equal(filledGroups[1].moduleName, "Lore");
const bornInPopulated = (field) => (field.key === "bornIn" ? 1 : 0);
const loreFilledFirst = groupedRelationshipFields(groupingFields, [lore, familyEnabled], bornInPopulated);
assert.equal(loreFilledFirst.find((group) => group.moduleName === "Lore")?.fields[0].key, "bornIn");
const stableLore = groupedRelationshipFields(groupingFields, [lore, familyEnabled], bornInPopulated, {
  sortByPopulated: false,
});
assert.equal(stableLore.find((group) => group.moduleName === "Lore")?.fields[0].key, lorePersonLinks[0].key);
const { filled, empty } = partitionPopulatedFields(filledGroups[0].fields, populated);
assert.deepEqual(
  filled.map((field) => field.key),
  ["parents"],
);
assert.equal(
  empty.some((field) => field.key === "parents"),
  false,
);

assert.deepEqual(parseRelationshipMetadata({ role: "head", notes: "regent" }), { role: "head", notes: "regent" });
assert.deepEqual(parseRelationshipMetadata('{"kind":"adoptive"}'), { kind: "adoptive" });
assert.deepEqual(parseRelationshipMetadata(""), {});

const parentDef = relationshipFieldForType("family_parent_of", [lore, familyEnabled]);
assert.equal(parentDef?.key, "parents");
assert.equal(parentDef?.metadataFields?.[0]?.label, "Parent type");
const parentAttrs = relationshipAttributeRows('{"kind":"adoptive","notes":"ward"}', parentDef);
assert.deepEqual(
  parentAttrs.map((row) => `${row.label}:${row.raw}`),
  ["Parent type:adoptive", "Notes:ward"],
);

const leftover = groupRelationshipsByOwningModule(
  [
    { id: "a", relationship_type: "family_member_of" },
    { id: "b", relationship_type: "affiliated_with" },
  ],
  [lore, familyEnabled],
);
assert.deepEqual(
  leftover.map((group) => group.moduleName),
  ["Houses", "Lore"],
);

const wikiSource = readFileSync(join(root, "src/lib/lore/WikiView.svelte"), "utf8");
assert.match(wikiSource, /groupedWikiRelationships/);
assert.match(wikiSource, /info-rel-group/);
assert.match(wikiSource, /wikiAttrChips\(target\.attributes\)/);

console.log("houses lore field contribution checks passed");
