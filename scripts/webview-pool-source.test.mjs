import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

// Source checks for the keep-alive pool (docs/plans/keep-alive-webviews.md, slices 3 and 4): Writing and Language are
// the pooled views, the pooled and unpooled SandboxView paths stay apart, and the shell wires the pool the way the
// design needs.

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

// Every SandboxView in the shell; only the Writing and Language ones are pooled.
const usages = [...page.matchAll(/<SandboxView\b[\s\S]*?\/>/g)].map((match) => match[0]);
assert.equal(usages.length, 4, "the shell has four SandboxView usages");
const pooled = usages.filter((usage) => /\bpooled\b/.test(usage));
assert.equal(pooled.length, 2, "exactly two SandboxViews are pooled");
const pooledWriting = pooled.find((usage) => /pluginId="daena\.writing"/.test(usage));
const pooledLanguage = pooled.find((usage) => /pluginId="daena\.language"/.test(usage));
assert.ok(pooledWriting && pooledLanguage, "the pooled SandboxViews are the Writing and Language ones");
assert.match(
  pooledWriting,
  /prepareMount=\{beginPooledWritingMount\}/,
  "Writing's retain list and reuse come from the pool",
);
assert.match(pooledWriting, /onmounted=\{writingMounted\}/, "Writing's mount outcome reaches the shell");
assert.match(
  pooledLanguage,
  /prepareMount=\{beginPooledLanguageMount\}/,
  "Language's retain list and reuse come from the pool",
);
assert.match(pooledLanguage, /onmounted=\{languageMounted\}/, "Language's mount outcome reaches the shell");
assert.match(
  pooledLanguage,
  /moduleState=\{languageMountState\(shownWebviewInstance\)\}/,
  "Language's mount state does not follow the pane the user moved to",
);
for (const usage of usages.filter((candidate) => !/\bpooled\b/.test(candidate))) {
  assert.doesNotMatch(usage, /\bonmounted=/, "an unpooled SandboxView takes no mount outcome");
  // Unpooled embedded views retain the pooled instances that fit beside them, or visiting Lore would close every
  // hidden Writing webview.
  assert.match(usage, /prepareMount=\{prepareUnpooledMount\}/, "an unpooled SandboxView keeps the pool alive");
}
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
const begin = between(page, "function beginPooledMount(", "\n}\n");
assert.match(
  begin,
  /const known = Boolean\(before && before\.mountKey === mountKey\);[\s\S]*reuse: known/,
  "reuse only for an instance the pool already knew with the same mount key",
);
for (const name of ["Writing", "Language"]) {
  assert.match(
    between(page, `function beginPooled${name}Mount(`, "\n}\n"),
    /const plan = beginPooledMount\(self, mountKey\);[\s\S]*return \{ retain: plan\.retain, reuse: plan\.reuse \};/,
    `${name} takes its retain list and reuse from the shared pool step`,
  );
}
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

// Only the shown instance is ever shown by the visibility effect, and a suspended Writing or Language one waits
// for its own mount.
const visibility = between(page, "void writingHostedInstance;", "});\n\n");
assert.match(
  visibility,
  /visible && pluginId === WRITING_PLUGIN_ID && writingHostedInstance !== shownWebviewInstance/,
  "the visibility effect does not show a Writing instance that is not mounted yet",
);
assert.match(visibility, /void languageHostedInstance;/, "the effect reruns when a Language mount finishes");
assert.match(
  visibility,
  /visible && pluginId === LANGUAGE_PLUGIN_ID && languageHostedInstance !== shownWebviewInstance/,
  "the visibility effect does not show a Language instance that is not mounted yet",
);
assert.match(page, /async function setPluginWebviewVisibleQuietly[\s\S]*?const instanceId = shownWebviewInstance;/);

// Full screen ends on the instance that reported it, not on whichever tab is current by then.
const fullscreen = between(page, "function endWritingFullscreen() {", "\n}\n");
assert.match(fullscreen, /postWritingMessage\(writingFullscreenMessage\(false\), instance\)/);
assert.match(page, /writingFullscreenInstance = event\.on \? shownWebviewInstance : null/);

// A ready report from a hidden instance still reaches the pool, and is read before the parser of the shown path.
const writingState = between(page, 'listen<unknown>("writing-state"', "return () => {");
assert.ok(
  writingState.indexOf("foreignWebviewInstance(event.payload)") <
    writingState.indexOf("noteHiddenWebviewReady(WRITING_PLUGIN_ID"),
  "a foreign Writing event is noted for the pool, not handled",
);
const languageState = between(page, 'listen<unknown>("language-state"', ".then((cleanup)");
assert.ok(
  languageState.indexOf("foreignWebviewInstance(event.payload)") <
    languageState.indexOf("noteHiddenWebviewReady(LANGUAGE_PLUGIN_ID") &&
    languageState.indexOf("noteHiddenWebviewReady(LANGUAGE_PLUGIN_ID") <
      languageState.indexOf("parseLanguageStateEvent"),
  "a foreign Language event is noted for the pool, not handled",
);
// The shown instance's own ready report reaches the pool too, for either plugin.
assert.match(
  between(page, "function handleLanguageStateEvent(", 'case "pane":'),
  /markReady\(webviewPool, languageRef\(shownWebviewInstance\), true\)/,
);

// Language: a reused, ready instance is marked ready from the pool and resumed; it does not wait for an event that
// already came. A created or still-loading one reports ready itself.
const settle = between(page, "function settlePooledMount(", "\n}\n");
assert.match(settle, /outcome === "reused" && priorReady/, "readiness is restored only for a reused webview");
const languageMountedSource = between(page, "function languageMounted(", "\n}\n");
assert.match(languageMountedSource, /if \(!settled\.ready\) return;/, "only a ready, reused instance is resumed");
assert.match(languageMountedSource, /languageWebview\.ready = true;/);
assert.match(
  languageMountedSource,
  /postLanguageMessage\(languageResumeMessage\(entityId, languagePane\), self\.instanceId\)/,
  "language.resume carries the selected language and the shell's pane",
);
assert.match(languageMountedSource, /languageHostedInstance = self\.instanceId;/);
// The mount generation guard: the old placeholder's cleanup must not clear what the new one set.
const trackLanguage = between(page, "function trackLanguageMount(", "\n}\n");
assert.match(trackLanguage, /generation !== languageMountGeneration\) return;/);
assert.equal(trackLanguage.split("languageHostedInstance = null;").length, 3, "set on attach, cleared on cleanup");
// The pane is not part of a pooled instance's identity, or every pane change would recreate the webview.
const mountState = between(page, "function languageMountState(", "\n}\n");
assert.match(
  mountState,
  /created\.entityId === loc\.selected\?\.id/,
  "a pinned state is used only for the same language",
);
assert.match(
  mountState,
  /findWebviewEntry\(webviewPool, languageRef\(instanceId\)\)/,
  "and only while the pool knows it",
);
assert.match(
  between(page, "function beginPooledLanguageMount(", "\n}\n"),
  /if \(!plan\.reuse\) languageCreatedState\.set\(/,
  "the state a webview is created with is recorded",
);
// Leaving a Language tab still runs the guard first: the switch is refused when the webview says so.
assert.match(
  between(page, "async function canLeaveLanguageSection(", "\n}\n"),
  /languageLeave\.begin\(requestId\);\s*const sent = await postLanguageMessage\(\{ type: "language\.can-leave", requestId \}, instanceId\);/,
);

// Full screen does not follow to the next tab: attaching the next Writing instance ends the outgoing one's.
assert.match(between(page, "function trackWritingMount(", "return () => {"), /endWritingFullscreen\(\);/);

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
