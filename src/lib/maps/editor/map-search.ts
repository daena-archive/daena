import {
  featureLayerId,
  featureName,
  featureSemanticType,
  type MapLayerDefinition,
  type VectorFeatureCollection,
} from "../native-vector/types";

export type MapFeatureSearchEntry = {
  featureId: string;
  name: string;
  semanticType: string;
  layerId: string;
  layerName: string;
  linkedEntityName: string | null;
  searchableText: string;
};

export type MapFeatureSearchResult = MapFeatureSearchEntry & { score: number };

function normalize(value: string) {
  return value.trim().toLocaleLowerCase();
}

export function buildMapSearchIndex(
  collection: VectorFeatureCollection,
  layers: readonly MapLayerDefinition[],
  linkedEntityNames: ReadonlyMap<string, string> = new Map(),
): MapFeatureSearchEntry[] {
  const layerNames = new Map(layers.map((layer) => [layer.id, layer.name]));
  return collection.features.map((feature) => {
    const name = featureName(feature) ?? "Untitled feature";
    const semanticType = featureSemanticType(feature);
    const layerId = featureLayerId(feature);
    const layerName = layerNames.get(layerId) ?? "Unknown layer";
    const linkedEntityName = linkedEntityNames.get(feature.id) ?? null;
    const custom = Object.entries(feature.properties.daena.custom)
      .map(([key, value]) => `${key} ${value ?? ""}`)
      .join(" ");
    return {
      featureId: feature.id,
      name,
      semanticType,
      layerId,
      layerName,
      linkedEntityName,
      searchableText: normalize([feature.id, name, semanticType, layerName, custom, linkedEntityName ?? ""].join(" ")),
    };
  });
}

export function searchMapFeatures(
  index: readonly MapFeatureSearchEntry[],
  query: string,
  limit = 50,
): MapFeatureSearchResult[] {
  const terms = normalize(query).split(/\s+/).filter(Boolean);
  if (terms.length === 0) return [];
  return index
    .flatMap((entry) => {
      if (!terms.every((term) => entry.searchableText.includes(term))) return [];
      const name = normalize(entry.name);
      const score = terms.reduce(
        (sum, term) => sum + (name === term ? 100 : name.startsWith(term) ? 60 : name.includes(term) ? 35 : 10),
        0,
      );
      return [{ ...entry, score }];
    })
    .sort(
      (left, right) =>
        right.score - left.score ||
        left.name.localeCompare(right.name) ||
        left.featureId.localeCompare(right.featureId),
    )
    .slice(0, Math.max(0, limit));
}
