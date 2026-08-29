import assert from "node:assert/strict";
import { loadGenealogyNeighborhood } from "../src/lib/family-tree/fetch.ts";
import { LayoutGeneration, isCurrentGeneration } from "../src/lib/family-tree/layout.ts";
import {
  PERSON_TYPE,
  VISIBLE_EDGE_LIMIT,
  VISIBLE_PERSON_LIMIT,
  VISIBLE_UNION_LIMIT,
  layoutExceedsLimits,
  truncationWarning,
} from "../src/lib/family-tree/model.ts";
import {
  initialNeighborhood,
  normalizeGenealogy,
  parseFamilyRelationship,
  personFromRecord,
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
assert.ok(fetched.maxInflight() >= 2, "shared fields hydrate in parallel");

const aborted = fakeContext([root], []);
const controller = new AbortController();
controller.abort();
await assert.rejects(() => loadGenealogyNeighborhood(aborted, root.id, "occupation", controller.signal));

console.log("family-tree projection checks passed");
