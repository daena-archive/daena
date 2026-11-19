import { derived, writable } from "svelte/store";
import { translatePluginMessage, type PluginCatalogs } from "../../../packages/plugin-sdk/src/i18n.ts";
import { displayLanguage } from "./index.ts";
import { ui } from "./ui.ts";
import type { ModuleManifest } from "../../../packages/module-api/src/index";
import houses from "../../../packages/modules/houses/manifest.json" with { type: "json" };
import language from "../../../packages/modules/language/manifest.json" with { type: "json" };
import lore from "../../../packages/modules/lore/manifest.json" with { type: "json" };
import maps from "../../../packages/modules/maps/manifest.json" with { type: "json" };
import timeline from "../../../packages/modules/timeline/manifest.json" with { type: "json" };
import writing from "../../../packages/modules/writing/manifest.json" with { type: "json" };

type CopyKind = "module" | "type" | "option" | "metadata" | "field" | "template" | "templateDescription" | "timelineGroup" | "timelineLabel" | "view" | "component" | "command" | "theme" | "importer" | "importerDescription";
const bundled = [houses, language, lore, maps, timeline, writing] as unknown as ModuleManifest[];

export interface PackageLocalization {
  source: ModuleManifest;
  catalogs: PluginCatalogs;
}
type RegisteredPackage = { originals: Set<string>; catalogs: PluginCatalogs };

function localCopyId(moduleId: string, id: string) {
  return id.startsWith(moduleId + ":") ? id.slice(moduleId.length + 1) : id;
}

function declaredCopy(manifest: ModuleManifest) {
  const originals = new Set<string>();
  const remember = (kind: CopyKind, id: string, value?: string | null) => {
    if (value) originals.add(JSON.stringify([kind, localCopyId(manifest.id, id), value]));
  };
  remember("module", manifest.id, manifest.name);
  for (const schema of manifest.schemas ?? []) {
    for (const type of schema.entityTypes ?? []) remember("type", type.id, type.name);
    for (const field of schema.fields ?? []) {
      remember("field", field.key, field.label);
      remember("timelineGroup", field.key, field.timeline?.group);
      remember("timelineLabel", field.key, field.timeline?.label);
      for (const metadata of field.metadataFields ?? []) {
        remember("metadata", field.relationshipType + "/" + metadata.key, metadata.label);
      }
      for (const option of field.options ?? []) remember("option", field.key + "/" + option, option);
    }
  }
  for (const template of manifest.templates ?? []) {
    remember("template", template.id, template.name);
    remember("templateDescription", template.id, template.description);
  }
  for (const view of manifest.views ?? []) {
    remember("view", view.id, view.title);
    for (const part of view.components ?? []) {
      remember("component", view.id + "/" + part.id, "text" in part ? part.text : "title" in part ? part.title : part.label);
    }
  }
  for (const command of manifest.commands ?? []) remember("command", command.id, command.title);
  for (const theme of manifest.themes ?? []) remember("theme", theme.id, theme.name);
  for (const importer of manifest.importers ?? []) {
    remember("importer", importer.id, importer.name);
    remember("importerDescription", importer.id, importer.description);
  }
  return originals;
}

const bundledCopy = new Map<string, Set<string>>(bundled.map((manifest) => [manifest.id, declaredCopy(manifest)]));
const externalCopy = writable(new Map<string, RegisteredPackage>());

export function setPackageLocalizations(modules: readonly { id: string; localization?: PackageLocalization }[]) {
  const next = new Map<string, RegisteredPackage>();
  for (const { id, localization } of modules) {
    if (localization?.source.id === id) {
      next.set(id, { originals: declaredCopy(localization.source), catalogs: localization.catalogs });
    }
  }
  externalCopy.set(next);
}

/** Match the package definition before translating. Project-specific labels stay verbatim. */
export const manifestText = derived([ui, displayLanguage, externalCopy], ([translate, language, external]) =>
  (moduleId: string | null | undefined, kind: CopyKind, id: string, value: string): string => {
    if (!moduleId) return value;
    const key = JSON.stringify([kind, localCopyId(moduleId, id), value]);
    const plugin = external.get(moduleId);
    if (plugin?.originals.has(key)) return translatePluginMessage(plugin.catalogs, language, value);
    return bundledCopy.get(moduleId)?.has(key) ? translate(value) : value;
  },
);
