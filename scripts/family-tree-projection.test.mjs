import assert from "node:assert/strict";
import { loadExpansionLayer, loadGenealogyNeighborhood } from "../src/lib/family-tree/fetch.ts";
import { LayoutGeneration, buildElkGraph, isCurrentGeneration, placeUnions } from "../src/lib/family-tree/layout.ts";
import {
  PERSON_NODE_HEIGHT,
  PERSON_NODE_WIDTH,
  PERSON_TYPE,
  UNION_NODE_HEIGHT,
  UNION_NODE_WIDTH,
  VISIBLE_EDGE_LIMIT,
  VISIBLE_PERSON_LIMIT,
  VISIBLE_UNION_LIMIT,
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
} from "../src/lib/family-tree/projection.ts";
import { rememberRecentRoot, recentRoots, replaceRecentRoots } from "../src/lib/family-tree/state.ts";
import { buildLayoutGraph, layoutGraphExceedsLimits } from "../src/lib/family-tree/unions.ts";

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

console.log("family-tree projection checks passed");
