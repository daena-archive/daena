/** Provider-neutral Maps domain types. Provider-specific selectors stay opaque. */
export type NormalizedPoint = readonly [number, number];
export declare const MAP_ENTITY_TYPE: "daena.maps:map";
export declare const MAP_NAMESPACE: "maps";
export declare const FMG_PROVIDER: "azgaar-fmg";
export declare const VECTOR_PROVIDER: "daena-vector";
export declare const PHYSICAL_PROVIDER: "daena-physical";
export declare const IMAGE_SOURCE_FORMATS: readonly ["png", "jpeg", "svg"];
/** Recorded imported-image resource budgets. Mirrored from `daena-core` maps::image. */
export declare const IMAGE_MAX_ENCODED_BYTES: number;
export declare const IMAGE_MAX_PIXELS = 16777216;
export declare const IMAGE_MAX_DECODED_BYTES: number;
export declare const IMAGE_MAX_RASTER_LAYERS = 16;
export declare const IMAGE_MAX_UNDO_BYTES: number;
export declare const IMAGE_MAX_PATH_POINTS = 256;
export declare const IMAGE_MAX_AREA_RINGS = 8;
export declare const IMAGE_MAX_SEMANTIC_LAYERS = 32;
/** Recorded Native Vector Map resource budgets. Mirrored from `daena-core` maps::vector. */
export declare const VECTOR_MAX_BYTES: number;
export declare const VECTOR_MAX_FEATURES = 20000;
export declare const VECTOR_MAX_POSITIONS = 200000;
export declare const VECTOR_MAX_FEATURE_POSITIONS = 20000;
export declare const VECTOR_MAX_LAYERS = 64;
/** Hierarchy relationship types owned by `daena.maps`. */
export declare const MAP_RELATIONSHIP: {
    readonly DETAIL_MAP: "daena.maps:detail-map";
    readonly OVERVIEW_MAP: "daena.maps:overview-map";
    readonly RELATED_MAP: "daena.maps:related-map";
};
export type MapRelationshipType = (typeof MAP_RELATIONSHIP)[keyof typeof MAP_RELATIONSHIP];
export type MapAnchor = {
    kind: "point";
    point: NormalizedPoint;
} | {
    kind: "provider-feature";
    provider: string;
    featureKind: string;
    featureId: string;
    fallbackPoint: NormalizedPoint;
} | {
    kind: "path";
    points: readonly NormalizedPoint[];
} | {
    kind: "area";
    rings: readonly (readonly NormalizedPoint[])[];
};
export type ImageSourceFormat = (typeof IMAGE_SOURCE_FORMATS)[number];
export type MapDescriptor = {
    schemaVersion: 1;
    provider: {
        id: typeof FMG_PROVIDER;
        adapterVersion: 1;
        sourceFormat: "fmg-map";
    };
    sourceAssetId: string | null;
    previewAssetId: string | null;
    defaultView: {
        center: NormalizedPoint;
        zoom: number;
    };
} | {
    schemaVersion: 1;
    provider: {
        id: typeof VECTOR_PROVIDER;
        adapterVersion: 1;
        sourceFormat: "geojson";
    };
    sourceAssetId: string;
    previewAssetId: string | null;
    defaultView: {
        center: NormalizedPoint;
        zoom: number;
    };
    generation?: {
        id: "daena-landmass";
        version: 1;
        seed: number;
        settings: {
            landPercent: number;
            continentCount: number;
            coastlineRoughness: "low" | "medium" | "high";
            islandFrequency: "none" | "low" | "medium" | "high";
        };
    };
} | {
    schemaVersion: 1;
    provider: {
        id: typeof PHYSICAL_PROVIDER;
        adapterVersion: 1;
        sourceFormat: "physical-world-v1";
    };
    sourceAssetId: string;
    previewAssetId: null;
    defaultView: {
        center: NormalizedPoint;
        zoom: number;
    };
    generation: {
        id: "daena-physical-world";
        version: 1;
        seed: number;
        retryIndex: number;
        settings: {
            width: number;
            height: number;
            radiusMetres: number;
            targetLandFractionPpm: number;
            referenceWaterInventoryM3: number;
        };
    };
};
export interface MapDate {
    calendar: "gregorian";
    era: "BCE" | "CE";
    year: number;
    month?: number;
    day?: number;
    precision: "year" | "month" | "day";
}
export interface MapLocationReference {
    id: string;
    mapEntityId: string;
    role: string;
    label: string;
    anchor: MapAnchor;
    validity: {
        from: MapDate | null;
        to: MapDate | null;
    };
}
export interface MapLocationsField {
    schemaVersion: 1;
    locations: readonly MapLocationReference[];
}
export type MapLayerDefinition = {
    id: string;
    name: string;
    order: number;
    defaultVisible: boolean;
    style: Readonly<Record<string, unknown>>;
    selector: Readonly<Record<string, unknown>>;
    kind?: "semantic";
} | {
    id: string;
    name: string;
    order: number;
    defaultVisible: boolean;
    style: Readonly<Record<string, unknown>>;
    selector: Readonly<Record<string, unknown>>;
    kind: "raster";
    rasterAssetId: string;
    opacity: number;
    locked: boolean;
} | {
    id: string;
    name: string;
    order: number;
    defaultVisible: boolean;
    locked: boolean;
    selector: Readonly<Record<string, never>>;
    style: {
        fill: string;
        fillOpacity: number;
        stroke: string;
        strokeWidth: number;
        pointRadius: number;
    };
    kind: "vector";
};
export interface MapLayersField {
    schemaVersion: 1;
    layers: readonly MapLayerDefinition[];
}
export type MapFocusResult = {
    status: "focused";
    mapEntityId: string;
    linkId: string | null;
} | {
    status: "multiple-links";
    locations: readonly MapLocationReference[];
};
export type MapOpenResult = {
    mapEntityId: string;
    linkId: string | null;
};
export type MapShowResultsResult = {
    mapEntityId: string;
    locations: readonly MapLocationReference[];
};
export interface MapNavigationService {
    openMap(input: {
        mapEntityId: string;
        linkId?: string;
        mode?: "view" | "edit";
    }): Promise<MapOpenResult>;
    focusEntity(input: {
        entityId: string;
        mapEntityId?: string;
    }): Promise<MapFocusResult>;
    setDate(input: {
        date: MapDate | null;
    }): Promise<{
        accepted: boolean;
        date: MapDate | null;
    }>;
    showResults(input: {
        entityIds: readonly string[];
        mapEntityId?: string;
    }): Promise<MapShowResultsResult>;
    listLocations(input: {
        entityId: string;
    }): Promise<readonly MapLocationReference[]>;
}
/** Browser-side bridge implemented by a plugin that declares
 * `daena.maps/editor@1`. The host invokes these methods for shell actions;
 * all project reads, writes, and event publication remain brokered through the
 * plugin SDK. Every method is optional so a surface can support only the
 * interactions it understands. */
