import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { compile } from "svelte/compiler";

const root = resolve(import.meta.dirname, "..");
const read = (path) => readFile(resolve(root, path), "utf8");

const {
  createRequestGate,
  filterExcludedOptions,
  toAsyncEntityPage,
  toShellSortField,
  toShellSortDirection,
  runAsyncEntitySearch,
  emptyAsyncEntityPage,
} = await import("../src/lib/entity-lifecycle/asyncEntityQuery.ts");

const componentPaths = [
  "src/lib/entity-lifecycle/AsyncEntityPicker.svelte",
  "src/lib/RelationshipPicker.svelte",
  "src/lib/houses/FamilyRootPicker.svelte",
  "src/lib/houses/FamilyMemberDialog.svelte",
  "src/lib/editor/EntityReferenceDialog.svelte",
];

for (const path of componentPaths) {
  const source = await read(path);
  compile(source, { filename: resolve(root, path), css: "injected" });
}

const gate = createRequestGate();
const first = gate.next();
const second = gate.next();
assert.equal(gate.isCurrent(first), false);
assert.equal(gate.isCurrent(second), true);

const filtered = filterExcludedOptions(
  [
    { id: "a", name: "A", deleted: false },
    { id: "b", name: "B", deleted: true },
    { id: "c", name: "C", deleted: false },
  ],
  ["c"],
);
assert.deepEqual(
  filtered.map((item) => item.id),
  ["a"],
);

const page = toAsyncEntityPage(
  {
    items: [
      { id: "1", name: "Aria", entity_type: "daena.lore:person", deleted: false, revision: "r1" },
      { id: "2", name: "Bo", type: "daena.lore:person", deleted: false, revision: "r2" },
      { id: "3", name: "Gone", entity_type: "daena.lore:person", deleted: true, revision: "r3" },
    ],
    total: 10000,
    offset: 0,
    limit: 20,
    has_more: true,
  },
  { excludeIds: ["2"] },
);
assert.equal(page.items.length, 1);
assert.equal(page.items[0].name, "Aria");
assert.equal(page.items[0].entityType, "daena.lore:person");
assert.equal(page.total, 10000);
assert.equal(page.hasMore, true);

// Prefer server has_more even when client filtering shrinks the page.
const filteredLast = toAsyncEntityPage(
  {
    items: [
      { id: "keep", name: "Keep", entity_type: "person", deleted: false },
      { id: "drop", name: "Drop", entity_type: "person", deleted: false },
    ],
    total: 2,
    offset: 0,
    limit: 20,
    has_more: false,
  },
  { excludeIds: ["drop"] },
);
assert.equal(filteredLast.items.length, 1);
assert.equal(filteredLast.hasMore, false);

assert.equal(toShellSortField("updatedAt"), "updated_at");
assert.equal(toShellSortField("relevance"), "relevance");
assert.equal(toShellSortDirection("desc"), "desc");
assert.equal(toShellSortDirection(undefined), "asc");

let calls = 0;
const slow = runAsyncEntitySearch(
  gate,
  async () => {
    calls += 1;
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 30));
    return { items: [{ id: "late", name: "Late" }], total: 1, offset: 0, limit: 20, hasMore: false };
  },
  { text: "late", offset: 0, limit: 20 },
);
const fast = runAsyncEntitySearch(
  gate,
  async () => {
    calls += 1;
    return {
      items: Array.from({ length: 20 }, (_, index) => ({
        id: `e${index}`,
        name: `Entity ${index}`,
        entityType: "daena.lore:person",
      })),
      total: 10000,
      offset: 0,
      limit: 20,
      hasMore: true,
    };
  },
  { text: "fast", offset: 0, limit: 20, entityTypes: ["daena.lore:person"], excludeIds: ["e0"] },
);
const [slowResult, fastResult] = await Promise.all([slow, fast]);
assert.equal("stale" in slowResult, true);
assert.equal("page" in fastResult, true);
if ("page" in fastResult) {
  assert.equal(fastResult.page.items.length, 19);
  assert.equal(fastResult.page.total, 10000);
  assert.ok(!fastResult.page.items.some((item) => item.id === "e0"));
}
assert.equal(calls, 2);
assert.deepEqual(emptyAsyncEntityPage(25).limit, 25);

