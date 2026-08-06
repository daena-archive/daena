import manifestJson from "../manifest.json";
import type { DaenaModule, ModuleManifest } from "../../../module-api/src/index";
export * from "./adapter";

export const maps: DaenaModule = {
  manifest: manifestJson as ModuleManifest,
  views: [],
};
