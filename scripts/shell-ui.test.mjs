import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import { compile } from "svelte/compiler";

const root = resolve(import.meta.dirname, "..");
const read = (path) => readFile(resolve(root, path), "utf8");

const componentPaths = [
  "src/lib/ProjectCenter.svelte",
  "src/lib/SettingsView.svelte",
  "src/lib/GitSettingsPanel.svelte",
  "src/lib/EntityHoverCard.svelte",
  "src/lib/shell/AppSidebar.svelte",
  "src/lib/shell/GlobalToolbar.svelte",
  "src/lib/shell/InspectorPane.svelte",
  "src/lib/shell/InspectorOverview.svelte",
  "src/lib/shell/InspectorSection.svelte",
  "src/lib/shell/EntityFieldsDialog.svelte",
  "src/lib/shell/PaneResizeHandle.svelte",
  "src/lib/shell/ProjectSwitcher.svelte",
  "src/lib/shell/RailContextMenu.svelte",
  "src/lib/entity-lifecycle/EntityRowActions.svelte",
  "src/lib/shell/QuickOpen.svelte",
  "src/lib/shell/StatusCenter.svelte",
  "src/lib/shell/TabStrip.svelte",
  "src/lib/shell/WorkbenchState.svelte",
  "src/lib/shell/WorkspaceViewNav.svelte",
];

const components = new Map();
for (const path of componentPaths) {
  const source = await read(path);
  compile(source, { filename: resolve(root, path), css: "injected" });
  components.set(path, source);
}

const shell = await read("src/routes/+page.svelte");
compile(shell, { filename: resolve(root, "src/routes/+page.svelte"), css: "injected" });

for (const component of [
  "AppSidebar",
  "GlobalToolbar",
  "ProjectCenter",
  "QuickOpen",
  "StatusCenter",
  "WorkspaceViewNav",
]) {
  assert.match(shell, new RegExp(`<${component}`), `the shell mounts ${component}`);
}

const settings = components.get("src/lib/SettingsView.svelte");
assert.doesNotMatch(settings, /> Plugins<\/button>|> Snapshots<\/button>|> Schema<\/button>/);

const projectCenter = components.get("src/lib/ProjectCenter.svelte");
for (const section of ["Overview", "Data &amp; recovery", "Extensions", "Fields &amp; Types", "AI", "Snapshots"])
  assert.match(projectCenter, new RegExp(`> ${section}`), `Project Center includes ${section}`);
// Developer fixtures are allowed but must stay gated in Advanced > details.raw-controls
assert.match(projectCenter, /<details[^>]*class="raw-controls"[^>]*>/);
assert.match(projectCenter, /Developer fixtures/);
assert.match(projectCenter, /Add example world/);

const quickOpen = components.get("src/lib/shell/QuickOpen.svelte");
assert.match(quickOpen, /role="combobox"/);
assert.match(quickOpen, /aria-activedescendant/);
assert.match(quickOpen, /trapModalTab/);

const resizeHandle = components.get("src/lib/shell/PaneResizeHandle.svelte");
assert.match(resizeHandle, /ArrowLeft|ArrowRight/);
assert.match(resizeHandle, /onpointerdown/);

const projectSwitcher = components.get("src/lib/shell/ProjectSwitcher.svelte");
assert.match(projectSwitcher, />New project</);
assert.match(projectSwitcher, />Open folder</);
assert.match(projectSwitcher, /Projects live in a folder on disk/);
assert.doesNotMatch(projectSwitcher, /Open project folder/);

const statusCenter = components.get("src/lib/shell/StatusCenter.svelte");
assert.match(statusCenter, /aria-live="polite"/);
assert.match(statusCenter, /prefers-reduced-motion/);
assert.match(shell, /function inactiveGitStatus/);
assert.doesNotMatch(shell, /Snapshot status unavailable/);

const workspaceNav = components.get("src/lib/shell/WorkspaceViewNav.svelte");
assert.match(workspaceNav, /tabindex=\{activeView === view\.id \? 0 : -1\}/);