const shell = await read("src/routes/+page.svelte");
const relationship = await read("src/lib/RelationshipPicker.svelte");
const rootPicker = await read("src/lib/houses/FamilyRootPicker.svelte");
const memberDialog = await read("src/lib/houses/FamilyMemberDialog.svelte");
const picker = await read("src/lib/entity-lifecycle/AsyncEntityPicker.svelte");
const referenceDialog = await read("src/lib/editor/EntityReferenceDialog.svelte");
const richText = await read("src/lib/editor/RichTextEditor.svelte");

assert.match(picker, /runAsyncEntitySearch/);
assert.match(picker, /createRequestGate|gate/);
assert.match(picker, /role="combobox"/);
assert.match(picker, /aria-activedescendant/);
assert.match(picker, /aria-controls=\{listboxId\}/);
assert.match(relationship, /AsyncEntityPicker/);
assert.match(relationship, /excludedEntityTypes/);
assert.match(relationship, /search:/);
assert.doesNotMatch(relationship, /entities:\s*Entity\[\]/);
assert.match(rootPicker, /AsyncEntityPicker/);
assert.match(rootPicker, /toAsyncEntityPage/);
assert.match(memberDialog, /AsyncEntityPicker/);
assert.match(referenceDialog, /runAsyncEntitySearch/);
assert.match(referenceDialog, /AsyncEntitySearchFn/);
assert.doesNotMatch(referenceDialog, /entities\.filter/);
assert.match(richText, /searchEntities/);
assert.match(shell, /searchEntitiesPaged/);
assert.match(shell, /toShellSortField/);
assert.match(shell, /resolveSelectedEntities/);
assert.match(shell, /Promise\.all/);
assert.match(shell, /collectionRefreshEpoch/);
assert.match(shell, /refreshAfterEntityMutation/);
assert.match(shell, /bumpCollectionRefresh/);
assert.match(shell, /queueCollectionScroll/);
assert.match(shell, /searchEntities=\{searchEntitiesPaged\(\)\}/);
assert.doesNotMatch(shell, /RelationshipPicker[\s\S]{0,120}\{entities\}/);

// Hot mutation / map paths must not rematerialize the full entity list.
const archiveFn = shell.slice(
  shell.indexOf("async function archiveEntity"),
  shell.indexOf("async function archiveSelected"),
);
assert.doesNotMatch(archiveFn, /loadEntities\(\)/);
assert.match(archiveFn, /refreshAfterEntityMutation/);

const identitySave = shell.slice(
  shell.indexOf("async function saveEntityEditDialog"),
  shell.indexOf("async function renameSelected"),
);
assert.doesNotMatch(identitySave, /loadEntities\(\)/);
assert.match(identitySave, /refreshAfterEntityMutation/);

assert.doesNotMatch(shell, /status === "saved"[\s\S]{0,200}listEntities\(\)/);
assert.doesNotMatch(shell, /status === "linked"[\s\S]{0,200}listEntities\(\)/);
assert.doesNotMatch(shell, /maps-navigation[\s\S]{0,250}loadEntities\(\)/);
assert.doesNotMatch(shell, /section !== "language"[\s\S]{0,200}listEntities\(\)/);
assert.doesNotMatch(shell, /onEntityDeleted:\s*loadEntities/);
assert.match(shell, /onEntityDeleted:\s*\(\)\s*=>\s*\{[\s\S]*refreshAfterEntityMutation/);

// searchEntitiesPaged forwards sort + excluded types into queryEntities.
const searchFn = shell.slice(
  shell.indexOf("function searchEntitiesPaged"),
  shell.indexOf("async function resolveSelectedEntities"),
);
assert.match(searchFn, /toShellSortField\(query\.sortField\)/);
assert.match(searchFn, /toShellSortDirection\(query\.sortDirection\)/);
assert.match(searchFn, /excludedEntityTypes:\s*query\.excludedEntityTypes/);

console.log("async entity picker contracts passed");
