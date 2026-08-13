import { readFile, stat } from "node:fs/promises";

const bridge = await readFile("scripts/fmg-bridge-template.js", "utf8");
const editor = await readFile("src/lib/maps/image-map/ImageMapEditor.svelte", "utf8");
const adapter = await readFile("packages/modules/maps/src/adapter.ts", "utf8");
const engine = await readFile("src/lib/maps/image-map/engine.ts", "utf8");

for (const required of [
  "project.importImageMapFile",
  "project.readAssetBytes",
  "project.createRasterLayer",
  "project.updateMapLayer",
  "project.deleteRasterLayer",
  "project.replaceAssetBytes",
  "project.listMapPins",
  "onpick?.({",
  "registerImageMapSession",
  "Undo",
  "Eraser",
]) {
  if (!editor.includes(required)) throw new Error(`Image Map native editor regression: missing ${required}`);
}

for (const required of [
  'import Konva from "konva"',
  "class ImageMapStage",
  "setRasterLayer",
  "focusNormalized",
  "destination-out",
]) {
  if (!engine.includes(required)) throw new Error(`Image Map engine regression: missing ${required}`);
}

if (!adapter.includes("export class ImageMapAdapter") || adapter.includes("extends FmgBrowserAdapter")) {
  throw new Error("Image Map adapter must be provider-neutral and must not wrap FMG");
}
if (!adapter.includes("export function selectProviderAdapter")) {
  throw new Error("provider selection boundary is missing");
}

for (const forbidden of ["image-map-bridge.js", "daena-image", "mapMode"]) {
  if (bridge.includes(forbidden)) throw new Error(`FMG bridge must not route Image Maps: ${forbidden}`);
}

await stat("src/lib/maps/image-map/ImageMapEditor.svelte");

console.log("maps phase-3 native Image Map checks passed");
