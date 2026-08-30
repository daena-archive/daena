import assert from "node:assert/strict";
import {
  houseMemberCounts,
  listHouseMembers,
  loadExpansionLayer,
  loadGenealogyNeighborhood,
  loadHouseNeighborhood,
} from "../src/lib/family-tree/fetch.ts";
import {
  LayoutGeneration,
  buildElkGraph,
  familyEdgeHandles,
  isCurrentGeneration,
  placeUnions,
} from "../src/lib/family-tree/layout.ts";
import {
  PERSON_NODE_HEIGHT,
  PERSON_NODE_WIDTH,
  MEMBERSHIP_RELATIONSHIP,
  PERSON_TYPE,
  UNION_NODE_HEIGHT,
  UNION_NODE_WIDTH,
  VISIBLE_EDGE_LIMIT,
  VISIBLE_PERSON_LIMIT,
  VISIBLE_UNION_LIMIT,
  clampFamilyTreeLimits,
  familyTreeLimitsOverBudget,
  layoutExceedsLimits,
  truncationWarning,
} from "../src/lib/family-tree/model.ts";
import { expansionKey, familyTreeHistoryKey, sameFamilyTreeSession } from "../src/lib/family-tree/model.ts";
import { classifyMutationError } from "../src/lib/family-tree/mutations.ts";
import {
  expansionBlocked,
  formatParentCycleMessage,
  generationDistance,
  hiddenCounts,
  initialNeighborhood,
  normalizeGenealogy,
  parentCyclePath,
  parseFamilyRelationship,
  personFromRecord,
  seedInitialExpansions,
  visibleFromExpansions,
  wouldCreateDuplicate,
  wouldCreateParentCycle,
  wouldExceedVisibleLimit,
} from "../src/lib/family-tree/projection.ts";
import {
  readFamilyTreeLimits,
  rememberRecentRoot,
  recentRoots,
  replaceRecentRoots,
  writeFamilyTreeLimits,
} from "../src/lib/family-tree/state.ts";
import {
  buildLayoutGraph,
  coupleClickAction,
  layoutGraphExceedsLimits,
  unionClickAction,
} from "../src/lib/family-tree/unions.ts";

function person(id, name = id) {
  return { id, name, revision: "1", birth: null, death: null, secondaryLabel: null };
}

function parent(id, sourceId, targetId, kind = "biological") {
  return {
    id,
    sourceId,
    targetId,
    type: "family_parent_of",
    revision: "1",
    metadata: { kind },
  };
}

function partner(id, sourceId, targetId, kind = "marriage", status = "active") {
  return {
    id,
    sourceId,
    targetId,
    type: "family_partner_with",
    revision: "1",
    metadata: { kind, status },
  };
}

const root = person("root", "Root");
const mother = person("mother", "Mother");
const father = person("father", "Father");
const coparent = person("coparent", "Coparent");
const sibling = person("sibling", "Sibling");
const half = person("half", "Half");
const child = person("child", "Child");
const grandchild = person("grandchild", "Grandchild");
const partnerB = person("partner-b", "Partner B");
const ended = person("ended", "Ended");
const grandparent = person("grandparent", "Grandparent");
const outsider = person("outsider", "Outsider");
const childless = person("childless", "Childless Partner");

const { graph, warnings } = normalizeGenealogy(
  [root, mother, father, coparent, sibling, half, child, grandchild, partnerB, ended, grandparent, outsider, childless],
  [
    parent("p1", mother.id, root.id),
    parent("p2", father.id, root.id),
    parent("p3", coparent.id, root.id),
    parent("p4", mother.id, sibling.id),
    parent("p5", father.id, sibling.id),
    parent("p6", mother.id, half.id),
    parent("p7", root.id, child.id),
    parent("p8", partnerB.id, child.id),
    parent("p9", child.id, grandchild.id),
    parent("p10", grandparent.id, mother.id),
    partner("r1", mother.id, father.id),
    partner("r2", root.id, partnerB.id),
    partner("r3", root.id, ended.id, "marriage", "ended"),
    partner("r4", outsider.id, ended.id),
    partner("r5", root.id, childless.id, "partnership", "active"),
  ],
);

assert.equal(warnings.length, 0);
assert.equal(graph.people.get(root.id)?.name, "Root");
assert.deepEqual([...graph.parentsByChild.get(root.id)].sort(), ["coparent", "father", "mother"]);

const visible = initialNeighborhood(graph, root.id);
assert.equal(visible.has(root.id), true);
assert.equal(visible.has(mother.id), true);
assert.equal(visible.has(father.id), true);
assert.equal(visible.has(coparent.id), true);
assert.equal(visible.has(grandparent.id), true);
assert.equal(visible.has(sibling.id), true);
assert.equal(visible.has(half.id), true);
assert.equal(visible.has(child.id), true);
assert.equal(visible.has(grandchild.id), true);
assert.equal(visible.has(partnerB.id), true);
assert.equal(visible.has(childless.id), true, "childless partnership is included");
assert.equal(visible.has(ended.id), false);
assert.equal(visible.has(outsider.id), false);

const layout = buildLayoutGraph(graph, visible);
const unionIds = layout.nodes
  .filter((node) => node.kind === "union")
  .map((node) => node.id)
  .sort();
