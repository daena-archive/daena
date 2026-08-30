import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const read = (path) => readFile(resolve(root, path), "utf8");

const {
  UI_UX_SCENARIOS,
  SURFACE_IDS,
  EMPTY_PROJECT,
  LARGE_PROJECT,
  DISCONNECTED_HOUSE,
  MULTIPLE_MEMBERSHIPS,
  MALFORMED_EDGE,
  CUSTOM_SCHEMA_LIVE_DATA,
  REVISION_CONFLICT,
  synthesizeLargeProjectPeople,
  synthesizeLargeProjectHouses,
  scenarioById,
  CONTEXTUAL_NEW_DEFAULTS,
  TREE_KEYBOARD_CONTRACT,
} = await import("../src/lib/ui-ux/fixtures.ts");

const { ENTITY_ACTIONS, ENTITY_ACTION_CONFIRM, MUTATION_STATUS, MUTATION_STATUS_MESSAGES, TREE_KEYBOARD, TREE_SCOPES } =
  await import("../src/lib/ui-ux/vocabulary.ts");

const requiredScenarioIds = [
  "empty-project",
  "large-project",
  "disconnected-house",
  "multiple-memberships",
  "malformed-edge",
  "custom-schema-live-data",
  "revision-conflict",
];

assert.equal(UI_UX_SCENARIOS.length, requiredScenarioIds.length);
for (const id of requiredScenarioIds) {
  assert.ok(scenarioById(id), `scenario ${id} exists`);
}

assert.equal(EMPTY_PROJECT.entities.length, 0);
assert.equal(EMPTY_PROJECT.relationships.length, 0);

assert.equal(LARGE_PROJECT.scale?.personCount, 10_000);
assert.equal(LARGE_PROJECT.scale?.houseCount, 200);
assert.equal(synthesizeLargeProjectPeople(5).length, 5);
assert.equal(synthesizeLargeProjectHouses(3)[2]?.name, "House 3");

assert.equal(DISCONNECTED_HOUSE.relationships.filter((r) => r.type === "family_member_of").length, 3);
assert.equal(
  DISCONNECTED_HOUSE.relationships.filter((r) => r.type === "family_parent_of" || r.type === "family_partner_with")
    .length,
  0,
);

const ariaMemberships = MULTIPLE_MEMBERSHIPS.relationships.filter(
  (r) => r.sourceId === "p-aria" && r.type === "family_member_of",
);
assert.equal(ariaMemberships.length, 2);
assert.ok(ariaMemberships.some((r) => r.metadata?.role === "head"));
assert.ok(ariaMemberships.some((r) => r.metadata?.role === "consort"));

assert.ok(MALFORMED_EDGE.relationships.some((r) => r.malformed && r.malformedReason === "self-parent"));
assert.ok(MALFORMED_EDGE.relationships.some((r) => r.malformed && r.malformedReason === "missing-endpoint"));
assert.ok(MALFORMED_EDGE.relationships.some((r) => !r.malformed));

assert.equal(CUSTOM_SCHEMA_LIVE_DATA.schemaOverlays?.[0]?.revision, "overlay-rev-7");
assert.equal(CUSTOM_SCHEMA_LIVE_DATA.entities.filter((e) => e.type === "daena.lore:knightly-order").length, 2);

assert.equal(REVISION_CONFLICT.mutationProbe?.expectedStatus, MUTATION_STATUS.conflict);
assert.match(REVISION_CONFLICT.mutationProbe?.expectedMessageIncludes ?? "", /revision-conflict/);

for (const scenario of UI_UX_SCENARIOS) {
  assert.ok(scenario.title.trim());
  assert.ok(scenario.purpose.trim());
  assert.ok(scenario.surfaces.length > 0);
  assert.ok(scenario.expectedObservations.length > 0);
  for (const surface of scenario.surfaces) {
    assert.ok(SURFACE_IDS.includes(surface), `${scenario.id} references known surface ${surface}`);
  }
}

