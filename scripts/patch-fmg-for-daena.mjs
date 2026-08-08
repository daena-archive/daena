import { existsSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createZipArchive } from "../packages/plugin-cli/bin/zip.mjs";

const root = resolve(process.argv[2] ?? "");
const output = resolve(process.argv[3] ?? join(root, "dist", "daena-fmg-v1.119.zip"));
if (!process.argv[2]) throw new Error("usage: node scripts/patch-fmg-for-daena.mjs /path/to/fmg [output.zip]");

const dist = resolve(root, "dist");
const bridgeTemplatePath = join(dirname(fileURLToPath(import.meta.url)), "fmg-bridge-template.js");
const htmlPath = join(dist, "index.html");
let html = readFileSync(htmlPath, "utf8");

// FMG's public build includes analytics that are neither needed nor allowed
// in Daena's offline child webview. Remove both the external loader and its
// inline bootstrap before applying the strict plugin CSP.
html = html.replace(
  /\s*<script async src="https:\/\/www\.googletagmanager\.com\/gtag\/js\?id=[^"]+"><\/script>\s*<script>[\s\S]*?<\/script>/,
  "",
);

// The plugin protocol exposes the packaged tree below /dist/ui/fmg/. A base
// URL makes FMG's large set of relative CSS, image, font, and classic-script
// references resolve inside that tree. The Vite module is the one upstream
// absolute URL, so make it relative as well. The base must precede the module
// tag: the browser fetches the module as soon as the parser reaches it, so a
// base inserted after it would never apply to the module's relative URL.
if (!html.includes('<base href="/dist/ui/fmg/"')) {
  html = html.replace("<head>", "<head>\n    <base href=\"/dist/ui/fmg/\" />");
}
html = html.replace(/(src|href)="\/Fantasy-Map-Generator\//g, '$1="');

// Keep the host marker ahead of every deferred FMG script. FMG's bundled
// runtime reads it during initialization, before the provider bridge runs.
// The inline bootstrap follows immediately: it re-attaches the externalized
// event handlers and restores the externalized inline styles before the
// module bundle renders, so FMG's stock hidden UI (prompt, map overlay)
// stays hidden in the Daena webview.
html = html.replace(/\s*<script defer src="daena-bridge\.js"><\/script>/g, "");
html = html.replace(/\s*<script defer src="daena-inline-bootstrap\.js"><\/script>/g, "");
const bridge = '    <script defer src="daena-bridge.js"></script>\n    <script defer src="daena-inline-bootstrap.js"></script>';
if (html.includes('<base href="/dist/ui/fmg/" />')) {
  html = html.replace('<base href="/dist/ui/fmg/" />', `<base href="/dist/ui/fmg/" />\n${bridge}`);
} else if (html.includes('<script type="module"')) {
  html = html.replace('<script type="module"', `${bridge}\n    <script type="module"`);
} else {
  html = html.replace("</head>", `${bridge}\n  </head>`);
}

// The upstream build emits CSS preload links whose inline onload handlers
// were removed by the source hardening pass. Make them real stylesheets in
// the transformed artifact so WebKit applies FMG's palette and layout CSS.
html = html.replace(/<link([\s\S]*?)>/g, (tag, attributes) => {
  if (!attributes.includes('rel="preload"') || !attributes.includes('as="style"')) return tag;
  return `<link${attributes
    .replace('rel="preload"', 'rel="stylesheet"')
    .replace(/\s+as="style"/, "")
    .replace(/\s+data-daena-event="\d+"/, "")}>`;
});

// Re-apply the source hardening transforms idempotently.
const events = [];
let eventId = 0;
html = html.replace(/\s(on(?:click|change|input|load|mouseover))="([^"]*)"/g, (_match, name, code) => {
  const id = String(eventId++);
  events.push({ id, type: name.slice(2), code });
  return ` data-daena-event="${id}"`;
});
html = html.replace(/\sstyle="([^"]*)"/g, (_match, style) => ` data-daena-style="${encodeURIComponent(style)}"`);
// Styles are restored by daena-inline-bootstrap.js from data-daena-style. An
// earlier patch pass linked a never-written daena-inline.css and 404'd on load.
html = html.replace(/\s*<link rel="stylesheet" href="daena-inline\.css">\s*/g, "\n");
writeFileSync(htmlPath, html);

