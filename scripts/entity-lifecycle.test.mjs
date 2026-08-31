import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { compile } from "svelte/compiler";

const root = resolve(import.meta.dirname, "..");
const read = (path) => readFile(resolve(root, path), "utf8");

const { ENTITY_ACTIONS, ENTITY_ACTION_CONFIRM, MUTATION_STATUS, MUTATION_STATUS_MESSAGES } =
  await import("../src/lib/entity-lifecycle/vocabulary.ts");
const { archiveConfirmOptions, archivePendingLabel, archivedToastMessage } =
  await import("../src/lib/entity-lifecycle/archive.ts");
const { createMutationController } = await import("../src/lib/entity-lifecycle/mutationState.ts");

const componentPaths = [
  "src/lib/entity-lifecycle/EntityIdentityDialog.svelte",
  "src/lib/entity-lifecycle/EntityRowActions.svelte",
  "src/lib/entity-lifecycle/EntityArchiveAction.svelte",
  "src/lib/entity-lifecycle/EntityEmptyState.svelte",
  "src/lib/entity-lifecycle/MutationStatus.svelte",
];

for (const path of componentPaths) {
  const source = await read(path);
  compile(source, { filename: resolve(root, path), css: "injected" });
}

const shell = await read("src/routes/+page.svelte");
const overview = await read("packages/modules/language/src/panes/Overview.svelte");
const archivePanel = await read("src/lib/ArchivedDocumentsPanel.svelte");
const sidebar = await read("src/lib/shell/AppSidebar.svelte");
const identity = await read("src/lib/entity-lifecycle/EntityIdentityDialog.svelte");
const rowActions = await read("src/lib/entity-lifecycle/EntityRowActions.svelte");
const archiveAction = await read("src/lib/entity-lifecycle/EntityArchiveAction.svelte");
const mutationStatus = await read("src/lib/entity-lifecycle/MutationStatus.svelte");
const landing = await read("src/lib/houses/TreeLanding.svelte");
const treeSurface = await read("src/lib/houses/TreeSurface.svelte");

assert.equal(ENTITY_ACTIONS.editIdentity, "Edit identity");
assert.equal(ENTITY_ACTIONS.viewArchive, "View Archive");
assert.equal(ENTITY_ACTIONS.archive, "Archive");
assert.equal(ENTITY_ACTIONS.new, "New");
assert.equal(ENTITY_ACTIONS.newPerson, "New person");
assert.equal(ENTITY_ACTIONS.newHouse, "New house");
assert.equal(ENTITY_ACTION_CONFIRM.archiveConfirm, "Archive");
assert.equal(MUTATION_STATUS.saving, "Saving…");
assert.equal(MUTATION_STATUS_MESSAGES.conflictReload, "Reload current values");

const confirm = archiveConfirmOptions("Aria");
assert.equal(confirm.title, "Archive Aria?");
assert.match(confirm.message, /archived and hidden/);
assert.equal(confirm.confirmLabel, "Archive");
assert.equal(archivePendingLabel(true), MUTATION_STATUS.working);
assert.equal(archivedToastMessage("Aria"), `"Aria" archived.`);

const controller = createMutationController();
assert.equal(controller.phase, "idle");
controller.begin("test");
assert.equal(controller.phase, "saving");
assert.equal(controller.busy, true);
controller.conflict("stale");
assert.equal(controller.phase, "conflict");
const failed = await controller.run(async () => {
  throw new Error("revision-conflict: stale write");
});
assert.equal(failed.ok, false);
assert.equal(controller.phase, "conflict");
const ok = await controller.run(async () => 42);
assert.equal(ok.ok, true);
if (ok.ok) assert.equal(ok.value, 42);
assert.equal(controller.phase, "saved");

for (const needle of [
  "EntityIdentityDialog",
  "EntityRowActions",
  "EntityArchiveAction",
  "EntityEmptyState",
  "MutationStatus",
  "ENTITY_ACTIONS.viewArchive",
  "openHouseTree",
  "openNewPerson",
  "openNewHouse",
  "openCreationMenu",
  "archiveEntity",
  "reloadEntityEditFromServer",
  "createMutationController",
  "lifecycleToast",
]) {
  assert.match(shell, new RegExp(needle), `shell wires ${needle}`);
}

assert.match(shell, /openCreationMenu\(\)/);
assert.match(shell, /onclick=\{openContextualCreate\}/);
assert.match(shell, /entityMutation\.phase !== "idle"/);
assert.match(shell, /ENTITY_ACTIONS\.newHouse/);

assert.match(identity, /ENTITY_ACTIONS\.editIdentity/);
assert.match(rowActions, /ENTITY_ACTIONS\.open/);
assert.match(rowActions, /ENTITY_ACTIONS\.editIdentity/);
assert.match(rowActions, /ENTITY_ACTIONS\.archive/);
assert.match(rowActions, /ENTITY_ACTIONS\.openTree/);
assert.match(rowActions, /archiveConfirmOptions/);
assert.match(rowActions, /touch-target-min/);
assert.match(rowActions, /document\.contains\(triggerEl\)/);
assert.match(archiveAction, /archiveConfirmOptions/);
assert.match(archiveAction, /entityName/);
assert.match(mutationStatus, /conflictReload/);
assert.match(sidebar, />New</);
assert.doesNotMatch(sidebar, />New entry</);
assert.match(shell, /scheduleClearSavedMutation/);
assert.match(shell, /entityMutation\.reset\(\)/);

assert.match(landing, /ENTITY_ACTIONS\.newPerson/);
assert.match(landing, /ENTITY_ACTIONS\.newHouse/);
assert.match(landing, /landing-create/);
assert.match(treeSurface, /onNewPerson/);
assert.match(treeSurface, /onNewHouse/);
assert.match(treeSurface, /family-topbar-create/);
assert.doesNotMatch(treeSurface, />New house<\/button>/);

assert.match(overview, /archiveConfirmOptions/);
assert.match(overview, /archivePendingLabel/);
assert.match(overview, /MUTATION_STATUS/);
assert.doesNotMatch(overview, /Archive language/);

assert.match(archivePanel, /Delete permanently/);
assert.match(archivePanel, /Restore/);
assert.doesNotMatch(shell, /Delete permanently/);

const openIdx = rowActions.indexOf("ENTITY_ACTIONS.open}");
const editIdx = rowActions.indexOf("ENTITY_ACTIONS.editIdentity}");
const archiveIdx = rowActions.indexOf("ENTITY_ACTIONS.archive}");
const treeIdx = rowActions.indexOf("ENTITY_ACTIONS.openTree}");
assert.ok(openIdx > 0 && editIdx > openIdx && archiveIdx > editIdx && treeIdx > archiveIdx, "row action order");

console.log("entity lifecycle contracts passed");
