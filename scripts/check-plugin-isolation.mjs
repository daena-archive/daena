import assert from "node:assert/strict";
import fs from "node:fs";

const route = fs.readFileSync("src/routes/+page.svelte", "utf8");
const frame = fs.readFileSync("src/lib/modules/PluginFrame.svelte", "utf8");
const tauri = fs.readFileSync("src-tauri/tauri.conf.json", "utf8");
const capability = fs.readFileSync("src-tauri/capabilities/plugin.json", "utf8");

assert.equal(/packages\/modules\/(?:index\.ts|lore\/src|timeline\/src)/.test(route), false, "main route must not import plugin implementations");
assert.match(frame, /sandbox=\"allow-scripts\"/);
assert.match(frame, /event\.source !== frame\?\.contentWindow/);
assert.match(frame, /connect-src 'none'/);
assert.match(frame, /script-src 'unsafe-inline'/);
assert.match(tauri, /connect-src 'self'/);
assert.match(tauri, /frame-src 'none'/);
assert.equal(/core:default/.test(capability), false, "plugin webviews must not receive the core API");

console.log("plugin webview isolation checks passed");