export interface MapsHostSurfaceProvider {
    save?: () => void | Promise<void>;
    captureSelection?: () => void | Promise<void>;
    startPick?: () => void | Promise<void>;
    setSemanticOverlay?: (frame: unknown) => void | Promise<void>;
    setDate?: (date: MapDate | null) => void | Promise<void>;
    focusByLink?: (linkId: string) => void | Promise<void>;
}
declare global {
    interface Window {
        daenaMapProvider?: MapsHostSurfaceProvider;
    }
}
/** Register the browser bridge used by the Maps host surface. */
export declare function registerMapsHostSurfaceProvider(provider: MapsHostSurfaceProvider): () => void;
/** Shell-side location mutations available to module contexts (not part of
 * the public navigation service contract; both are revision-aware). */
export interface MapLocationsMutations {
    upsertLocation(input: {
        entityId: string;
        location: MapLocationReference;
    }): Promise<void>;
    unlinkLocation(input: {
        entityId: string;
        locationId: string;
    }): Promise<void>;
}
/** Payload of `daena.maps/selection@1`, forwarded to the shell as the
 * `maps-selection` Tauri event. `anchor` is null when nothing is capturable. */
export interface MapSelectionEvent {
    mapEntityId: string;
    anchor: MapAnchor | null;
}
//# sourceMappingURL=maps.d.ts.map