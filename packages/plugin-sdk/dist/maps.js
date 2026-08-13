export const MAP_ENTITY_TYPE = "daena.maps:map";
export const MAP_NAMESPACE = "maps";
export const FMG_PROVIDER = "azgaar-fmg";
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