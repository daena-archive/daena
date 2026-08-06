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
  sourceAssetId: string;
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

export interface MapNavigationService {
  openMap(input: { mapEntityId: string; linkId?: string; mode?: "view" | "edit" }): Promise<void>;
  focusEntity(input: { entityId: string; mapEntityId?: string }): Promise<void>;
  setDate(input: { date: MapDate | null }): Promise<void>;
  showResults(input: { entityIds: readonly string[]; mapEntityId?: string }): Promise<void>;
  listLocations(input: { entityId: string }): Promise<readonly MapLocationReference[]>;
}