const requiredActions = [
  "new",
  "open",
  "openIn",
  "editIdentity",
  "archive",
  "viewArchive",
  "restore",
  "deletePermanently",
  "openTree",
  "openInLore",
  "makeRoot",
  "newPerson",
  "newHouse",
];
for (const action of requiredActions) {
  assert.equal(typeof ENTITY_ACTIONS[action], "string");
  assert.ok(ENTITY_ACTIONS[action].length > 0);
}

assert.equal(ENTITY_ACTIONS.editIdentity, "Edit identity");
assert.equal(ENTITY_ACTIONS.viewArchive, "View Archive");
assert.equal(ENTITY_ACTIONS.deletePermanently, "Delete permanently");
assert.equal(ENTITY_ACTION_CONFIRM.archiveConfirm, "Archive");
assert.match(ENTITY_ACTION_CONFIRM.removeMembershipMessage, /remains in Lore/);

assert.equal(MUTATION_STATUS.saving, "Saving…");
assert.equal(MUTATION_STATUS_MESSAGES.conflictReload, "Reload current values");
assert.equal(MUTATION_STATUS_MESSAGES.revisionConflictCode, "revision-conflict");

assert.deepEqual(TREE_KEYBOARD.keys.moveSelection, ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"]);
assert.equal(TREE_KEYBOARD.keys.openPersonDock, "Enter");
assert.equal(TREE_KEYBOARD.keys.makeRoot, "Shift+Enter");
assert.equal(TREE_KEYBOARD.keys.closeDockOrPopover, "Escape");
assert.equal(TREE_KEYBOARD_CONTRACT.canvasAriaLabel, "Family tree canvas");
assert.equal(TREE_SCOPES.membersOnly.id, "members-only");
assert.equal(TREE_SCOPES.membersPlusImmediateFamily.id, "members-plus-immediate-family");

assert.ok(CONTEXTUAL_NEW_DEFAULTS.lore);
assert.deepEqual(CONTEXTUAL_NEW_DEFAULTS["houses.tree"], [ENTITY_ACTIONS.newPerson, ENTITY_ACTIONS.newHouse]);
assert.equal(CONTEXTUAL_NEW_DEFAULTS.maps, "provider-menu");

const docs = await Promise.all([
  read("docs/ui-ux-slice0/README.md"),
  read("docs/ui-ux-slice0/INTERACTION_SPEC.md"),
  read("docs/ui-ux-slice0/SURFACES.md"),
  read("docs/ui-ux-slice0/BASELINE_A11Y.md"),
  read("docs/TEMP_UI_UX_ENTITY_SCHEMA_PLAN.md"),
]);

for (const [index, name] of ["README", "INTERACTION_SPEC", "SURFACES", "BASELINE_A11Y"].entries()) {
  assert.ok(docs[index].includes("Slice 0") || docs[index].includes("slice 0") || name === "SURFACES");
}

assert.match(docs[1], /Edit identity/);
assert.match(docs[1], /Shift\+Enter/);
assert.match(docs[1], /Members \+ immediate family/);
assert.match(docs[2], /workspace\.houses\.tree\.landing/);
assert.match(docs[2], /project\.fields\.types/);
assert.match(docs[3], /A11Y-TREE-01/);
assert.match(docs[3], /role="application"/);
assert.match(docs[4], /docs\/ui-ux-slice0/);

const requiredSurfaceDocs = [
  "workspace.lore.library",
  "workspace.houses.tree.landing",
  "workspace.houses.tree.open-house",
  "project.fields.types",
  "project.fields.fields",
  "project.fields.templates",
];
for (const surface of requiredSurfaceDocs) {
  assert.match(docs[2], new RegExp(surface.replaceAll(".", "\\.")));
}

console.log(`ui-ux-slice0: ${UI_UX_SCENARIOS.length} scenarios, ${SURFACE_IDS.length} surfaces ok`);
