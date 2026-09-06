import difference from "@turf/difference";
import { feature, featureCollection, multiPolygon, polygon } from "@turf/helpers";
import union from "@turf/union";
import type { AtlasRegionProposal } from "../../project/types.ts";
import { daenaProperties, type VectorFeature } from "../native-vector/types.ts";
import { formatEpoch } from "../native-vector/epoch-utils.ts";

export type RegionGeometry = AtlasRegionProposal["geometry"];
export type CombineMode = "replace" | "add" | "subtract";

export type LandmassSelection = {
  detector: string;
  label: string;
  epochDependent: boolean;
  epochOffsetYears: number;
  regionIds: number[];
  cellCount: number;
  geometry: RegionGeometry;
};

type Position = number[];
type Polygon = { type: "Polygon"; coordinates: Position[][] };
type MultiPolygon = { type: "MultiPolygon"; coordinates: Position[][][] };
type TurfFeature = { type: "Feature"; geometry: Polygon | MultiPolygon; properties: Record<string, never> };

const MICRO_SCALE = 1_000_000;

export function selectionFromProposal(proposal: AtlasRegionProposal, epochOffsetYears: number): LandmassSelection {
  return {
    detector: proposal.detector,
    label: proposal.label,
    epochDependent: proposal.epochDependent,
    epochOffsetYears,
    regionIds: proposal.detector === "land" ? [] : [proposal.regionId],
    cellCount: proposal.cellCount,
    geometry: proposal.geometry,
  };
}

export function combineSelection(
  current: LandmassSelection | null,
  proposal: AtlasRegionProposal,
  epochOffsetYears: number,
  mode: CombineMode,
): { ok: true; selection: LandmassSelection | null } | { ok: false; detail: string } {
  const incoming = selectionFromProposal(proposal, epochOffsetYears);
  if (!current || mode === "replace") return { ok: true, selection: incoming };
  if (current.epochOffsetYears !== epochOffsetYears) {
    return { ok: false, detail: "Clear the selection before mixing landmasses from another epoch." };
  }
  const geometry =
    mode === "add"
      ? unionGeometries(current.geometry, incoming.geometry)
      : differenceGeometries(current.geometry, incoming.geometry);
  if (mode === "subtract" && !geometry) return { ok: true, selection: null };
  if (!geometry) return { ok: false, detail: mode === "add" ? "Could not add that landmass." : "Nothing remained." };
  const regionIds =
    mode === "add"
      ? [...new Set([...current.regionIds, ...incoming.regionIds])]
      : current.regionIds.filter((id) => !incoming.regionIds.includes(id));
  return {
    ok: true,
    selection: {
      detector: current.detector === incoming.detector ? current.detector : "combined",
      label: mode === "add" ? combinedLabel(current, incoming) : `${current.label} minus ${incoming.label}`,
      epochDependent: true,
      epochOffsetYears,
      regionIds,
      cellCount:
        mode === "add" ? current.cellCount + incoming.cellCount : Math.max(0, current.cellCount - incoming.cellCount),
      geometry,
    },
  };
}

export function invertSelection(
  current: LandmassSelection,
  land: AtlasRegionProposal,
): { ok: true; selection: LandmassSelection | null } | { ok: false; detail: string } {
  const geometry = differenceGeometries(land.geometry, current.geometry);
  if (!geometry) return { ok: true, selection: null };
  return {
    ok: true,
    selection: {
      detector: "land",
      label: "Land except selection",
      epochDependent: true,
      epochOffsetYears: current.epochOffsetYears,
      regionIds: [],
      cellCount: Math.max(0, land.cellCount - current.cellCount),
      geometry,
    },
  };
}

export function featureFromSelection(selection: LandmassSelection, layerId: string): VectorFeature {
  const properties = daenaProperties(layerId, "region", selection.label);
  properties.daena.custom = {
    detector: selection.detector,
    epochDependent: false,
    capturedOffsetYears: selection.epochOffsetYears,
    cellCount: selection.cellCount,
  };
  return {
    type: "Feature",
    id: crypto.randomUUID(),
    properties,
    geometry: selection.geometry,
  };
}

