import assert from "node:assert/strict";
import fs from "node:fs";

// Source checks for the shell side of the isolated Writing webview (docs/plans/writing-isolation.md, slice 3b).
// The guarded paths are not exported from the route, so these read src/routes/+page.svelte and check its order.
const page = fs.readFileSync("src/routes/+page.svelte", "utf8");
// The per-location functions (selection, save, autosave) live in the location store; the page reaches them as `loc.<name>`.
const store = fs.readFileSync("src/lib/location/location-store.svelte.ts", "utf8");

/** The source of a store method (or arrow-function property): its declaration to the brace that closes it. */
function storeMethodBody(name) {
  const match = new RegExp(`\\n  (?:async )?${name}\\(|\\n  ${name} = (?:async )?\\(`).exec(store);
  assert.ok(match, `${name} is declared in the location store`);
  const end = store.slice(match.index + 1).search(/\n {2}\};?\n/);
  assert.ok(end > 0, `${name} closes`);
  return store.slice(match.index, match.index + 1 + end);
}

/** The source of a top-level function, from its declaration to the brace that closes it at column 0. */
function functionBody(name) {
  const match = new RegExp(`\\n(?:async )?function ${name}\\(`).exec(page);
  assert.ok(match, `${name} is declared`);
  const end = page.indexOf("\n}\n", match.index);
  assert.ok(end > match.index, `${name} closes`);
  return page.slice(match.index, end);
}

/** The source between two markers, for blocks that are not top-level functions. */
function betweenIn(source, from, to) {
  const start = source.indexOf(from);
  assert.ok(start >= 0, `${from} exists`);
  const end = source.indexOf(to, start + from.length);
  assert.ok(end > start, `${to} follows ${from}`);
  return source.slice(start, end);
}
function between(from, to) {
  return betweenIn(page, from, to);
}

