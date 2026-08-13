import { readFile } from "node:fs/promises";

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), "utf8");

const docs = await read("docs/IMAGE_MAP_INTEGRATION.md");
const rust = await read("crates/daena-core/src/maps/image.rs");
const sdk = await read("packages/plugin-sdk/src/maps.ts");
const editor = await read("src/lib/maps/image-map/ImageMapEditor.svelte");
const engine = await read("src/lib/maps/image-map/engine.ts");

for (const required of [
  "IMAGE_MAX_ENCODED_BYTES: usize = 32 * 1024 * 1024",
  "IMAGE_MAX_PIXELS: u64 = 16_777_216",
  "IMAGE_MAX_RASTER_LAYERS: usize = 16",
  "IMAGE_MAX_UNDO_BYTES: usize = 64 * 1024 * 1024",
  "IMAGE_MAX_DECODED_BYTES",
]) {
  if (!rust.includes(required)) throw new Error(`Rust is missing recorded budget ${required}`);
}

for (const required of [
  "IMAGE_MAX_ENCODED_BYTES = 32 * 1024 * 1024",
  "IMAGE_MAX_PIXELS = 16_777_216",
  "IMAGE_MAX_RASTER_LAYERS = 16",
  "IMAGE_MAX_UNDO_BYTES = 64 * 1024 * 1024",
  "IMAGE_MAX_DECODED_BYTES",
]) {
  if (!sdk.includes(required)) throw new Error(`TypeScript contract is missing ${required}`);
}

for (const required of [
  "Recorded resource budgets (Phase 4)",
  "IMAGE_MAX_ENCODED_BYTES",
  "IMAGE_MAX_PIXELS",
  "IMAGE_MAX_DECODED_BYTES",
  "IMAGE_MAX_RASTER_LAYERS",
  "IMAGE_MAX_UNDO_BYTES",
  "32 MiB",
  "16,777,216",
  "Hidden layers keep only their asset id",
]) {
  if (!docs.includes(required)) throw new Error(`IMAGE_MAP_INTEGRATION.md missing ${required}`);
}

for (const required of [
  "image is empty",
  "encoded-byte budget of {}",
  "pixel budget of {IMAGE_MAX_PIXELS}",
  "decoded-memory budget of {}",
  "choose a smaller",
]) {
  if (!rust.includes(required)) throw new Error(`Rust diagnostics missing ${required}`);
}

for (const required of ["ensureLayerCanvas", "releaseHiddenLayer", "if (next.visible) await ensureLayerCanvas"]) {
  if (!editor.includes(required)) throw new Error(`Hidden-layer lazy decode missing ${required}`);
}

for (const required of [
  'role="toolbar"',
  "aria-pressed",
  "aria-label={layer.visible ? `Hide ${layer.name}`",
  "prefers-reduced-motion",
  "onCanvasKey",
  "visually-hidden",
  "focus-visible",
]) {
  if (!editor.includes(required)) throw new Error(`Image Map accessibility missing ${required}`);
}

for (const required of ["panBy", "zoomAtCenter", "isEditableTarget"]) {
  if (!engine.includes(required)) throw new Error(`Image Map keyboard viewport missing ${required}`);
}

if (editor.includes("canvas: await canvasFromPng(bytes, image.naturalWidth")) {
  throw new Error("Image Map load still decodes every raster layer eagerly");
}

console.log("maps phase-4 budgets and accessibility checks passed");
