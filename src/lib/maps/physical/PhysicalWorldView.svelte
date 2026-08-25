<script lang="ts">
import { untrack } from "svelte";
import type { MapAnchor } from "../../../../packages/plugin-sdk/src/maps";
import {
  RENDERER_UNAVAILABLE,
  createNativeVectorEditor,
  type NativeVectorEditor,
  type NativeVectorView,
} from "../native-vector/runtime";
import { physicalWorldOverlayCoordinates } from "../native-vector/coordinates";
import type { VectorFeatureCollection, VectorLayerDefinition } from "../native-vector/types";
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
  layers: VectorLayerDefinition[];
  raster: HTMLCanvasElement | null;
  showRaster?: boolean;
  pickArmed?: boolean;
  onMapPick?: (anchor: MapAnchor) => void;
  onready?: (editor: NativeVectorEditor | null) => void;
} = $props();

let host = $state<HTMLDivElement | null>(null);
let notice = $state("");
let editor = $state<NativeVectorEditor | null>(null);
let view = $state<NativeVectorView | null>(null);

function backgroundFrom(canvas: HTMLCanvasElement | null) {
  return canvas
    ? {
        url: "",
        canvas,
        width: canvas.width,
        height: canvas.height,
        coordinates: physicalWorldOverlayCoordinates(),
      }
    : null;
}

$effect(() => {
  const container = host;
  if (!container) return;
  const created = untrack(() =>
    createNativeVectorEditor(container, {
      get draft() {
        return collection;
      },
      get layers() {
        return layers;
      },
      activeLayerId: null,
      center: [0.5, 0.5],
      zoom: 0,
      setDraft() {},
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
      background: backgroundFrom(raster),
      projection: "globe",
      initialView: view,
      onViewChange(next) {
        view = next;
      },
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
  current.syncLayers(layers);
});

$effect(() => {
  editor?.setBackground(backgroundFrom(raster));
});

$effect(() => {
  editor?.setBackgroundVisible(showRaster);
});
</script>

<div class="frame">
  {#if notice}<p class="map-reconcile-notice" role="alert">{notice}</p>{/if}
  <div class="viewport" bind:this={host} role="img" aria-label="Physical world map"></div>
  <MapViewControls
    zoom={view?.zoom ?? 0}
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
.viewport :global(.maplibregl-map) {
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