assert.equal(unionIds.includes("union:parents:coparent:father:mother"), true, "three visible parents share one union");
assert.equal(unionIds.includes("union:parents:partner-b:root"), true);
assert.equal(
  layout.edges.some((edge) => edge.role === "direct-parent" && edge.source === "mother" && edge.target === "half"),
  true,
);
assert.equal(
  layout.nodes.some((node) => node.id === "union:partner:r2"),
  false,
  "partner edge reuses the two-parent union",
);
assert.equal(
  layout.edges.filter((edge) => edge.source === "root" && edge.target === "union:parents:partner-b:root").length,
  1,
  "married coparents use one person-to-union edge",
);
assert.equal(
  layout.edges.some(
    (edge) => edge.source === "root" && edge.target === "union:parents:partner-b:root" && edge.role === "partner",
  ),
  true,
  "shared parent union keeps the marriage line",
);
assert.equal(
  layout.nodes.some((node) => node.id === "union:partner:r5"),
  true,
  "childless partnership gets its own union",
);

const layoutAgain = buildLayoutGraph(graph, visible);
assert.deepEqual(
  layout.nodes.map((node) => node.id),
  layoutAgain.nodes.map((node) => node.id),
);
assert.deepEqual(
  layout.edges.map((edge) => edge.id),
  layoutAgain.edges.map((edge) => edge.id),
);

const { graph: unmarriedParents } = normalizeGenealogy(
  [person("mom"), person("dad"), person("kid")],
  [parent("u1", "mom", "kid"), parent("u2", "dad", "kid")],
);
const unmarriedLayout = buildLayoutGraph(unmarriedParents, ["mom", "dad", "kid"]);
const unmarriedEdge = unmarriedLayout.edges.find((edge) => edge.role === "parent");
assert.deepEqual(coupleClickAction(unmarriedEdge, unmarriedLayout.nodes, unmarriedLayout.edges), {
  memberIds: ["dad", "mom"],
});
const unmarriedUnion = unmarriedLayout.nodes.find((node) => node.kind === "union");
assert.deepEqual(unionClickAction(unmarriedUnion.id, unmarriedLayout.nodes, unmarriedLayout.edges), {
  memberIds: ["dad", "mom"],
});
const marriedEdge = layout.edges.find((edge) => edge.role === "partner" && edge.relationshipId === "r2");
assert.deepEqual(coupleClickAction(marriedEdge, layout.nodes, layout.edges), { relationshipId: "r2" });
assert.deepEqual(unionClickAction("union:parents:partner-b:root", layout.nodes, layout.edges), {
  relationshipId: "r2",
});

const elk = buildElkGraph(layout);
assert.equal(
  (elk.children ?? []).some((node) => node.id.startsWith("union:")),
  false,
  "ELK lays out people only",
);
assert.equal(
  (elk.edges ?? []).some((edge) => edge.sources.includes("root") && edge.targets.includes("child")),
  true,
  "ELK uses parent-to-child edges for couple offspring",
);
assert.equal(
  (elk.edges ?? []).some((edge) => edge.sources.includes("mother") && edge.targets.includes("partner-b")),
  true,
  "ELK keeps a spouse on the same layer as the blood relative",
);

function blankEdge(id, source, target, role) {
  return {
    id,
    source,
    target,
    relationshipId: null,
    role,
    parentKind: null,
    partnerKind: null,
    label: "",
    arrow: role === "child",
    start: null,
    end: null,
  };
}

const snapped = placeUnions({
  generation: 1,
  nodes: [
    { id: "a", kind: "person", personId: "a", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 0, y: 10 },
    { id: "b", kind: "person", personId: "b", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 520, y: 40 },
    { id: "u", kind: "union", memberIds: ["a", "b"], width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT, x: 0, y: 300 },
    { id: "c", kind: "person", personId: "c", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 800, y: 400 },
  ],
  edges: [
    blankEdge("p1", "a", "u", "partner"),
    blankEdge("p2", "b", "u", "partner"),
    blankEdge("k1", "u", "c", "child"),
  ],
});
const placedA = snapped.nodes.find((node) => node.id === "a");
const placedB = snapped.nodes.find((node) => node.id === "b");
const placedU = snapped.nodes.find((node) => node.id === "u");
const placedC = snapped.nodes.find((node) => node.id === "c");
assert.equal(placedA.y, placedB.y, "spouses share a row");
assert.equal(placedU.x > placedA.x + placedA.width, true, "union sits after the left spouse");
assert.equal(placedU.x + placedU.width < placedB.x, true, "union sits before the right spouse");
assert.equal(placedC.y > placedA.y + placedA.height, true, "child sits below the marriage");
assert.ok(
  Math.abs(placedC.x + placedC.width / 2 - (placedU.x + placedU.width / 2)) < 1,
  "child is centered under the union",
);

const crowded = placeUnions({
  generation: 1,
  nodes: [
    { id: "a", kind: "person", personId: "a", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 0, y: 0 },
    { id: "b", kind: "person", personId: "b", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 180, y: 0 },
    { id: "u1", kind: "union", memberIds: ["a", "b"], width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT, x: 0, y: 0 },
    { id: "c", kind: "person", personId: "c", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 200, y: 0 },
    { id: "d", kind: "person", personId: "d", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 380, y: 0 },
    { id: "u2", kind: "union", memberIds: ["c", "d"], width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT, x: 0, y: 0 },
  ],
  edges: [
    blankEdge("p1", "a", "u1", "partner"),
    blankEdge("p2", "b", "u1", "partner"),
    blankEdge("p3", "c", "u2", "partner"),
    blankEdge("p4", "d", "u2", "partner"),
  ],
});
const crowdedB = crowded.nodes.find((node) => node.id === "b");
const crowdedC = crowded.nodes.find((node) => node.id === "c");
assert.ok(crowdedB.x + crowdedB.width + 40 < crowdedC.x, "neighboring couples keep a gap");

