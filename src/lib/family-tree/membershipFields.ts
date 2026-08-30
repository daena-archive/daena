import type { MetadataFieldDefinition, ModuleContext } from "../../../packages/module-api/src/index";
import { MEMBER_ROLES, MEMBERSHIP_RELATIONSHIP } from "./model.ts";

/** Resolve membership metadata fields from the merged module schema. */
export function membershipMetadataFields(context: ModuleContext): MetadataFieldDefinition[] {
  for (const schema of context.module.schemas ?? []) {
    for (const field of schema.fields ?? []) {
      if (field.relationshipType === MEMBERSHIP_RELATIONSHIP && field.metadataFields?.length) {
        return field.metadataFields;
      }
    }
  }
  // Builtin Houses package always contributes these; keep a minimal fallback so
  // Tree can still edit role when a project overlay temporarily omits metadataFields.
  return [
    { key: "role", label: "Role", type: "enum", required: true, options: [...MEMBER_ROLES] },
    { key: "customLabel", label: "Custom label", type: "text" },
    { key: "notes", label: "Notes", type: "text" },
  ];
}

export function defaultMembershipDraft(
  seed: { role?: string | null; customLabel?: string | null; notes?: string | null } = {},
): Record<string, unknown> {
  return {
    role: seed.role?.trim() || "member",
    customLabel: seed.customLabel ?? "",
    notes: seed.notes ?? "",
  };
}
