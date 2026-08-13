import type { VectorFeatureCollection, VectorLayerDefinition } from "./types";
import fixture from "../../../../docs/maps/native-vector-fixtures/phase0-land.geojson";

export const PHASE0_VECTOR_LAYERS: VectorLayerDefinition[] = [
  {
    id: "018f89ec-25fc-7816-8b47-6f80905f2869",
    kind: "vector",
    name: "Routes and regions",
    order: 10,
    defaultVisible: true,
    locked: false,
    selector: {},
    style: {
      fill: "#8f6fd1",
      fillOpacity: 0.35,
      stroke: "#5e4893",
      strokeWidth: 1.5,
      pointRadius: 5,
    },
  },
  {
    id: "018f89ec-25fc-7816-8b47-6f80905f286a",
    kind: "vector",
    name: "Markers",
    order: 20,
    defaultVisible: true,
    locked: false,
    selector: {},
    style: {
      fill: "#d5ab6c",
      fillOpacity: 1,
      stroke: "#8a7048",
      strokeWidth: 1.5,
      pointRadius: 6,
    },
  },
];

export function phase0Fixture(): VectorFeatureCollection {
  return structuredClone(fixture) as VectorFeatureCollection;
}
