import type { EntityTemplate, FieldDefinition } from "$lib/project/client";

export function defaultCreateFieldValue(field: FieldDefinition, template: EntityTemplate) {
  if (Object.prototype.hasOwnProperty.call(template.fields, field.key)) return template.fields[field.key];
  return field.type === "boolean"
    ? false
    : field.type === "relationship" || (field.type === "enum" && field.multiple)
      ? []
      : "";
}

export function isCreateValuePopulated(value: unknown) {
  if (Array.isArray(value)) return value.length > 0;
  return value !== "" && value !== null && value !== undefined && value !== false;
}

export function isCreateDropdownField(field: FieldDefinition) {
  return field.type === "enum";
}
