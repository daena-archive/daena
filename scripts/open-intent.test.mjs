import assert from "node:assert/strict";
import { openIntent } from "../src/lib/navigation/open-intent.ts";

const pointer = (button, metaKey = false, ctrlKey = false) => ({ button, metaKey, ctrlKey });
const keys = (metaKey, ctrlKey) => ({ metaKey, ctrlKey });

assert.equal(openIntent(null, true), "current", "a missing event stays in the current tab");
assert.equal(openIntent(undefined, false), "current", "an undefined event stays in the current tab");
assert.equal(openIntent(pointer(0), true), "current", "a plain click stays in the current tab");
assert.equal(openIntent(pointer(1), false), "new-tab", "a middle click opens a new tab");
assert.equal(openIntent(pointer(1), true), "new-tab", "a middle click opens a new tab on macOS too");
assert.equal(openIntent(pointer(2), true), "current", "a secondary button without a modifier stays");

assert.equal(openIntent(pointer(0, true, false), true), "new-tab", "command-click opens a new tab on macOS");
assert.equal(openIntent(pointer(0, false, true), true), "current", "ctrl-click on macOS is a secondary click");
assert.equal(openIntent(pointer(0, false, true), false), "new-tab", "ctrl-click opens a new tab off macOS");
assert.equal(openIntent(pointer(0, true, false), false), "current", "the meta key alone is not the modifier off macOS");
assert.equal(openIntent(pointer(2, true, false), true), "new-tab", "a modified secondary click still opens a new tab");

assert.equal(openIntent(keys(false, false), true), "current", "enter stays in the current tab");
assert.equal(openIntent(keys(true, false), true), "new-tab", "command-enter opens a new tab on macOS");
assert.equal(openIntent(keys(false, true), true), "current", "ctrl-enter on macOS stays in the current tab");
assert.equal(openIntent(keys(false, true), false), "new-tab", "ctrl-enter opens a new tab off macOS");
assert.equal(openIntent(keys(true, false), false), "current", "command-enter off macOS stays in the current tab");
assert.equal(openIntent(keys(true, true), false), "new-tab", "ctrl-enter still opens a new tab when meta is also down");
