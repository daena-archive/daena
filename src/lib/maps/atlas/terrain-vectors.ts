import type { FeatureLike } from "ol/Feature.js";
import Fill from "ol/style/Fill.js";
import Stroke from "ol/style/Stroke.js";
import Style from "ol/style/Style.js";
import Text from "ol/style/Text.js";

export const TERRAIN_IMAGERY_STYLE_ID = "daena-atlas-imagery";
export const TERRAIN_RELIEF_STYLE_ID = "daena-atlas-relief";

export const TERRAIN_VECTOR_LAYERS = ["rivers", "coastline", "lakes", "contours"] as const;
export type TerrainVectorLayerId = (typeof TERRAIN_VECTOR_LAYERS)[number];

export function isTerrainStyle(styleId: string): boolean {
  return styleId === TERRAIN_IMAGERY_STYLE_ID || styleId === TERRAIN_RELIEF_STYLE_ID;
}

/** Highest Strahler order the engine produces on Earthlike worlds. */
const MAX_RIVER_ORDER = 5;

/** Order 1 is a hairline at z12. The largest rivers are about 3 px at z12 and 1 px at z5. */
export function riverWidthPx(order: number, zoom: number): number {
  const t = Math.min(1, Math.max(0, (order - 1) / (MAX_RIVER_ORDER - 1)));
  const at = (major: number) => 0.35 + t * (major - 0.35);
  const u = Math.min(1, Math.max(0, (zoom - 5) / 7));
  return at(1) + (at(3) - at(1)) * u;
}

const coastStroke = new Stroke({ color: "#1c242a", width: 1 });
const lakeStroke = new Stroke({ color: "#3a6f96", width: 1 });
const contourStroke = new Stroke({ color: "rgba(110, 80, 50, 0.55)", width: 0.7 });
const indexStroke = new Stroke({ color: "rgba(110, 80, 50, 0.55)", width: 1.25 });

export function coastVectorStyle(): Style {
  return new Style({ stroke: coastStroke });
}

export function lakeVectorStyle(): Style {
  return new Style({ stroke: lakeStroke });
}

export function contourVectorStyle(
  zoomFor: (resolution: number) => number,
): (feature: FeatureLike, resolution: number) => Style {
  return (feature, resolution) => {
    const index = feature.get("index") === true;
    const zoom = zoomFor(resolution);
    const elevation = feature.get("elevation_m");
    const label = index && zoom >= 9 && elevation != null ? String(elevation) : "";
    return new Style({
      stroke: index ? indexStroke : contourStroke,
      text: label
        ? new Text({
            text: label,
            placement: "line",
            repeat: 420,
            font: "600 11px system-ui",
            fill: new Fill({ color: "rgba(90, 64, 36, 0.9)" }),
            stroke: new Stroke({ color: "rgba(255, 250, 240, 0.8)", width: 2 }),
          })
        : undefined,
    });
  };
}

export function riverVectorStyle(
  relief: boolean,
  zoomFor: (resolution: number) => number,
  labelFor: (id: string) => string,
): (feature: FeatureLike, resolution: number) => Style {
  return (feature, resolution) => {
    const raw = feature.get("strahler_order");
    const order = typeof raw === "number" ? raw : Number(raw);
    const zoom = zoomFor(resolution);
    const id = String(feature.get("id") ?? "");
    const label = zoom >= 7 ? labelFor(id) : "";
    return new Style({
      stroke: new Stroke({
        color: relief ? "#3b6ea5" : "#2e5868",
        width: riverWidthPx(Number.isFinite(order) ? order : 1, zoom),
      }),
      text: label
        ? new Text({
            text: label,
            placement: "line",
            repeat: 520,
            font: "600 12px system-ui",
            fill: new Fill({ color: "#edf2ec" }),
            stroke: new Stroke({ color: "#1b2822", width: 3 }),
          })
        : undefined,
    });
  };
}