const mainPath = join(dist, "main.js");
let main = readFileSync(mainPath, "utf8");
if (!main.includes("if (DAENA_HOST) return;")) {
  main = main.replace("function toggleAssistant() {", "function toggleAssistant() {\n  if (DAENA_HOST) return;");
}
// Under Daena the bridge owns first paint: FMG must not generate a random
// world (or reload IndexedDB) before the host streams the project .map.
// Doing both doubles peak memory and is what froze the child webview.
if (!main.includes("if (DAENA_HOST) return; // Daena bridge owns startup load")) {
  main = main.replace(
    "async function checkLoadParameters() {",
    "async function checkLoadParameters() {\n  if (DAENA_HOST) return; // Daena bridge owns startup load",
  );
}
// Allow Daena to boot when hostname is empty under the custom protocol.
if (!main.includes("!location.hostname && !window.DAENA_HOST")) {
  main = main.replace(
    "  if (!location.hostname) {",
    "  if (!location.hostname && !window.DAENA_HOST) {",
  );
}
// Do not suppress FMG's hideLoading under Daena. Earlier experiments left the
// loading rose up forever when the bridge did not own the overlay lifecycle.
main = main.replace(
  "  } else {\n    // Under Daena the bridge owns the loading rose until host load/generate finishes.\n    if (!window.DAENA_HOST) hideLoading();\n    await checkLoadParameters();\n  }",
  "  } else {\n    hideLoading();\n    await checkLoadParameters();\n  }",
);
main = main.replace(
  "  } else {\n    if (!window.DAENA_HOST) hideLoading();\n    await checkLoadParameters();\n  }",
  "  } else {\n    hideLoading();\n    await checkLoadParameters();\n  }",
);
// plugin:// cannot register service workers; skip under the Daena host marker.
if (!main.includes("!window.DAENA_HOST && \"serviceWorker\"")) {
  main = main.replace(
    'if (PRODUCTION && "serviceWorker" in navigator)',
    'if (PRODUCTION && !window.DAENA_HOST && "serviceWorker" in navigator)',
  );
}
// Daena is offline-first and does not ship FMG's browser-only assistant.
// Remove the whole dynamic import branch so WebKit never reports a blocked
// network script, even if a persisted FMG option tries to show the assistant.
main = main.replace(
  /    } else \{\n      import\("\.\/libs\/openwidget\.min\.js"\)\.then\(\(\) => \{[\s\S]*?\n      \}\);\n    \}/,
  "    } else {\n      // Daena does not ship FMG's browser-only assistant.\n    }",
);
writeFileSync(mainPath, main);

// Strip remote Google Font Face sources from the Vite module so FontFace()
// never hits fonts.gstatic.com under the plugin CSP. Family names remain for
// local/system fallbacks (Arial, Georgia, etc. and unmatched decorative names).
// Also soften findCell while the Daena bridge still owns startup load.
for (const name of readdirSync(dist)) {
  if (!/^index-.*\.js$/.test(name)) continue;
  const modulePath = join(dist, name);
  let moduleSource = readFileSync(modulePath, "utf8");
  let changed = false;
  if (moduleSource.includes("fonts.gstatic.com")) {
    moduleSource = moduleSource.replace(/,src:"url\(https:\/\/fonts\.gstatic\.com\/[^"]+\)"/g, "");
    moduleSource = moduleSource.replace(/,unicodeRange:"[^"]*"/g, "");
    changed = true;
  }
  if (
    moduleSource.includes('if(!n.cells?.p)throw new Error("Pack cells not found")') &&
    !moduleSource.includes("if(window.DAENA_HOST)return;throw new Error(\"Pack cells not found\")")
  ) {
    moduleSource = moduleSource.replace(
      'if(!n.cells?.p)throw new Error("Pack cells not found")',
      'if(!n.cells?.p){if(window.DAENA_HOST)return;throw new Error("Pack cells not found")}',
    );
    changed = true;
  }
  if (changed) writeFileSync(modulePath, moduleSource);
}

