import { readFile } from "node:fs/promises";

const generator = await readFile("src/lib/maps/native-vector/NativeVectorGenerator.svelte", "utf8");
const editor = await readFile("src/lib/maps/native-vector/NativeVectorMapEditor.svelte", "utf8");
const runtime = await readFile("src/lib/maps/native-vector/runtime.ts", "utf8");
const adapter = await readFile("packages/modules/maps/src/adapter.ts", "utf8");
const bridge = await readFile("scripts/fmg-bridge-template.js", "utf8");

for (const required of ["importImageMapFile", "Import image", "autostartImport"]) {
  if (!generator.includes(required)) throw new Error(`Native vector generator missing ${required}`);
}
for (const required of ['start?: "generate" | "import"', "previewAssetId", "Could not decode the imported map image"]) {
  if (!editor.includes(required)) throw new Error(`Native vector editor missing ${required}`);
}
for (const required of ["IMAGE_SOURCE_ID", "imageOverlayCoordinates", 'type: "image"']) {
  if (!runtime.includes(required)) throw new Error(`Native vector runtime missing ${required}`);
}
if (adapter.includes("ImageMapAdapter") || adapter.includes("daena-image")) {
  throw new Error("Image Map adapter must be removed after merging into native vector maps");
}
for (const forbidden of ["image-map-bridge.js", "daena-image", "mapMode"]) {
  if (bridge.includes(forbidden)) throw new Error(`FMG bridge must not route imported images: ${forbidden}`);
}

console.log("maps phase-3 native vector image import checks passed");
