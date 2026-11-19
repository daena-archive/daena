import assert from "node:assert/strict";
import { get } from "svelte/store";
import { formatUpdateMessage } from "../src/lib/appUpdate.ts";

import { loreFieldLabel, loreTypeLabel } from "../src/lib/i18n/lore-labels.ts";

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

const { displayLanguage, languageOptions, messages, setDisplayLanguage } = await import("../src/lib/i18n/index.ts");
assert.equal(get(displayLanguage), "en");
assert.equal(get(messages).projectSwitcher.open, "Open folder");
assert.equal(languageOptions.find(option => option.value === "ja")?.status, "alpha");

const seen = [];
const unsubscribe = messages.subscribe((catalog) => seen.push(catalog.common.settings));
setDisplayLanguage("ja");
assert.deepEqual(seen, ["Settings", "設定"]);
unsubscribe();
assert.equal(stored, "ja");
const japanese = get(messages);
const { manifestText, setPackageLocalizations, schemaLabels } = await import("../src/lib/i18n/manifest-copy.ts");
const { formatUi } = await import("../src/lib/i18n/ui.ts");
const { profileComponentCopy } = await import("../src/lib/i18n/profile-copy.ts");
assert.equal(get(manifestText)("daena.lore", "template", "person", "Person"), "人物");
assert.equal(get(manifestText)("daena.lore", "template", "person", "Map"), "Map");
assert.equal(get(manifestText)("third.party", "template", "person", "Person"), "Person");
assert.equal(get(manifestText)("daena.writing", "option", "status/drafting", "drafting"), "執筆中");
assert.equal(get(manifestText)("daena.writing", "option", "custom/drafting", "drafting"), "drafting");
assert.equal(get(manifestText)("daena.houses", "metadata", "family_member_of/role", "Role"), "役割");
const { default: notes } = await import("../examples/plugins/declarative/manifest.json", { with: { type: "json" } });
const { default: notesJa } = await import("../examples/plugins/declarative/locales/ja.json", { with: { type: "json" } });
const installedNotes = structuredClone(notes);
installedNotes.schemas[0].entityTypes[0].id = notes.id + ":note";
const otherNotes = { ...notes, id: "other.plugin" };
setPackageLocalizations([
  { id: notes.id, localization: { source: installedNotes, catalogs: { ja: notesJa } } },
  { id: otherNotes.id, localization: { source: otherNotes, catalogs: { ja: { Color: "表示色" } } } },
]);
assert.equal(get(manifestText)(notes.id, "type", notes.id + ":note", "Note"), "ノート");
assert.equal(get(manifestText)(notes.id, "module", notes.id, notes.name), "観察ノート");
assert.equal(get(manifestText)(notes.id, "field", "color", "Color"), "色");
assert.equal(get(manifestText)(notes.id, "field", "color", "Note"), "Note", "preserve a project override even if it is a catalog key");
assert.equal(get(manifestText)(notes.id, "component", "notes/refresh", "Refresh notes"), "ノートを再読み込み");
assert.equal(get(manifestText)("other.plugin", "field", "color", "Color"), "表示色");
assert.equal(get(schemaLabels).type(notes.id + ":note"), "ノート");
assert.equal(get(schemaLabels).type("other.plugin:note"), "Note", "an untranslated type keeps its own package label");
assert.equal(get(schemaLabels).type("uninstalled:travel-log"), "Travel Log");
assert.equal(get(schemaLabels).type(null), "未分類");
assert.equal(get(manifestText)("daena.houses", "metadataOption", "family_parent_of/kind", "adoptive"), "養親");
setDisplayLanguage("en");
assert.equal(get(schemaLabels).type(notes.id + ":note"), "Note");
assert.equal(get(manifestText)(notes.id, "field", "color", "Color"), "Color");
setDisplayLanguage("ja");
setPackageLocalizations([]);
assert.equal(get(manifestText)(notes.id, "field", "color", "Color"), "Color", "uninstalled plugin");
assert.equal(profileComponentCopy("dnd", "attribute-strength", "Strength"), "筋力");
assert.equal(profileComponentCopy("dnd", "attribute-strength", "Renamed"), "Renamed");
assert.equal(formatUi("ja", "Close {0}", "$& {0}"), "$& {0}を閉じる");

assert.equal(loreTypeLabel(japanese.loreTypes, "daena.lore:person", "Person"), "人物");
assert.equal(loreTypeLabel(japanese.loreTypes, "other:person", "Person"), "Person");
assert.equal(loreTypeLabel(japanese.loreTypes, "daena.lore:person", "Hero"), "Hero");
assert.equal(loreFieldLabel(japanese.loreFields, "occupation", "Occupation"), "職業");
assert.equal(loreFieldLabel(japanese.loreFields, "occupation", "Day job"), "Day job");
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

