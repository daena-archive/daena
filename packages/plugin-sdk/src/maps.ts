/** Provider-neutral Maps domain types. Provider-specific selectors stay opaque. */
export type NormalizedPoint = readonly [number, number];

export type MapAnchor =
  | { kind: "point"; point: NormalizedPoint }
  | { kind: "provider-feature"; provider: string; featureKind: string; featureId: string; fallbackPoint: NormalizedPoint }
  | { kind: "path"; points: readonly NormalizedPoint[] }
  | { kind: "area"; rings: readonly (readonly NormalizedPoint[])[] };

export interface MapDescriptor {
  schemaVersion: 1;
  provider: { id: string; adapterVersion: number; sourceFormat: string };
  sourceAssetId: string | null;
  previewAssetId: string | null;
  defaultView: { center: NormalizedPoint; zoom: number };
}

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
  validity: { from: MapDate | null; to: MapDate | null };
}

export interface MapLocationsField {
  schemaVersion: 1;
  locations: readonly MapLocationReference[];
}

export interface MapLayerDefinition {
  id: string;
  name: string;
  order: number;
  defaultVisible: boolean;
  style: Readonly<Record<string, unknown>>;
  selector: Readonly<Record<string, unknown>>;
}

export interface MapLayersField {
  schemaVersion: 1;
  layers: readonly MapLayerDefinition[];
}

export type MapFocusResult =
  | { status: "focused"; mapEntityId: string; linkId: string | null }
  | { status: "multiple-links"; locations: readonly MapLocationReference[] };

export type MapOpenResult = { mapEntityId: string; linkId: string | null };

export type MapShowResultsResult = { mapEntityId: string; locations: readonly MapLocationReference[] };

export interface MapNavigationService {
  openMap(input: { mapEntityId: string; linkId?: string; mode?: "view" | "edit" }): Promise<MapOpenResult>;
  focusEntity(input: { entityId: string; mapEntityId?: string }): Promise<MapFocusResult>;
  setDate(input: { date: MapDate | null }): Promise<{ accepted: boolean; date: MapDate | null }>;
  showResults(input: { entityIds: readonly string[]; mapEntityId?: string }): Promise<MapShowResultsResult>;
  listLocations(input: { entityId: string }): Promise<readonly MapLocationReference[]>;
}

/** Shell-side location mutations available to module contexts (not part of
 * the public navigation service contract; both are revision-aware). */
export interface MapLocationsMutations {
  upsertLocation(input: { entityId: string; location: MapLocationReference }): Promise<void>;
  unlinkLocation(input: { entityId: string; locationId: string }): Promise<void>;
}

/** Payload of `daena.maps/selection@1`, forwarded to the shell as the
 * `maps-selection` Tauri event. `anchor` is null when nothing is capturable. */
export interface MapSelectionEvent {
  mapEntityId: string;
  anchor: MapAnchor | null;
}
