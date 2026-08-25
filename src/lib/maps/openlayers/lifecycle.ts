import type Map from "ol/Map.js";

const liveMaps = new Set<Map>();

export function liveOpenLayersMapCount(): number {
  return liveMaps.size;
}

export type MapLifecycle = {
  resize: () => void;
  dispose: () => void;
};

/** Resize observer, updateSize, dispose, and live-instance tracking for any OpenLayers map. */
export function bindMapLifecycle(map: Map, container: HTMLElement, onResize?: () => void): MapLifecycle {
  let disposed = false;
  liveMaps.add(map);
  const resize = () => {
    if (disposed) return;
    onResize?.();
    if (container.clientWidth > 0 && container.clientHeight > 0) map.updateSize();
  };
  const observer = new ResizeObserver(resize);
  observer.observe(container);
  requestAnimationFrame(resize);
  return {
    resize,
    dispose() {
      if (disposed) return;
      disposed = true;
      observer.disconnect();
      map.setTarget(undefined);
      map.dispose();
      liveMaps.delete(map);
    },
  };
}
