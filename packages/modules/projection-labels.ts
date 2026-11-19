import { derived } from "svelte/store";
import { createManifestLabels } from "./manifest-copy.ts";
import { displayLanguage } from "./plugin-i18n.ts";
import { uiMessage } from "./ui-catalogs.ts";

const translate = derived(displayLanguage, language => (source: string) => uiMessage(language, source));

export const { manifestText, setPackageLocalizations, schemaLabels } = createManifestLabels(displayLanguage, translate);
