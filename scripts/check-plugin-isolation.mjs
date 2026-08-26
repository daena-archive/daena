import assert from "node:assert/strict";
import fs from "node:fs";

const route = fs.readFileSync("src/routes/+page.svelte", "utf8");
const frame = fs.readFileSync("src/lib/plugins/SandboxView.svelte", "utf8");
const plugin = fs.readFileSync("src-tauri/plugin-assets/shared/plugin.js", "utf8");
const tauriSource = fs.readFileSync("src-tauri/src/lib.rs", "utf8");
const tauri = fs.readFileSync("src-tauri/tauri.conf.json", "utf8");
const capability = fs.readFileSync("src-tauri/capabilities/plugin.json", "utf8");

assert.equal(
  /packages\/modules\/(?:index\.ts|[^/]+\/src\/index\.ts)/.test(route),
  false,
  "main route must not import plugin runtime entrypoints",
);
assert.doesNotMatch(frame, /srcdoc|sandbox=/, "main webview must not host plugin code");
assert.match(frame, /mountPluginWebview/);
assert.match(frame, /resizePluginWebview/);
assert.match(frame, /unmountPluginWebview/);
assert.doesNotMatch(
  plugin,
  /__TAURI_INTERNALS__|@tauri-apps/,
  "plugin bundle must use the broker protocol, not Tauri APIs",
);
assert.match(plugin, /createBrowserPluginRpcTransport/);
assert.match(tauriSource, /plugin-sdk\.js/);
assert.match(tauriSource, /WebviewWindowBuilder::new/);
assert.match(tauriSource, /main\.add_child\(\s*builder/);
assert.match(tauriSource, /PageLoadEvent::Finished/);
assert.match(tauriSource, /LogicalSize::new\(1\.0, 1\.0\)/);
assert.match(tauriSource, /PLUGIN_WEBVIEW_ISOLATION_SCRIPT/);
assert.match(tauriSource, /__TAURI_INTERNALS__/);
assert.doesNotMatch(
  tauriSource,
  /initialization_script\("Object\.defineProperty\(window, '__TAURI_INTERNALS__'/,
  "plugin isolation script must neutralize Tauri internals fail-soft",
);
assert.match(tauriSource, /plugin_unmount_webview/);
assert.match(tauriSource, /use_https_scheme\(true\)/);
assert.match(tauriSource, /register_uri_scheme_protocol\("plugin"/);
assert.doesNotMatch(
  tauriSource,
  /register_uri_scheme_protocol\("plugin-daena-(?:lore|timeline)"/,
  "plugin assets must use the shared broker protocol",
);
assert.match(tauriSource, /plugin_protocol_response\(/);
assert.match(tauriSource, /plugin_window_label\(plugin_id\) != webview_label/);
assert.match(tauri, /connect-src 'self'/);
assert.match(tauri, /frame-src 'none'/);
assert.equal(/core:default/.test(capability), false, "plugin webviews must not receive the core API");

console.log("plugin webview isolation checks passed");
