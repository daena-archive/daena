import assert from "node:assert/strict";
import fs from "node:fs";

// Source checks for the shell side of the isolated Writing webview (docs/plans/writing-isolation.md, slice 3b).
// The guarded paths are not exported from the route, so these read src/routes/+page.svelte and check its order.
const page = fs.readFileSync("src/routes/+page.svelte", "utf8");

/** The source of a top-level function, from its declaration to the brace that closes it at column 0. */
function functionBody(name) {
  const match = new RegExp(`\\n(?:async )?function ${name}\\(`).exec(page);
  assert.ok(match, `${name} is declared`);
  const end = page.indexOf("\n}\n", match.index);
  assert.ok(end > match.index, `${name} closes`);
  return page.slice(match.index, end);
}

/** The source between two markers, for blocks that are not top-level functions. */
function between(from, to) {
  const start = page.indexOf(from);
  assert.ok(start >= 0, `${from} exists`);
  const end = page.indexOf(to, start + from.length);
  assert.ok(end > start, `${to} follows ${from}`);
  return page.slice(start, end);
}

// Every path that hides or unmounts the Writing webview, or changes its entry, saves through flushAutoSave first.
// Each entry names the first statement that navigates, which must come after the flush.
const flushesBeforeNavigating = [
  ["toggleWorkbenchPane", /workbenchPaneVisibility = \{/],
  ["navigateShellHistory", /shellNavigationBusy = true/],
  ["switchToTab", /activateTab\(/],
  ["openInNewTab", /openTab\(/],
  ["closeShellTab", /applyTabsChange\(/],
  ["applyTabsChange", /change\(snapshotActiveTab\(\)\)/],
  ["createWithOption", /entities\.create\(/],
  ["openTreePerson", /leavePluginView\(\)/],
  ["openPluginView", /leavePluginView\(\)|openHostView\(|dismissSettings\(\)/],
  ["switchSection", /dismissSettings\(\)|section = next/],
  ["switchWritingView", /writingView = resolved/],
  ["switchTimelineView", /timelineView = resolved/],
  ["switchHousesView", /housesView = resolved/],
  ["switchLanguagePane", /requestLanguagePane\(next\)/],
  ["openProjectHome", /projectHomeOpen = true/],
  ["openSettings", /showSettings = true/],
  ["openProjectCenter", /showSettings = true/],
  ["selectSearchResult", /leavePluginView\(\)|section = nextSection/],
  ["selectEntity", /selected = entity;/],
  // activateBreadcrumb is gone: the breadcrumb no longer navigates (tabs slice 3).
  ["reconcileWorkspaceSection", /leavePluginView\(\)/],
  ["openHouseTree", /leavePluginView\(\)/],
  ["openLoreLibrary", /dismissSettings\(\)|leavePluginView\(\)/],
  ["openLoreWiki", /leavePluginView\(\)/],
  ["closeLoreWiki", /loreWikiOpen = false/],
  ["closeProjectionView", /sandboxView = null/],
  ["closePluginView", /leavePluginView\(\)/],
  ["openProjection", /leavePluginView\(\)/],
  ["createMap", /leavePluginView\(\)/],
  ["ensureMapEditorOpen", /selected = map;/],
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
for (const [name, navigates] of flushesBeforeNavigating) {
  const body = functionBody(name);
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
  ["archiveSelected", /archiveEntity\(selected/],
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
  functionBody("toggleWorkbenchPane"),
  /pane === "content" && workbenchPaneVisibility\.content\)\s*\{\s*if \(!\(await flushAutoSave\(\)\)\) return;/,
);

// Writing's full screen is the webview's own. It must not be part of the overlay that hides every embedded webview.
{
  const modal = between("const modalOverlayOpen = $derived(", "\n);");
  assert.doesNotMatch(modal, /writingFullscreen/, "the webview's full screen does not hide the webviews");
  assert.match(modal, /editorFullscreen \|\|/, "the shell editor's full screen still does");
  assert.match(page, /let writingFullscreen = \$state\(false\);/);
  assert.match(page, /const writingFullscreenShown = \$derived\(writingWebviewShown && writingFullscreen\);/);
  assert.match(page, /class:writing-fullscreen=\{writingFullscreenShown\}/);
  assert.match(workspaceStyle(), /writingFullscreenShown\) return "grid-template-columns: minmax\(0, 1fr\)"/);
}

function workspaceStyle() {
  return functionBody("workspaceGridStyle");
}

// The full screen ends with the selection, with navigation out of the pane, and with Escape in the shell.
{
  const reset = between("$effect(() => {\n  void selected?.id;", "/**\n * Runs a shell shortcut.");
  assert.match(reset, /void writingWebviewShown;/, "leaving the Writing pane ends the full screen");
  assert.match(reset, /untrack\(\(\) => exitWritingFullscreen\(\)\)/);
  assert.match(functionBody("exitWritingFullscreen"), /writingFullscreenMessage\(false\)/, "the webview is told");
  const escape = between("const handleWritingFullscreenKey", 'window.addEventListener("resize"');
  assert.match(escape, /event\.key === "Escape"\)\s*exitWritingFullscreen\(\)/);
  const mount = between("function trackWritingMount()", "async function postWritingMessage(");
  assert.match(mount, /writingFullscreen = false;/, "unmounting resets the full screen");
  const ready = between('    case "ready":\n      if (!writingWebview.mounted) return;', '    case "draft":');
  assert.match(ready, /writingFullscreen = false;/, "a restarted webview starts out of full screen");
  const stateEvent = functionBody("handleWritingStateEvent");
  assert.match(
    stateEvent,
    /case "fullscreen":[\s\S]*event\.entityId === selected\?\.id\)\s*writingFullscreen = event\.on;/,
  );
}

// Rename and archive from the webview: the shell checks the entry is the selection before it acts.
{
  const stateEvent = functionBody("handleWritingStateEvent");
  assert.match(
    stateEvent,
    /case "request-rename":\s*if \(writingWebview\.mounted && selected && event\.entityId === selected\.id\) void openEntityEditDialog\(selected\);/,
  );
  assert.match(
    stateEvent,
    /case "request-archive":\s*if \(writingWebview\.mounted\) void confirmWritingArchive\(event\.entityId\);/,
  );
  const archive = functionBody("confirmWritingArchive");
  assert.match(archive, /target\.id !== entityId/, "archive needs the entry to be the selection");
  assert.match(archive, /confirmDialog\(archiveConfirmOptions\(target\.name\)\)/, "archive keeps its confirmation");
  assert.match(
    archive,
    /selected\?\.id !== entityId\) return;\s*await archiveSelected\(\)/,
    "checked again after the dialog",
  );
}

// Shortcuts from the webview run the shell's own handlers, and the window listeners share them.
{
  const stateEvent = functionBody("handleWritingStateEvent");
  assert.match(stateEvent, /case "shortcut":\s*if \(writingWebview\.mounted\) runShellShortcut\(event\.shortcut\);/);
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
    "{:else if workbenchPaneVisibility.content && !writingEntrySelected}",
  );
  assert.match(slot, /class="writing-content"/);
  assert.match(
    slot,
    /\{#if !writingFullscreen && \(error \|\| projectDiagnostics\.length > 0\)\}/,
    "hidden in full screen",
  );
  assert.ok(slot.indexOf("writing-notices") < slot.indexOf("writing-mount"), "the notices are above the slot");
}

// The webview is the only editor for a manuscript or reference page. Whenever one is selected in the Writing section,
// the shell editor must not render, whether or not the webview is shown (not ready, content pane hidden, an
// overlay view open): a second editor would be a second writer next to the webview's drafts.
{
  assert.match(
    page,
    /const writingEntrySelected = \$derived\(section === "writing" && writingWebviewHandles\(selected\?\.entity_type\)\);/,
    "one derived says a webview-handled Writing entry is selected",
  );
  const shown = between("const writingWebviewShown = $derived(", "\n);");
  assert.match(
    shown,
    /^const writingWebviewShown = \$derived\(\s*writingEntrySelected &&/,
    "shown only for such an entry",
  );
  for (const gate of [
    "ready",
    "workbenchPaneVisibility.content",
    "!projectHomeOpen",
    "!showSettings",
    "!hostView",
    "!sandboxView",
    "!loreWikiOpen",
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
    ["writingWebviewShown", "workbenchPaneVisibility.content && !writingEntrySelected"],
    "the shell editor branch excludes webview-handled Writing entries",
  );
  // The shell editor (RichTextEditor and the content pane body) renders only inside that last branch.
  assert.equal(page.match(/<RichTextEditor\b/g)?.length, 1, "one shell RichTextEditor");
  assert.ok(
    page.indexOf("<RichTextEditor") >
      page.indexOf("{:else if workbenchPaneVisibility.content && !writingEntrySelected}"),
    "the shell RichTextEditor sits behind the guard",
  );
  assert.equal(page.match(/\{#snippet contentPaneBody\(\)\}/g)?.length, 1);
  assert.equal(page.match(/children=\{contentPaneBody\}/g)?.length, 1, "the content pane body is rendered once");
  // Lore, Maps and the Writing types the webview does not handle keep the shell editor.
  assert.match(
    page,
    /placeholder=\{section === "writing"\s*\? `Write this \$\{createLabel\(\)\}…`\s*: section === "maps"/,
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
    functionBody("flushAutoSave"),
    /flushWritingWebview\(\)/,
    "the shell is the writer, after the webview's flush",
  );
  assert.match(page, /await project\.saveEntry\(\s*\{\s*document: \{ entity_id: entityId, body, format: "markdown" \}/);
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
  const load = between("async function postWritingLoad", "$effect(() => {");
  assert.match(load, /writingTitleMessage\(entityId, selected\.name\)/, "the header follows the rename");
}

console.log("writing shell guard checks passed");
