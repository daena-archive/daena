import { createManifestLabels } from "../../../packages/modules/manifest-copy.ts";
import { displayLanguage } from "./index.ts";
import { ui } from "./ui.ts";

export type { PackageLocalization } from "../../../packages/modules/manifest-copy.ts";
export const { manifestText, setPackageLocalizations, schemaLabels } = createManifestLabels(displayLanguage, ui);
