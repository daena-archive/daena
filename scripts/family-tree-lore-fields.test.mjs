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
  relationshipDirection,
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

console.log("family-tree lore field contribution checks passed");
