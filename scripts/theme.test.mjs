import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import vm from "node:vm";

import { BUILTIN_THEME_TOKENS, THEME_TOKEN_IDS } from "../packages/plugin-sdk/src/generated.ts";
import {
  applyThemePreference,
  cacheThemePackTokens,
  cacheThemePreference,
  collectInstalledThemePacks,
  normalizeThemePackRef,
  normalizeThemePreference,
  overlayForThemePack,
  readCachedThemePackTokens,
  readCachedThemePreference,
  resolveAppliedTokens,
  resolveBuiltinTokens,
  resolveTheme,
  resolvedPackTokenCache,
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
  removeItem: (key) => values.delete(key),
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

assert.deepEqual(normalizeThemePackRef({ pluginId: "com.example.skins", themeId: "parchment" }), {
  pluginId: "com.example.skins",
  themeId: "parchment",
});
assert.equal(normalizeThemePackRef({ pluginId: "", themeId: "parchment" }), null);

const packs = collectInstalledThemePacks([
  {
    id: "com.example.skins",
    name: "Skins",
    themes: [
      { id: "parchment", name: "Parchment", tokens: { light: { accent: "#aa7744" }, dark: { accent: "#c58a4a" } } },
    ],
  },
]);
assert.equal(packs.length, 1);
assert.equal(
  collectInstalledThemePacks([
    {
      id: "com.example.skins",
      name: "Skins",
      themes: [{ id: "broken", name: "Broken", tokens: { dark: { accent: "#c58a4a" } } }],
    },
  ]).length,
  0,
);
assert.equal(overlayForThemePack(packs, { pluginId: "com.example.skins", themeId: "missing" }, "light"), null);
assert.equal(
  overlayForThemePack(packs, { pluginId: "com.example.skins", themeId: "parchment" }, "light").accent,
  "#aa7744",
);
assert.equal(resolveAppliedTokens("light", { accent: "#aa7744" }).accent, "#aa7744");
assert.equal(resolveAppliedTokens("light", { ink: "#fffefa", surface: "#fffefa" }).ink, BUILTIN_THEME_TOKENS.light.ink);

const overlayRoot = themeRoot();
applyThemePreference("light", overlayRoot, false, { accent: "#aa7744" });
assert.equal(overlayRoot.properties.get("--accent"), "#aa7744");
applyThemePreference("light", overlayRoot, false, { ink: "#fffefa", surface: "#fffefa" });
assert.equal(overlayRoot.properties.get("--ink"), BUILTIN_THEME_TOKENS.light.ink);

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

cacheThemePackTokens({ light: BUILTIN_THEME_TOKENS.light, dark: BUILTIN_THEME_TOKENS.dark }, storage);
assert.equal(readCachedThemePackTokens(storage).light.accent, BUILTIN_THEME_TOKENS.light.accent);
const cachedPack = resolvedPackTokenCache(packs, { pluginId: "com.example.skins", themeId: "parchment" });
assert.equal(cachedPack.light.accent, "#aa7744");
assert.equal(resolvedPackTokenCache(packs, { pluginId: "com.example.skins", themeId: "missing" }), null);
cacheThemePackTokens(cachedPack, storage);
assert.equal(readCachedThemePackTokens(storage).light.accent, "#aa7744");
cacheThemePackTokens(null, storage);
assert.equal(readCachedThemePackTokens(storage), null);

const packStartupRoot = themeRoot();
vm.runInNewContext(startupSource, {
  document: { documentElement: packStartupRoot },
  localStorage: {
    getItem: (key) => {
      if (key === "daena-theme") return "light";
      if (key === "daena-theme-pack") {
        return JSON.stringify({
          light: { ...BUILTIN_THEME_TOKENS.light, accent: "#aa7744" },
          dark: BUILTIN_THEME_TOKENS.dark,
        });
      }
      return null;
    },
  },
  matchMedia: () => ({ matches: false }),
});
assert.equal(packStartupRoot.properties.get("--accent"), "#aa7744");

const corruptStartupRoot = themeRoot();
vm.runInNewContext(startupSource, {
  document: { documentElement: corruptStartupRoot },
  localStorage: {
    getItem: (key) => {
      if (key === "daena-theme") return "light";
      if (key === "daena-theme-pack") return JSON.stringify({ light: {}, dark: {} });
      return null;
    },
  },
  matchMedia: () => ({ matches: false }),
});
assert.equal(corruptStartupRoot.properties.get("--accent"), BUILTIN_THEME_TOKENS.light.accent);

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
