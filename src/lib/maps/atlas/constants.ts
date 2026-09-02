/** Must match `daena_atlas::ATLAS_DETAIL_ALGORITHM_VERSION`. */
export const ATLAS_DETAIL_ALGORITHM_VERSION = 1;

/** Must match `daena_atlas::request::ATLAS_DEFAULT_VISIBLE_LAYER_IDS`. */
export const ATLAS_DEFAULT_VISIBLE_LAYER_IDS = ["ocean", "relief", "ice", "lakes", "graticule"] as const;

export function isAtlasLayerEnabledByDefault(layerId: string): boolean {
  return (ATLAS_DEFAULT_VISIBLE_LAYER_IDS as readonly string[]).includes(layerId);
}
