import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

// Matches a quoted import of packages/modules/index or packages/modules/*/src/index.
const runtimeImport = /packages\/modules\/(?:index(?:\.ts)?|[^"'/]+\/src\/index(?:\.ts)?)["']/;
// The host imports no bundled plugin runtime: Lore and Chronology both run in isolated webviews.
const allowedRuntimeImports = new Set();
// Bundles are minified, so identifiers are mangled; the broker endpoint is a string literal and survives.
const brokerEndpoint = /(["'`])\/__rpc\1/;

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
  `host must not import bundled plugin runtime entrypoints (packages/modules/index or packages/modules/*/src/index): ${leakedRuntimeImports.join(", ")}`,
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
assert.match(loreHtml, /lore\.js/, "lore document must load lore.js");
assert.match(loreHtml, /lore\.css/, "lore document must load lore.css");
assert.doesNotMatch(loreHtml, /plugin\.js/, "lore document must not load the shared plugin.js stub");
const loreBundlePath = "src-tauri/plugin-assets/lore/lore.js";
if (fs.existsSync(loreBundlePath) && fs.statSync(loreBundlePath).size > 512) {
  const loreBundle = fs.readFileSync(loreBundlePath, "utf8");
  assert.doesNotMatch(
    loreBundle,
    /__TAURI_INTERNALS__|@tauri-apps|trusted_module_rpc/,
    "the lore bundle must use the broker protocol, not Tauri APIs or the trusted module RPC",
  );
  assert.match(loreBundle, brokerEndpoint, "the lore bundle must talk to the broker endpoint");
}
assert.match(writingHtml, /writing\.js/, "writing document must load writing.js");
assert.match(writingHtml, /writing\.css/, "writing document must load writing.css");
assert.doesNotMatch(writingHtml, /plugin\.js/, "writing document must not load the shared plugin.js stub");
const writingBundlePath = "src-tauri/plugin-assets/writing/writing.js";
if (fs.existsSync(writingBundlePath) && fs.statSync(writingBundlePath).size > 512) {
  const writingBundle = fs.readFileSync(writingBundlePath, "utf8");
  assert.doesNotMatch(
    writingBundle,
    /__TAURI_INTERNALS__|@tauri-apps|trusted_module_rpc/,
    "the writing bundle must use the broker protocol, not Tauri APIs or the trusted module RPC",
  );
  assert.match(writingBundle, brokerEndpoint, "the writing bundle must talk to the broker endpoint");
}
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
assert.match(timelineBundle, brokerEndpoint, "timeline bundle must talk to the broker endpoint");
assert.match(timelineBundle, /Chronological outline/, "timeline bundle must render the chronology outline heading");
const policyStart = webviewPolicy.indexOf("pub fn webview_policy");
const policyEnd = webviewPolicy.indexOf("pub fn validate_bridge_request");
const policy = webviewPolicy.slice(policyStart, policyEnd);
// The inline-style CSP is chosen by the condition between `csp: if` and the first `{`.
const inlineStyleCondition = policy.match(/csp: if ([^{]+)\{/)?.[1] ?? "";
for (const pluginId of ["daena.maps", "daena.timeline", "daena.language", "daena.writing", "daena.lore"]) {
  assert.ok(
    inlineStyleCondition.includes(`manifest.id == "${pluginId}"`),
    `${pluginId} must share the inline-style CSP for its vis or graph styling`,
  );
}
const cspStrings = [...policy.matchAll(/"(default-src[^"]*)"/g)].map((match) => match[1]);
assert.equal(
  cspStrings.filter((csp) => csp.includes("style-src-attr 'unsafe-inline'")).length,
  1,
  "maps, chronology, language, writing and lore must share one inline-style CSP",
);
assert.equal(
  cspStrings.filter((csp) => !csp.includes("unsafe-inline")).length,
  1,
  "every other plugin keeps the stricter CSP",
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

assert.equal(allowedRuntimeImports.size, 0, "the host runtime-import allowlist must stay empty");
for (const retired of ["src/lib/modules/projections.ts", "src/lib/ProjectionView.svelte"]) {
  assert.equal(fs.existsSync(retired), false, `${retired} is retired: Lore and Chronology are sandboxed webviews`);
}
const transfer = fs.readFileSync("src-tauri/src/transfer.rs", "utf8");
assert.match(transfer, /\("daena\.lore", "\/dist\/ui\/lore\.js"\)/);
assert.match(transfer, /\("daena\.lore", "\/dist\/ui\/lore\.css"\)/);
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

// The World graph and Chronology share one sandbox path: a table of plugin, shell view id, and surface.
assert.match(route, /function projectionSandboxKey\(\): ProjectionSandboxKey \| null/);
assert.match(route, /function projectionSandboxOpen\(key\?: ProjectionSandboxKey\): boolean/);
const projectionTable = sourceSlice("const PROJECTION_SANDBOXES", "/** The projection whose sandbox");
assert.match(
  projectionTable,
  /lore: \{[^}]*pluginId: "daena\.lore"[^}]*viewId: "graph"[^}]*title: "World graph"[^}]*subtitle: "Explore entities and their connections"[^}]*icon: Network[^}]*surfaceKey: "workspace:lore:graph"/,
  "the World graph must open the daena.lore sandbox as the graph view",
);
assert.match(
  projectionTable,
  /timeline: \{[^}]*pluginId: "daena\.timeline"[^}]*viewId: "chronology"[^}]*title: "Chronology"[^}]*subtitle: "Explore your world across time"[^}]*surfaceKey: "workspace:timeline:timeline"/,
  "Chronology must keep its daena.timeline sandbox",
);
const openProjection = sourceSlice("async function openProjection", "async function openLoreLibrary");
assert.match(openProjection, /section === "timeline" \? "timeline" : "lore"/);
assert.match(openProjection, /projectionSandboxOpen\(key\)/);
assert.match(openProjection, /entry\.id === surface\.pluginId && entry\.enabled/);
assert.match(openProjection, /is disabled, so \$\{surface\.title\} cannot open/);
assert.match(openProjection, /renderer: \{ type: "sandboxed" \}/);
assert.match(openProjection, /renderer: "webview"/);
assert.doesNotMatch(openProjection, /projectionView|projectionModule/);
const workspaceView = sourceSlice("function currentWorkspaceLocationView", "function currentModuleState");
assert.match(workspaceView, /projectionSandboxOpen\("lore"\)\) return "graph"/);
assert.match(workspaceView, /projectionSandboxOpen\("timeline"\)\) return "timeline"/);
const surfaceKey = sourceSlice("function specializedSurfaceKey()", "function specializedSurfaceKeyForLocation");
assert.match(
  surfaceKey,
  /const projection = projectionSandboxKey\(\);\s*if \(projection\) return PROJECTION_SANDBOXES\[projection\]\.surfaceKey/,
);
assert.ok(
  surfaceKey.indexOf("projectionSandboxKey()") < surfaceKey.indexOf("sandboxView?.view"),
  "projection surfaces must be matched before the generic sandbox branch",
);
const shellLocation = sourceSlice("function currentShellLocation()", "function recordCurrentShellLocation");
assert.match(shellLocation, /sandboxView\?\.view && !projectionSandboxOpen\(\)/);
const breadcrumbs = sourceSlice("function breadcrumbItems()", "function workspaceHeadingKicker");
assert.match(breadcrumbs, /!projectionSandboxOpen\(\)/);
const closeProjection = sourceSlice("async function closeProjectionView", "async function closePluginView");
assert.match(closeProjection, /recordShellDeparture\(departure\)/);
assert.match(closeProjection, /if \(projectionSandboxOpen\(\)\) sandboxView = null/);
assert.doesNotMatch(closeProjection, /shellHistoryBack|navigateShellHistory/);
assert.match(route, /<WorkspaceTopbar/);
assert.match(route, /\{:else if projectionSandboxKey\(\)\}/);
assert.match(
  route,
  /<SandboxView pluginId=\{projection\.pluginId\} title=\{projection\.title\} \/>/,
  "the World graph and Chronology mount one SandboxView with their table entry",
);
assert.match(
  route,
  /!hostView && \(!sandboxView \|\| projectionSandboxOpen\(\)\) && \(section === "lore"/,
  "workspace view tabs stay visible while the World graph or Chronology is open",
);
assert.doesNotMatch(
  route,
  /\bprojectionView\b|projectionModule|\bProjectionView\b|ProjectionKind|lib\/modules\/projections|chronologySandboxOpen/,
  "the shell must not carry the trusted projection path",
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
  /pluginId="daena\.(?:timeline|lore)"[^>]*viewId=|<SandboxView[^>]*pluginId=\{projection\.pluginId\}[^>]*viewId=/,
  "projection webviews must omit viewId; the manifests declare no views",
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
  assert.match(languageBundle, brokerEndpoint, "the Language bundle must talk to the broker endpoint");
}
assert.match(tauriSource, /\("daena\.language", "\/dist\/ui\/language\.js"\)/);
assert.match(tauriSource, /\("daena\.language", "\/dist\/ui\/language\.css"\)/);
assert.match(tauriSource, /\("daena\.writing", "\/dist\/ui\/writing\.js"\)/);
assert.match(tauriSource, /\("daena\.writing", "\/dist\/ui\/writing\.css"\)/);

// Writing workspace: behind a developer flag, the content pane for manuscripts and reference pages is the
// isolated daena.writing webview, kept mounted across selections.
assert.match(
  route,
  /<SandboxView pluginId="daena\.writing" viewId="writing-workspace" title="Writing" \/>/,
  "the Writing content pane must mount the isolated daena.writing webview",
);
assert.doesNotMatch(
  route,
  /<SandboxView[^>]*pluginId="daena\.writing"[^>]*entityId=/,
  "the Writing webview is told the entity with writing.load, not remounted per entity",
);
assert.doesNotMatch(
  route,
  /\{#key[^}]*\}\s*<div class="writing-mount"/,
  "the Writing webview must not be keyed on the entity",
);
assert.match(
  route,
  /localStorage\.getItem\("daena:writing-webview"\) === "1"/,
  "the Writing webview is behind a dev flag",
);
assert.match(route, /listen<unknown>\("writing-state"/, "the shell must listen for the Writing webview state");
assert.match(
  route,
  /if \(writingWebview\.mounted\) ids\.add\(WRITING_PLUGIN_ID\)/,
  "overlays must hide the Writing webview",
);

console.log("plugin webview isolation checks passed");
