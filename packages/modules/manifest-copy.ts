import { derived, writable, type Readable } from "svelte/store";
import { translatePluginMessage, type PluginCatalogs } from "../plugin-sdk/src/i18n.ts";
import { humanizeTypeId } from "../module-api/src/entityTypeLabels.ts";
import type { ModuleManifest } from "../module-api/src/index";
import houses from "./houses/manifest.json" with { type: "json" };
import language from "./language/manifest.json" with { type: "json" };
import lore from "./lore/manifest.json" with { type: "json" };
import maps from "./maps/manifest.json" with { type: "json" };
import timeline from "./timeline/manifest.json" with { type: "json" };
import writing from "./writing/manifest.json" with { type: "json" };

type CopyKind = "module" | "type" | "option" | "metadata" | "metadataOption" | "variant" | "field" | "template" | "templateDescription" | "timelineGroup" | "timelineLabel" | "view" | "component" | "command" | "theme" | "importer" | "importerDescription";
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
      for (const variant of field.oneOf ?? []) {
        remember("variant", field.key, variant.label);
        for (const option of variant.options ?? []) remember("option", field.key + "/" + option, option);
      }
      for (const metadata of field.metadataFields ?? []) {
        const key = field.relationshipType + "/" + metadata.key;
        remember("metadata", key, metadata.label);
        for (const option of metadata.options ?? []) remember("metadataOption", key, option);
        for (const variant of metadata.oneOf ?? []) {
          remember("variant", key, variant.label);
          for (const option of variant.options ?? []) remember("metadataOption", key, option);
        }
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
export function createManifestLabels(
  displayLanguage: Readable<string>,
  ui: Readable<(source: string) => string>,
) {
  const externalCopy = writable(new Map<string, RegisteredPackage>());

  type DisplayManifest = Pick<ModuleManifest, "id"> & Partial<Pick<ModuleManifest, "schemas">> & { localization?: PackageLocalization };
  const projectManifests = writable<readonly DisplayManifest[]>([]);

  function setPackageLocalizations(modules: readonly DisplayManifest[]) {
    projectManifests.set(modules);
    const next = new Map<string, RegisteredPackage>();
    for (const { id, localization } of modules) {
      if (localization?.source.id === id) {
        next.set(id, { originals: declaredCopy(localization.source), catalogs: localization.catalogs });
      }
    }
    externalCopy.set(next);
  }

  /** Match the package definition before translating. Project-specific labels stay verbatim. */
  const manifestText = derived([ui, displayLanguage, externalCopy], ([translate, language, external]) =>
    (moduleId: string | null | undefined, kind: CopyKind, id: string, value: string): string => {
      if (!moduleId) return value;
      const key = JSON.stringify([kind, localCopyId(moduleId, id), value]);
      const plugin = external.get(moduleId);
      if (plugin?.originals.has(key)) return translatePluginMessage(plugin.catalogs, language, value);
      return bundledCopy.get(moduleId)?.has(key) ? translate(value) : value;
    },
  );

  /** Rebuild the display index when the project schema or the display language changes. */
  const schemaLabels = derived([projectManifests, manifestText, ui], ([installed, translate, text]) => {
    const types = new Map<string, string>();
    const relations = new Map<string, string>();
    const manifests = new Map<string, DisplayManifest>(bundled.map(module => [module.id, module]));
    for (const module of installed) manifests.set(module.id, module);
    for (const module of manifests.values()) {
      for (const schema of module.schemas ?? module.localization?.source.schemas ?? []) {
        for (const type of schema.entityTypes) {
          const id = type.id.includes(":") ? type.id : module.id + ":" + type.id;
          types.set(id, translate(module.id, "type", id, type.name));
        }
        for (const field of schema.fields) {
          if (field.relationshipType && !relations.has(field.relationshipType)) {
            relations.set(field.relationshipType, translate(module.id, "field", field.key, field.label));
          }
        }
      }
    }
    return {
      type(id: string | null | undefined, fallback?: string): string {
        if (!id) return text("Uncategorized");
        return types.get(id) ?? fallback ?? humanizeTypeId(id);
      },
      relationship(id: string): string {
        return relations.get(id) ?? humanizeTypeId(id);
      },
    };
  });
  return { setPackageLocalizations, manifestText, schemaLabels };
}
