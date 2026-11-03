import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const runtimeImport = /packages\/modules\/(?:index\.ts|[^"'/]+\/src\/index(?:\.ts)?)/;
const allowedRuntimeImports = new Set(["src/lib/modules/projections.ts"]);

function listSourceFiles(directory) {
  const files = [];
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const next = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      if (entry.name !== "node_modules") files.push(...listSourceFiles(next));
    } else if (/\.(?:ts|svelte|js)$/.test(entry.name)) files.push(next);
  }
  return files;
}

const leakedRuntimeImports = listSourceFiles("src").filter((file) => {
  if (allowedRuntimeImports.has(file)) return false;
  return runtimeImport.test(fs.readFileSync(file, "utf8"));
});
const hostImport = /from ["'](?:\.\.\/)+src\/lib|from ["']\$lib\//;
const leakedHostImports = listSourceFiles("packages/modules/timeline/src").filter((file) =>
  hostImport.test(fs.readFileSync(file, "utf8")),
);
assert.deepEqual(leakedHostImports, [], `timeline plugin must not import host $lib: ${leakedHostImports.join(", ")}`);
assert.deepEqual(
  leakedRuntimeImports,
  [],
  `host must not import bundled plugin runtime entrypoints (remaining allowlist: ${[...allowedRuntimeImports].join(", ")}): ${leakedRuntimeImports.join(", ")}`,
);

const route = fs.readFileSync("src/routes/+page.svelte", "utf8");
const frame = fs.readFileSync("src/lib/plugins/SandboxView.svelte", "utf8");
const plugin = fs.readFileSync("src-tauri/plugin-assets/shared/plugin.js", "utf8");
const timelineHtml = fs.readFileSync("src-tauri/plugin-assets/timeline/index.html", "utf8");
const loreHtml = fs.readFileSync("src-tauri/plugin-assets/lore/index.html", "utf8");
const writingHtml = fs.readFileSync("src-tauri/plugin-assets/writing/index.html", "utf8");
const webviewPolicy = fs.readFileSync("crates/daena-plugin-host/src/runtime.rs", "utf8");
const tauriSource = [
  fs.readFileSync("src-tauri/src/lib.rs", "utf8"),
  fs.readFileSync("src-tauri/src/plugin_webview.rs", "utf8"),
  fs.readFileSync("src-tauri/src/transfer.rs", "utf8"),
].join("\n");
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
assert.match(loreHtml, /plugin\.js/, "lore stub stays on the shared plugin document");
assert.match(writingHtml, /plugin\.js/, "writing stub stays on the shared plugin document");
assert.match(timelineHtml, /id="chronology"/, "timeline document must mount the chronology root");
assert.match(
  timelineHtml,
  /id="chronology-status"[^]*<\/p>\s*<div id="chronology"><\/div>/,
  "the status line must sit outside the root that mountTimeline replaces",
);
assert.match(timelineHtml, /timeline\.css/, "timeline document must load timeline.css");
assert.match(timelineHtml, /timeline\.js/, "timeline document must load timeline.js");
assert.doesNotMatch(timelineHtml, /plugin\.js/, "timeline document must not load the shared plugin.js stub");
const timelineBundle = fs.readFileSync("src-tauri/plugin-assets/timeline/timeline.js", "utf8");
assert.doesNotMatch(
  timelineBundle,
  /__TAURI_INTERNALS__|@tauri-apps/,
  "timeline bundle must use the broker protocol, not Tauri APIs",
);
assert.doesNotMatch(
  timelineBundle,
  /(?:from|import\()\s*["']vis-timeline/,
  "timeline bundle must include vis-timeline instead of importing it at runtime",
);
assert.match(timelineBundle, /"\/__rpc"/, "timeline bundle must talk to the broker endpoint");
assert.match(timelineBundle, /Chronological outline/);
const policyStart = webviewPolicy.indexOf("pub fn webview_policy");
const policyEnd = webviewPolicy.indexOf("pub fn validate_bridge_request");
const policy = webviewPolicy.slice(policyStart, policyEnd);
assert.match(
  policy,
  /manifest\.id == "daena\.maps"\s*\|\|\s*manifest\.id == "daena\.timeline"\s*\|\|\s*manifest\.id == "daena\.language"/,
);
const cspStrings = [...policy.matchAll(/"(default-src[^"]*)"/g)].map((match) => match[1]);
assert.deepEqual(
  cspStrings.filter((csp) => csp.includes("style-src-attr 'unsafe-inline'")).length,
  1,
  "chronology must share the maps inline-style CSP and no other plugin may",
);
assert.equal(
  cspStrings.filter((csp) => !csp.includes("unsafe-inline")).length,
  1,
  "lore and writing keep the stricter CSP",
);
assert.match(tauriSource, /\("daena\.timeline", "\/dist\/ui\/timeline\.js"\)/);
assert.match(tauriSource, /\("daena\.timeline", "\/dist\/ui\/timeline\.css"\)/);
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

const projections = fs.readFileSync("src/lib/modules/projections.ts", "utf8");
assert.deepEqual([...allowedRuntimeImports], ["src/lib/modules/projections.ts"]);
assert.doesNotMatch(
  projections,
  /packages\/modules\/timeline\/src\/index/,
  "projections allowlist must not import the timeline runtime",
);
assert.match(fs.readFileSync("src/routes/+page.svelte", "utf8"), /CalendarEditor\.svelte/);
assert.match(fs.readFileSync("src/lib/date.ts", "utf8"), /calendar-date\.ts/);
assert.match(fs.readFileSync("src/lib/date.ts", "utf8"), /calendar\.ts/);
assert.match(fs.readFileSync("src/lib/modules/chronology.ts", "utf8"), /era-scope\.ts/);
assert.match(fs.readFileSync("src/lib/date/dateField.ts", "utf8"), /calendar\.ts/);

function sourceSlice(start, end) {
  const from = route.indexOf(start);
  const to = route.indexOf(end);
  assert.notEqual(from, -1, start);
  assert.notEqual(to, -1, end);
  return route.slice(from, to);
}

assert.match(route, /function chronologySandboxOpen\(\)/);
const openProjection = sourceSlice("async function openProjection", "async function openLoreLibrary");
assert.match(openProjection, /chronologySandboxOpen\(\)/);
assert.match(openProjection, /id === "daena\.timeline"/);
assert.match(openProjection, /id: "chronology"/);
assert.match(openProjection, /title: "Chronology"/);
assert.match(openProjection, /renderer: \{ type: "sandboxed" \}/);
assert.match(openProjection, /renderer: "webview"/);
assert.match(openProjection, /projectionView = null/);
assert.match(openProjection, /projectionModule\("lore"\)/);
assert.doesNotMatch(openProjection, /projectionModule\(\s*(?:section|"timeline")/);
const workspaceView = sourceSlice("function currentWorkspaceLocationView", "function currentModuleState");
assert.match(workspaceView, /chronologySandboxOpen\(\)\) return "timeline"/);
const surfaceKey = sourceSlice("function specializedSurfaceKey()", "function specializedSurfaceKeyForLocation");
assert.match(surfaceKey, /if \(chronologySandboxOpen\(\)\) return "workspace:timeline:timeline"/);
const shellLocation = sourceSlice("function currentShellLocation()", "function recordCurrentShellLocation");
assert.match(shellLocation, /sandboxView\?\.view && !chronologySandboxOpen\(\)/);
const breadcrumbs = sourceSlice("function breadcrumbItems()", "function workspaceHeadingKicker");
assert.match(breadcrumbs, /!chronologySandboxOpen\(\)/);
const closeProjection = sourceSlice("async function closeProjectionView", "async function closePluginView");
assert.match(closeProjection, /recordShellDeparture\(departure\)/);
assert.match(closeProjection, /if \(chronologySandboxOpen\(\)\) sandboxView = null/);
assert.doesNotMatch(closeProjection, /shellHistoryBack|navigateShellHistory/);
assert.match(route, /title="Chronology"/);
assert.match(route, /subtitle="Explore your world across time"/);
assert.match(route, /<WorkspaceTopbar/);
assert.match(route, /<SandboxView pluginId="daena\.timeline" title="Chronology" \/>/);
assert.match(
  route,
  /!hostView && \(!sandboxView \|\| chronologySandboxOpen\(\)\) && \(section === "lore"/,
  "workspace view tabs stay visible while Chronology is open",
);
assert.match(
  fs.readFileSync("packages/modules/timeline/src/plugin-main.ts", "utf8"),
  /import "\.\/plugin\.css"/,
  "chronology webview maps shell theme aliases in its own stylesheet",
);
assert.match(
  fs.readFileSync("src-tauri/src/transfer.rs", "utf8"),
  /request\.method == "schema\.list"/,
  "schema.list must be served on the /__rpc bridge that plugin webviews use",
);
assert.doesNotMatch(
  route,
  /pluginId="daena\.timeline"[^>]*viewId=/,
  "chronology webview must omit viewId; the manifest declares no views",
);

// Language workspace: the content pane is the isolated daena.language webview, and the trusted
// shell no longer carries a Language projection or a window-global leave guard.
assert.match(
  route,
  /<SandboxView\s+pluginId="daena\.language"\s+viewId="language-workspace"/,
  "the Languages content pane must mount the isolated daena.language webview",
);
assert.doesNotMatch(route, /<ModuleMount\b/, "the shell must not mount Language through the trusted ModuleMount");
assert.doesNotMatch(route, /projectionModule\("language"\)/, "Language must not be a trusted projection");
assert.doesNotMatch(
  projections,
  /modules\/language/,
  "the projections registry must not import the Language runtime (Lore remains the only trusted projection)",
);
assert.match(route, /listen<unknown>\(\s*"language-state"/, "the shell must listen for the Language webview state");
const canLeaveGlobal = [...listSourceFiles("src"), ...listSourceFiles("packages")].filter((file) =>
  fs.readFileSync(file, "utf8").includes("__daena_canLeaveLanguage"),
);
assert.deepEqual(canLeaveGlobal, [], "the __daena_canLeaveLanguage window global is replaced by the host message");
const languageDocument = fs.readFileSync("src-tauri/plugin-assets/language/index.html", "utf8");
assert.match(languageDocument, /language\.js/, "the Language document must load language.js");
assert.match(languageDocument, /language\.css/, "the Language document must load language.css");
assert.doesNotMatch(languageDocument, /plugin\.js/, "the Language document must not load the shared plugin.js stub");
const languageBundlePath = "src-tauri/plugin-assets/language/language.js";
if (fs.existsSync(languageBundlePath) && fs.statSync(languageBundlePath).size > 512) {
  const languageBundle = fs.readFileSync(languageBundlePath, "utf8");
  assert.doesNotMatch(
    languageBundle,
    /__TAURI_INTERNALS__|@tauri-apps|trusted_module_rpc/,
    "the Language bundle must use the broker protocol, not Tauri APIs or the trusted module RPC",
  );
}
assert.match(tauriSource, /\("daena\.language", "\/dist\/ui\/language\.js"\)/);
assert.match(tauriSource, /\("daena\.language", "\/dist\/ui\/language\.css"\)/);

console.log("plugin webview isolation checks passed");
