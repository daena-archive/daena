import { readFile } from "node:fs/promises";

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), "utf8");

const rust = await read("crates/daena-core/src/maps/image.rs");
const sdk = await read("packages/plugin-sdk/src/maps.ts");
const docs = await read("docs/NATIVE_MAP_INTEGRATION.md");
const editor = await read("src/lib/maps/native-vector/NativeVectorMapEditor.svelte");
const generator = await read("src/lib/maps/native-vector/NativeVectorGenerator.svelte");

for (const required of [
  "IMAGE_MAX_ENCODED_BYTES: usize = 32 * 1024 * 1024",
  "IMAGE_MAX_PIXELS: u64 = 16_777_216",
  "IMAGE_MAX_RASTER_LAYERS: usize = 16",
  "IMAGE_MAX_DECODED_BYTES",
]) {
  if (!rust.includes(required)) throw new Error(`Rust is missing recorded budget ${required}`);
}

for (const required of [
  "IMAGE_MAX_ENCODED_BYTES = 32 * 1024 * 1024",
  "IMAGE_MAX_PIXELS = 16_777_216",
  "IMAGE_MAX_RASTER_LAYERS = 16",
  "IMAGE_MAX_DECODED_BYTES",
]) {
  if (!sdk.includes(required)) throw new Error(`TypeScript contract is missing ${required}`);
}

for (const required of ["IMAGE_MAX_ENCODED_BYTES", "previewAssetId", "Import image"]) {
  if (!docs.includes(required)) throw new Error(`NATIVE_MAP_INTEGRATION.md missing ${required}`);
}

for (const required of ['role="toolbar"', "aria-pressed"]) {
  if (!editor.includes(required)) throw new Error(`Native vector accessibility missing ${required}`);
}
if (!generator.includes("visually-hidden")) {
  throw new Error("Native vector generator missing visually-hidden candidate labels");
}

console.log("maps phase-4 image import budgets and accessibility checks passed");
