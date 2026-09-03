import type { EntityTemplate, FieldDefinition, ModuleSchemaOverlay } from "$lib/project/client";
import type { EntityRemovalDisposition, ExclusiveFieldDisposition } from "./model";

export type TypeRemovalPrompt = {
  typeId: string;
  exclusiveFields: FieldDefinition[];
  sharedFields: FieldDefinition[];
  templates: EntityTemplate[];
  removeSharedFields: boolean;
  exclusiveDispositions: Record<string, ExclusiveFieldDisposition | undefined>;
  entityCount: number | null;
  entityDisposition: EntityRemovalDisposition | undefined;
  busy: boolean;
  error: string;
};

export function isDisabled(list: string[] | undefined, id: string) {
  return (list ?? []).includes(id);
}

export function toggleInList(list: string[], id: string): string[] {
  const next = new Set(list);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  return [...next].sort();
}

export function overlaySummary(value: ModuleSchemaOverlay): string {
  const types = value.customEntityTypes?.length ?? 0;
  const fields = value.customFields?.length ?? 0;
  const templates = value.customTemplates?.length ?? 0;
  const disabledTypes = value.disabledEntityTypes?.length ?? 0;
  const disabledFields = value.disabledFields?.length ?? 0;
  return [
    `custom types ${types}`,
    `custom fields ${fields}`,
    `custom templates ${templates}`,
    `disabled types ${disabledTypes}`,
    `disabled fields ${disabledFields}`,
  ].join(" · ");
}

export function exclusiveReassignmentTarget(prompt: TypeRemovalPrompt, fieldKey: string): string {
  const disposition = prompt.exclusiveDispositions[fieldKey];
  return disposition?.action === "reassign" ? disposition.toTypeId : "";
}