export const HOVER_PREVIEW_ID = "landmass-hover-preview";
export const LANDMASS_HOVER_DELAY_MS = 240;

export function previewFeature(selection: LandmassSelection): VectorFeature {
  return {
    type: "Feature",
    id: "landmass-selection-preview",
    properties: daenaProperties("preview", "region", selection.label),
    geometry: selection.geometry,
  };
}

export function hoverPreviewFeature(selection: LandmassSelection): VectorFeature {
  return {
    type: "Feature",
    id: HOVER_PREVIEW_ID,
    properties: daenaProperties("preview", "region", selection.label),
    geometry: selection.geometry,
  };
}

export function geometryContainsPoint(geometry: RegionGeometry, longitude: number, latitude: number): boolean {
  if (geometry.type === "Polygon") return polygonContainsPoint(geometry.coordinates, longitude, latitude);
  if (geometry.type === "MultiPolygon") {
    return geometry.coordinates.some((polygon) => polygonContainsPoint(polygon, longitude, latitude));
  }
  return false;
}

export function hoverCoversPoint(selection: LandmassSelection | null, longitude: number, latitude: number): boolean {
  return Boolean(selection && geometryContainsPoint(selection.geometry, longitude, latitude));
}

export function hoverIsRedundant(hover: LandmassSelection, selection: LandmassSelection | null): boolean {
  return Boolean(
    selection && hover.regionIds.length > 0 && hover.regionIds.every((id) => selection.regionIds.includes(id)),
  );
}

export function featureRegionGeometry(feature: VectorFeature): RegionGeometry | null {
  const geometry = feature.geometry;
  if (geometry.type === "Polygon" || geometry.type === "MultiPolygon") return geometry;
  return null;
}

export function occupiedGeometries(features: readonly VectorFeature[]): RegionGeometry[] {
  return features.flatMap((feature) => {
    const geometry = featureRegionGeometry(feature);
    return geometry ? [geometry] : [];
  });
}

export function leftoverGeometry(geometry: RegionGeometry, occupied: readonly RegionGeometry[]): RegionGeometry | null {
  if (occupied.length === 0) return geometry;
  let remaining: RegionGeometry | null = geometry;
  for (const cutter of occupied) {
    if (!remaining) return null;
    remaining = differenceGeometries(remaining, cutter);
  }
  return remaining;
}

export function displayLandmassGeometry(
  selection: LandmassSelection,
  occupied: readonly RegionGeometry[],
  includeOccupied: boolean,
): RegionGeometry | null {
  if (includeOccupied) return selection.geometry;
  return leftoverGeometry(selection.geometry, occupied);
}

export function selectionMinusOccupied(
  selection: LandmassSelection,
  occupied: readonly RegionGeometry[],
  includeOccupied: boolean,
): { ok: true; selection: LandmassSelection } | { ok: false; detail: string } {
  if (includeOccupied || occupied.length === 0) return { ok: true, selection };
  const geometry = leftoverGeometry(selection.geometry, occupied);
  if (!geometry) {
    return { ok: false, detail: "That landmass is already covered on this overlay." };
  }
  return { ok: true, selection: { ...selection, geometry } };
}

export function withGeometry(selection: LandmassSelection, geometry: RegionGeometry): LandmassSelection {
  return { ...selection, geometry };
}

export function epochNotice(offsetYears: number): string {
  return `This landmass is for ${formatEpoch(offsetYears).toLowerCase()}. Saving captures the current geometry; later sea-level changes will not rewrite it.`;
}

export function combineModeFromModifiers(shiftKey: boolean, altKey: boolean): CombineMode {
  if (altKey) return "subtract";
  if (shiftKey) return "add";
  return "replace";
}