const marriedChild = placeUnions({
  generation: 1,
  nodes: [
    { id: "a", kind: "person", personId: "a", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 0, y: 0 },
    { id: "b", kind: "person", personId: "b", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 400, y: 0 },
    { id: "u1", kind: "union", memberIds: ["a", "b"], width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT, x: 0, y: 0 },
    { id: "c", kind: "person", personId: "c", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 80, y: 300 },
    { id: "d", kind: "person", personId: "d", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 520, y: 300 },
    { id: "u2", kind: "union", memberIds: ["c", "d"], width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT, x: 0, y: 0 },
  ],
  edges: [
    blankEdge("p1", "a", "u1", "partner"),
    blankEdge("p2", "b", "u1", "partner"),
    blankEdge("k1", "u1", "c", "child"),
    blankEdge("p3", "c", "u2", "partner"),
    blankEdge("p4", "d", "u2", "partner"),
  ],
});
const parentUnion = marriedChild.nodes.find((node) => node.id === "u1");
const childSpouse = marriedChild.nodes.find((node) => node.id === "c");
const inLaw = marriedChild.nodes.find((node) => node.id === "d");
assert.equal(childSpouse.y, inLaw.y, "married child stays on the spouse row");
assert.ok(
  Math.abs(childSpouse.x + childSpouse.width / 2 - (parentUnion.x + parentUnion.width / 2)) > 20,
  "married child is not forced under the parent union",
);

const inLawFloated = placeUnions({
  generation: 1,
  nodes: [
    { id: "g", kind: "person", personId: "g", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 0, y: 0 },
    { id: "a", kind: "person", personId: "a", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 0, y: 200 },
    { id: "b", kind: "person", personId: "b", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 400, y: 0 },
    { id: "u", kind: "union", memberIds: ["a", "b"], width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT, x: 0, y: 0 },
    { id: "c", kind: "person", personId: "c", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 80, y: 400 },
  ],
  edges: [
    blankEdge("d1", "g", "a", "direct-parent"),
    blankEdge("p1", "a", "u", "partner"),
    blankEdge("p2", "b", "u", "partner"),
    blankEdge("k1", "u", "c", "child"),
  ],
});
const floatedG = inLawFloated.nodes.find((node) => node.id === "g");
const floatedA = inLawFloated.nodes.find((node) => node.id === "a");
const floatedB = inLawFloated.nodes.find((node) => node.id === "b");
const floatedC = inLawFloated.nodes.find((node) => node.id === "c");
assert.equal(floatedA.y, floatedB.y, "in-law shares the blood relative row");
assert.ok(floatedA.y > floatedG.y, "adding a partner does not lift the couple above their parents");
assert.ok(floatedC.y > floatedA.y + floatedA.height, "children stay below the marriage");

const coparents = placeUnions({
  generation: 1,
  nodes: [
    { id: "a", kind: "person", personId: "a", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 0, y: 0 },
    { id: "b", kind: "person", personId: "b", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 400, y: 0 },
    { id: "u", kind: "union", memberIds: ["a", "b"], width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT, x: 0, y: 0 },
    { id: "k", kind: "person", personId: "k", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 80, y: 300 },
  ],
  edges: [blankEdge("p1", "a", "u", "parent"), blankEdge("p2", "b", "u", "parent"), blankEdge("k1", "u", "k", "child")],
});
const coparentLeft = coparents.nodes.find((node) => node.id === "a");
const coparentUnion = coparents.nodes.find((node) => node.id === "u");
const coparentRight = coparents.nodes.find((node) => node.id === "b");
const leftToUnion = familyEdgeHandles({ role: "parent", source: "a", target: "u" }, coparents.nodes);
const rightToUnion = familyEdgeHandles({ role: "parent", source: "b", target: "u" }, coparents.nodes);
const unionToChild = familyEdgeHandles({ role: "child", source: "u", target: "k" }, coparents.nodes);
assert.equal(coparentLeft.y, coparentRight.y, "unmarried coparents share a row");
assert.ok(coparentUnion.x > coparentLeft.x + coparentLeft.width, "union sits after the left coparent");
assert.ok(coparentUnion.x + coparentUnion.width < coparentRight.x, "union sits before the right coparent");
assert.deepEqual(leftToUnion, { sourceHandle: "east", targetHandle: "west" });
assert.deepEqual(rightToUnion, { sourceHandle: "west", targetHandle: "east" });
assert.deepEqual(unionToChild, { sourceHandle: "south", targetHandle: "north" });

const trio = [
  { id: "a", x: 0, y: 0, width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT },
  { id: "b", x: 240, y: 0, width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT },
  { id: "c", x: 480, y: 0, width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT },
  { id: "u", x: 350, y: PERSON_NODE_HEIGHT + 24, width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT },
];
assert.deepEqual(familyEdgeHandles({ role: "parent", source: "a", target: "u" }, trio), {
  sourceHandle: "south",
  targetHandle: "north",
});

function overlappingPeople(nodes) {
  const people = nodes.filter((node) => node.kind === "person");
  for (let left = 0; left < people.length; left += 1) {
    for (let right = left + 1; right < people.length; right += 1) {
      const a = people[left];
      const b = people[right];
      if (a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y) {
        return [a.id, b.id];
      }
    }
  }
  return null;
}

