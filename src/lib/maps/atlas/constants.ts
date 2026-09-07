/** Must match `daena_atlas::ATLAS_DETAIL_ALGORITHM_VERSION`. */
export const ATLAS_DETAIL_ALGORITHM_VERSION = 5;

/** Must match `daena_atlas::request::ATLAS_DEFAULT_VISIBLE_LAYER_IDS`. */
export const ATLAS_DEFAULT_VISIBLE_LAYER_IDS = ["ocean", "relief", "ice", "lakes", "graticule"] as const;

const ATLAS_STYLE_LABELS: Record<string, string> = {
  "daena-atlas-relief": "Elevation",
  "daena-atlas-biome": "Biomes",
  "daena-atlas-temperature": "Annual temperature",
  "daena-atlas-temperature-nh-summer": "Northern-summer solstice",
  "daena-atlas-temperature-nh-winter": "Northern-winter solstice",
  "daena-atlas-freeze": "Freeze",
  "daena-atlas-precipitation": "Annual rainfall",
  "daena-atlas-precipitation-nh-summer": "Northern-summer rainfall",
  "daena-atlas-precipitation-nh-winter": "Northern-winter rainfall",
  "daena-atlas-humidity": "Humidity",
  "daena-atlas-aridity": "Aridity",
  "daena-atlas-storms": "Storm genesis",
  "daena-atlas-storm-tracks": "Storm tracks",
  "daena-atlas-bathymetry": "Bathymetry",
  "daena-atlas-hydrology": "Hydrology",
  "daena-atlas-antique": "Antique",
  "daena-atlas-political": "Political",
};

export function isAtlasLayerEnabledByDefault(layerId: string): boolean {
  return (ATLAS_DEFAULT_VISIBLE_LAYER_IDS as readonly string[]).includes(layerId);
}

export function atlasStyleLabel(id: string): string {
  return ATLAS_STYLE_LABELS[id] ?? id;
}