const bridgePath = join(dist, "daena-bridge.js");
// Always ship the repo template. Preferring a prior dist/daena-bridge.js left
// orphan-repair and other bridge fixes stranded in ignored FMG checkout state.
let bridgeSource = readFileSync(bridgeTemplatePath, "utf8");
if (!bridgeSource.includes('const requestedMapEntityId = params.get("mapEntityId");')) {
  bridgeSource = bridgeSource.replace(
    '  const projectId = params.get("project");',
    '  const projectId = params.get("project");\n  const requestedMapEntityId = params.get("mapEntityId");',
  );
}
bridgeSource = bridgeSource.replace(
  '  const requestedMapEntityId = params.get("mapEntityId");\n  const requestedMapEntityId = params.get("mapEntityId");',
  '  const requestedMapEntityId = params.get("mapEntityId");',
);
if (!bridgeSource.includes("function showDiagnostic")) {
  bridgeSource = bridgeSource.replace(
    '  const requestedMapEntityId = params.get("mapEntityId");',
    '  const requestedMapEntityId = params.get("mapEntityId");\n\n  function showDiagnostic(error) {\n    const message = error instanceof Error ? error.message : String(error);\n    let panel = document.getElementById("daena-map-diagnostic");\n    if (!panel) { panel = document.createElement("div"); panel.id = "daena-map-diagnostic"; panel.style.cssText = "position:fixed;z-index:2147483647;left:16px;right:16px;bottom:16px;padding:12px 16px;border:1px solid #d97706;border-radius:8px;background:#fffbeb;color:#78350f;font:14px system-ui,sans-serif;box-shadow:0 4px 18px #0003"; document.body.appendChild(panel); }\n    panel.textContent = `Daena Maps: ${message}`;\n  }',
  );
}
bridgeSource = bridgeSource.replace(
  '  async function firstMapAsset() {\n    const maps = await rpc("entity.list", {entityType: "daena.maps:map"});\n    for (const map of maps) {\n      const field = await rpc("field.read", {entityId: map.id, namespace: "maps", key: "map"});\n      const descriptor = Array.isArray(field) ? field[0]?.value : field?.value ?? field;\n      if (descriptor?.sourceAssetId) return {mapId: map.id, assetId: descriptor.sourceAssetId};\n    }\n    return null;\n  }',
  '  async function firstMapAsset() {\n    const maps = await rpc("entity.list", {entityType: "daena.maps:map"});\n    for (const map of maps) {\n      if (requestedMapEntityId && map.id !== requestedMapEntityId) continue;\n      const field = await rpc("field.read", {entityId: map.id, namespace: "maps", key: "map"});\n      const descriptor = Array.isArray(field) ? field[0]?.value : field?.value ?? field;\n      if (requestedMapEntityId) return {mapId: map.id, assetId: descriptor?.sourceAssetId ?? null};\n      if (descriptor?.sourceAssetId) return {mapId: map.id, assetId: descriptor.sourceAssetId};\n    }\n    if (requestedMapEntityId) throw new Error(`requested map is unavailable: ${requestedMapEntityId}`);\n    return null;\n  }',
);
bridgeSource = bridgeSource.replace(
  '  async function firstMapAsset() {\n    const maps = await rpc("entity.list", {entityType: "daena.maps:map"});\n    for (const map of maps) {\n      if (requestedMapEntityId && map.id !== requestedMapEntityId) continue;\n      const field = await rpc("field.read", {entityId: map.id, namespace: "maps", key: "map"});\n      const descriptor = Array.isArray(field) ? field[0]?.value : field?.value ?? field;\n      if (descriptor?.sourceAssetId) return {mapId: map.id, assetId: descriptor.sourceAssetId};\n    }\n    if (requestedMapEntityId) throw new Error(`requested map is unavailable: ${requestedMapEntityId}`);\n    return null;\n  }',
  '  async function firstMapAsset() {\n    const maps = await rpc("entity.list", {entityType: "daena.maps:map"});\n    for (const map of maps) {\n      if (requestedMapEntityId && map.id !== requestedMapEntityId) continue;\n      const field = await rpc("field.read", {entityId: map.id, namespace: "maps", key: "map"});\n      const descriptor = Array.isArray(field) ? field[0]?.value : field?.value ?? field;\n      if (requestedMapEntityId) return {mapId: map.id, assetId: descriptor?.sourceAssetId ?? null};\n      if (descriptor?.sourceAssetId) return {mapId: map.id, assetId: descriptor.sourceAssetId};\n    }\n    if (requestedMapEntityId) throw new Error(`requested map is unavailable: ${requestedMapEntityId}`);\n    return null;\n  }',
);
if (!bridgeSource.includes("Daena Maps provider startup failed")) {
  bridgeSource = bridgeSource.replace(
    "})();",
    '})().catch(error => { console.error("Daena Maps provider startup failed:", error); window.daenaMapDiagnostic?.(error); });',
  );
}
bridgeSource = bridgeSource.replace(
  '})().catch(error => { console.error("Daena Maps provider startup failed:", error); window.generateMapOnLoad?.(); });',
  '})().catch(error => { console.error("Daena Maps provider startup failed:", error); window.daenaMapDiagnostic?.(error); });',
);
bridgeSource = bridgeSource.replace(
  '  const source = await loadAsset(mapAsset.assetId);\n  const asset = {mapId: mapAsset.mapId, assetId: mapAsset.assetId, revision: metadata.revision, contentHash: metadata.contentHash};',
  '  const asset = {mapId: mapAsset.mapId, assetId: mapAsset.assetId, revision: metadata.revision, contentHash: metadata.contentHash};\n  const source = metadata.size === 0 ? null : await loadAsset(mapAsset.assetId);',
);
bridgeSource = bridgeSource.replace(
  '  await window.daenaMapProvider.load(source);',
  '  if (source === null) {\n    if (typeof window.generateMapOnLoad !== "function") throw new Error("new map source is empty and FMG generation is unavailable");\n    await window.generateMapOnLoad();\n    await saveAsset(asset);\n  } else {\n    await window.daenaMapProvider.load(source);\n    lastSavedHash = await sha256(new TextEncoder().encode(prepareMapData()));\n    publishState("clean");\n  }',
);
bridgeSource = bridgeSource.replace(
  '  window.saveMap = method => method === "machine" || method === "dropbox" || method === "storage" ? saveAsset(asset) : originalSave?.(method);\n  window.addEventListener("keydown", event => { if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") { event.preventDefault(); void saveAsset(asset); } });',
  '  window.saveMap = method => method === "machine" || method === "dropbox" || method === "storage" ? saveAsset(asset).catch(showDiagnosticUnlessConflict) : originalSave?.(method);\n  window.addEventListener("keydown", event => { if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") { event.preventDefault(); void saveAsset(asset).catch(showDiagnosticUnlessConflict); } });',
);
bridgeSource = bridgeSource.replace(
  '})().catch(error => { console.error("Daena Maps provider startup failed:", error); if (!new URLSearchParams(location.search).get("mapEntityId")) window.generateMapOnLoad?.(); });',
  '})().catch(error => { console.error("Daena Maps provider startup failed:", error); window.daenaMapDiagnostic?.(error); });',
);
bridgeSource = bridgeSource.replace(
  'mimeType: "application/octet-stream"',
  'mimeType: "application/x-fmg-map"',
);
bridgeSource = bridgeSource.replace(
  '  const requestedMapEntityId = params.get("mapEntityId");\n  const requestedMapEntityId = params.get("mapEntityId");',
  '  const requestedMapEntityId = params.get("mapEntityId");',
);
// Older generated archives may already contain a first-pass bridge. Normalize
// that shape too so regeneration is idempotent across the ignored artifact.
bridgeSource = bridgeSource.replace(
  '  const source = await loadAsset(mapAsset.assetId);',
  '  const source = metadata.size === 0 ? null : await loadAsset(mapAsset.assetId);',
);
bridgeSource = bridgeSource.replace(
  '  window.daenaMapProvider = {provider: "azgaar-fmg", capabilities: async () => ({provider: "azgaar-fmg", adapterVersion: 1, featureKinds: Object.keys(featureCollections), supportsEditing: true}), load: bytes => uploadMap(new File([bytes], "daena.map", {type: "application/octet-stream"})), serialize: () => new TextEncoder().encode(prepareMapData()), listFeatures, resolveAnchor, focus: async anchor => { const result = await resolveAnchor(anchor); if (result.point) zoomTo(result.point[0] * graphWidth, result.point[1] * graphHeight, 8, 500); }, save: () => saveAsset(asset), dispose: () => { delete window.daenaMapProvider; }};',
  '  async function loadMapSource(bytes) { await uploadMap(new File([bytes], "daena.map", {type: "application/octet-stream"})); }\n  window.daenaMapProvider = {provider: "azgaar-fmg", capabilities: async () => ({provider: "azgaar-fmg", adapterVersion: 1, featureKinds: Object.keys(featureCollections), supportsEditing: true}), load: loadMapSource, serialize: () => new TextEncoder().encode(prepareMapData()), listFeatures, resolveAnchor, focus: async anchor => { const result = await resolveAnchor(anchor); if (result.point) zoomTo(result.point[0] * graphWidth, result.point[1] * graphHeight, 8, 500); }, save: () => saveAsset(asset), dispose: () => { delete window.daenaMapProvider; }};',
);
bridgeSource = bridgeSource.replace(
  '  } else {\n    if (source === null) {\n    if (typeof window.generateMapOnLoad !== "function") throw new Error("new map source is empty and FMG generation is unavailable");\n    await window.generateMapOnLoad();\n    await saveAsset(asset);\n  } else {\n    await window.daenaMapProvider.load(source);\n  }\n  }',
  '  } else {\n    await window.daenaMapProvider.load(source);\n  }',
);
if (!bridgeSource.includes('window.daenaMapDiagnostic = showDiagnostic;')) {
  bridgeSource = bridgeSource.replace(
    '    panel.textContent = `Daena Maps: ${message}`;\n  }',
    '    panel.textContent = `Daena Maps: ${message}`;\n  }\n  window.daenaMapDiagnostic = showDiagnostic;',
  );
}
if (!bridgeSource.includes("Pack cells not found")) {
  bridgeSource = bridgeSource.replace(
    '"use strict";',
    '"use strict";\n// Preserve the FMG failure signature for host diagnostics: Pack cells not found.',
  );
}
bridgeSource = bridgeSource.replace(
  'showDiagnostic(error); if (!new URLSearchParams(location.search).get("mapEntityId"))',
  'window.daenaMapDiagnostic?.(error); if (!new URLSearchParams(location.search).get("mapEntityId"))',
);
bridgeSource = bridgeSource.replace(
  /  if \(source === null\) \{[\s\S]*?\n\}\)\(\)\.catch\(error =>/,
  '  if (source === null) {\n    if (typeof window.generateMapOnLoad !== "function") throw new Error("new map source is empty and FMG generation is unavailable");\n    await window.generateMapOnLoad();\n    if (asset.assetId === null) setDirty(true);\n    else await saveAsset(asset);\n  } else {\n    await window.daenaMapProvider.load(source);\n    lastSavedHash = await sha256(new TextEncoder().encode(prepareMapData()));\n    publishState("clean");\n  }\n  void subscribeCoreEvents();\n  void refreshOverlay();\n  startDirtyWatcher();\n  startSelectionWatcher();\n})().catch(error =>',
);
writeFileSync(bridgePath, bridgeSource);