function combinedLabel(current: LandmassSelection, incoming: LandmassSelection): string {
  if (current.regionIds.length + incoming.regionIds.length <= 1) return incoming.label;
  return `${current.regionIds.length + incoming.regionIds.length} landmasses`;
}

function unionGeometries(left: RegionGeometry, right: RegionGeometry): RegionGeometry | null {
  try {
    const features = [toTurf(left), toTurf(right)].filter((item): item is TurfFeature => Boolean(item));
    if (features.length < 2) return features[0] ? fromTurf(features[0]) : null;
    return fromTurf(union(featureCollection(features)) as TurfFeature | null);
  } catch {
    return null;
  }
}

function differenceGeometries(base: RegionGeometry, cutter: RegionGeometry): RegionGeometry | null {
  try {
    const features = [toTurf(base), toTurf(cutter)].filter((item): item is TurfFeature => Boolean(item));
    if (features.length < 2) return null;
    return fromTurf(difference(featureCollection(features)) as TurfFeature | null);
  } catch {
    return null;
  }
}

function toTurf(geometry: RegionGeometry): TurfFeature | null {
  if (geometry.type === "Polygon") return polygon(geometry.coordinates.map(closeRing) as Position[][]) as TurfFeature;
  if (geometry.type === "MultiPolygon") {
    return multiPolygon(geometry.coordinates.map((poly) => poly.map(closeRing)) as Position[][][]) as TurfFeature;
  }
  return feature(geometry) as TurfFeature;
}

function fromTurf(result: TurfFeature | null): RegionGeometry | null {
  const geometry = result?.geometry;
  if (!geometry) return null;
  if (geometry.type === "Polygon") {
    const rings = geometry.coordinates.map((ring) => orientExterior(ring.map(roundPosition)));
    if (rings.length === 0 || rings[0].length < 4) return null;
    return { type: "Polygon", coordinates: rings };
  }
  if (geometry.type === "MultiPolygon") {
    const polygons = geometry.coordinates
      .map((poly) => poly.map((ring) => orientExterior(ring.map(roundPosition))))
      .filter((poly) => poly[0] && poly[0].length >= 4);
    if (polygons.length === 0) return null;
    return { type: "MultiPolygon", coordinates: polygons };
  }
  return null;
}

function ringArea(ring: Position[]): number {
  let sum = 0;
  for (let index = 0; index < ring.length - 1; index += 1) {
    const current = ring[index];
    const next = ring[index + 1];
    sum += current[0] * next[1] - next[0] * current[1];
  }
  return sum;
}

function orientExterior(ring: Position[]): Position[] {
  const closed = closeRing(ring);
  if (closed.length < 4) return closed;
  return ringArea(closed) < 0 ? [...closed].reverse() : closed;
}

function roundPosition(position: Position): Position {
  return [Math.round(position[0] * MICRO_SCALE) / MICRO_SCALE, Math.round(position[1] * MICRO_SCALE) / MICRO_SCALE];
}

function closeRing(ring: Position[]): Position[] {
  if (ring.length < 3) return ring;
  const first = ring[0];
  const last = ring[ring.length - 1];
  if (first[0] === last[0] && first[1] === last[1]) return ring;
  return [...ring, [...first]];
}

function polygonContainsPoint(rings: Position[][], longitude: number, latitude: number): boolean {
  const exterior = rings[0];
  if (!exterior || !ringContainsPoint(exterior, longitude, latitude)) return false;
  return rings.slice(1).every((hole) => !ringContainsPoint(hole, longitude, latitude));
}

function ringContainsPoint(ring: Position[], longitude: number, latitude: number): boolean {
  const closed = closeRing(ring);
  let inside = false;
  for (let index = 0; index < closed.length - 1; index += 1) {
    const current = closed[index];
    const next = closed[index + 1];
    const crosses =
      current[1] > latitude !== next[1] > latitude &&
      longitude < ((next[0] - current[0]) * (latitude - current[1])) / (next[1] - current[1]) + current[0];
    if (crosses) inside = !inside;
  }
  return inside;
}
