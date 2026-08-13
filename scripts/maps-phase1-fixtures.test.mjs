#!/usr/bin/env node
/** Structural Phase 1 Maps contract checks against docs/maps fixtures. */
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const readJson = async (path) => JSON.parse(await readFile(resolve(root, path), "utf8"));

const schema = await readJson("docs/maps/maps-contract-v1.schema.json");
const fixtures = await readJson("docs/maps/phase-1-fixtures.json");
const titles = new Set((schema.oneOf ?? []).map((entry) => entry.title));
for (const required of ["Map descriptor", "Location references", "Layer definitions"]) {
  if (!titles.has(required)) throw new Error(`maps contract missing ${required}`);
}

const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const pointOk = (point) =>
  Array.isArray(point) &&
  point.length === 2 &&
  point.every((value) => typeof value === "number" && value >= 0 && value <= 1);

function assertAnchor(anchor, id) {
  if (!anchor || typeof anchor !== "object") throw new Error(`${id}: anchor required`);
  switch (anchor.kind) {
    case "point":
      if (!pointOk(anchor.point)) throw new Error(`${id}: invalid point`);
      break;
    case "provider-feature":
      if (
        anchor.provider !== "azgaar-fmg" ||
        !anchor.featureKind ||
        !anchor.featureId ||
        !pointOk(anchor.fallbackPoint)
      ) {
        throw new Error(`${id}: invalid provider-feature`);
      }
      break;
    case "path":
      if (!Array.isArray(anchor.points) || anchor.points.length < 2 || !anchor.points.every(pointOk)) {
        throw new Error(`${id}: invalid path`);
      }
      break;
    case "area":
      if (!Array.isArray(anchor.rings) || anchor.rings.length < 1) throw new Error(`${id}: invalid area`);
      for (const ring of anchor.rings) {
        if (!Array.isArray(ring) || ring.length < 4 || !ring.every(pointOk))
          throw new Error(`${id}: invalid area ring`);
        const first = ring[0];
        const last = ring[ring.length - 1];
        if (first[0] !== last[0] || first[1] !== last[1]) throw new Error(`${id}: area ring must be closed`);
      }
      break;
    default:
      throw new Error(`${id}: unknown anchor kind`);
  }
}

for (const fixture of fixtures.fixtures) {
  const value = fixture.value;
  if (fixture.shape === "map") {
    const provider = value.provider;
    const imageFormat = provider?.id === "daena-image" && ["png", "jpeg", "svg"].includes(provider.sourceFormat);
    if ((provider?.id !== "azgaar-fmg" || provider?.sourceFormat !== "fmg-map") && !imageFormat) {
      throw new Error(`${fixture.id}: invalid map descriptor`);
    }
    if (value.defaultView?.zoom <= 0 || !pointOk(value.defaultView?.center)) {
      throw new Error(`${fixture.id}: invalid map descriptor`);
    }
    if (provider?.id === "daena-image" && !uuid.test(value.sourceAssetId ?? "")) {
      throw new Error(`${fixture.id}: image maps require sourceAssetId`);
    }
  } else if (fixture.shape === "locations") {
    if (!Array.isArray(value.locations) || value.locations.length < 2)
      throw new Error(`${fixture.id}: need multiple locations`);
    for (const location of value.locations) {
      if (!uuid.test(location.id) || !uuid.test(location.mapEntityId))
        throw new Error(`${fixture.id}: location ids must be UUIDs`);
      assertAnchor(location.anchor, `${fixture.id}:${location.id}`);
    }
  } else if (fixture.shape === "layers") {
    if (!Array.isArray(value.layers) || value.layers.length < 1) throw new Error(`${fixture.id}: need layers`);
    for (const layer of value.layers) {
      if (
        !uuid.test(layer.id) ||
        !layer.name ||
        typeof layer.order !== "number" ||
        typeof layer.defaultVisible !== "boolean"
      ) {
        throw new Error(`${fixture.id}: invalid layer`);
      }
      if (!layer.style || typeof layer.style !== "object" || !layer.selector || typeof layer.selector !== "object") {
        throw new Error(`${fixture.id}: layer style/selector required`);
      }
      if (layer.kind === "raster") {
        if (
          !uuid.test(layer.rasterAssetId ?? "") ||
          typeof layer.opacity !== "number" ||
          layer.opacity < 0 ||
          layer.opacity > 1 ||
          typeof layer.locked !== "boolean"
        ) {
          throw new Error(`${fixture.id}: invalid raster layer`);
        }
      } else if (layer.kind != null) {
        throw new Error(`${fixture.id}: unsupported layer kind`);
      }
    }
  } else {
    throw new Error(`${fixture.id}: unknown shape`);
  }
}

console.log(`maps phase-1 contract fixtures passed (${fixtures.fixtures.length} shapes)`);
