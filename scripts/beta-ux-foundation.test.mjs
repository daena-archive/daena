import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const shell = await readFile(new URL("../src/routes/+page.svelte", import.meta.url), "utf8");
const appSidebar = await readFile(new URL("../src/lib/shell/AppSidebar.svelte", import.meta.url), "utf8");
const projectSwitcher = await readFile(new URL("../src/lib/shell/ProjectSwitcher.svelte", import.meta.url), "utf8");
const projectHome = await readFile(new URL("../src/lib/shell/ProjectHome.svelte", import.meta.url), "utf8");
const globalToolbar = await readFile(new URL("../src/lib/shell/GlobalToolbar.svelte", import.meta.url), "utf8");
const workspaceHeader = await readFile(new URL("../src/lib/shell/WorkspaceHeader.svelte", import.meta.url), "utf8");
const workspaceViewNav = await readFile(new URL("../src/lib/shell/WorkspaceViewNav.svelte", import.meta.url), "utf8");
const collectionPane = await readFile(new URL("../src/lib/shell/CollectionPane.svelte", import.meta.url), "utf8");
const contentPane = await readFile(new URL("../src/lib/shell/ContentPane.svelte", import.meta.url), "utf8");
const inspectorPane = await readFile(new URL("../src/lib/shell/InspectorPane.svelte", import.meta.url), "utf8");
const statusSummary = await readFile(new URL("../src/lib/shell/StatusSummary.svelte", import.meta.url), "utf8");
const specializedSurface = await readFile(
  new URL("../src/lib/shell/SpecializedSurface.svelte", import.meta.url),
  "utf8",
);
const history = await readFile(new URL("../src/lib/navigation/history.ts", import.meta.url), "utf8");
const plan = await readFile(new URL("../docs/BETA_UX_REDESIGN_TEMP.md", import.meta.url), "utf8");

