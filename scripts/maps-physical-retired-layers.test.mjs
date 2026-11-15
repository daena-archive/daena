import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import {
  dropRetiredPhysicalLayers,
  parseVectorLayers,
  RETIRED_PHYSICAL_LAYER_IDS,
} from "../src/lib/maps/native-vector/source.ts";

const root = resolve(import.meta.dirname, "..");
const stored = JSON.parse(readFileSync(resolve(root, "scripts/fixtures/maps/old-physical-layers.json"), "utf8"));
const parsed = parseVectorLayers(stored);
assert.ok(
  parsed.some((layer) => layer.id === "currents"),
  "the old layer list still parses",
);
assert.ok(
  parsed.some((layer) => layer.id === "humidity"),
  "the old layer list still parses",
);

const cleaned = dropRetiredPhysicalLayers(parsed);
assert.deepEqual(
  cleaned.map((layer) => layer.id),
  ["base", "rivers"],
  "retired physical layers are dropped on read",
);
for (const id of RETIRED_PHYSICAL_LAYER_IDS) {
  assert.equal(
    cleaned.some((layer) => layer.id === id),
    false,
    `${id} must not reach a toggle`,
  );
}

const nativeEditor = readFileSync(resolve(root, "src/lib/maps/native-vector/NativeVectorMapEditor.svelte"), "utf8");
assert.match(nativeEditor, /dropRetiredPhysicalLayers\(parsed\)/, "the editor drops retired layers on load");
assert.doesNotMatch(nativeEditor, /id: "currents"/, "no dead currents layer is created");
assert.doesNotMatch(nativeEditor, /id === "currents"/, "no dead currents toggle is wired");

console.log("retired physical layer lists load cleanly");
