import type { RouteSuggestion } from "$lib/project/types";
import { daenaProperties, featureSemanticType, type VectorFeature } from "../native-vector/types.ts";

export function toMicrodegrees(lon: number, lat: number): [number, number] {
  return [Math.round(lon * 1_000_000), Math.round(lat * 1_000_000)];
}

export function routeEndpoints(suggestion: RouteSuggestion): { start: [number, number]; end: [number, number] } | null {
  const first = suggestion.coordinates[0]?.[0];
  const lastLine = suggestion.coordinates[suggestion.coordinates.length - 1];
  const last = lastLine?.[lastLine.length - 1];
  if (!first || !last) return null;
  return {
    start: [first[0] / 1_000_000, first[1] / 1_000_000],
    end: [last[0] / 1_000_000, last[1] / 1_000_000],
  };
}

export function routeGeometryFromSuggestion(suggestion: RouteSuggestion): VectorFeature["geometry"] | null {
  const parts = suggestion.coordinates.map((line) => line.map((point) => [point[0] / 1_000_000, point[1] / 1_000_000]));
  if (parts.length === 0 || parts.some((line) => line.length < 2)) return null;
  if (parts.length === 1) return { type: "LineString", coordinates: parts[0] };
  return { type: "MultiLineString", coordinates: parts };
}

export function routeFeatureFromSuggestion(
  suggestion: RouteSuggestion,
  epochOffsetYears: number,
  layerId: string,
): VectorFeature | null {
  const geometry = routeGeometryFromSuggestion(suggestion);
  if (!geometry) return null;
  const properties = daenaProperties(layerId, "route", suggestion.label);
  properties.daena.custom = {
    routingStrategy: suggestion.strategy,
    capturedOffsetYears: epochOffsetYears,
    lengthM: suggestion.lengthM,
    climbM: suggestion.climbM,
  };
  return {
    type: "Feature",
    id: crypto.randomUUID(),
    properties,
    geometry,
  };
}

export function existingRoutePolylines(features: readonly VectorFeature[]): number[][][] {
  const lines: number[][][] = [];
  for (const feature of features) {
    if (featureSemanticType(feature) !== "route") continue;
    const geometry = feature.geometry;
    if (geometry.type === "LineString") {
      lines.push(geometry.coordinates.map(([lon, lat]) => toMicrodegrees(lon, lat)));
    } else if (geometry.type === "MultiLineString") {
      for (const line of geometry.coordinates) {
        lines.push(line.map(([lon, lat]) => toMicrodegrees(lon, lat)));
      }
    }
  }
  return lines;
}