const { formatNativeError, nativeErrorText } = await import("../src/lib/i18n/native-errors.ts");
const { friendlyError, rawErrorMessage } = await import("../src/lib/shell/errors.ts");
const codedPath = "/projects/あお {0}/project.json [project.name] name cannot be empty";
assert.equal(
  get(nativeErrorText)(codedPath),
  "/projects/あお {0}/project.json [project.name] 名前を入力してください。",
);
assert.equal(formatNativeError("en", codedPath), codedPath);
assert.equal(formatNativeError("ja", "unknown commit: branch {1}"), "コミットが見つかりません：branch {1}");
assert.equal(
  formatNativeError("ja", "relationship.self: relationship cannot target the same entity"),
  "relationship.self: 項目を自分自身に関連づけることはできません。",
);
assert.equal(formatNativeError("ja", "external-service: external diagnostic"), "external-service: external diagnostic");
assert.equal(
  formatNativeError("ja", "plugin Sea {1} version 2.0 does not satisfy ^3.0"),
  "プラグイン「Sea {1}」のバージョン 2.0 は条件 ^3.0 を満たしていません。",
);
assert.equal(
  formatNativeError("ja", "atlas.save.failed: refused to follow a symlink"),
  "atlas.save.failed: シンボリックリンクを経由した保存はできません。",
);
const inner = "atlas request ID must be a UUID";
assert.ok(formatNativeError("ja", "maps.physical: invalid settings: " + inner)
  .includes(formatNativeError("ja", inner)));
assert.ok(formatNativeError("ja", "database error: " + inner).endsWith(inner));
const externalDiagnostic = "storm ".repeat(16000);
assert.equal(formatNativeError("ja", externalDiagnostic), externalDiagnostic);
const { default: nativeCatalog } = await import("../src/lib/i18n/native-errors.ja.json", {
  with: { type: "json" },
});
for (const [source, translation] of Object.entries(nativeCatalog)) {
  const substitute = (text) => text.replace(/\{(\d+)\}/g, (_, i) => `外部${i}値`);
  assert.equal(formatNativeError("ja", substitute(source)), substitute(translation), source);
  assert.deepEqual(
    [...translation.matchAll(/\{\d+\}/g)].map(([parameter]) => parameter).sort(),
    [...source.matchAll(/\{\d+\}/g)].map(([parameter]) => parameter).sort(),
    source,
  );
}
assert.equal(friendlyError("cannot find undefined.md"), "cannot find undefined.md");
const conflict = new Error("entity revision conflict: expected abc, current xyz");
assert.match(friendlyError(conflict), /編集開始時：abc、現在：xyz/);
assert.equal(rawErrorMessage(conflict), conflict.message);

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
// A detached plugin uses the host's language even when its own origin cannot persist preferences.
const { listenForPluginLanguage, translatePluginText } = await import("../packages/modules/plugin-i18n.ts");
globalThis.window = new EventTarget();
globalThis.document = { documentElement: { lang: "en" }, title: "" };
window.__DAENA_DISPLAY_LANGUAGE__ = "ja";
listenForPluginLanguage("World graph");
assert.equal(document.title, "世界の関係図");
assert.equal(
  translatePluginText("Displayed on the {calendar} calendar", { calendar: "Haru {1}" }),
  "表示する暦：Haru {1}",
);
window.__DAENA_DISPLAY_LANGUAGE__ = "en";
window.dispatchEvent(new Event("daena:language"));
assert.equal(document.documentElement.lang, "en");
assert.equal(document.title, "World graph");
assert.equal(
  translatePluginText("Displayed on the {calendar} calendar", { calendar: "Haru" }),
  "Displayed on the Haru calendar",
);

const { createPluginI18n, translatePluginMessage } = await import("../packages/plugin-sdk/dist/index.js");
assert.equal(translatePluginMessage({ "ja-jp": { Note: " " }, ja: { Note: "ノート" } }, "ja-JP", "Note"), "ノート");
assert.equal(translatePluginMessage({ ja: { Note: "" } }, "ja", "Note"), "Note");
window.__DAENA_PLUGIN_CATALOGS__ = { ja: { "Open {name}": "{name}を開く" } };
const pluginUi = createPluginI18n();
const pluginLabels = [];
const stop = pluginUi.subscribe(() => pluginLabels.push(pluginUi.t("Open {name}", { name: "{name} $&" })));
window.__DAENA_DISPLAY_LANGUAGE__ = "ja-JP";
window.dispatchEvent(new Event("daena:language"));
window.dispatchEvent(new Event("daena:language"));
stop();
window.__DAENA_DISPLAY_LANGUAGE__ = "en";
window.dispatchEvent(new Event("daena:language"));
assert.deepEqual(pluginLabels, ["Open {name} $&", "{name} $&を開く"]);
assert.equal(pluginUi.t("constructor"), "constructor");
assert.equal(translatePluginMessage({ ja: { label: "親" }, "ja-jp": { label: "地域" } }, "ja-JP", "label"), "地域");
console.log("Host and SDK language switching, package scoping and persistence checks passed");
