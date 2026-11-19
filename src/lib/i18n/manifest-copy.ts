import type { ModuleManifest } from "../../../packages/module-api/src/index";
import houses from "../../../packages/modules/houses/manifest.json" with { type: "json" };
import language from "../../../packages/modules/language/manifest.json" with { type: "json" };
import lore from "../../../packages/modules/lore/manifest.json" with { type: "json" };
import maps from "../../../packages/modules/maps/manifest.json" with { type: "json" };
import timeline from "../../../packages/modules/timeline/manifest.json" with { type: "json" };
import writing from "../../../packages/modules/writing/manifest.json" with { type: "json" };

type CopyKind = "module" | "type" | "option" | "metadata" | "field" | "template" | "templateDescription" | "timelineGroup" | "timelineLabel" | "view";
const originals = new Map<string, string>();
const bundled = [houses, language, lore, maps, timeline, writing] as unknown as ModuleManifest[];

export function bundledWindowTitles(translate: (source: string) => string) {
  return Object.fromEntries(bundled.map(({ id, name }) => [id, { source: name, title: translate(name) }]));
}

for (const manifest of bundled) {
  const remember = (kind: CopyKind, id: string, value?: string | null) => {
    if (value) originals.set(JSON.stringify([manifest.id, kind, id]), value);
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
  for (const view of manifest.views ?? []) remember("view", view.id, view.title);
}

/** Only translate the shipped value for this specific definition. Custom names are author data. */
export function manifestCopy(
  translate: (source: string) => string,
  moduleId: string | null | undefined,
  kind: CopyKind,
  id: string,
  value: string,
): string {
  if (!moduleId) return value;
  const localId = id.startsWith(moduleId + ":") ? id.slice(moduleId.length + 1) : id;
  return originals.get(JSON.stringify([moduleId, kind, localId])) === value ? translate(value) : value;
}
