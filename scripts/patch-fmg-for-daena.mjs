import { readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import { createZipArchive } from "../packages/plugin-cli/bin/zip.mjs";

const root = resolve(process.argv[2] ?? "");
const output = resolve(process.argv[3] ?? join(root, "dist", "daena-fmg-v1.119.zip"));
if (!process.argv[2]) throw new Error("usage: node scripts/patch-fmg-for-daena.mjs /path/to/fmg [output.zip]");

const dist = resolve(root, "dist");
const htmlPath = join(dist, "index.html");
let html = readFileSync(htmlPath, "utf8");

// Keep the host marker ahead of every deferred FMG script. FMG's bundled
// runtime reads it during initialization, before the provider bridge runs.
html = html.replace(/\s*<script defer src="daena-bridge\.js"><\/script>/g, "");
const bridge = '    <script defer src="daena-bridge.js"></script>';
if (html.includes('<base href="/dist/ui/fmg/" />')) {
  html = html.replace('<base href="/dist/ui/fmg/" />', `<base href="/dist/ui/fmg/" />\n${bridge}`);
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
if (!html.includes('href="daena-inline.css"')) {
  html = html.replace("</head>", '    <link rel="stylesheet" href="daena-inline.css">\n  </head>');
}
writeFileSync(htmlPath, html);

const mainPath = join(dist, "main.js");
let main = readFileSync(mainPath, "utf8");
if (!main.includes("if (DAENA_HOST) return;")) {
  main = main.replace("function toggleAssistant() {", "function toggleAssistant() {\n  if (DAENA_HOST) return;");
}
// Daena is offline-first and does not ship FMG's browser-only assistant.
// Remove the whole dynamic import branch so WebKit never reports a blocked
// network script, even if a persisted FMG option tries to show the assistant.
main = main.replace(
  /    } else \{\n      import\("\.\/libs\/openwidget\.min\.js"\)\.then\(\(\) => \{[\s\S]*?\n      \}\);\n    \}/,
  "    } else {\n      // Daena does not ship FMG's browser-only assistant.\n    }",
);
writeFileSync(mainPath, main);

const bridgePath = join(dist, "daena-bridge.js");
let bridgeSource = readFileSync(bridgePath, "utf8");
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
  '  async function firstMapAsset() {\n    const maps = await rpc("entity.list", {entityType: "daena.maps:map"});\n    for (const map of maps) {\n      const field = await rpc("field.read", {entityId: map.id, namespace: "daena.maps", key: "map"});\n      const descriptor = Array.isArray(field) ? field[0]?.value : field?.value ?? field;\n      if (descriptor?.sourceAssetId) return {mapId: map.id, assetId: descriptor.sourceAssetId};\n    }\n    return null;\n  }',
  '  async function firstMapAsset() {\n    const maps = await rpc("entity.list", {entityType: "daena.maps:map"});\n    for (const map of maps) {\n      if (requestedMapEntityId && map.id !== requestedMapEntityId) continue;\n      const field = await rpc("field.read", {entityId: map.id, namespace: "daena.maps", key: "map"});\n      const descriptor = Array.isArray(field) ? field[0]?.value : field?.value ?? field;\n      if (descriptor?.sourceAssetId) return {mapId: map.id, assetId: descriptor.sourceAssetId};\n    }\n    if (requestedMapEntityId) throw new Error(`requested map is unavailable: ${requestedMapEntityId}`);\n    return null;\n  }',
);
if (!bridgeSource.includes("Daena Maps provider startup failed")) {
  bridgeSource = bridgeSource.replace(
    "})();",
    '})().catch(error => { console.error("Daena Maps provider startup failed:", error); if (!new URLSearchParams(location.search).get("mapEntityId")) window.generateMapOnLoad?.(); });',
  );
}
bridgeSource = bridgeSource.replace(
  '})().catch(error => { console.error("Daena Maps provider startup failed:", error); window.generateMapOnLoad?.(); });',
  '})().catch(error => { console.error("Daena Maps provider startup failed:", error); if (!new URLSearchParams(location.search).get("mapEntityId")) window.generateMapOnLoad?.(); });',
);
bridgeSource = bridgeSource.replace(
  '  const source = await loadAsset(mapAsset.assetId);\n  const asset = {mapId: mapAsset.mapId, assetId: mapAsset.assetId, revision: metadata.revision, contentHash: metadata.contentHash};',
  '  const asset = {mapId: mapAsset.mapId, assetId: mapAsset.assetId, revision: metadata.revision, contentHash: metadata.contentHash};\n  const source = metadata.size === 0 ? null : await loadAsset(mapAsset.assetId);',
);
bridgeSource = bridgeSource.replace(
  '  await window.daenaMapProvider.load(source);',
  '  if (source === null) {\n    if (typeof window.generateMapOnLoad !== "function") throw new Error("new map source is empty and FMG generation is unavailable");\n    await window.generateMapOnLoad();\n    await saveAsset(asset);\n  } else {\n    await window.daenaMapProvider.load(source);\n  }',
);
bridgeSource = bridgeSource.replace(
  '  window.saveMap = method => method === "machine" || method === "dropbox" || method === "storage" ? saveAsset(asset) : originalSave?.(method);\n  window.addEventListener("keydown", event => { if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") { event.preventDefault(); void saveAsset(asset); } });',
  '  window.saveMap = method => method === "machine" || method === "dropbox" || method === "storage" ? saveAsset(asset).catch(error => { showDiagnostic(error); throw error; }) : originalSave?.(method);\n  window.addEventListener("keydown", event => { if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") { event.preventDefault(); void saveAsset(asset).catch(showDiagnostic); } });',
);
bridgeSource = bridgeSource.replace(
  '})().catch(error => { console.error("Daena Maps provider startup failed:", error); if (!new URLSearchParams(location.search).get("mapEntityId")) window.generateMapOnLoad?.(); });',
  '})().catch(error => { console.error("Daena Maps provider startup failed:", error); showDiagnostic(error); if (!new URLSearchParams(location.search).get("mapEntityId")) window.generateMapOnLoad?.(); });',
);
bridgeSource = bridgeSource.replace(
  'mimeType: "application/octet-stream"',
  'mimeType: "application/x-fmg-map"',
);
bridgeSource = bridgeSource.replace(
  '  const requestedMapEntityId = params.get("mapEntityId");\n  const requestedMapEntityId = params.get("mapEntityId");',
  '  const requestedMapEntityId = params.get("mapEntityId");',
);
writeFileSync(bridgePath, bridgeSource);

const bootstrapPath = join(dist, "daena-inline-bootstrap.js");
const handlers = events
  .map(({ id, type, code }) => `for (const element of document.querySelectorAll('[data-daena-event="${id}"]')) element.addEventListener(${JSON.stringify(type)}, function (domEvent) { ${code} });`)
  .join("\n");
writeFileSync(bootstrapPath, `"use strict";\n${handlers}\nfor (const element of document.querySelectorAll("[data-daena-style]")) element.style.cssText = decodeURIComponent(element.dataset.daenaStyle);\n`);

const files = [];
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
