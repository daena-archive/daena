import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import vm from "node:vm";

import { BUILTIN_THEME_TOKENS, THEME_TOKEN_IDS } from "../packages/plugin-sdk/src/generated.ts";
import {
  applyThemePreference,
  cacheThemePreference,
  normalizeThemePreference,
  readCachedThemePreference,
  resolveBuiltinTokens,
  resolveTheme,
} from "../src/lib/theme.ts";

assert.equal(normalizeThemePreference("dark"), "dark");
assert.equal(normalizeThemePreference("sepia"), "system");
assert.equal(resolveTheme("system", true), "dark");
assert.equal(resolveTheme("system", false), "light");
assert.equal(resolveTheme("light", true), "light");

const values = new Map();
const storage = {
  getItem: (key) => values.get(key) ?? null,
  setItem: (key, value) => values.set(key, value),
};
assert.equal(readCachedThemePreference(storage), "system");
cacheThemePreference("dark", storage);
assert.equal(readCachedThemePreference(storage), "dark");

function themeRoot() {
  const properties = new Map();
  return {
    dataset: {},
    properties,
    style: {
      colorScheme: "",
      setProperty(name, value) {
        properties.set(name, value);
      },
    },
  };
}

const root = themeRoot();
assert.equal(applyThemePreference("system", root, true), "dark");
assert.deepEqual(root.dataset, { theme: "dark", themePreference: "system" });
assert.equal(root.style.colorScheme, "dark");
for (const id of THEME_TOKEN_IDS) {
  assert.equal(root.properties.get(`--${id}`), BUILTIN_THEME_TOKENS.dark[id]);
}

const lightTokens = resolveBuiltinTokens("light");
assert.equal(lightTokens.canvas, "#f7f6f2");
assert.equal(Object.keys(lightTokens).length, THEME_TOKEN_IDS.length);

const startupSource = await readFile(new URL("../static/theme-init.js", import.meta.url), "utf8");
const startupRoot = themeRoot();
vm.runInNewContext(startupSource, {
  document: { documentElement: startupRoot },
  localStorage: { getItem: () => "system" },
  matchMedia: () => ({ matches: true }),
});
assert.deepEqual(startupRoot.dataset, { theme: "dark", themePreference: "system" });
assert.equal(startupRoot.style.colorScheme, "dark");
assert.equal(startupRoot.properties.get("--ink"), BUILTIN_THEME_TOKENS.dark.ink);
assert.equal(startupRoot.properties.get("--rail-bg"), BUILTIN_THEME_TOKENS.dark["rail-bg"]);

function cssRule(source, selector) {
  const start = source.indexOf(`${selector} {`);
  assert.notEqual(start, -1, `missing CSS rule ${selector}`);
  const end = source.indexOf("}", start);
  assert.notEqual(end, -1, `unterminated CSS rule ${selector}`);
  return source.slice(start, end + 1);
}

const pageSource = await readFile(new URL("../src/routes/+page.svelte", import.meta.url), "utf8");
assert.doesNotMatch(cssRule(pageSource, ":global(:root)"), /--ink:/);
assert.doesNotMatch(cssRule(pageSource, ':global(:root[data-theme="dark"])'), /--ink:/);
assert.match(cssRule(pageSource, ":global(:root)"), /--theme-surface-bg: var\(--surface\)/);
assert.match(cssRule(pageSource, ':global(:root[data-theme="dark"])'), /--shadow-sm:/);

const schemaSettingsSource = await readFile(new URL("../src/lib/SchemaSettingsPanel.svelte", import.meta.url), "utf8");
assert.match(cssRule(schemaSettingsSource, ".schema-plugin-toolbar"), /var\(--surface\)/);

const moduleSchemaSource = await readFile(new URL("../src/lib/ModuleSchemaPanel.svelte", import.meta.url), "utf8");
assert.match(cssRule(moduleSchemaSource, ".save-bar"), /var\(--surface\)/);

const gitSettingsSource = await readFile(new URL("../src/lib/GitSettingsPanel.svelte", import.meta.url), "utf8");
assert.match(cssRule(gitSettingsSource, ".git-diff-view"), /background: var\(--canvas\)/);
assert.match(cssRule(gitSettingsSource, ".git-diff-view span"), /width: max-content/);
assert.match(cssRule(gitSettingsSource, ".git-diff-view span"), /min-width: 100%/);
assert.match(cssRule(gitSettingsSource, ".git-diff-view.diff-wrap span"), /width: auto/);

console.log("theme tests passed");
