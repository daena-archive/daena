import manifestJson from "../manifest.json";
import type { DaenaModule, ModuleManifest } from "../../../module-api/src/index";

export const maps: DaenaModule = {
  manifest: manifestJson as ModuleManifest,
  views: [],
};
