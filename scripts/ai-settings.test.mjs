import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const { BUNDLED_PROMPT_TEMPLATES, mergePromptTemplates, overlayFromTemplates, instructionFor, emptyAiProvider } =
  await import(new URL("../src/lib/ai/promptTemplates.ts", import.meta.url).href);

assert.equal(emptyAiProvider().endpoint, "");
assert.equal(emptyAiProvider().id, "");
assert.ok(BUNDLED_PROMPT_TEMPLATES.some((template) => template.id === "rewrite"));

const merged = mergePromptTemplates({
  templates: [
    { id: "rewrite", instruction: "Keep it dry.", label: "Dry rewrite" },
    { id: "grammar", enabled: false },
    { id: "house-voice", label: "House voice", instruction: "Write in the house idiom.", kind: "editor" },
  ],
});
assert.equal(instructionFor(merged, "rewrite"), "Keep it dry.");
assert.equal(merged.find((template) => template.id === "rewrite")?.label, "Dry rewrite");
assert.equal(merged.find((template) => template.id === "grammar")?.enabled, false);
assert.ok(merged.some((template) => template.id === "house-voice" && template.bundled === false));

const overlay = overlayFromTemplates(merged);
assert.ok(overlay.templates.some((item) => item.id === "rewrite"));
assert.ok(overlay.templates.some((item) => item.id === "grammar" && item.enabled === false));
assert.ok(overlay.templates.some((item) => item.id === "house-voice"));
assert.ok(!overlay.templates.some((item) => item.id === "expand"));

const settings = await readFile(new URL("../src-tauri/src/settings.rs", import.meta.url), "utf8");
assert.match(settings, /SETTINGS_FORMAT_VERSION: u32 = 1/);
assert.match(settings, /project_bindings/);
assert.doesNotMatch(settings, /DEFAULT_AI_ENDPOINT/);

const workspace = await readFile(new URL("../src/lib/modules/workspace.ts", import.meta.url), "utf8");
assert.match(workspace, /"ai"/);

const settingsView = await readFile(new URL("../src/lib/SettingsView.svelte", import.meta.url), "utf8");
assert.doesNotMatch(settingsView, /ProjectAiSettingsPanel|aiProviderPresets|Ask AI/);

const projectCenter = await readFile(new URL("../src/lib/ProjectCenter.svelte", import.meta.url), "utf8");
assert.match(projectCenter, /goToSection\("ai"\)/);

console.log("ai settings checks passed");
