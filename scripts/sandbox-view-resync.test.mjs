import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import { compile } from "svelte/compiler";

// Position re-sync contract for SandboxView, which places a native plugin webview over its container:
// - the container is re-measured when anything above it moves it, not only when it resizes;
// - a scroll in any ancestor triggers a re-sync (capture listener on window, since scroll does not bubble);
// - a re-sync runs once fonts finish loading;
// - bounds identical to the last ones sent are not sent to the host again.
// The projection sandboxes (Lore graph, Chronology) are shell webviews, so they belong to the hidden set too.

const root = resolve(import.meta.dirname, "..");
const read = (path) => readFile(resolve(root, path), "utf8");

const sandboxPath = "src/lib/plugins/SandboxView.svelte";
const sandbox = await read(sandboxPath);
compile(sandbox, { filename: resolve(root, sandboxPath), css: "injected" });

const between = (source, start, end) => {
  const from = source.indexOf(start);
  assert.notEqual(from, -1, `source contains ${start}`);
  const to = source.indexOf(end, from + start.length);
  assert.notEqual(to, -1, `source contains ${end} after ${start}`);
  return source.slice(from, to);
};

const observation = between(sandbox, "function observePositionSources", "onMount(");
assert.match(observation, /observer\.observe\(node\)/, "the container and each ancestor are observed");
assert.match(observation, /node = node\.parentElement/, "the walk climbs through the ancestors");
assert.match(observation, /node !== document\.body/, "the walk stops at document.body");
assert.match(observation, /node\.previousElementSibling/, "preceding siblings of each node are observed");
assert.match(observation, /observer\.observe\(sibling\)/, "each preceding sibling is observed");
assert.doesNotMatch(
  sandbox,
  /querySelector<HTMLElement>\("\.topbar"\)/,
  "the .topbar observation is covered by the sibling of an ancestor",
);

const mounting = sandbox.slice(sandbox.indexOf("onMount("), sandbox.indexOf("</script>"));
assert.match(mounting, /observePositionSources\(observer\)/, "onMount observes position sources");
assert.match(
  mounting,
  /window\.addEventListener\("scroll", scheduleResize, \{ capture: true, passive: true \}\)/,
  "scroll is listened to in the capture phase on window",
);
assert.match(
  mounting,
  /window\.removeEventListener\("scroll", scheduleResize, \{ capture: true \}\)/,
  "the scroll listener is removed in teardown with the matching capture flag",
);
assert.match(mounting, /document\.fonts\?\.ready\.then\(\(\) => \{\s*if \(alive\) scheduleResize\(\);/);

const scheduling = between(sandbox, "function scheduleResize", "/**");
assert.match(scheduling, /resizeFrame = window\.requestAnimationFrame/, "re-syncs stay coalesced to one frame");
assert.match(scheduling, /if \(key === lastBoundsKey\) return;/, "unchanged rounded bounds skip the IPC call");
assert.ok(
  scheduling.indexOf("key === lastBoundsKey") < scheduling.indexOf("resizePluginWebview"),
  "the unchanged check runs before resizePluginWebview",
);
assert.match(mounting, /lastBoundsKey = boundsKey\(bounds\);/, "the bounds sent on mount are recorded");
assert.match(mounting, /lastBoundsKey = null;/, "a mount starts with no bounds recorded");

const shell = await read("src/routes/+page.svelte");
for (const pluginId of ["daena.lore", "daena.timeline"]) {
  assert.match(shell, new RegExp(`pluginId: "${pluginId.replace(".", "\\.")}"`), `${pluginId} is a projection sandbox`);
}
const projectionKey = between(shell, "function projectionSandboxKey()", "/** True while");
assert.match(projectionKey, /sandboxView\.renderer !== "webview"/, "projection sandboxes are sandboxView webviews");
const opener = between(shell, "async function openProjection()", "\n}\n");
assert.match(opener, /renderer: "webview"/, "openProjection opens the projection as a webview sandbox view");
const embedded = between(shell, "const embeddedPluginWebviewIds", "async function setPluginWebviewVisibleQuietly");
assert.match(
  embedded,
  /sandboxView\?\.renderer === "webview"\) ids\.add\(sandboxView\.plugin\.id\)/,
  "the open sandbox view's plugin, which is a projection while one is open, is in the hidden set",
);

console.log("sandbox view resync: ok");
