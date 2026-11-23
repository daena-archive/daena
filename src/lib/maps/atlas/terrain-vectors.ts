import type { FeatureLike } from "ol/Feature.js";
import Stroke from "ol/style/Stroke.js";
import Style from "ol/style/Style.js";

export const TERRAIN_IMAGERY_STYLE_ID = "daena-atlas-imagery";
export const TERRAIN_RELIEF_STYLE_ID = "daena-atlas-relief";

export function isTerrainStyle(styleId: string): boolean {
  return styleId === TERRAIN_IMAGERY_STYLE_ID || styleId === TERRAIN_RELIEF_STYLE_ID;
}

export type HydroKind = "coast" | "river" | "lake";

/** Classify one derived-GeoJSON feature. Unknown features are left out. */
export function hydroKind(feature: FeatureLike): HydroKind | null {
  const kind = String(feature.get("kind") ?? "").toLowerCase();
  const name = String(feature.get("name") ?? "").toLowerCase();
  const layer = String(feature.get("daenaLayerId") ?? feature.get("layerId") ?? "").toLowerCase();
  const id = String(feature.getId() ?? "").toLowerCase();
  if (kind.includes("river") || layer === "rivers" || name.includes("river") || id.startsWith("river")) {
    return "river";
  }
  if (kind.includes("lake") || layer === "lakes" || name.includes("lake") || id.startsWith("lake")) {
    return "lake";
  }
  if (
    kind.includes("coast") ||
    layer === "coastlines" ||
    layer === "islands" ||
    name.includes("coast") ||
    id.startsWith("coastline")
  ) {
    return "coast";
  }
  return null;
}

export function riverWidth(feature: FeatureLike): number {
  const raw = feature.get("strahlerOrder") ?? feature.get("order") ?? feature.get("strahler");
  const order = typeof raw === "number" ? raw : Number(raw);
  if (!Number.isFinite(order) || order <= 0) return 1;
  return Math.min(4, 0.6 + order * 0.35);
}

export const coastStyle = new Style({
  stroke: new Stroke({ color: "#1c242a", width: 1 }),
});

export const lakeStyle = new Style({
  stroke: new Stroke({ color: "#366c94", width: 1.25 }),
});

export function riverStyle(feature: FeatureLike): Style {
  return new Style({
    stroke: new Stroke({ color: "#24527a", width: riverWidth(feature) }),
  });
}
