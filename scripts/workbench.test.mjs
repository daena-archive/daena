import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const shell = await readFile(new URL("../src/routes/+page.svelte", import.meta.url), "utf8");
const resizeHandle = await readFile(new URL("../src/lib/shell/PaneResizeHandle.svelte", import.meta.url), "utf8");
const inspector = await readFile(new URL("../src/lib/shell/InspectorPane.svelte", import.meta.url), "utf8");
const inspectorSection = await readFile(new URL("../src/lib/shell/InspectorSection.svelte", import.meta.url), "utf8");
const workbenchState = await readFile(new URL("../src/lib/shell/WorkbenchState.svelte", import.meta.url), "utf8");
const relatedPreview = await readFile(new URL("../src/lib/EntityHoverCard.svelte", import.meta.url), "utf8");

assert.match(shell, /workbenchPaneVisibility/, "the workbench tracks pane visibility explicitly");
assert.match(shell, /daena:workbench-layout:/, "workbench layout persists per plugin");
assert.match(shell, /workbenchLayoutStorageKey/, "layout storage is keyed by workspace plugin id");
assert.match(shell, /loadWorkbenchLayout/, "each plugin restores its own workbench layout");
assert.match(shell, /saveWorkbenchLayout/, "workbench layout changes are saved per plugin");
assert.match(shell, /resizeWorkbenchPane/, "collection and inspector widths have a shared resize boundary");
assert.match(shell, /resetWorkbenchPane/, "pane widths can be restored to defaults");
assert.match(resizeHandle, /ArrowLeft/, "pane resizing is keyboard accessible");
assert.match(resizeHandle, /onpointerdown/, "pane resizing supports direct manipulation");
assert.match(resizeHandle, /ondblclick/, "double-click restores the default pane width");

assert.match(shell, />Article<\/button/, "the compact entity header exposes Article mode");
assert.match(shell, />Edit<\/button/, "the compact entity header exposes Edit mode");
assert.match(shell, /setDocumentMode/, "document modes use an explicit transition");
assert.match(
  shell,
  /documentMode === "edit" && !\(await flushAutoSave\(\)\)/,
  "leaving Edit preserves autosave guards",
);

for (const title of ["Details", "Relationships", "Assets", "Backlinks"])
  assert.match(shell, new RegExp(`title="${title}"`), `the inspector includes the ${title} group`);
assert.match(inspectorSection, /<details/, "inspector groups are progressively collapsible");
assert.match(shell, /relationship\.target_id === selected\?\.id/, "backlinks derive from incoming relationships");
assert.match(shell, /related-item-trigger/, "relationship and backlink rows expose previews");
assert.match(relatedPreview, /Open fully/, "related previews provide an explicit full-navigation action");

assert.match(inspector, /WorkbenchState/, "the inspector uses shared loading, failure, and empty states");
assert.match(shell, /kind="loading"/, "content uses the shared loading state");
assert.match(shell, /kind="error"/, "content uses the shared failure state");
assert.match(shell, /kind="conflict"/, "content uses the shared conflict state");
assert.match(shell, /kind="empty"/, "content uses the shared empty state");
assert.match(workbenchState, /aria-busy=\{kind === "loading"\}/, "loading state is exposed accessibly");
assert.match(
  shell,
  /context\.relationships\.list/,
  "the inspector retains the module-authorized relationship boundary",
);
assert.match(shell, /fieldAppliesToEntity/, "the workbench retains manifest-driven field visibility");

console.log("common entity workbench contracts passed");