const stackedSiblings = placeUnions({
  generation: 1,
  nodes: [
    { id: "a", kind: "person", personId: "a", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 0, y: 0 },
    { id: "b", kind: "person", personId: "b", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 400, y: 0 },
    { id: "u1", kind: "union", memberIds: ["a", "b"], width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT, x: 0, y: 0 },
    { id: "c", kind: "person", personId: "c", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 40, y: 170 },
    { id: "d", kind: "person", personId: "d", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 520, y: 190 },
    { id: "u2", kind: "union", memberIds: ["c", "d"], width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT, x: 0, y: 0 },
    { id: "e", kind: "person", personId: "e", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 60, y: 175 },
    { id: "f", kind: "person", personId: "f", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 80, y: 185 },
  ],
  edges: [
    blankEdge("p1", "a", "u1", "partner"),
    blankEdge("p2", "b", "u1", "partner"),
    blankEdge("k1", "u1", "c", "child"),
    blankEdge("k2", "u1", "e", "child"),
    blankEdge("k3", "u1", "f", "child"),
    blankEdge("p3", "c", "u2", "partner"),
    blankEdge("p4", "d", "u2", "partner"),
  ],
});
assert.equal(overlappingPeople(stackedSiblings.nodes), null, "no two person cards share space");

const twoMarriages = placeUnions({
  generation: 1,
  nodes: [
    { id: "a", kind: "person", personId: "a", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 0, y: 0 },
    { id: "b", kind: "person", personId: "b", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 700, y: 0 },
    { id: "c", kind: "person", personId: "c", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 1400, y: 0 },
    { id: "u1", kind: "union", memberIds: ["a", "b"], width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT, x: 0, y: 0 },
    { id: "u2", kind: "union", memberIds: ["b", "c"], width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT, x: 0, y: 0 },
    { id: "k1", kind: "person", personId: "k1", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 40, y: 300 },
    { id: "k2", kind: "person", personId: "k2", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 80, y: 300 },
    { id: "k3", kind: "person", personId: "k3", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 1200, y: 300 },
    { id: "k4", kind: "person", personId: "k4", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 1500, y: 300 },
  ],
  edges: [
    blankEdge("p1", "a", "u1", "partner"),
    blankEdge("p2", "b", "u1", "partner"),
    blankEdge("p3", "b", "u2", "partner"),
    blankEdge("p4", "c", "u2", "partner"),
    blankEdge("c1", "u1", "k1", "child"),
    blankEdge("c2", "u1", "k2", "child"),
    blankEdge("c3", "u2", "k3", "child"),
    blankEdge("c4", "u2", "k4", "child"),
  ],
});
const tmA = twoMarriages.nodes.find((node) => node.id === "a");
const tmB = twoMarriages.nodes.find((node) => node.id === "b");
const tmC = twoMarriages.nodes.find((node) => node.id === "c");
const tmU1 = twoMarriages.nodes.find((node) => node.id === "u1");
const tmU2 = twoMarriages.nodes.find((node) => node.id === "u2");
const tmK1 = twoMarriages.nodes.find((node) => node.id === "k1");
const tmK2 = twoMarriages.nodes.find((node) => node.id === "k2");
const tmK3 = twoMarriages.nodes.find((node) => node.id === "k3");
const tmK4 = twoMarriages.nodes.find((node) => node.id === "k4");
assert.equal(tmA.y, tmB.y, "serial spouses share a row");
assert.equal(tmB.y, tmC.y, "both marriages stay on one row");
assert.ok(tmU1.x > tmA.x + tmA.width && tmU1.x + tmU1.width < tmB.x, "first union sits between its spouses");
assert.ok(tmU2.x > tmB.x + tmB.width && tmU2.x + tmU2.width < tmC.x, "second union sits between its spouses");
assert.ok(tmU1.x - (tmA.x + tmA.width) < 40, "first marriage line does not run through other cards");
assert.ok(tmC.x - (tmU2.x + tmU2.width) < 40, "second marriage line does not run through other cards");
assert.equal(overlappingPeople(twoMarriages.nodes), null, "serial-marriage children do not overlap");
assert.ok(tmK2.x + tmK2.width <= tmK3.x, "children of different marriages do not share an x-range");
assert.ok(
  Math.abs((tmK1.x + tmK2.x + tmK2.width) / 2 - (tmU1.x + tmU1.width / 2)) < 2,
  "first marriage children stay under their union",
);
assert.ok(
  Math.abs((tmK3.x + tmK4.x + tmK4.width) / 2 - (tmU2.x + tmU2.width / 2)) < 2,
  "second marriage children stay under their union",
);

