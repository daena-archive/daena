declare module "*.geojson" {
  import type { VectorFeatureCollection } from "./types";
  const fixture: VectorFeatureCollection;
  export default fixture;
}

declare module "maplibre-gl/dist/maplibre-gl-csp.js" {
  import * as MapLibre from "maplibre-gl";
  const maplibregl: typeof MapLibre;
  export default maplibregl;
}

declare module "maplibre-gl/dist/maplibre-gl-csp-worker.js?url" {
  const workerUrl: string;
  export default workerUrl;
}
