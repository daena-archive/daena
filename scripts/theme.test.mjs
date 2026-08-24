import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import vm from "node:vm";

import {
  applyThemePreference,
  cacheThemePreference,
  normalizeThemePreference,
  readCachedThemePreference,
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

const root = { dataset: {}, style: {} };
assert.equal(applyThemePreference("system", root, true), "dark");
assert.deepEqual(root.dataset, { theme: "dark", themePreference: "system" });
assert.equal(root.style.colorScheme, "dark");

const startupSource = await readFile(new URL("../static/theme-init.js", import.meta.url), "utf8");
const startupRoot = { dataset: {}, style: {} };
vm.runInNewContext(startupSource, {
  document: { documentElement: startupRoot },
  localStorage: { getItem: () => "system" },
  matchMedia: () => ({ matches: true }),
});
assert.deepEqual(startupRoot.dataset, { theme: "dark", themePreference: "system" });
assert.equal(startupRoot.style.colorScheme, "dark");

console.log("theme tests passed");