const intruder = placeUnions({
  generation: 1,
  nodes: [
    { id: "a", kind: "person", personId: "a", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 0, y: 0 },
    { id: "b", kind: "person", personId: "b", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 900, y: 0 },
    { id: "u1", kind: "union", memberIds: ["a", "b"], width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT, x: 0, y: 0 },
    { id: "d", kind: "person", personId: "d", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 300, y: 0 },
    { id: "e", kind: "person", personId: "e", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 520, y: 0 },
    { id: "u2", kind: "union", memberIds: ["d", "e"], width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT, x: 0, y: 0 },
    { id: "c", kind: "person", personId: "c", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 1100, y: 0 },
    { id: "u3", kind: "union", memberIds: ["b", "c"], width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT, x: 0, y: 0 },
  ],
  edges: [
    blankEdge("p1", "a", "u1", "partner"),
    blankEdge("p2", "b", "u1", "partner"),
    blankEdge("p3", "d", "u2", "partner"),
    blankEdge("p4", "e", "u2", "partner"),
    blankEdge("p5", "b", "u3", "partner"),
    blankEdge("p6", "c", "u3", "partner"),
  ],
});
const inA = intruder.nodes.find((node) => node.id === "a");
const inC = intruder.nodes.find((node) => node.id === "c");
const inD = intruder.nodes.find((node) => node.id === "d");
const inE = intruder.nodes.find((node) => node.id === "e");
const chainLeft = inA.x;
const chainRight = inC.x + inC.width;
const otherLeft = Math.min(inD.x, inE.x);
const otherRight = Math.max(inD.x + inD.width, inE.x + inE.width);
assert.ok(
  otherRight <= chainLeft || otherLeft >= chainRight,
  "another couple cannot sit inside a serial marriage chain",
);

const manyChildren = placeUnions({
  generation: 1,
  nodes: [
    { id: "z", kind: "person", personId: "z", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 0, y: 0 },
    { id: "k", kind: "person", personId: "k", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 400, y: 0 },
    { id: "f", kind: "person", personId: "f", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 800, y: 0 },
    { id: "u1", kind: "union", memberIds: ["z", "k"], width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT, x: 0, y: 0 },
    { id: "u2", kind: "union", memberIds: ["k", "f"], width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT, x: 0, y: 0 },
    { id: "h", kind: "person", personId: "h", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 40, y: 200 },
    { id: "a", kind: "person", personId: "a", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 200, y: 360 },
    { id: "s", kind: "person", personId: "s", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 480, y: 360 },
    { id: "ua", kind: "union", memberIds: ["a", "s"], width: UNION_NODE_WIDTH, height: UNION_NODE_HEIGHT, x: 0, y: 0 },
    { id: "m1", kind: "person", personId: "m1", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 500, y: 200 },
    { id: "m2", kind: "person", personId: "m2", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 760, y: 200 },
    { id: "m3", kind: "person", personId: "m3", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 1020, y: 200 },
    { id: "m4", kind: "person", personId: "m4", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 1280, y: 200 },
    { id: "m5", kind: "person", personId: "m5", width: PERSON_NODE_WIDTH, height: PERSON_NODE_HEIGHT, x: 1540, y: 200 },
  ],
  edges: [
    blankEdge("p1", "z", "u1", "partner"),
    blankEdge("p2", "k", "u1", "partner"),
    blankEdge("p3", "k", "u2", "partner"),
    blankEdge("p4", "f", "u2", "partner"),
    blankEdge("c1", "u1", "h", "child"),
    blankEdge("c2", "u1", "a", "child"),
    blankEdge("pa", "a", "ua", "partner"),
    blankEdge("ps", "s", "ua", "partner"),
    blankEdge("d1", "u2", "m1", "child"),
    blankEdge("d2", "u2", "m2", "child"),
    blankEdge("d3", "u2", "m3", "child"),
    blankEdge("d4", "u2", "m4", "child"),
    blankEdge("d5", "u2", "m5", "child"),
  ],
});
const mcA = manyChildren.nodes.find((node) => node.id === "a");
const mcH = manyChildren.nodes.find((node) => node.id === "h");
const mcS = manyChildren.nodes.find((node) => node.id === "s");
const mcM1 = manyChildren.nodes.find((node) => node.id === "m1");
const mcM5 = manyChildren.nodes.find((node) => node.id === "m5");
assert.equal(mcA.y, mcH.y, "married child stays on the same row as hanging siblings");
assert.equal(mcA.y, mcM1.y, "married child stays on the same row as the other marriage's children");
assert.equal(mcA.y, mcS.y, "spouse stays on the married child's row");
assert.equal(overlappingPeople(manyChildren.nodes), null, "many children do not overlap cards");
assert.ok(mcA.x + mcA.width <= mcM1.x, "first marriage's children stay left of the second marriage's children");
assert.ok(mcH.x + mcH.width <= mcM1.x, "hanging sibling stays left of the other marriage");
assert.ok(mcM1.x + mcM1.width <= mcM5.x, "second marriage children stay ordered");

const gpa = person("gpa");
const pa = person("pa");
const gpb = person("gpb");
const pb = person("pb");
const cousin = person("cousin");
const kid = person("kid");
const { graph: cousins } = normalizeGenealogy(
  [root, gpa, pa, gpb, pb, cousin, kid],
  [
    parent("c1", gpa.id, pa.id),
    parent("c2", pa.id, root.id),
    parent("c3", gpb.id, pb.id),
    parent("c4", pb.id, cousin.id),
    partner("c5", root.id, cousin.id),
    parent("c6", root.id, kid.id),
    parent("c7", cousin.id, kid.id),
  ],
);
const cousinVisible = initialNeighborhood(cousins, root.id);
assert.equal(cousinVisible.has(cousin.id), true);
assert.equal(cousinVisible.has(kid.id), true);
assert.equal(cousinVisible.has(pa.id), true);
assert.equal(cousinVisible.has(gpa.id), true);
assert.equal(cousinVisible.has(pb.id), false, "cousin marriage does not recurse into the other lineage");
assert.equal(cousinVisible.has(gpb.id), false);

