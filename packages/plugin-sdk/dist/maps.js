export const MAP_ENTITY_TYPE = "daena.maps:map";
export const MAP_NAMESPACE = "maps";
export const FMG_PROVIDER = "azgaar-fmg";
export const VECTOR_PROVIDER = "daena-vector";
export const PHYSICAL_PROVIDER = "daena-physical";
export const IMAGE_SOURCE_FORMATS = ["png", "jpeg", "svg"];
/** Recorded imported-image resource budgets. Mirrored from `daena-core` maps::image. */
export const IMAGE_MAX_ENCODED_BYTES = 32 * 1024 * 1024;
export const IMAGE_MAX_PIXELS = 16_777_216;
export const IMAGE_MAX_DECODED_BYTES = IMAGE_MAX_PIXELS * 4 + 1024;
export const IMAGE_MAX_RASTER_LAYERS = 16;
export const IMAGE_MAX_UNDO_BYTES = 64 * 1024 * 1024;
export const IMAGE_MAX_PATH_POINTS = 256;
export const IMAGE_MAX_AREA_RINGS = 8;
export const IMAGE_MAX_SEMANTIC_LAYERS = 32;
/** Recorded Native Vector Map resource budgets. Mirrored from `daena-core` maps::vector. */
export const VECTOR_MAX_BYTES = 16 * 1024 * 1024;
export const VECTOR_MAX_FEATURES = 20_000;
export const VECTOR_MAX_POSITIONS = 200_000;
export const VECTOR_MAX_FEATURE_POSITIONS = 20_000;
export const VECTOR_MAX_LAYERS = 64;
/** Hierarchy relationship types owned by `daena.maps`. */
export const MAP_RELATIONSHIP = {
    DETAIL_MAP: "daena.maps:detail-map",
    OVERVIEW_MAP: "daena.maps:overview-map",
    RELATED_MAP: "daena.maps:related-map",
};
/** Register the browser bridge used by the Maps host surface. */
export function registerMapsHostSurfaceProvider(provider) {
    if (typeof window === "undefined")
        throw new Error("Maps host surface requires a browser runtime");
    window.daenaMapProvider = provider;
    return () => {
        if (window.daenaMapProvider === provider)
            delete window.daenaMapProvider;
    };
}
//# sourceMappingURL=maps.js.map