assert.doesNotMatch(shell, /Private studio/, "MVP Private studio crumb is gone");
assert.match(shell, /<TabStrip/, "the shell renders TabStrip");
assert.match(
  shell,
  /function currentTabPresentation\([\s\S]*?shellBreadcrumbs\(/,
  "currentTabPresentation uses shellBreadcrumbs",
);
assert.match(shell, /entityBelongs/, "entity crumbs drop when the module no longer owns the entry");
assert.match(shell, /activeTab\(shellTabs\)\.history/, "history reads go through the active tab");
const toolbar = components.get("src/lib/shell/GlobalToolbar.svelte");
assert.match(toolbar, /tabs\?: Snippet/, "GlobalToolbar takes a tabs snippet");
const tabStrip = components.get("src/lib/shell/TabStrip.svelte");
assert.match(tabStrip, /role="tablist"/);
assert.match(tabStrip, /role="tab"/);
assert.match(tabStrip, /aria-selected/);
assert.match(tabStrip, /onClose/, "TabStrip closes a tab");
assert.match(tabStrip, /onNew/, "TabStrip opens a new tab");
assert.match(tabStrip, /"Close all tabs"/, "the tab menu closes every unpinned tab");
assert.match(tabStrip, /onCloseAll/, "TabStrip reports Close all tabs to the shell");
assert.match(tabStrip, /aria-label=\{`Close \$\{/, "a tab's close button names the tab");
assert.match(tabStrip, /role="presentation"/, "the tab pill is not a nested button");
assert.match(tabStrip, /role="menu"/);
assert.match(tabStrip, /data-shell-popover="tab-menu"/);
assert.match(tabStrip, /data-shell-popover="tab-list"/);
assert.match(tabStrip, /menuitemradio/);
assert.match(tabStrip, /aria-label="All tabs"/);
assert.match(tabStrip, /ContextMenu/);
assert.match(tabStrip, /F10/);
assert.match(tabStrip, /shiftKey/);
assert.match(tabStrip, /tabDropIndex/);
const runItem = tabStrip.slice(tabStrip.indexOf("function runItem("), tabStrip.indexOf("function checkOverflow("));
assert.ok(
  runItem.indexOf("action()") >= 0 && runItem.indexOf("action()") < runItem.indexOf("closePopover("),
  "menu items run before the popover closes, because closing clears the menu's tab",
);
assert.match(shell, /showProjectMenu \|\| statusCenterOpen \|\| tabStripPopoverOpen \|\| railMenuOpen/);
assert.match(
  shell,
  /openInNewTab\(\(\) => openNavigationItem\(item\), \{ dedupe: false \}\)/,
  "opening a workspace from the rail in a new tab never folds into an existing tab of it",
);
assert.match(
  shell,
  /identity === originIdentity\s*\?\s*null\s*:\s*otherTabByIdentity\(shellTabs, identity, \[id, originId\]\)/,
  "opening the current tab's own place in a new tab always makes a copy, however many tabs already show it",
);
for (const name of [
  "switchToTab",
  "openInNewTab",
  "closeShellTab",
  "closeAllShellTabs",
  "persistShellTabs",
  "loadShellTabs",
  "snapshotActiveTab",
  "applyTabsChange",
  "pruneUnavailableShellTabs",
  "closeTabsForEntity",
  "reopenShellTab",
]) {
  assert.match(shell, new RegExp(`function ${name}\\(`), `the shell defines ${name}`);
}

const vocabulary = await read("src/lib/entity-lifecycle/vocabulary.ts");
assert.match(vocabulary, /openInNewTab: "Open in new tab"/);

const rowActions = components.get("src/lib/entity-lifecycle/EntityRowActions.svelte");
assert.match(rowActions, /ENTITY_ACTIONS\.openInNewTab/, "the row menu offers Open in new tab");
assert.match(rowActions, /addEventListener\("contextmenu"/, "a right-click on the row opens the row menu");
const rowActionUses = shell.match(/<EntityRowActions[\s\S]*?\/>/g) ?? [];
assert.equal(rowActionUses.length, 3, "the shell has three collection row menus");
for (const use of rowActionUses) {
  assert.match(use, /onOpenInNewTab=\{/, "every collection row menu passes onOpenInNewTab");
  assert.match(use, /contextHost="\.collection-item"/, "every collection row menu opens on right-click of its row");
}

const railMenu = components.get("src/lib/shell/RailContextMenu.svelte");
assert.match(railMenu, /role="menu"/);
assert.match(railMenu, /data-shell-popover="rail-menu"/);
const railRunItem = railMenu.slice(railMenu.indexOf("function runItem("), railMenu.indexOf("function onMenuKeydown("));
assert.ok(
  railRunItem.indexOf("action()") >= 0 && railRunItem.indexOf("action()") < railRunItem.indexOf("onClose()"),
  "a rail menu item runs its action before the menu closes",
);

const sidebar = components.get("src/lib/shell/AppSidebar.svelte");
assert.match(sidebar, /RailContextMenu/, "the rail opens RailContextMenu");
assert.match(
  sidebar,
  /oncontextmenu=/,
  "the rail's Home, workspace and tool buttons open the rail menu on right-click",
);
assert.match(shell, /bind:menuOpen=\{railMenuOpen\}/, "the shell binds the rail menu state");
assert.match(
  shell,
  /if \(railMenuOpen\) \{\s*railMenuOpen = false;/,
  "dismissing transient menus closes the rail menu",
);

const controls = await read("src/lib/shell/controls.css");
assert.match(controls, /:focus-visible/);
assert.match(controls, /@media \(pointer: coarse\)/);
assert.match(controls, /@media \(prefers-reduced-motion: reduce\)/);
assert.match(controls, /@media \(forced-colors: active\)/);

const inspector = components.get("src/lib/shell/InspectorSection.svelte");
assert.match(inspector, /nested\?: boolean/);
assert.match(
  inspector,
  /\.inspector-group\[open\] > summary/,
  "open chevron rotation must not apply to nested relationship groups",
);
assert.match(shell, /groupedInspectorRelationships/);
assert.match(shell, /sortByPopulated: false/);
assert.match(shell, /sticky/);
assert.match(shell, /InspectorOverview/);
assert.match(shell, /EntityFieldsDialog/);
assert.match(shell, /openFieldsDialog/);
assert.match(inspector, /sticky\?: boolean/);

console.log(`shell UI boundary checks passed (${componentPaths.length + 1} Svelte files compiled)`);