const { warnings: missingWarnings } = normalizeGenealogy(
  [root],
  [parent("missing", "ghost", root.id), partner("gone", root.id, "deleted-person")],
);
assert.equal(
  missingWarnings.some((warning) => warning.relationshipId === "missing"),
  true,
);
assert.equal(
  missingWarnings.some((warning) => warning.relationshipId === "gone"),
  true,
);

assert.equal(personFromRecord({ id: "x", name: "X", revision: "1", type: PERSON_TYPE, deleted: true }, {}), null);

const parsedUnknown = parseFamilyRelationship({
  id: "bad",
  sourceId: "a",
  targetId: "b",
  type: "family_parent_of",
  revision: "1",
  metadata: { kind: "cousin" },
});
assert.equal(parsedUnknown.parsed.unknown, true);
assert.equal(parsedUnknown.parsed.label, "Unknown");
assert.ok(parsedUnknown.warning);

assert.equal(layoutExceedsLimits(VISIBLE_PERSON_LIMIT + 1, 1, 1), true);
assert.equal(layoutExceedsLimits(1, VISIBLE_UNION_LIMIT + 1, 1), true);
assert.equal(layoutExceedsLimits(1, 1, VISIBLE_EDGE_LIMIT + 1), true);
assert.equal(layoutExceedsLimits(VISIBLE_PERSON_LIMIT, VISIBLE_UNION_LIMIT, VISIBLE_EDGE_LIMIT), false);
assert.equal(
  layoutExceedsLimits(300, 1, 1, { visiblePersonLimit: 300, visibleUnionLimit: 200, visibleEdgeLimit: 800 }),
  false,
);
assert.equal(wouldExceedVisibleLimit(251), true);
assert.equal(wouldExceedVisibleLimit(251, 300), false);
assert.equal(familyTreeLimitsOverBudget(clampFamilyTreeLimits({})), false);
assert.equal(familyTreeLimitsOverBudget(clampFamilyTreeLimits({ ancestorGenerations: 4 })), true);
assert.equal(clampFamilyTreeLimits({ ancestorGenerations: 99 }).ancestorGenerations, 12);
assert.equal(clampFamilyTreeLimits({ visiblePersonLimit: 500 }).visibleUnionLimit, 300);
const memory = { value: "" };
const fakeStorage = {
  getItem: () => memory.value || null,
  setItem: (_key, value) => {
    memory.value = value;
  },
};
assert.equal(writeFamilyTreeLimits({ ancestorGenerations: 4 }, fakeStorage).ancestorGenerations, 4);
assert.equal(readFamilyTreeLimits(fakeStorage).ancestorGenerations, 4);
assert.equal(layoutGraphExceedsLimits(layout), false);
assert.equal(truncationWarning(0).message.includes("99+"), true);
assert.equal(truncationWarning(1200).message.includes("1200+"), true);

replaceRecentRoots("proj", []);
assert.deepEqual(rememberRecentRoot("proj", "a"), ["a"]);
assert.deepEqual(rememberRecentRoot("proj", "b"), ["b", "a"]);
for (let index = 0; index < 12; index += 1) rememberRecentRoot("proj", `r${index}`);
assert.equal(recentRoots("proj").length, 10);
assert.equal(recentRoots("proj")[0], "r11");

const generations = new LayoutGeneration();
const first = generations.start();
const second = generations.start();
assert.equal(isCurrentGeneration(second, first), false);
assert.equal(generations.accept(first), false);
assert.equal(generations.accept(second), true);
assert.equal(PERSON_TYPE, "daena.lore:person");

function fakeContext(peopleRecords, relationships) {
  const calls = [];
  let inflight = 0;
  let maxInflight = 0;
  return {
    calls,
    maxInflight: () => maxInflight,
    relationships: {
      query: async ({ entityIds, relationshipTypes, direction, offset = 0, limit = 200 }) => {
        calls.push({
          entityIds: [...entityIds].sort(),
          relationshipTypes: [...(relationshipTypes ?? [])],
          direction,
        });
        const items = relationships
          .filter((relationship) => {
            if (relationshipTypes?.length && !relationshipTypes.includes(relationship.type)) return false;
            if (direction === "incoming") return entityIds.includes(relationship.targetId);
            if (direction === "outgoing") return entityIds.includes(relationship.sourceId);
            return entityIds.includes(relationship.sourceId) || entityIds.includes(relationship.targetId);
          })
          .sort((left, right) => left.id.localeCompare(right.id));
        const page = items.slice(offset, offset + limit);
        return { items: page, total: items.length, offset, limit, hasMore: offset + page.length < items.length };
      },
    },
    entities: {
      getMany: async (ids) =>
        peopleRecords
          .filter((record) => ids.includes(record.id))
          .map((record) => ({
            id: record.id,
            name: record.name,
            type: PERSON_TYPE,
            deleted: false,
            revision: "1",
          })),
    },
    fields: {
      listShared: async () => {
        inflight += 1;
        maxInflight = Math.max(maxInflight, inflight);
        await Promise.resolve();
        inflight -= 1;
        return [];
      },
    },
  };
}

