import type { MapBackgroundRef, MapCoordinateSpace } from "../../../../packages/plugin-sdk/src/maps.ts";
import { type VectorFeature } from "../native-vector/types.ts";
import {
  backgroundsFromDescriptor,
  flipYBackgrounds,
  flipYCollection,
  isOpenLayersDescriptor,
  patchOpenLayersDescriptor,
  type OpenLayersMapDescriptor,
} from "./coordinate-space.ts";
import { removeFeatures, replaceFeature, type MapDocument } from "./model.ts";
import type { MapCommand } from "./commands.ts";
import { openLayersDescriptor, withCollection, withDescriptor } from "./command-utils.ts";

export function addBackgroundCommand(background: MapBackgroundRef): MapCommand {
  return {
    kind: "AddBackground",
    label: "Add raster",
    apply(document) {
      const descriptor = openLayersDescriptor(document);
      if (!descriptor) return document;
      if (descriptor.backgrounds.some((item) => item.id === background.id)) return document;
      return withDescriptor(
        document,
        patchOpenLayersDescriptor(descriptor, {
          backgrounds: [...descriptor.backgrounds, background],
        }) as OpenLayersMapDescriptor,
      );
    },
    invert() {
      return removeBackgroundCommand(background.id, background);
    },
  };
}

export function removeBackgroundCommand(id: string, removed: MapBackgroundRef): MapCommand {
  return {
    kind: "RemoveBackground",
    label: "Remove raster",
    apply(document) {
      const descriptor = openLayersDescriptor(document);
      if (!descriptor) return document;
      return withDescriptor(
        document,
        patchOpenLayersDescriptor(descriptor, {
          backgrounds: descriptor.backgrounds.filter((item) => item.id !== id),
        }) as OpenLayersMapDescriptor,
      );
    },
    invert() {
      return addBackgroundCommand(removed);
    },
  };
}

export function replaceBackgroundCommand(id: string, next: MapBackgroundRef, previous: MapBackgroundRef): MapCommand {
  return {
    kind: "ReplaceBackground",
    label: "Replace raster",
    apply(document) {
      const descriptor = openLayersDescriptor(document);
      if (!descriptor) return document;
      return withDescriptor(
        document,
        patchOpenLayersDescriptor(descriptor, {
          backgrounds: descriptor.backgrounds.map((item) => (item.id === id ? next : item)),
        }) as OpenLayersMapDescriptor,
      );
    },
    invert() {
      return replaceBackgroundCommand(id, previous, next);
    },
  };
}

export function reorderBackgroundCommand(
  id: string,
  order: number,
  previousOrder: number,
  neighborId: string,
  neighborOrder: number,
  neighborPreviousOrder: number,
): MapCommand {
  return {
    kind: "ReorderBackground",
    label: "Reorder raster",
    apply(document) {
      const descriptor = openLayersDescriptor(document);
      if (!descriptor) return document;
      return withDescriptor(
        document,
        patchOpenLayersDescriptor(descriptor, {
          backgrounds: descriptor.backgrounds.map((item) => {
            if (item.id === id) return { ...item, order };
            if (item.id === neighborId) return { ...item, order: neighborOrder };
            return item;
          }),
        }) as OpenLayersMapDescriptor,
      );
    },
    invert() {
      return reorderBackgroundCommand(id, previousOrder, order, neighborId, neighborPreviousOrder, neighborOrder);
    },
  };
}

export function setBackgroundOpacityCommand(id: string, opacity: number, previous: number): MapCommand {
  return {
    kind: "SetBackgroundOpacity",
    label: "Raster opacity",
    coalesceKey: `background-opacity:${id}`,
    apply(document) {
      const descriptor = openLayersDescriptor(document);
      if (!descriptor) return document;
      return withDescriptor(
        document,
        patchOpenLayersDescriptor(descriptor, {
          backgrounds: descriptor.backgrounds.map((item) => (item.id === id ? { ...item, opacity } : item)),
        }) as OpenLayersMapDescriptor,
      );
    },
    invert() {
      return setBackgroundOpacityCommand(id, previous, opacity);
    },
  };
}

export function setBackgroundVisibilityCommand(id: string, visible: boolean, previous: boolean): MapCommand {
  return {
    kind: "SetBackgroundVisibility",
    label: visible ? "Show raster" : "Hide raster",
    apply(document) {
      const descriptor = openLayersDescriptor(document);
      if (!descriptor) return document;
      return withDescriptor(
        document,
        patchOpenLayersDescriptor(descriptor, {
          backgrounds: descriptor.backgrounds.map((item) => (item.id === id ? { ...item, visible } : item)),
        }) as OpenLayersMapDescriptor,
      );
    },
    invert() {
      return setBackgroundVisibilityCommand(id, previous, visible);
    },
  };
}

