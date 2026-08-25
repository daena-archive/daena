import ImageLayer from "ol/layer/Image.js";
import ImageStatic from "ol/source/ImageStatic.js";
import {
  imageOverlayCoordinates,
  type ImageOverlayCoordinates,
} from "../native-vector/coordinates";
import { worldProjection } from "./projection";

export type MapBackground = {
  url: string;
  width: number;
  height: number;
  canvas?: HTMLCanvasElement;
  coordinates?: ImageOverlayCoordinates;
};

export function extentFromCoordinates(coordinates: ImageOverlayCoordinates): [number, number, number, number] {
  const xs = coordinates.map((coordinate) => coordinate[0]);
  const ys = coordinates.map((coordinate) => coordinate[1]);
  return [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
}

export type BackgroundRegistry = {
  layer: ImageLayer<any>;
  current: MapBackground | null;
  setBackground: (background: MapBackground | null) => void;
  setVisible: (visible: boolean) => void;
};

export function createBackgroundRegistry(onError?: (detail: string) => void): BackgroundRegistry {
  const layer = new ImageLayer<any>({ visible: true });
  let current: MapBackground | null = null;

  const update = () => {
    if (!current) {
      layer.setSource(null);
      return;
    }
    const coordinates = current.coordinates ?? imageOverlayCoordinates(current.width, current.height);
    let url = current.url;
    if (current.canvas) {
      try {
        url = current.canvas.toDataURL("image/png");
      } catch (cause) {
        onError?.(cause instanceof Error ? cause.message : "OpenLayers could not read the raster canvas.");
        return;
      }
    }
    layer.setSource(
      new ImageStatic({
        url,
        projection: worldProjection,
        imageExtent: extentFromCoordinates(coordinates),
        interpolate: true,
      }),
    );
  };

  return {
    layer,
    get current() {
      return current;
    },
    set current(value) {
      current = value;
    },
    setBackground(background) {
      current = background;
      update();
    },
    setVisible(visible) {
      layer.setVisible(visible);
    },
  };
}