const fetched = fakeContext(
  [root, mother, father, child, grandchild, partnerB, childless, grandparent, sibling, half, coparent],
  [
    parent("p1", mother.id, root.id),
    parent("p2", father.id, root.id),
    parent("p7", root.id, child.id),
    parent("p8", partnerB.id, child.id),
    parent("p9", child.id, grandchild.id),
    parent("p10", grandparent.id, mother.id),
    partner("r2", root.id, partnerB.id),
    partner("r5", root.id, childless.id, "partnership", "active"),
  ],
);
const neighborhood = await loadGenealogyNeighborhood(fetched, root.id);
assert.equal(
  fetched.calls.some((call) => call.direction === "any" && call.relationshipTypes.includes("family_parent_of")),
  false,
  "parent walk is directed, not three-hop any",
);
assert.equal(
  fetched.calls.some((call) => call.direction === "incoming" && call.relationshipTypes.includes("family_parent_of")),
  true,
);
assert.equal(
  fetched.calls.some((call) => call.direction === "outgoing" && call.relationshipTypes.includes("family_parent_of")),
  true,
);
assert.equal(
  neighborhood.people.some((entry) => entry.id === childless.id),
  true,
);
assert.equal(
  neighborhood.relationships.some((relationship) => relationship.id === "p8"),
  true,
  "co-parent links on known children are fetched",
);
assert.ok(fetched.maxInflight() >= 2, "shared fields hydrate in parallel");

const aborted = fakeContext([root], []);
const controller = new AbortController();
controller.abort();
await assert.rejects(() => loadGenealogyNeighborhood(aborted, root.id, "occupation", controller.signal));

const seeded = seedInitialExpansions(graph, root.id);
const fromKeys = visibleFromExpansions(graph, root.id, seeded);
assert.deepEqual([...fromKeys.visible].sort(), [...visible].sort(), "seeded expansions match the initial neighborhood");

const withoutSiblings = [...seeded].filter((key) => key !== expansionKey(root.id, "siblings"));
const afterSiblingCollapse = visibleFromExpansions(graph, root.id, withoutSiblings);
assert.equal(
  afterSiblingCollapse.visible.has(sibling.id),
  true,
  "shared parent-child path keeps siblings after collapsing the siblings key",
);
assert.equal((afterSiblingCollapse.refs.get(sibling.id) ?? 0) > 0, true);

const withoutParentChildren = withoutSiblings.filter(
  (key) =>
    key !== expansionKey(mother.id, "children") &&
    key !== expansionKey(father.id, "children") &&
    key !== expansionKey(coparent.id, "children"),
);
const isolated = visibleFromExpansions(graph, root.id, withoutParentChildren);
assert.equal(isolated.visible.has(sibling.id), false, "collapsing every remaining sibling path hides the sibling");
assert.equal(isolated.visible.has(root.id), true);

const counts = hiddenCounts(graph, root.id, isolated.visible);
assert.equal(counts.siblings > 0, true);
assert.equal(hiddenCounts(graph, root.id, isolated.visible, true, 99).truncated, true);
assert.equal(hiddenCounts(graph, root.id, isolated.visible, true, 99).lowerBound, 99);
const endedHidden = hiddenCounts(graph, root.id, new Set([root.id]));
assert.equal(endedHidden.partners > 0, true, "expandable partners include ended partners");

assert.equal(wouldCreateParentCycle(graph, child.id, root.id), true);
assert.equal(wouldCreateParentCycle(graph, outsider.id, root.id), false);
assert.equal(wouldCreateParentCycle(graph, root.id, root.id), true);
assert.deepEqual(parentCyclePath(graph, child.id, root.id), [root.id, child.id]);
assert.equal(
  formatParentCycleMessage([root.id, child.id], (id) => graph.people.get(id)?.name ?? id),
  "That parent link would create a cycle: Root → Child.",
);
assert.equal(wouldCreateDuplicate(graph, "parent", root.id, mother.id), true);
assert.equal(wouldCreateDuplicate(graph, "partner", root.id, partnerB.id), true);
assert.equal(wouldCreateDuplicate(graph, "child", root.id, outsider.id), false);
assert.equal(expansionBlocked(graph, root.id, grandchild.id, "children"), false);

const deep = [];
const deepPeople = [person("d0")];
for (let index = 1; index <= 8; index += 1) {
  deepPeople.push(person(`d${index}`));
  deep.push(parent(`deep${index}`, `d${index - 1}`, `d${index}`));
}
const deepGraph = normalizeGenealogy(deepPeople, deep).graph;
assert.equal(generationDistance(deepGraph, "d0", "d8", "children"), 8);
assert.equal(expansionBlocked(deepGraph, "d0", "d6", "children"), true);
assert.equal(expansionBlocked(deepGraph, "d0", "d5", "children"), false);
assert.equal(expansionBlocked(deepGraph, "d0", "d6", "children", 8), false);

const g3 = person("g3");
const g2 = person("g2");
const g1 = person("g1");
const r = person("r");
const c1 = person("c1");
const c2 = person("c2");
const { graph: chain } = normalizeGenealogy(
  [g3, g2, g1, r, c1, c2],
  [
    parent("n1", g3.id, g2.id),
    parent("n2", g2.id, g1.id),
    parent("n3", g1.id, r.id),
    parent("n4", r.id, c1.id),
    parent("n5", c1.id, c2.id),
  ],
);
const twoHop = initialNeighborhood(chain, r.id, 2, 2);
assert.equal(twoHop.has(g3.id), false);
assert.equal(twoHop.has(g2.id), true);
assert.equal(twoHop.has(c2.id), true);
const threeHop = initialNeighborhood(chain, r.id, 3, 3);
assert.equal(threeHop.has(g3.id), true);
assert.deepEqual(
  [...visibleFromExpansions(chain, r.id, seedInitialExpansions(chain, r.id, 3, 3)).visible].sort(),
  [...threeHop].sort(),
);

