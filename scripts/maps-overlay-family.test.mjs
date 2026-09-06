import assert from "node:assert/strict";

import {
  nextOverlayLayerName,
  overlayFamilyFromName,
  overlayNameSuggestions,
  uniqueOverlayLayerName,
} from "../src/lib/maps/atlas/overlay-family.ts";

assert.equal(overlayFamilyFromName(""), "custom");
assert.equal(overlayFamilyFromName("  Kingdoms  "), "custom");
assert.equal(overlayFamilyFromName("Political"), "political");
assert.equal(overlayFamilyFromName("political"), "political");
assert.equal(overlayFamilyFromName("Language"), "linguistic");
assert.equal(overlayFamilyFromName("linguistic"), "linguistic");
assert.equal(overlayFamilyFromName("Political 2"), "custom");

assert.equal(uniqueOverlayLayerName([], ""), "Overlay");
assert.equal(uniqueOverlayLayerName([], "Kingdoms"), "Kingdoms");
assert.equal(uniqueOverlayLayerName([{ name: "Kingdoms" }], "Kingdoms"), "Kingdoms 2");
assert.equal(uniqueOverlayLayerName([{ name: "Kingdoms" }, { name: "Kingdoms 2" }], "Kingdoms"), "Kingdoms 3");

assert.equal(nextOverlayLayerName([], "political"), "Political");
assert.equal(nextOverlayLayerName([{ name: "Political" }], "political"), "Political 2");

const all = overlayNameSuggestions();
assert.deepEqual(
  all.map((item) => item.label),
  ["Political", "Cultural", "Religious", "Language", "Economic", "Military"],
);
assert.equal(
  overlayNameSuggestions("lang")
    .map((item) => item.family)
    .join(),
  "linguistic",
);
assert.equal(overlayNameSuggestions("xyz").length, 0);
