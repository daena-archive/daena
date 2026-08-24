import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const shell = await readFile(new URL("../src/routes/+page.svelte", import.meta.url), "utf8");
const settings = await readFile(new URL("../src/lib/SettingsView.svelte", import.meta.url), "utf8");
const projectCenter = await readFile(new URL("../src/lib/ProjectCenter.svelte", import.meta.url), "utf8");
const projectSwitcher = await readFile(new URL("../src/lib/shell/ProjectSwitcher.svelte", import.meta.url), "utf8");
const statusCenter = await readFile(new URL("../src/lib/shell/StatusCenter.svelte", import.meta.url), "utf8");
const fields = await readFile(new URL("../src/lib/SchemaSettingsPanel.svelte", import.meta.url), "utf8");
const history = await readFile(new URL("../src/lib/navigation/history.ts", import.meta.url), "utf8");

assert.match(shell, /settingsSurface === "application"/, "application Settings is an explicit surface");
assert.match(shell, /openProjectCenter/, "Project Center has a dedicated navigation boundary");
assert.match(history, /kind: "project"; section: ProjectSection/, "Project Center participates in shell history");
assert.doesNotMatch(settings, /> Plugins<\/button>/, "application Settings does not contain project extensions");
assert.doesNotMatch(settings, /> Snapshots<\/button>/, "application Settings does not contain project history");
assert.doesNotMatch(settings, /> Schema<\/button>/, "application Settings does not expose technical Schema navigation");

for (const section of ["Overview", "Data &amp; recovery", "Extensions", "Fields &amp; Types", "Snapshots", "Advanced"])
  assert.match(projectCenter, new RegExp(`> ${section}`), `Project Center includes ${section}`);
for (const operation of [
  "Import material",
  "Export Markdown",
  "Create portable backup",
  "Create recovery backup",
  "Restore recovery backup",
])
  assert.match(projectCenter, new RegExp(operation), `${operation} has one discoverable Project home`);
assert.match(projectCenter, /Project diagnostics/, "project diagnostics are available in Project Advanced");
assert.doesNotMatch(
  projectCenter,
  /Developer fixtures|Add example world/,
  "beta developer fixtures stay out of author surfaces",
);
assert.doesNotMatch(
  projectSwitcher,
  /Export Markdown|Import external material|Rebuild index|Seed example/,
  "the switcher no longer duplicates project administration",
);
assert.match(fields, /Fields &amp; Types/, "normal schema customization uses author-facing language");

for (const durabilityState of ["save", "checkpoint", "snapshot", "background"])
  assert.match(shell, new RegExp(`id: "${durabilityState}"`), `status center reports ${durabilityState} state`);
assert.match(shell, /documentConflict/, "conflicts feed the persistent status center");
assert.match(
  shell,
  /projectTransitionBusy\s*\|\|\s*aiIndexBusy\s*\|\|\s*moduleSchemaBusy/,
  "background operations feed the status center",
);
assert.match(statusCenter, /aria-live="polite"/, "status changes are announced without interrupting authors");
assert.match(statusCenter, /prefers-reduced-motion/, "status animation respects reduced-motion preferences");

console.log("project center and status contracts passed");
