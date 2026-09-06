import {
  DEFAULT_VECTOR_LAYER_STYLE,
  type MapLayerDefinition,
  type OverlayFamily,
  type VectorFeature,
  type VectorLayerDefinition,
  type VectorLayerStyle,
} from "../native-vector/types.ts";

export type { OverlayFamily };

export type OverlayDrawTool = "select" | "freehand" | "polygon" | "landmass";

export const OVERLAY_FAMILIES: readonly OverlayFamily[] = [
  "political",
  "cultural",
  "religious",
  "linguistic",
  "economic",
  "military",
  "custom",
];

const FAMILY_LABELS: Record<OverlayFamily, string> = {
  political: "Political",
  cultural: "Cultural",
  religious: "Religious",
  linguistic: "Language",
  economic: "Economic",
  military: "Military",
  custom: "Custom",
};

const FAMILY_STYLES: Record<OverlayFamily, VectorLayerStyle> = {
  political: { ...DEFAULT_VECTOR_LAYER_STYLE, fill: "#b03030", fillOpacity: 0.38, stroke: "#5c1818", strokeWidth: 1.6 },
  cultural: { ...DEFAULT_VECTOR_LAYER_STYLE, fill: "#6b4c9a", fillOpacity: 0.32, stroke: "#3d2a5c" },
  religious: { ...DEFAULT_VECTOR_LAYER_STYLE, fill: "#c4a35a", fillOpacity: 0.32, stroke: "#7a6228" },
  linguistic: { ...DEFAULT_VECTOR_LAYER_STYLE, fill: "#2a6f7f", fillOpacity: 0.32, stroke: "#164650" },
  economic: { ...DEFAULT_VECTOR_LAYER_STYLE, fill: "#3d7a4a", fillOpacity: 0.32, stroke: "#21522c" },
  military: { ...DEFAULT_VECTOR_LAYER_STYLE, fill: "#5c4a3a", fillOpacity: 0.34, stroke: "#2f241c" },
  custom: { ...DEFAULT_VECTOR_LAYER_STYLE },
};

export type AtlasOverlayAuthoring = {
  layers: VectorLayerDefinition[];
  features: VectorFeature[];
  activeLayerId: string | null;
  dirty: boolean;
  canUndo: boolean;
  canRedo: boolean;
  busy: boolean;
  createLayer: (name?: string) => string | null;
  setActiveLayer: (id: string) => void;
  renameLayer: (id: string, name: string) => void;
  deleteLayer: (id: string) => void;
  setVisible: (id: string, visible: boolean) => void;
  updateLayerStyle: (id: string, patch: Partial<VectorLayerStyle>) => void;
  updateLayerOpacity: (id: string, opacity: number) => void;
  addFeature: (feature: VectorFeature) => void;
  routeFeatures: VectorFeature[];
  addRoute: (feature: VectorFeature) => string | null;
  renameFeature: (id: string, name: string | null) => void;
  updateFeatureStyle: (id: string, patch: Partial<VectorLayerStyle>) => void;
  deleteFeatures: (ids: string[]) => void;
  undo: () => void;
  redo: () => void;
  save: () => Promise<void>;
};

export function overlayFamilyLabel(family: OverlayFamily): string {
  return FAMILY_LABELS[family];
}

export function overlayFamilyStyle(family: OverlayFamily): VectorLayerStyle {
  return { ...FAMILY_STYLES[family] };
}

export function overlayLayerFamily(layer: VectorLayerDefinition): OverlayFamily {
  return layer.overlayFamily ?? "custom";
}

export function isOverlayLayer(layer: MapLayerDefinition): layer is VectorLayerDefinition {
  return layer.kind === "vector" && Boolean(layer.overlayFamily);
}

export function sortLayersForBook<T extends { id: string; order: number }>(layers: readonly T[]): T[] {
  return [...layers].sort((left, right) => right.order - left.order || left.id.localeCompare(right.id));
}

export function layerBookGroups(layers: readonly MapLayerDefinition[]): {
  overlays: VectorLayerDefinition[];
  base: MapLayerDefinition[];
} {
  const sorted = sortLayersForBook(layers);
  return {
    overlays: sorted.filter(isOverlayLayer),
    base: sorted.filter((layer) => !isOverlayLayer(layer)),
  };
}

export function reorderLayerBookIds(
  layers: readonly MapLayerDefinition[],
  sourceId: string,
  targetId: string,
): string[] | null {
  const source = layers.find((layer) => layer.id === sourceId);
  const target = layers.find((layer) => layer.id === targetId);
  if (!source || !target || isOverlayLayer(source) !== isOverlayLayer(target)) return null;
  const { overlays, base } = layerBookGroups(layers);
  const book = [...(isOverlayLayer(source) ? overlays : base)];
  const from = book.findIndex((layer) => layer.id === sourceId);
  const to = book.findIndex((layer) => layer.id === targetId);
  if (from < 0 || to < 0) return null;
  const [moved] = book.splice(from, 1);
  book.splice(to, 0, moved);
  const display = isOverlayLayer(source) ? [...book, ...base] : [...overlays, ...book];
  return display.reverse().map((layer) => layer.id);
}

export function overlayFamilyFromName(name: string): OverlayFamily {
  const key = name.trim().toLowerCase();
  if (!key) return "custom";
  for (const family of OVERLAY_FAMILIES) {
    if (family === key || FAMILY_LABELS[family].toLowerCase() === key) return family;
  }
  return "custom";
}

export function uniqueOverlayLayerName(layers: readonly { name: string }[], desired: string): string {
  const base = desired.trim() || "Overlay";
  const used = new Set(layers.map((layer) => layer.name));
  if (!used.has(base)) return base;
  let index = 2;
  while (used.has(`${base} ${index}`)) index += 1;
  return `${base} ${index}`;
}

export function overlayNameSuggestions(query = ""): { family: OverlayFamily; label: string }[] {
  const needle = query.trim().toLowerCase();
  return OVERLAY_FAMILIES.filter((family) => family !== "custom")
    .map((family) => ({ family, label: FAMILY_LABELS[family] }))
    .filter((item) => !needle || item.label.toLowerCase().includes(needle) || item.family.includes(needle));
}

export function nextOverlayLayerName(layers: readonly { name: string }[], family: OverlayFamily): string {
  return uniqueOverlayLayerName(layers, FAMILY_LABELS[family]);
}

export function overlayFeaturesForLayer(features: readonly VectorFeature[], layerId: string): VectorFeature[] {
  return features.filter((feature) => feature.properties.daena.layerId === layerId);
}