assert.equal(classifyMutationError(new Error("relationship would introduce a cycle")).code, "relationship.cycle");
assert.equal(classifyMutationError({ code: "relationship.cycle", message: "cycle" }).code, "relationship.cycle");
assert.equal(classifyMutationError(new Error("relationship.duplicate: already exists")).code, "relationship.duplicate");
assert.equal(
  classifyMutationError(new Error("a relationship already exists for these endpoints")).code,
  "relationship.duplicate",
);
assert.equal(
  classifyMutationError(new Error("relationship revision conflict: expected 1, current 2")).code,
  "revision-conflict",
);

const sessionA = {
  expansions: ["root:parents"],
  selectedPersonId: "root",
  selectedRelationshipId: null,
  viewport: { x: 0, y: 0, zoom: 1 },
};
const sessionB = { ...sessionA, viewport: { x: 8, y: 2, zoom: 1.2 } };
assert.equal(familyTreeHistoryKey(sessionA), familyTreeHistoryKey(sessionB));
assert.notEqual(familyTreeHistoryKey(sessionA), familyTreeHistoryKey({ ...sessionA, houseId: "h1" }));
assert.equal(sameFamilyTreeSession(sessionA, sessionB), false);
assert.equal(sameFamilyTreeSession(sessionA, { ...sessionA, viewport: { x: 0, y: 0, zoom: 1 } }), true);

const siblingContext = fakeContext(
  [root, mother, sibling],
  [parent("p1", mother.id, root.id), parent("p4", mother.id, sibling.id)],
);
const already = new Map([["p1", parent("p1", mother.id, root.id)]]);
const siblingLayer = await loadExpansionLayer(
  siblingContext,
  root.id,
  "siblings",
  already,
  new Set([root.id, mother.id]),
);
assert.equal(
  siblingLayer.people.some((entry) => entry.id === sibling.id),
  true,
  "sibling expansion uses collected parents, not only newly added incoming rows",
);
assert.equal(
  siblingContext.calls.some((call) => call.direction === "outgoing" && call.entityIds.includes(mother.id)),
  true,
);

const houseContext = fakeContext(
  [person("p1", "Ada"), person("p2", "Bea")],
  [
    {
      id: "m1",
      sourceId: "p1",
      targetId: "h1",
      type: MEMBERSHIP_RELATIONSHIP,
      metadata: {},
      revision: "1",
    },
    {
      id: "m2",
      sourceId: "p2",
      targetId: "h1",
      type: MEMBERSHIP_RELATIONSHIP,
      metadata: {},
      revision: "1",
    },
  ],
);
const membershipCounts = await houseMemberCounts(houseContext, ["h1", "h2"]);
assert.equal(membershipCounts.get("h1"), 2);
assert.equal(membershipCounts.get("h2"), 0);
const members = await listHouseMembers(houseContext, "h1");
assert.deepEqual(
  members.map((entry) => entry.name),
  ["Ada", "Bea"],
);

const houseTreeContext = fakeContext(
  [person("p1", "Ada"), person("p2", "Bea"), person("p3", "Cal"), person("p4", "Dee")],
  [
    {
      id: "m1",
      sourceId: "p1",
      targetId: "h1",
      type: MEMBERSHIP_RELATIONSHIP,
      metadata: {},
      revision: "1",
    },
    {
      id: "m2",
      sourceId: "p2",
      targetId: "h1",
      type: MEMBERSHIP_RELATIONSHIP,
      metadata: {},
      revision: "1",
    },
    parent("hp1", "p1", "p2"),
    partner("hr1", "p1", "p3"),
    parent("hp2", "p4", "p1"),
  ],
);
const houseTree = await loadHouseNeighborhood(houseTreeContext, "h1");
assert.deepEqual(houseTree.people.map((entry) => entry.id).sort(), ["p1", "p2"]);
assert.deepEqual(
  houseTree.relationships.map((entry) => entry.id),
  ["hp1"],
);
assert.deepEqual(houseTree.memberIds.sort(), ["p1", "p2"]);
assert.equal(houseTree.scopeTruncated, false);

const houseTreeWide = await loadHouseNeighborhood(houseTreeContext, "h1", "occupation", undefined, {
  scope: "members-plus-immediate-family",
});
assert.deepEqual(houseTreeWide.people.map((entry) => entry.id).sort(), ["p1", "p2", "p3", "p4"]);
assert.deepEqual(houseTreeWide.relationships.map((entry) => entry.id).sort(), ["hp1", "hp2", "hr1"]);
assert.deepEqual(houseTreeWide.memberIds.sort(), ["p1", "p2"]);

const houseTreeCapped = await loadHouseNeighborhood(houseTreeContext, "h1", "occupation", undefined, {
  scope: "members-plus-immediate-family",
  visiblePersonLimit: 3,
});
assert.equal(houseTreeCapped.people.length, 3);
assert.equal(houseTreeCapped.scopeTruncated, true);
assert.ok(
  houseTreeCapped.people.every((entry) => ["p1", "p2"].includes(entry.id) || entry.id === "p3" || entry.id === "p4"),
);
assert.ok(houseTreeCapped.people.some((entry) => entry.id === "p1"));
assert.ok(houseTreeCapped.people.some((entry) => entry.id === "p2"));

console.log("family-tree projection checks passed");