assert.match(plan, /## Findings/, "the temporary design authority records the audit findings");
assert.match(plan, /## Planned implementation/, "the redesign is divided into planned implementation slices");
assert.match(plan, /### Iteration 1 — orientation and navigation foundation/, "the active slice has an exit gate");
assert.match(plan, /## Explicit non-goals before beta/, "the beta redesign has bounded non-goals");

assert.match(shell, /let projectHomeOpen = \$state\(true\)/, "projects start with Home as their orientation state");
assert.match(shell, /async function openProjectHome\(\)[\s\S]*?flushAutoSave\(\)/, "Home respects pending autosave");
assert.match(
  shell,
  /async function openProjectHome\(\)[\s\S]*?dismissSettings\(\)/,
  "Home respects Settings leave guards",
);
assert.match(
  shell,
  /async function openProjectHome\(\)[\s\S]*?leavePluginView\(\)/,
  "Home respects plugin and map guards",
);
assert.match(shell, /projectHomeOpen = true;\s*ready = true;/, "opening a project lands on Home");

const homeNavigation = appSidebar.indexOf("<span>Home</span>");
const workspaceNavigation = appSidebar.indexOf('aria-label="Workspace sections"');
assert.ok(homeNavigation >= 0, "Home is a visible sidebar destination");
assert.ok(workspaceNavigation > homeNavigation, "Home appears before workspace destinations");
assert.match(shell, /<AppSidebar/, "the application sidebar is isolated from page orchestration");
assert.match(shell, /workspaceNavigationItems\(\)\.map/, "sidebar workspaces remain manifest-derived");
assert.match(shell, /pluginViews\(\)\.map/, "sidebar tools remain enabled-contribution-derived");
assert.match(appSidebar, /onOpenProject/, "project opening remains reachable from the extracted sidebar");
assert.match(appSidebar, /onCloseProject/, "project closing remains reachable from the extracted sidebar");
assert.match(appSidebar, /<ProjectSwitcher/, "project lifecycle controls use the extracted project switcher");
assert.match(projectSwitcher, /RECENT PROJECTS/, "the project switcher owns launcher recents");
assert.match(projectSwitcher, /Project center/, "the project switcher routes administration to the Project area");
assert.match(shell, /\{:else if projectHomeOpen\}/, "Home has a first-class main content surface");
assert.match(shell, /recentlyUpdatedEntities\(\)/, "Home exposes cross-workspace recent work");
assert.match(
  shell,
  /enabledWorkspaceSections\(\)\.map\(\(target\)/,
  "Home workspace cards remain enabled-manifest driven",
);
assert.match(shell, /count: workspaceEntityCount\(target\)/, "workspace summaries use manifest-derived entity scopes");
assert.match(shell, /<ProjectHome/, "Project Home is isolated from the page orchestration shell");
assert.match(projectHome, /Choose where to work/, "the extracted Home retains workspace orientation");

assert.match(shell, /<WorkspaceViewNav/, "built-in secondary views use one shared navigation component");
assert.match(workspaceViewNav, /id: "events"/, "Timeline Events is directly reachable");
assert.match(workspaceViewNav, /id: "eras"/, "Timeline Eras is directly reachable");
assert.match(workspaceViewNav, /id: "calendars"/, "Timeline Calendars is directly reachable");
assert.match(workspaceViewNav, /id: "manuscripts"/, "Writing Manuscripts is directly reachable");
assert.match(workspaceViewNav, /id: "reference"/, "Writing Reference is directly reachable");
assert.match(workspaceViewNav, /id: "library"/, "Lore Library is directly reachable from alternate Lore views");

assert.match(shell, /<GlobalToolbar/, "the global toolbar is isolated from page orchestration");
assert.match(globalToolbar, /aria-label="Go back"/, "the toolbar exposes Back navigation");
assert.match(globalToolbar, /aria-label="Go forward"/, "the toolbar exposes Forward navigation");
assert.match(shell, /<WorkspaceHeader/, "workspace orientation uses a shared header component");
assert.match(workspaceHeader, /<h1>\{title\}<\/h1>/, "the workspace header owns its title hierarchy");
assert.match(shell, /async function restoreShellLocation/, "history restoration is routed through the shell guards");
assert.match(history, /SHELL_HISTORY_LIMIT = 40/, "navigation history has a defined memory bound");
assert.match(history, /collection: WorkspaceCollectionLocation/, "workspace history includes collection context");
assert.match(history, /surfaceScrollTop: number/, "workspace and plugin history include specialized surface scroll");
assert.match(history, /panes: WorkspacePaneDimensions/, "workspace history retains pane dimensions");
assert.match(
  shell,
  /applyWorkspaceCollectionLocation\(target\.collection\)/,
  "Back and Forward restore collection filters",
);
assert.match(shell, /onScroll=\{rememberCollectionScroll\}/, "collection scroll position is retained for history");
assert.match(shell, /restoreSpecializedSurfaceScroll\(target\)/, "Back and Forward restore specialized surface scroll");
assert.match(shell, /restoreWorkspacePaneDimensions\(target\.panes\)/, "Back and Forward restore pane dimensions");
assert.match(appSidebar, />TOOLS</, "unowned plugin views are grouped as tools rather than primary workspaces");

assert.match(shell, /<CollectionPane/, "the collection pane is isolated from page orchestration");
assert.match(collectionPane, /bind:this=\{listElement\}/, "the collection pane owns its scroll boundary");
assert.match(shell, /<ContentPane/, "the content pane is isolated from page orchestration");
assert.match(contentPane, /class:editor-fullscreen/, "the content pane owns fullscreen presentation");
assert.match(shell, /<InspectorPane/, "the inspector pane is isolated from page orchestration");
assert.match(inspectorPane, /aria-label="Inspector"/, "the inspector pane owns its accessible landmark");
assert.match(shell, /<StatusSummary/, "save and load state use the shared status summary");
assert.match(statusSummary, /aria-live="polite"/, "status changes remain assistive-technology visible");
assert.match(shell, /<SpecializedSurface/, "plugin-owned surfaces use a shared scroll boundary");
assert.match(specializedSurface, /data-surface-key/, "specialized surfaces expose a stable restoration key");

assert.doesNotMatch(shell, />Open wiki</, "workspace views are no longer duplicated as heading actions");
assert.doesNotMatch(shell, />Open graph</, "Graph navigation is represented as a peer view");
assert.doesNotMatch(shell, />Open timeline</, "Timeline navigation is represented as a peer view");

console.log("beta UX navigation foundation contracts passed");
