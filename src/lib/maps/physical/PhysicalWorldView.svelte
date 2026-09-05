<script lang="ts">
import { untrack } from "svelte";
import type { MapAnchor } from "../../../../packages/plugin-sdk/src/maps";
import { RENDERER_UNAVAILABLE, createMapAdapter, type MapAdapter, type MapAdapterView } from "../openlayers/MapAdapter";
import type { RuntimeBackground } from "../openlayers/background-registry";
import { PHYSICAL_COORDINATE_SPACE, extentOf } from "../editor/coordinate-space";
import type { MapLayerDefinition, VectorFeatureCollection } from "../native-vector/types";
import MapViewControls from "../native-vector/MapViewControls.svelte";

let {
  collection,
  layers,
  raster,
  showRaster = true,
  pickArmed = false,
  onMapPick,
  onready,
}: {
  collection: VectorFeatureCollection;
  layers: MapLayerDefinition[];
  raster: HTMLCanvasElement | null;
  showRaster?: boolean;
  pickArmed?: boolean;
  onMapPick?: (anchor: MapAnchor) => void;
  onready?: (editor: MapAdapter | null) => void;
} = $props();

let host = $state<HTMLDivElement | null>(null);
let notice = $state("");
let editor = $state<MapAdapter | null>(null);
let view = $state<MapAdapterView | null>(null);

function backgroundFrom(canvas: HTMLCanvasElement | null): RuntimeBackground | null {
  return canvas
    ? {
        id: "physical",
        url: "",
        canvas,
        width: canvas.width,
        height: canvas.height,
        extent: extentOf(PHYSICAL_COORDINATE_SPACE),
        visible: true,
        locked: true,
        opacity: 1,
        order: 0,
      }
    : null;
}

$effect(() => {
  const container = host;
  if (!container) return;
  const created = untrack(() =>
    createMapAdapter(container, {
      get draft() {
        return collection;
      },
      get layers() {
        return layers;
      },
      activeLayerId: null,
      coordinateSpace: PHYSICAL_COORDINATE_SPACE,
      setActiveLayerId() {},
      onDiagnostic(code, detail) {
        if (code === RENDERER_UNAVAILABLE) notice = detail;
      },
      get pickArmed() {
        return pickArmed;
      },
      onMapPick(anchor) {
        onMapPick?.(anchor);
      },
      get backgrounds() {
        const next = backgroundFrom(raster);
        return next ? [next] : [];
      },
      initialView: view ?? { center: [0, 0], zoom: 1, rotation: 0 },
      onViewChange(next) {
        view = next;
      },
      readOnly: true,
      labelsVisible: false,
    }),
  );
  if ("error" in created) {
    notice = created.detail;
    onready?.(null);
    return;
  }
  editor = created;
  created.setMode("static");
  onready?.(created);
  return () => {
    created.dispose();
    if (editor === created) editor = null;
    onready?.(null);
  };
});

$effect(() => {
  const current = editor;
  if (!current) return;
  layers;
  collection;
  current.syncDocument(collection, layers);
});

$effect(() => {
  const next = backgroundFrom(raster);
  editor?.syncBackgrounds(next ? [next] : []);
});

$effect(() => {
  editor?.setBackgroundVisible(showRaster);
});
</script>

<div class="frame">
  {#if notice}<p class="map-reconcile-notice" role="alert">{notice}</p>{/if}
  <div class="viewport" bind:this={host} role="img" aria-label="Physical world map"></div>
  <MapViewControls
    zoom={view?.zoom ?? 1}
    min={0}
    max={8}
    onzoom={(zoom) => editor?.setZoom(zoom)}
    onpan={(longitude, latitude) => editor?.panBy(longitude, latitude)} />
</div>

<style>
.frame {
  position: relative;
  display: flex;
  width: 100%;
  height: 100%;
  min-height: 360px;
  min-width: 0;
  flex: 1;
  background: #0d1b2a;
}
.viewport {
  flex: 1;
  min-width: 0;
  min-height: 0;
}
.viewport :global(.ol-viewport) {
  width: 100%;
  height: 100%;
}
.map-reconcile-notice {
  position: absolute;
  z-index: 1;
  margin: 0;
  padding: 0.55rem 1rem;
}
</style>
