import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

// Source checks for the keep-alive pool (docs/plans/keep-alive-webviews.md, slice 3): Writing is the only pooled
// view, the pooled and unpooled SandboxView paths stay apart, and the shell wires the pool the way the design needs.

const root = resolve(import.meta.dirname, "..");
const read = (path) => readFile(resolve(root, path), "utf8");

const between = (source, start, end) => {
  const from = source.indexOf(start);
  assert.notEqual(from, -1, `source contains ${start}`);
  const to = source.indexOf(end, from + start.length);
  assert.notEqual(to, -1, `source contains ${end} after ${start}`);
  return source.slice(from, to);
};

const page = await read("src/routes/+page.svelte");
const sandbox = await read("src/lib/plugins/SandboxView.svelte");
const client = await read("src/lib/project/client.ts");
const lib = await read("src-tauri/src/lib.rs");
const webview = await read("src-tauri/src/plugin_webview.rs");

// Every SandboxView in the shell; only the Writing one is pooled.
const usages = [...page.matchAll(/<SandboxView\b[\s\S]*?\/>/g)].map((match) => match[0]);
assert.equal(usages.length, 4, "the shell has four SandboxView usages");
const pooled = usages.filter((usage) => /\bpooled\b/.test(usage));
assert.equal(pooled.length, 1, "exactly one SandboxView is pooled");
assert.match(pooled[0], /pluginId="daena\.writing"/, "the pooled SandboxView is the Writing one");
assert.match(
  pooled[0],
  /prepareMount=\{beginPooledWritingMount\}/,
  "Writing's retain list and reuse come from the pool",
);
assert.match(pooled[0], /onmounted=\{writingMounted\}/, "Writing's mount outcome reaches the shell");
for (const usage of usages.filter((candidate) => !/\bpooled\b/.test(candidate))) {
  assert.doesNotMatch(usage, /\bonmounted=/, "an unpooled SandboxView takes no mount outcome");
  // Unpooled embedded views retain the pooled instances that fit beside them, or visiting Lore would close every
  // hidden Writing webview.
  assert.match(usage, /prepareMount=\{prepareUnpooledMount\}/, "an unpooled SandboxView keeps the pool alive");
}
const language = usages.find((usage) => /pluginId="daena\.language"/.test(usage));
assert.ok(language && !/\bpooled\b/.test(language), "Language is not pooled in this slice");
const sandboxViews = usages.filter((usage) => /pluginId=\{/.test(usage));
assert.equal(sandboxViews.length, 2, "the graph, Chronology and sandbox views use the two plugin-id-driven usages");
for (const usage of sandboxViews)
  assert.doesNotMatch(usage, /\bpooled\b/, "graph, Chronology and sandbox views are not pooled");

// SandboxView: unpooled behaves as before; pooled adopts and suspends.
assert.match(sandbox, /pooled = false/, "pooled defaults to false");
const mount = between(sandbox, "const mount = async () => {", "void mount().catch");
assert.match(
  mount,
  /if \(!isPooled\) \{[\s\S]*?await project\.unmountPluginWebview\(mountedPluginId, mountedInstanceId\)/,
  "only an unpooled mount drops its own label first",
);
assert.match(mount, /const reuse = isPooled && \(plan\?\.reuse \?\? false\);/, "only a pooled mount can ask for reuse");
assert.match(
  mount,
  /const keep = plan\?\.retain \?\? \(isPooled \? \[self\] : \[\]\);/,
  "without a plan, a pooled mount keeps itself and an unpooled one keeps nothing",
);
assert.match(mount, /serialisedState,\s*reuse,\s*keep,/, "reuse and the kept list reach the host");
const begin = between(page, "function beginPooledWritingMount(", "\n}\n");
assert.match(
  begin,
  /const known = Boolean\(before && before\.mountKey === mountKey\);[\s\S]*reuse: known/,
  "reuse only for an instance the pool already knew with the same mount key",
);
const unpooled = between(page, "function prepareUnpooledMount(", "\n}\n");
assert.match(
  unpooled,
  /retainSet\(webviewPool, self, WEBVIEW_POOL_LIMIT\)/,
  "the unpooled view counts toward the limit",
);
assert.match(
  unpooled,
  /removeWebviews\(webviewPool, evicted\(webviewPool, retain\)\)/,
  "evicted entries leave the pool",
);
assert.match(unpooled, /reuse: false/);
assert.match(
  mount,
  /if \(isPooled\) onmounted\?\.\(outcome, self, mountKey\)/,
  "only a pooled view reports the outcome",
);
assert.match(
  mount,
  /if \(!alive\) \{[\s\S]*?suspendPluginWebview/,
  "a pooled view torn down mid-mount suspends what the host just created",
);
const teardown = sandbox.slice(sandbox.lastIndexOf("return () => {"), sandbox.indexOf("</script>"));
assert.match(
  teardown,
  /if \(isPooled\) void project\.suspendPluginWebview\(mountedPluginId, mountedInstanceId\)[^;]*;\s*else void project\.unmountPluginWebview\(mountedPluginId, mountedInstanceId\);/,
  "pooled teardown suspends; unpooled teardown unmounts",
);
assert.doesNotMatch(sandbox, /closePluginWebview/, "teardown never closes every instance of the plugin");

// The shell prunes the pool on one effect over the live tab ids, and clears it with the project session.
const prune = between(page, "$effect(() => {\n  // One place covers every way", "/**\n * `{#key}` creates");
assert.match(prune, /pruneWebviewPoolTabs\(webviewPool, live\)/, "the pool is pruned to the live tab ids");
assert.match(prune, /prunePlugins\(next, enabled\)/, "the pool is pruned to the enabled plugins");
assert.match(
  prune,
  /unmountPluginWebview\(instance\.pluginId, instance\.instanceId\)/,
  "pruned instances are unmounted",
);
assert.match(prune, /\bliveTabIdKey\b/, "the effect depends on the open tab ids");
const reset = between(page, "function resetProjectSessionState() {", "mapsEditorKey = ");
assert.match(reset, /webviewPool = emptyWebviewPool;/, "a project switch clears the pool");

// Only the shown instance is ever shown by the visibility effect, and a suspended Writing one waits for its mount.
const visibility = between(page, "void writingHostedInstance;", "});\n\n");
assert.match(
  visibility,
  /visible && pluginId === WRITING_PLUGIN_ID && writingHostedInstance !== shownWebviewInstance/,
  "the visibility effect does not show a Writing instance that is not mounted yet",
);
assert.match(page, /async function setPluginWebviewVisibleQuietly[\s\S]*?const instanceId = shownWebviewInstance;/);

// Full screen ends on the instance that reported it, not on whichever tab is current by then.
const fullscreen = between(page, "function endWritingFullscreen() {", "\n}\n");
assert.match(fullscreen, /postWritingMessage\(writingFullscreenMessage\(false\), instance\)/);
assert.match(page, /writingFullscreenInstance = event\.on \? shownWebviewInstance : null/);

// A ready report from a hidden instance still reaches the pool, and is read before the parser of the shown path.
const writingState = between(page, 'listen<unknown>("writing-state"', "return () => {");
assert.ok(
  writingState.indexOf("foreignWebviewInstance(event.payload)") < writingState.indexOf("noteHiddenWritingReady"),
  "a foreign event is noted for the pool, not handled",
);

// Host side: suspend is a registered command, and the mount takes reuse and retain.
assert.match(lib, /plugin_suspend_webview,/, "plugin_suspend_webview is in the invoke handler");
assert.match(client, /invoke<void>\("plugin_suspend_webview", \{ pluginId, instanceId \}\)/);
assert.match(webview, /reuse: bool,\s*retain: Vec<WebviewInstanceRef>,/, "the mount takes reuse and retain");
assert.match(webview, /Result<MountOutcome, String>/, "the mount reports reused or created");
const suspend = between(webview, "pub(super) fn plugin_suspend_webview(", "\n}\n");
assert.match(suspend, /cancel_plugin_webview_ai\(&app, &label\)/, "suspend cancels the label's AI requests");
assert.match(suspend, /hide_embedded_webview/, "suspend hides the webview");
assert.doesNotMatch(suspend, /\.close\(\)/, "suspend never closes");
const hide = between(webview, "fn hide_embedded_webview(", "\n}\n");
assert.ok(hide.indexOf('get_webview("main")') < hide.indexOf(".hide()"), "main takes focus before the webview hides");
assert.match(
  webview,
  /embedded_labels_to_close\(&existing, &label, &retained\)/,
  "the mount closes only what is not retained",
);

console.log("webview pool source: ok");
