<script lang="ts">
import { untrack } from "svelte";
import {
  RENDERER_UNAVAILABLE,
  createNativeVectorEditor,
  type NativeVectorEditor,
} from "../native-vector/runtime";
import { physicalWorldOverlayCoordinates } from "../native-vector/coordinates";
import type { VectorFeatureCollection, VectorLayerDefinition } from "../native-vector/types";

let {
  collection,
  layers,
  raster,
  showRaster = true,
}: {
  collection: VectorFeatureCollection;
  layers: VectorLayerDefinition[];
  raster: HTMLCanvasElement | null;
  showRaster?: boolean;
} = $props();

let host = $state<HTMLDivElement | null>(null);
let notice = $state("");
let editor = $state<NativeVectorEditor | null>(null);

$effect(() => {
  const container = host;
  const canvas = raster;
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
      zoom: 1,
      setDraft() {},
      setActiveLayerId() {},
      onDiagnostic(code, detail) {
        if (code === RENDERER_UNAVAILABLE) notice = detail;
      },
      background: canvas
        ? {
            url: "",
            canvas,
            width: canvas.width,
            height: canvas.height,
            coordinates: physicalWorldOverlayCoordinates(),
          }
        : null,
      projection: "globe",
    }),
  );
  if ("error" in created) {
    notice = created.detail;
    return;
  }
  editor = created;
  created.setMode("static");
  return () => {
    created.dispose();
    if (editor === created) editor = null;
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
  editor?.setBackgroundVisible(showRaster);
});
</script>

<div class="frame">
  {#if notice}<p class="map-reconcile-notice" role="alert">{notice}</p>{/if}
  <div class="viewport" bind:this={host} role="img" aria-label="Physical world map"></div>
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

