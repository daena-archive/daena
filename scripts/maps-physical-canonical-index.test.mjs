import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";

import { physicalViewIndex } from "../src/lib/maps/native-vector/coordinates.ts";

const root = resolve(import.meta.dirname, "..");
const roots = ["src/lib/maps", "src/lib/project"];
const canonicalFields = ["minimumCell", "spillCell", "sourceCell", "mouthCell"];

function filesUnder(path) {
  return readdirSync(path, { withFileTypes: true }).flatMap((entry) => {
    const full = resolve(path, entry.name);
    if (entry.isDirectory()) return filesUnder(full);
    return /\.(svelte|ts)$/.test(entry.name) ? [full] : [];
  });
}

const offenders = [];
for (const directory of roots) {
  for (const file of filesUnder(resolve(root, directory))) {
    const source = readFileSync(file, "utf8");
    for (const field of canonicalFields) {
      const indexUse = new RegExp(`\\[[^\\]\\n]*\\.${field}\\s*\\]`);
      const match = indexUse.exec(source);
      if (match) {
        offenders.push(`${file.slice(root.length + 1)}: ${field} used as a per-cell index (${match[0]})`);
      }
    }
    if (/\[\s*(?:source|sources|volcanic|volcano)[A-Za-z0-9_$.]*\.cell\s*\]/.test(source)) {
      offenders.push(`${file.slice(root.length + 1)}: a canonical volcanic cell is used as a view index`);
    }
  }
}

assert.deepEqual(
  offenders,
  [],
  "canonical cells must not index per-cell view arrays; use coordinates or the GeoJSON geometry instead",
);

const raster = readFileSync(resolve(root, "src/lib/maps/physical/raster.ts"), "utf8");
assert.match(raster, /source\.longitudeMicro/, "volcanic markers are placed by longitude");
assert.match(raster, /source\.latitudeMicro/, "volcanic markers are placed by latitude");

// Product views are south-origin: a northern click must read a northern row.
assert.equal(physicalViewIndex(-180, -90, 1024, 512), 0, "south-west corner is cell 0");
assert.equal(physicalViewIndex(179.99, 89.99, 1024, 512), 512 * 1024 - 1, "north-east corner is the last cell");
assert.equal(Math.floor(physicalViewIndex(0, 60, 1024, 512) / 1024), Math.floor(((60 + 90) / 180) * 512));
assert.ok(physicalViewIndex(0, 60, 1024, 512) > physicalViewIndex(0, -60, 1024, 512), "north is a higher row");
const editor = readFileSync(resolve(root, "src/lib/maps/physical/PhysicalMapEditor.svelte"), "utf8");
assert.match(editor, /physicalViewIndex\(point\[0\], point\[1\]/, "the click readout uses the south-origin view index");

console.log("canonical hydrology and hazard cells are never used as view indices");
