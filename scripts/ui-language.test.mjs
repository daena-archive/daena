import assert from "node:assert/strict";
import { get } from "svelte/store";
import { formatUpdateMessage } from "../src/lib/appUpdate.ts";

const storageKey = "daena:display-language";
let stored = "unsupported";
Object.defineProperty(globalThis, "localStorage", {
  configurable: true,
  value: {
    getItem: (key) => (key === storageKey ? stored : null),
    setItem: (key, value) => {
      assert.equal(key, storageKey);
      stored = value;
    },
  },
});

const { displayLanguage, messages, setDisplayLanguage } = await import("../src/lib/i18n/index.ts");
assert.equal(get(displayLanguage), "en");
assert.equal(get(messages).projectSwitcher.open, "Open folder");

const seen = [];
const unsubscribe = messages.subscribe((catalog) => seen.push(catalog.common.settings));
setDisplayLanguage("ja");
assert.deepEqual(seen, ["Settings", "設定"]);
unsubscribe();
assert.equal(stored, "ja");
assert.equal(get(messages).projectSwitcher.removeNamedRecent("My world"), "「My world」を最近のプロジェクトから削除");
assert.equal(
  formatUpdateMessage(
    {
      current: "0.1.0",
      latest: "0.2.0-beta.1",
      newer: true,
      htmlUrl: "",
      releaseChannel: "beta",
      latestPrerelease: true,
      updateChannelPreference: "auto",
    },
    get(messages).settings,
  ),
  "アップデートがあります（0.2.0-beta.1、ベータ版）",
);

// Load a fresh store as the app would on restart.
const restarted = await import("../src/lib/i18n/index.ts?restart");
assert.equal(get(restarted.displayLanguage), "ja");
restarted.setDisplayLanguage("en");
assert.equal(get(restarted.messages).common.settings, "Settings");
assert.equal(stored, "en");

Object.defineProperty(globalThis, "localStorage", {
  configurable: true,
  get() {
    throw new Error("Storage unavailable");
  },
});
const ephemeral = await import("../src/lib/i18n/index.ts?unavailable");
assert.equal(get(ephemeral.displayLanguage), "en");
ephemeral.setDisplayLanguage("ja");
assert.equal(get(ephemeral.displayLanguage), "ja");
console.log("UI language switching and persistence checks passed");
