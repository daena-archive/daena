import Collection from "ol/Collection.js";
import ImageLayer from "ol/layer/Image.js";
import LayerGroup from "ol/layer/Group.js";
import ImageStatic from "ol/source/ImageStatic.js";
import type Projection from "ol/proj/Projection.js";
import type { MapCoordinateSpace } from "../../../../packages/plugin-sdk/src/maps";
import { authoredExtentToViewExtent } from "../editor/coordinate-space";

export type RuntimeBackground = {
  id: string;
  url: string;
  width?: number;
  height?: number;
  canvas?: HTMLCanvasElement;
  extent: readonly [number, number, number, number];
  visible: boolean;
  locked: boolean;
  opacity: number;
  order: number;
};

export type BackgroundRegistry = {
  group: LayerGroup;
  sync: (backgrounds: readonly RuntimeBackground[], space: MapCoordinateSpace, projection: Projection) => void;
  dispose: () => void;
};

function sourceUrl(background: RuntimeBackground, onError?: (detail: string) => void): string | null {
  if (background.canvas) {
    const cached = canvasDataUrls.get(background.canvas);
    if (cached) return cached;
    try {
      const url = background.canvas.toDataURL("image/png");
      canvasDataUrls.set(background.canvas, url);
      return url;
    } catch (cause) {
      onError?.(cause instanceof Error ? cause.message : "OpenLayers could not read the raster canvas.");
      return null;
    }
  }
  return background.url || null;
}

const canvasDataUrls = new WeakMap<HTMLCanvasElement, string>();

export function createBackgroundRegistry(onError?: (detail: string) => void): BackgroundRegistry {
  const group = new LayerGroup({ layers: [] });
  const layers = new Map<string, ImageLayer<any>>();

  const disposeLayers = () => {
    for (const layer of layers.values()) {
      layer.setSource(null);
      group.getLayers().remove(layer);
    }
    layers.clear();
  };

  return {
    group,
    sync(backgrounds, space, projection) {
      const ordered = [...backgrounds].sort(
        (left, right) => left.order - right.order || left.id.localeCompare(right.id),
      );
      const keep = new Set(ordered.map((background) => background.id));
      for (const [id, layer] of layers) {
        if (!keep.has(id)) {
          layer.setSource(null);
          group.getLayers().remove(layer);
          layers.delete(id);
        }
      }
      const nextLayers = ordered.flatMap((background, index) => {
        const url = sourceUrl(background, onError);
        if (!url) return [];
        let layer = layers.get(background.id);
        if (!layer) {
          layer = new ImageLayer({ visible: background.visible });
          layers.set(background.id, layer);
          group.getLayers().push(layer);
        }
        layer.setVisible(background.visible);
        layer.setOpacity(Math.max(0, Math.min(1, background.opacity)));
        layer.setZIndex(index);
        layer.set("locked", background.locked);
        layer.setSource(
          new ImageStatic({
            url,
            projection,
            imageExtent: authoredExtentToViewExtent(background.extent, space),
            interpolate: true,
          }),
        );
        return [layer];
      });
      group.setLayers(new Collection(nextLayers));
    },
    dispose() {
      disposeLayers();
    },
  };
}
