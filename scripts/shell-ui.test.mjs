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
  "src/lib/shell/InspectorSection.svelte",
  "src/lib/shell/PaneResizeHandle.svelte",
  "src/lib/shell/ProjectSwitcher.svelte",
  "src/lib/shell/QuickOpen.svelte",
  "src/lib/shell/StatusCenter.svelte",
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
for (const section of ["Overview", "Data &amp; recovery", "Extensions", "Fields &amp; Types", "Snapshots"])
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
assert.match(shell, /activateBreadcrumb/, "workspace crumbs navigate");
assert.match(shell, /shellBreadcrumbs/, "crumbs follow the current shell location");
assert.match(shell, /entityBelongs/, "entity crumbs drop when the module no longer owns the entry");
const toolbar = components.get("src/lib/shell/GlobalToolbar.svelte");
assert.match(toolbar, /crumb\.onSelect/, "ancestor breadcrumbs are buttons");
assert.match(toolbar, /aria-current/, "the current crumb is marked");

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
assert.match(shell, /open=\{assets\.length > 0\}/);
assert.match(inspector, /sticky\?: boolean/);

console.log(`shell UI boundary checks passed (${componentPaths.length + 1} Svelte files compiled)`);