// Every path that hides or unmounts the Writing webview, or changes its entry, saves through flushAutoSave first.
// Each entry names the first statement that navigates, which must come after the flush.
const flushesBeforeNavigating = [
  ["toggleWorkbenchPane", /this\.workbenchPaneVisibility = \{/, "store"],
  ["navigateShellHistory", /shellNavigationBusy = true/],
  ["switchToTab", /activateTab\(/],
  ["openInNewTab", /openTab\(/],
  ["closeShellTab", /applyTabsChange\(/],
  ["applyTabsChange", /change\(snapshotActiveTab\(\)\)/],
  ["createWithOption", /entities\.create\(/],
  ["openTreePerson", /leavePluginView\(\)/],
  ["openPluginView", /leavePluginView\(\)|openHostView\(|dismissSettings\(\)/, "store"],
  ["switchSection", /dismissSettings\(\)|section = next/, "store"],
  ["switchWritingView", /writingView = resolved/, "store"],
  ["switchTimelineView", /timelineView = resolved/, "store"],
  ["switchHousesView", /housesView = resolved/, "store"],
  ["switchLanguagePane", /requestLanguagePane\(next\)/, "store"],
  ["openProjectHome", /projectHomeOpen = true/, "store"],
  ["openSettings", /showSettings = true/],
  ["openProjectCenter", /showSettings = true/],
  ["selectSearchResult", /leavePluginView\(\)|section = nextSection/, "store"],
  ["selectEntity", /this\.selected = entity;/, "store"],
  // activateBreadcrumb is gone: the breadcrumb no longer navigates (tabs slice 3).
  ["reconcileWorkspaceSection", /leavePluginView\(\)/, "store"],
  ["openHouseTree", /leavePluginView\(\)/],
  ["openLoreLibrary", /dismissSettings\(\)|leavePluginView\(\)/, "store"],
  ["openLoreWiki", /leavePluginView\(\)/, "store"],
  ["closeLoreWiki", /loreWikiOpen = false/, "store"],
  ["closeProjectionView", /sandboxView = null/, "store"],
  ["closePluginView", /leavePluginView\(\)/, "store"],
  ["openProjection", /leavePluginView\(\)/, "store"],
  ["createMap", /leavePluginView\(\)/],
  ["ensureMapEditorOpen", /loc\.selected = map;/],
  ["archiveEntity", /clearSelection\(\)|entities\.delete\(/],
  ["openExternalImport", /showExternalImport = true/],
  ["seedExample", /clearSelection\(\)/],
  ["openEntityEditDialog", /selectEntity\(target\)|getEntity\(target\.id\)/],
  ["openRecentProject", /leavePluginView\(\{ closeWebviews: true \}\)/],
  ["closeProject", /leavePluginView\(\{ closeWebviews: true \}\)/],
  ["pickAndOpenProject", /leavePluginView\(\{ closeWebviews: true \}\)/],
  ["importPortableCheckpoint", /leavePluginView\(\{ closeWebviews: true \}\)/],
  ["restoreRecoveryBackup", /leavePluginView\(\{ closeWebviews: true \}\)/],
];
for (const [name, navigates, owner] of flushesBeforeNavigating) {
  const body = owner === "store" ? storeMethodBody(name) : functionBody(name);
  const step = body.search(navigates);
  assert.ok(step >= 0, `${name} still navigates where its check expects (${navigates})`);
  if (body.includes("flushAutoSave()")) {
    const flush = body.indexOf("flushAutoSave()");
    assert.ok(flush < step, `${name} saves before it navigates`);
    continue;
  }
  // closeShellTab delegates to applyTabsChange, which saves before it changes the active tab.
  assert.match(body, /applyTabsChange\(/, `${name} saves through applyTabsChange`);
  const delegated = functionBody("applyTabsChange");
  const flush = delegated.indexOf("flushAutoSave()");
  const delegatedStep = delegated.search(/change\(snapshotActiveTab\(\)\)/);
  assert.ok(flush >= 0 && delegatedStep >= 0 && flush < delegatedStep, `${name} saves before it navigates`);
}

// These reach a navigation through a function that saves first, so they only need to call it.
const navigatesThroughGuardedPath = [
  ["openEntityById", /selectSearchResult\(target\)[\s\S]*selectEntity\(target\)/],
  ["selectQuickOpenItem", /selectSearchResult\(matchedEntity\)[\s\S]*openProjectHome\(\)/],
  ["archiveSelected", /archiveEntity\(loc\.selected/],
  ["openSelectedMapEditor", /openPluginView\(mapsView\)/],
  ["confirmWritingArchive", /archiveSelected\(\)/],
];
for (const [name, through] of navigatesThroughGuardedPath) {
  assert.match(functionBody(name), through, `${name} goes through a path that saves first`);
}

// The maps navigation listener and the window close handshake save before they leave.
{
  const mapsListener = between(
    'listen<{ mapEntityId: string; linkId?: string }>("maps-navigation"',
    "let unlistenMapsState",
  );
  assert.ok(
    mapsListener.indexOf("flushAutoSave()") < mapsListener.indexOf("leavePluginView()"),
    "maps navigation saves first",
  );
  const closeRequested = between(".onCloseRequested(async (event) => {", ".then((cleanup) => {");
  assert.ok(
    closeRequested.indexOf("flushAutoSave()") < closeRequested.indexOf("leavePluginView({ closeWebviews: true })"),
    "closing the window saves first",
  );
}

// Hiding the content column, and every other place the webview leaves the screen, is covered above. Toggling the
// content column is the one that needs a flush of its own: it used to ask only the Language webview.
assert.match(
  storeMethodBody("toggleWorkbenchPane"),
  /pane === "content" && this\.workbenchPaneVisibility\.content\)\s*\{\s*if \(!\(await this\.flushAutoSave\(\)\)\) return;/,
);

// Writing's full screen is the webview's own. It must not be part of the overlay that hides every embedded webview.
{
  const modal = between("const modalOverlayOpen = $derived(", "\n);");
  assert.doesNotMatch(modal, /writingFullscreen/, "the webview's full screen does not hide the webviews");
  assert.match(modal, /editorFullscreen \|\|/, "the shell editor's full screen still does");
  assert.match(store, /writingFullscreen = \$state\(false\);/);
  assert.match(page, /const writingFullscreenShown = \$derived\(writingWebviewShown && loc\.writingFullscreen\);/);
  assert.match(page, /class:writing-fullscreen=\{writingFullscreenShown\}/);
  assert.match(workspaceStyle(), /writingFullscreenShown\) return "grid-template-columns: minmax\(0, 1fr\)"/);
}

function workspaceStyle() {
  return functionBody("workspaceGridStyle");
}

// The full screen ends with the selection, with navigation out of the pane, and with Escape in the shell.
{
  const reset = betweenIn(store, "void this.selected?.id;", "untrack(() => this.exitWritingFullscreen())");
  assert.match(reset, /void this\.ctx\.writingWebviewShown;/, "leaving the Writing pane ends the full screen");
  assert.match(store, /untrack\(\(\) => this\.exitWritingFullscreen\(\)\)/);
  assert.match(storeMethodBody("exitWritingFullscreen"), /endWritingFullscreen\(\)/, "the shell ends the full screen");
  assert.match(
    storeMethodBody("endWritingFullscreen"),
    /writingFullscreenMessage\(false\), instance\)/,
    "the instance that was fullscreen is told, not the tab that is shown by then",
  );
  const escape = between("const handleWritingFullscreenKey", 'window.addEventListener("resize"');
  assert.match(escape, /event\.key === "Escape"\)\s*loc\.exitWritingFullscreen\(\)/);
  const mount = betweenIn(store, "trackWritingMount = () => {", "return () => {");
  assert.match(mount, /endWritingFullscreen\(\);/, "unmounting and switching instances end the full screen");
  const ready = betweenIn(store, 'case "ready":\n        if (!this.writingWebview.mounted) return;', 'case "draft":');
  assert.match(ready, /writingFullscreen = false;/, "a restarted webview starts out of full screen");
  const stateEvent = storeMethodBody("handleWritingStateEvent");
  assert.match(
    stateEvent,
    /case "fullscreen":[\s\S]*event\.entityId === this\.selected\?\.id\) \{\s*this\.writingFullscreen = event\.on;\s*this\.writingFullscreenInstance = event\.on \? this\.ctx\.shownWebviewInstance : null;/,
  );
}

// Rename and archive from the webview: the shell checks the entry is the selection before it acts.
{
  const stateEvent = storeMethodBody("handleWritingStateEvent");
  assert.match(
    stateEvent,
    /case "request-rename":\s*if \(this\.writingWebview\.mounted && this\.selected && event\.entityId === this\.selected\.id\)\s*void this\.ctx\.openEntityEditDialog\(this\.selected\);/,
  );
  assert.match(
    stateEvent,
    /case "request-archive":\s*if \(this\.writingWebview\.mounted\) void this\.ctx\.confirmWritingArchive\(event\.entityId\);/,
  );
  const archive = functionBody("confirmWritingArchive");
  assert.match(archive, /target\.id !== entityId/, "archive needs the entry to be the selection");
  assert.match(archive, /confirmDialog\(archiveConfirmOptions\(target\.name\)\)/, "archive keeps its confirmation");
  assert.match(
    archive,
    /loc\.selected\?\.id !== entityId\) return;\s*await archiveSelected\(\)/,
    "checked again after the dialog",
  );
}

// Shortcuts from the webview run the shell's own handlers, and the window listeners share them.
{
  const stateEvent = storeMethodBody("handleWritingStateEvent");
  assert.match(
    stateEvent,
    /case "shortcut":\s*if \(this\.writingWebview\.mounted\) this\.ctx\.runShellShortcut\(event\.shortcut\);/,
  );
  const run = functionBody("runShellShortcut");
  // Create is the fall-through: every other shortcut is named, and anything else opens the gallery.
  for (const shortcut of ["history-back", "history-forward", "quick-open"]) {
    assert.match(run, new RegExp(`"${shortcut}"`), `${shortcut} is mapped`);
  }
  assert.match(run, /navigateShellHistory\("back"\)/);
  assert.match(run, /openCreationMenu\(\)/);
  const onMount = between("const handleHistoryKey = (event: KeyboardEvent) => {", "const handleWorkbenchResize");
  assert.match(onMount, /runShellShortcut\(event\.key === "ArrowLeft" \? "history-back" : "history-forward"\)/);
  assert.match(onMount, /runShellShortcut\("quick-open"\)/);
  assert.match(onMount, /runShellShortcut\("create"\)/);
}

// The webview's notices sit above its slot in normal flow, and the shell's own copies are left out while it shows.
{
  assert.match(
    page,
    /\{#if projectDiagnostics\.length && !writingWebviewShown\}/,
    "the top banner is left to the webview",
  );
  assert.match(page, /\{#if error && !writingWebviewShown\}/, "the toast is left to the webview");
  const slot = between(
    "{:else if writingWebviewShown}",
    "{:else if loc.workbenchPaneVisibility.content && !loc.writingEntrySelected}",
  );
  assert.match(slot, /class="writing-content"/);
  assert.match(
    slot,
    /\{#if !loc\.writingFullscreen && \(error \|\| projectDiagnostics\.length > 0\)\}/,
    "hidden in full screen",
  );
  assert.ok(slot.indexOf("writing-notices") < slot.indexOf("writing-mount"), "the notices are above the slot");
}

// The webview is the only editor for a manuscript or reference page. Whenever one is selected in the Writing section,
// the shell editor must not render, whether or not the webview is shown (not ready, content pane hidden, an
// overlay view open): a second editor would be a second writer next to the webview's drafts.
{
  assert.match(
    store,
    /readonly writingEntrySelected = \$derived\(\s*this\.section === "writing" && writingWebviewHandles\(this\.selected\?\.entity_type\),?\s*\);/,
    "one derived says a webview-handled Writing entry is selected",
  );
  const shown = between("const writingWebviewShown = $derived(", "\n);");
  assert.match(
    shown,
    /^const writingWebviewShown = \$derived\(\s*loc\.writingEntrySelected &&/,
    "shown only for such an entry",
  );
  for (const gate of [
    "ready",
    "workbenchPaneVisibility.content",
    "!loc.projectHomeOpen",
    "!loc.showSettings",
    "!loc.hostView",
    "!loc.sandboxView",
    "!loc.loreWikiOpen",
  ]) {
    assert.ok(shown.includes(gate), `the webview needs ${gate}`);
  }
  assert.doesNotMatch(page, /daena:writing-webview|writingWebviewEnabled/, "no flag chooses between two editors");

  // The content column chooses between the Language slot, the Writing slot and the shell editor (the chain starts
  // at `{#if languagePaneShown}`; its `{:else if}` branches are checked here).
  const chain = between("{#if languagePaneShown}", "{#snippet contentPaneBody()}");
  const branches = [...chain.matchAll(/^ {8}\{(?:#|:else )if ([^}]+)\}/gm)].map((match) => match[1]);
  assert.deepEqual(
    branches,
    ["writingWebviewShown", "loc.workbenchPaneVisibility.content && !loc.writingEntrySelected"],
    "the shell editor branch excludes webview-handled Writing entries",
  );
  // The shell editor (RichTextEditor and the content pane body) renders only inside that last branch.
  assert.equal(page.match(/<RichTextEditor\b/g)?.length, 1, "one shell RichTextEditor");
  assert.ok(
    page.indexOf("<RichTextEditor") >
      page.indexOf("{:else if loc.workbenchPaneVisibility.content && !loc.writingEntrySelected}"),
    "the shell RichTextEditor sits behind the guard",
  );
  assert.equal(page.match(/\{#snippet contentPaneBody\(\)\}/g)?.length, 1);
  assert.equal(page.match(/children=\{contentPaneBody\}/g)?.length, 1, "the content pane body is rendered once");
  // Lore, Maps and the Writing types the webview does not handle keep the shell editor.
  assert.match(
    page,
    /placeholder=\{loc\.section === "writing"\s*\? `Write this \$\{createLabel\(\)\}…`\s*: loc\.section === "maps"/,
  );
  assert.match(page, /Write the canonical story of this entry…/);
}

// Writing does not declare `document.write`. A module that does (Lore, Language) still creates the entry
// and its opening note in one `entities.create`. A module that does not creates the entry first, then the shell
// saves a non-empty note through `project.saveEntry` before selection. `writing_plugin_cannot_write_documents`
// checks the real manifest denies both document writes.
{
  const manifest = JSON.parse(fs.readFileSync("packages/modules/writing/manifest.json", "utf8"));
  assert.ok(!manifest.capabilities.includes("document.write"), "Writing cannot save documents itself");
  assert.ok(
    manifest.templates.length > 0 && manifest.templates.every((template) => typeof template.document === "string"),
    "Writing templates offer an opening note",
  );
  const create = functionBody("createWithOption");
  assert.match(create, /buildModuleContext\(option\.module,/);
  assert.match(create, /option\.module\.capabilities\.includes\("document\.write"\)/);
  assert.match(
    create,
    /document:\s*writesDocuments && openingNote \? \{ body: openingNote, format: "markdown" \} : undefined/,
  );
  assert.match(create, /if \(!writesDocuments && openingNote\)/);
  assert.match(create, /entity_id: created\.id, body: openingNote, format: "markdown"/);
  assert.match(create, /fields: \[\]/);
  const saveAt = create.indexOf("await project.saveEntry(");
  const refreshAt = create.indexOf("refreshAfterEntityMutation");
  const selectAt = create.indexOf("selectEntity(");
  const errorAt = create.indexOf("if (openingNoteError) error = openingNoteError");
  assert.ok(saveAt >= 0 && saveAt < refreshAt, "the opening note is saved before refresh");
  assert.ok(refreshAt < selectAt, "refresh happens before the selection moves");
  assert.ok(selectAt < errorAt, "the note failure is reported after selectEntity clears error");
  assert.match(create, /was created, but its opening note was not saved: \$\{friendlyError\(cause\)\}/);
  assert.match(
    storeMethodBody("flushAutoSave"),
    /flushWritingWebview\(\)/,
    "the shell is the writer, after the webview's flush",
  );
  assert.match(
    store,
    /await project\.saveEntry\(\s*\{\s*document: \{ entity_id: entityId, body, format: "markdown" \}/,
  );
  const command = fs.readFileSync("src-tauri/src/project_commands.rs", "utf8");
  assert.match(
    command,
    /fn project_save_entry\([\s\S]*?core\.project\(trusted_shell\(\)\)\?\.save_entry_with_options\(/,
    "saving is the trusted shell, not a plugin identity",
  );
  const webviewSources = fs
    .readdirSync("packages/modules/writing/src")
    .filter((file) => /\.(ts|svelte)$/.test(file))
    .map((file) => [file, fs.readFileSync(`packages/modules/writing/src/${file}`, "utf8")]);
  for (const [file, source] of webviewSources) {
    assert.doesNotMatch(source, /document\.save|documents\.save|saveEntry|document\.write/, `${file} never saves`);
  }
}

// A rename refreshes the header in place when the editor keeps its text.
{
  const load = storeMethodBody("postWritingLoad");
  assert.match(load, /writingTitleMessage\(entityId, this\.selected\.name\)/, "the header follows the rename");
}

console.log("writing shell guard checks passed");
