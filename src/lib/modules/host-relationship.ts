import type { Relationship } from "$lib/project/client";

export function toHostRelationship(relationship: {
  id: string;
  sourceId: string;
  targetId: string;
  type: string;
  metadata: Record<string, unknown>;
  revision: string;
}): Relationship {
  return {
    id: relationship.id,
    source_id: relationship.sourceId,
    target_id: relationship.targetId,
    relationship_type: relationship.type,
    metadata: JSON.stringify(relationship.metadata ?? {}),
    revision: relationship.revision,
  };
}
