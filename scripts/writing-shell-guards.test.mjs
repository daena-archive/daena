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
  ["activateBreadcrumb", /dismissSettings\(\)|leavePluginView\(\)/],
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
  const flush = body.indexOf("flushAutoSave()");
  const step = body.search(navigates);
  assert.ok(flush >= 0, `${name} saves the open entry through flushAutoSave`);
  assert.ok(step >= 0, `${name} still navigates where its check expects (${navigates})`);
  assert.ok(flush < step, `${name} saves before it navigates`);
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
  const slot = between("{:else if writingWebviewShown}", "{:else if workbenchPaneVisibility.content}");
  assert.match(slot, /class="writing-content"/);
  assert.match(
    slot,
    /\{#if !writingFullscreen && \(error \|\| projectDiagnostics\.length > 0\)\}/,
    "hidden in full screen",
  );
  assert.ok(slot.indexOf("writing-notices") < slot.indexOf("writing-mount"), "the notices are above the slot");
}

// A rename refreshes the header in place when the editor keeps its text.
{
  const load = between("async function postWritingLoad", "$effect(() => {");
  assert.match(load, /writingTitleMessage\(entityId, selected\.name\)/, "the header follows the rename");
}

console.log("writing shell guard checks passed");