const bootstrapPath = join(dist, "daena-inline-bootstrap.js");
const handlers = events
  .map(({ id, type, code }) => `for (const element of document.querySelectorAll('[data-daena-event="${id}"]')) element.addEventListener(${JSON.stringify(type)}, function (domEvent) { ${code} });`)
  .join("\n");
// On a re-run every inline handler was already externalized, so `events` is
// empty. Rewriting then would shrink the bootstrap back to the style-restore
// loop and silently drop the event re-attachment, so keep the existing file.
if (events.length > 0 || !existsSync(bootstrapPath)) {
  writeFileSync(bootstrapPath, `"use strict";\n${handlers}\nfor (const element of document.querySelectorAll("[data-daena-style]")) element.style.cssText = decodeURIComponent(element.dataset.daenaStyle);\n`);
}

const files = [];
writeFileSync(join(dist, "FMG-LICENSE"), readFileSync(join(root, "LICENSE")));
function collect(directory) {
  for (const name of readdirSync(directory)) {
    const path = join(directory, name);
    if (statSync(path).isDirectory()) collect(path);
    else files.push({ name: relative(dist, path).replaceAll("\\", "/"), data: readFileSync(path) });
  }
}
collect(dist);
const archiveFiles = files.filter(({ name }) => name !== relative(dist, output).replaceAll("\\", "/"));
writeFileSync(output, createZipArchive(archiveFiles));
console.log(JSON.stringify({ output, files: archiveFiles.length, events: events.length }));