export function setDefaultViewCommand(
  next: { center: [number, number]; zoom: number; rotation: number },
  previous: { center: [number, number]; zoom: number; rotation: number },
): MapCommand {
  const stored = {
    center: next.center,
    zoom: Math.max(next.zoom, 1e-6),
    rotation: next.rotation,
  };
  return {
    kind: "SetDefaultView",
    label: "Set view",
    coalesceKey: "default-view",
    apply(document) {
      const descriptor = openLayersDescriptor(document);
      if (!descriptor) return document;
      return withDescriptor(
        document,
        patchOpenLayersDescriptor(descriptor, {
          defaultView: stored,
        }) as OpenLayersMapDescriptor,
      );
    },
    invert() {
      return setDefaultViewCommand(previous, next);
    },
  };
}

export function setCoordinateSpaceCommand(
  next: MapCoordinateSpace,
  previous: MapCoordinateSpace,
  nextCollection: MapDocument["collection"] | null,
  previousCollection: MapDocument["collection"] | null,
  nextBackgrounds: readonly MapBackgroundRef[] | null,
  previousBackgrounds: readonly MapBackgroundRef[] | null,
): MapCommand {
  return {
    kind: "SetCoordinateSpace",
    label: "Calibrate map",
    apply(document) {
      const descriptor = openLayersDescriptor(document);
      if (!descriptor) return document;
      let patched = withDescriptor(
        document,
        patchOpenLayersDescriptor(descriptor, {
          coordinateSpace: next,
          backgrounds: nextBackgrounds ?? descriptor.backgrounds,
        }) as OpenLayersMapDescriptor,
      );
      if (nextCollection) patched = withCollection(patched, nextCollection);
      return patched;
    },
    invert() {
      return setCoordinateSpaceCommand(
        previous,
        next,
        previousCollection,
        nextCollection,
        previousBackgrounds,
        nextBackgrounds,
      );
    },
  };
}

export function calibrateImageToWorld(
  document: MapDocument,
  metresPerUnit: number | null,
  label = "metres",
): MapCommand | null {
  const descriptor = openLayersDescriptor(document);
  if (!descriptor || descriptor.coordinateSpace.kind !== "image") return null;
  const previous = descriptor.coordinateSpace;
  const next: MapCoordinateSpace = {
    kind: "world",
    extent: previous.extent,
    origin: "bottom-left",
    units: { id: "metre", label, metresPerUnit },
    wrapX: false,
  };
  return setCoordinateSpaceCommand(
    next,
    previous,
    flipYCollection(document.collection, previous),
    document.collection,
    flipYBackgrounds(descriptor.backgrounds, previous),
    descriptor.backgrounds,
  );
}

export function calibrateWorldUnits(
  document: MapDocument,
  metresPerUnit: number | null,
  label?: string,
): MapCommand | null {
  const descriptor = openLayersDescriptor(document);
  if (!descriptor || descriptor.coordinateSpace.kind !== "world") return null;
  const previous = descriptor.coordinateSpace;
  const next: MapCoordinateSpace = {
    ...previous,
    units: {
      id: previous.units.id,
      label: label ?? previous.units.label,
      metresPerUnit,
    },
  };
  return setCoordinateSpaceCommand(next, previous, null, null, null, null);
}

export function listedBackgrounds(document: MapDocument): MapBackgroundRef[] {
  return [...backgroundsFromDescriptor(document.descriptor)].sort(
    (left, right) => right.order - left.order || left.id.localeCompare(right.id),
  );
}

export function nextBackgroundOrder(backgrounds: readonly MapBackgroundRef[]): number {
  if (backgrounds.length === 0) return 0;
  return Math.max(...backgrounds.map((item) => item.order)) + 1;
}

export function applyGeometryOperationCommand(
  removedFeatures: VectorFeature[],
  addedFeatures: VectorFeature[],
  label: string,
): MapCommand {
  const removedIds = new Set(removedFeatures.map((feature) => feature.id));
  return {
    kind: "ApplyGeometryOperation",
    label,
    apply(document) {
      let collection = removeFeatures(document.collection, removedIds);
      for (const feature of addedFeatures) {
        collection = replaceFeature(collection, feature);
      }
      return withCollection(document, collection);
    },
    invert(before) {
      return applyGeometryOperationCommand(addedFeatures, removedFeatures, `Undo ${label}`);
    },
  };
}

export function setSnapSettingsCommand(enabled: boolean, previous: boolean): MapCommand {
  return {
    kind: "SetSnapSettings",
    label: enabled ? "Enable snap" : "Disable snap",
    coalesceKey: "snap-enabled",
    apply(document) {
      const descriptor = openLayersDescriptor(document);
      if (!descriptor) return document;
      return withDescriptor(
        document,
        patchOpenLayersDescriptor(descriptor, {
          settings: { ...descriptor.settings, snapEnabled: enabled },
        }) as OpenLayersMapDescriptor,
      );
    },
    invert() {
      return setSnapSettingsCommand(previous, enabled);
    },
  };
}

export function snapEnabledFromDescriptor(descriptor: unknown): boolean {
  if (!isOpenLayersDescriptor(descriptor)) return true;
  return descriptor.settings?.snapEnabled !== false;
}
