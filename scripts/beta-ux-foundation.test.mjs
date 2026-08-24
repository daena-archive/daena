import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const shell = await readFile(new URL("../src/routes/+page.svelte", import.meta.url), "utf8");
const projectHome = await readFile(new URL("../src/lib/shell/ProjectHome.svelte", import.meta.url), "utf8");
const workspaceViewNav = await readFile(new URL("../src/lib/shell/WorkspaceViewNav.svelte", import.meta.url), "utf8");
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

const homeNavigation = shell.indexOf("<span>Home</span>");
const workspaceNavigation = shell.indexOf('aria-label="Workspace sections"');
assert.ok(homeNavigation >= 0, "Home is a visible sidebar destination");
assert.ok(workspaceNavigation > homeNavigation, "Home appears before workspace destinations");
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

assert.match(shell, /aria-label="Go back"/, "the shell exposes Back navigation");
assert.match(shell, /aria-label="Go forward"/, "the shell exposes Forward navigation");
assert.match(shell, /async function restoreShellLocation/, "history restoration is routed through the shell guards");
assert.match(history, /SHELL_HISTORY_LIMIT = 40/, "navigation history has a defined memory bound");
assert.match(shell, />TOOLS</, "unowned plugin views are grouped as tools rather than primary workspaces");

assert.doesNotMatch(shell, />Open wiki</, "workspace views are no longer duplicated as heading actions");
assert.doesNotMatch(shell, />Open graph</, "Graph navigation is represented as a peer view");
assert.doesNotMatch(shell, />Open timeline</, "Timeline navigation is represented as a peer view");

console.log("beta UX navigation foundation contracts passed");
