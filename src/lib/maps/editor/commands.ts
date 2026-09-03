import { cloneDocument, type MapDocument } from "./model.ts";

export type MapCommandKind =
  | "CreateFeature"
  | "DeleteFeatures"
  | "ReplaceGeometry"
  | "DuplicateFeatures"
  | "MoveFeaturesToLayer"
  | "SetFeatureMetadata"
  | "CreateLayer"
  | "DuplicateLayer"
  | "DeleteLayer"
  | "RenameLayer"
  | "ReorderLayer"
  | "SetLayerVisibility"
  | "SetLayerLocked"
  | "SetLayerOpacity"
  | "SetLayerStyle"
  | "AddBackground"
  | "ReplaceBackground"
  | "RemoveBackground"
  | "ReorderBackground"
  | "SetBackgroundOpacity"
  | "SetBackgroundVisibility"
  | "SetDefaultView"
  | "SetCoordinateSpace"
  | "ApplyGeometryOperation"
  | "SetSnapSettings"
  | "DetachPhysicalFeatures";

export type MapCommand = {
  kind: MapCommandKind;
  label: string;
  coalesceKey?: string;
  apply: (document: MapDocument) => MapDocument;
  /** Inverse that restores prior document state for this command. */
  invert: (before: MapDocument) => MapCommand;
};

export function applyCommand(document: MapDocument, command: MapCommand): MapDocument {
  return command.apply(cloneDocument(document));
}

export type { FeatureMetadataPatch } from "./feature-commands.ts";
export {
  captureDeleteFeatures,
  captureReplaceCollection,
  createFeatureCommand,
  deleteFeaturesCommand,
  duplicateFeaturesCommand,
  duplicateFeaturesOntoLayer,
  moveFeaturesToLayerCommand,
  replaceCollectionCommand,
  replaceGeometryCommand,
  setFeatureMetadataCommand,
  setFeaturesMetadataByIdCommand,
  setFeaturesMetadataCommand,
} from "./feature-commands.ts";
export {
  buildCreateLayer,
  buildCreateRasterLayer,
  buildDuplicateLayer,
  createLayerCommand,
  deleteLayerCommand,
  detachPhysicalFeaturesCommand,
  duplicateLayerCommand,
  layersFieldValue,
  newRasterLayer,
  newVectorLayer,
  renameLayerCommand,
  reorderLayerCommand,
  reorderLayersByIdsCommand,
  setLayerLockedCommand,
  setLayerOpacityCommand,
  setLayerStyleCommand,
  setLayerVisibilityCommand,
} from "./layer-commands.ts";
export {
  addBackgroundCommand,
  applyGeometryOperationCommand,
  calibrateImageToWorld,
  calibrateWorldUnits,
  listedBackgrounds,
  nextBackgroundOrder,
  removeBackgroundCommand,
  reorderBackgroundCommand,
  replaceBackgroundCommand,
  setBackgroundOpacityCommand,
  setBackgroundVisibilityCommand,
  setCoordinateSpaceCommand,
  setDefaultViewCommand,
  setSnapSettingsCommand,
  snapEnabledFromDescriptor,
} from "./background-commands.ts";
