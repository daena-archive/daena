import assert from "node:assert/strict";
import {
  TAB_SHORTCUTS as shellShortcuts,
  isTabShortcut,
  tabShortcutForKey as shellForKey,
} from "../src/lib/navigation/tab-shortcuts.ts";
import {
  TAB_SHORTCUTS as writingShortcuts,
  tabShortcutForKey as writingForKey,
} from "../packages/modules/writing/src/plugin-protocol.ts";
import {
  TAB_SHORTCUTS as languageShortcuts,
  tabShortcutForKey as languageForKey,
} from "../packages/modules/language/src/plugin-protocol.ts";

const base = { metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, repeat: false };
const press = (key, mods = {}) => ({ ...base, key, ...mods });

/** [press, macOS result, Windows/Linux result]. Null means the keydown path does not handle it. */
const rows = [
  [press("t", { metaKey: true }), null, null],
  [press("w", { metaKey: true }), null, null],
  [press("t", { metaKey: true, shiftKey: true }), null, null],
  [press("T", { metaKey: true, shiftKey: true }), null, null],
  [press("1", { metaKey: true }), "tab-1", null],
  [press("8", { metaKey: true }), "tab-8", null],
  [press("9", { metaKey: true }), "tab-last", null],
  [press("1", { metaKey: true, shiftKey: true }), null, null],
  [press("1", { metaKey: true, repeat: true }), null, null],
  [press("t", { metaKey: true, repeat: true }), null, null],
  [press("t", { metaKey: true, ctrlKey: true }), null, null],
  [press("]", { metaKey: true, shiftKey: true }), null, null],
  [press("Tab", { ctrlKey: true }), "tab-next", "tab-next"],
  [press("Tab", { ctrlKey: true, shiftKey: true }), "tab-previous", "tab-previous"],
  [press("Tab", { ctrlKey: true, repeat: true }), "tab-next", "tab-next"],
  [press("Tab", { ctrlKey: true, shiftKey: true, repeat: true }), "tab-previous", "tab-previous"],
  [press("Tab", { ctrlKey: true, metaKey: true }), null, null],
  [press("Tab", { ctrlKey: true, altKey: true }), null, null],
  [press("PageDown", { ctrlKey: true }), null, "tab-next"],
  [press("PageUp", { ctrlKey: true }), null, "tab-previous"],
  [press("PageDown", { ctrlKey: true, repeat: true }), null, "tab-next"],
  [press("PageUp", { ctrlKey: true, repeat: true }), null, "tab-previous"],
  [press("PageDown", { ctrlKey: true, shiftKey: true }), null, null],
  [press("PageDown", { ctrlKey: true, altKey: true }), null, null],
  [press("PageDown", { metaKey: true }), null, null],
  [press("t", { ctrlKey: true }), null, "tab-new"],
  [press("T", { ctrlKey: true }), null, "tab-new"],
  [press("w", { ctrlKey: true }), null, "tab-close"],
  [press("W", { ctrlKey: true }), null, "tab-close"],
  [press("t", { ctrlKey: true, shiftKey: true }), null, "tab-reopen"],
  [press("T", { ctrlKey: true, shiftKey: true }), null, "tab-reopen"],
  [press("t", { ctrlKey: true, repeat: true }), null, null],
  [press("w", { ctrlKey: true, repeat: true }), null, null],
  [press("t", { ctrlKey: true, shiftKey: true, repeat: true }), null, null],
  [press("1", { ctrlKey: true }), null, "tab-1"],
  [press("8", { ctrlKey: true }), null, "tab-8"],
  [press("9", { ctrlKey: true }), null, "tab-last"],
  [press("0", { ctrlKey: true }), null, null],
  [press("1", { ctrlKey: true, shiftKey: true }), null, null],
  [press("1", { ctrlKey: true, repeat: true }), null, null],
  [press("1", { ctrlKey: true, metaKey: true }), null, null],
  [press("t", { ctrlKey: true, altKey: true }), null, null],
  [press("w", { ctrlKey: true, altKey: true }), null, null],
  [press("1", { altKey: true, metaKey: true }), null, null],
  [press("1"), null, null],
  [press("9"), null, null],
  [press("t"), null, null],
  [press("w"), null, null],
  [press("k", { metaKey: true }), null, null],
  [press("k", { ctrlKey: true }), null, null],
  [press("n", { ctrlKey: true }), null, null],
  [press("Tab"), null, null],
  [press("a", { metaKey: true, code: "Digit1" }), null, null],
  [press("!", { metaKey: true, code: "Digit1" }), null, null],
];

assert.deepEqual(writingShortcuts, shellShortcuts, "Writing's tab shortcut list matches the shell");
assert.deepEqual(languageShortcuts, shellShortcuts, "Language's tab shortcut list matches the shell");

for (const name of shellShortcuts) assert.equal(isTabShortcut(name), true, name);
assert.equal(isTabShortcut("tab-9"), false);
assert.equal(isTabShortcut("tab-0"), false);
assert.equal(isTabShortcut("quick-open"), false);
assert.equal(isTabShortcut("TAB-NEW"), false);
assert.equal(isTabShortcut(""), false);
assert.equal(isTabShortcut(1), false);
assert.equal(isTabShortcut(undefined), false);

for (const isMac of [true, false]) {
  for (const [input, onMac, onOther] of rows) {
    const expected = isMac ? onMac : onOther;
    const label = `${isMac ? "mac" : "other"} ${JSON.stringify(input)}`;
    const shell = shellForKey(input, isMac);
    assert.equal(shell, expected, label);
    assert.equal(writingForKey(input, isMac), shell, `writing agrees: ${label}`);
    assert.equal(languageForKey(input, isMac), shell, `language agrees: ${label}`);
  }
}

console.log("tab shortcuts: ok");
