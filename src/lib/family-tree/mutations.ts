import type { ModuleContext, Relationship, UUID } from "../../../packages/module-api/src/index";
import { PARENT_RELATIONSHIP, PARTNER_RELATIONSHIP, PERSON_TYPE, type RelativeRole } from "./model.ts";

export type MutationCode =
  "relationship.cycle" | "relationship.duplicate" | "relationship.self" | "revision-conflict" | "unknown";

export interface MutationFailure {
  code: MutationCode;
  message: string;
}

function readErrorCode(error: unknown): string {
  if (!error || typeof error !== "object") return "";
  const record = error as Record<string, unknown>;
  if (typeof record.code === "string" && record.code.trim()) return record.code;
  const details = record.details;
  if (details && typeof details === "object") {
    const nested = (details as { code?: unknown }).code;
    if (typeof nested === "string" && nested.trim()) return nested;
  }
  return "";
}

export function classifyMutationError(error: unknown): MutationFailure {
  const message =
    error instanceof Error
      ? error.message
      : error &&
          typeof error === "object" &&
          "message" in error &&
          typeof (error as { message: unknown }).message === "string"
        ? (error as { message: string }).message
        : String(error);
  const prefixed = message.match(/^(relationship\.(?:cycle|duplicate|self)|revision[^\s:]*)\b/);
  const code = readErrorCode(error) || prefixed?.[1] || "";
  if (code === "relationship.cycle" || /would introduce a cycle/i.test(message)) {
    return { code: "relationship.cycle", message: message || "That parent link would create a cycle." };
  }
  if (code === "relationship.duplicate" || /already exists for these endpoints/i.test(message)) {
    return { code: "relationship.duplicate", message: message || "That family relationship already exists." };
  }
  if (code === "relationship.self" || /cannot target the same entity/i.test(message)) {
    return { code: "relationship.self", message: message || "A person cannot be their own parent or partner." };
  }
  if (code.includes("revision") || /revision conflict/i.test(message)) {
    return { code: "revision-conflict", message: message || "This record changed in another view." };
  }
  return { code: "unknown", message: message || "The change could not be saved." };
}

export function metadataFingerprint(draft: Record<string, unknown>): string {
  return JSON.stringify(serializeMetadata(draft));
}

export function serializeMetadata(draft: Record<string, unknown>): Record<string, unknown> {
  const metadata: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(draft)) {
    if (value === undefined || value === null) continue;
    if (typeof value === "string") {
      const trimmed = value.trim();
      if (!trimmed) continue;
      metadata[key] = trimmed;
      continue;
    }
    metadata[key] = value;
  }
  return metadata;
}

export function canonicalPair(left: string, right: string): [string, string] {
  return left < right ? [left, right] : [right, left];
}

export async function createMinimalPerson(context: ModuleContext, name: string, requestId: string) {
  return context.entities.create({ name: name.trim(), type: PERSON_TYPE }, { requestId });
}

export async function createFamilyRelationship(
  context: ModuleContext,
  input: {
    role: RelativeRole;
    currentId: string;
    otherId: string;
    metadata: Record<string, unknown>;
    sourceRevision: string;
    requestId: string;
  },
): Promise<Relationship> {
  const metadata = serializeMetadata(input.metadata);
  if (input.role === "parent") {
    return context.relationships.create(
      {
        sourceId: input.otherId as UUID,
        targetId: input.currentId as UUID,
        type: PARENT_RELATIONSHIP,
        metadata,
      },
      { expectedRevision: input.sourceRevision, requestId: input.requestId },
    );
  }
  if (input.role === "child") {
    return context.relationships.create(
      {
        sourceId: input.currentId as UUID,
        targetId: input.otherId as UUID,
        type: PARENT_RELATIONSHIP,
        metadata,
      },
      { expectedRevision: input.sourceRevision, requestId: input.requestId },
    );
  }
  const [sourceId, targetId] = canonicalPair(input.currentId, input.otherId);
  return context.relationships.create(
    {
      sourceId: sourceId as UUID,
      targetId: targetId as UUID,
      type: PARTNER_RELATIONSHIP,
      metadata,
    },
    { expectedRevision: input.sourceRevision, requestId: input.requestId },
  );
}

export async function updateFamilyMetadata(
  context: ModuleContext,
  id: string,
  metadata: Record<string, unknown>,
  expectedRevision: string,
  requestId: string,
) {
  return context.relationships.update(
    { id: id as UUID, metadata: serializeMetadata(metadata) },
    { expectedRevision, requestId },
  );
}

export async function deleteFamilyRelationship(
  context: ModuleContext,
  id: string,
  type: string,
  expectedRevision: string,
  requestId: string,
) {
  await context.relationships.delete(id as UUID, type, { expectedRevision, requestId });
}
