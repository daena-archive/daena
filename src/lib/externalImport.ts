import type { ProjectModuleManifest } from "./project/client";
import type { FieldDefinition } from "../../packages/module-api/src/index";

export interface ImportEntityTypeChoice {
  id: string;
  label: string;
  moduleId: string;
  moduleName: string;
}

export interface ImportFieldChoice {
  id: string;
  namespace: string;
  key: string;
  label: string;
  moduleId: string;
  moduleName: string;
  definition: FieldDefinition;
}

export interface ExternalImportMappingCatalog {
  entityTypes: ImportEntityTypeChoice[];
  fields: ImportFieldChoice[];
  relationships: ImportFieldChoice[];
  fingerprint: string;
}

export function buildExternalImportMappingCatalog(modules: ProjectModuleManifest[]): ExternalImportMappingCatalog {
  const enabled = modules.filter((module) => module.enabled);
  const entityTypes = new Map<string, ImportEntityTypeChoice>();
  const fields = new Map<string, ImportFieldChoice>();
  const relationships = new Map<string, ImportFieldChoice>();

  for (const module of enabled) {
    const templateNames = new Map(module.templates.map((template) => [template.entityType, template.name] as const));
    for (const schema of module.schemas) {
      for (const entityType of schema.entityTypes) {
        if (!entityTypes.has(entityType)) {
          entityTypes.set(entityType, {
            id: entityType,
            label: templateNames.get(entityType) ?? humanize(entityType),
            moduleId: module.id,
            moduleName: module.name,
          });
        }
      }
      for (const field of schema.fields) {
        const id = `${schema.namespace}:${field.key}`;
        if (field.type !== "relationship" && !fields.has(id)) {
          fields.set(id, {
            id,
            namespace: schema.namespace,
            key: field.key,
            label: field.label,
            moduleId: module.id,
            moduleName: module.name,
            definition: field,
          });
        }
        if (
          field.type === "relationship" &&
          field.relationshipType &&
          !field.metadataFields?.some((metadataField) => metadataField.required)
        ) {
          relationships.set(field.relationshipType, {
            id: field.relationshipType,
            namespace: schema.namespace,
            key: field.key,
            label: field.label,
            moduleId: module.id,
            moduleName: module.name,
            definition: field,
          });
        }
      }
    }
    for (const template of module.templates) {
      if (!entityTypes.has(template.entityType)) {
        entityTypes.set(template.entityType, {
          id: template.entityType,
          label: template.name,
          moduleId: module.id,
          moduleName: module.name,
        });
      }
    }
  }

  const entityTypeList = [...entityTypes.values()].sort(compareChoice);
  const fieldList = [...fields.values()].sort(compareChoice);
  const fingerprintSource = enabled
    .map((module) => ({
      id: module.id,
      version: module.version,
      entityTypes: module.schemas.flatMap((schema) => schema.entityTypes).sort(),
      fields: module.schemas
        .flatMap((schema) => schema.fields.map((field) => `${schema.namespace}:${field.key}`))
        .sort(),
      relationships: module.schemas
        .flatMap((schema) =>
          schema.fields
            .filter((field) => field.type === "relationship" && field.relationshipType)
            .map((field) => field.relationshipType!),
        )
        .sort(),
    }))
    .sort((left, right) => left.id.localeCompare(right.id));

  return {
    entityTypes: entityTypeList,
    fields: fieldList,
    relationships: [...relationships.values()].sort(compareChoice),
    fingerprint: `enabled-manifests:${JSON.stringify(fingerprintSource)}`,
  };
}

export function importFolderFor(sourcePath: string): string {
  const boundary = sourcePath.lastIndexOf("/");
  return boundary > 0 ? sourcePath.slice(0, boundary) : "";
}

function humanize(value: string): string {
  const tail = value.split(/[.:/]/).pop() ?? value;
  return tail.replace(/[-_]+/g, " ").replace(/\b\w/g, (character) => character.toUpperCase());
}

function compareChoice(
  left: { moduleName: string; label: string; id: string },
  right: { moduleName: string; label: string; id: string },
): number {
  return (
    left.moduleName.localeCompare(right.moduleName) ||
    left.label.localeCompare(right.label) ||
    left.id.localeCompare(right.id)
  );
}
