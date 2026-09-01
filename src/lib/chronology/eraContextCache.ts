import type { EraContext } from "$lib/modules/chronology";
import type { Relationship } from "$lib/project/client";
import { CALENDAR_ADOPTION_TYPE } from "$lib/modules/chronology";

type EraContextDeps = {
  listFields: (entityId: string) => Promise<Array<{ key: string; value: unknown }>>;
  listRelationships: (entityId: string) => Promise<Relationship[]>;
  entityName: (entityId: string) => string;
};

const cache = new Map<string, EraContext>();

export function clear() {
  cache.clear();
}

export function invalidate(eraId?: string) {
  if (eraId) cache.delete(eraId);
  else cache.clear();
}

export function invalidateMany(eraIds: readonly string[]) {
  for (const eraId of eraIds) cache.delete(eraId);
}

async function loadOne(id: string, deps: EraContextDeps): Promise<EraContext> {
  let start: unknown;
  let end: unknown;
  let calendarIds: string[] = [];
  try {
    const stored = await deps.listFields(id);
    start = stored.find((field) => field.key === "startsAt")?.value;
    end = stored.find((field) => field.key === "endsAt")?.value;
  } catch {
    // Missing fields are treated as open-ended era bounds.
  }
  try {
    const linked = await deps.listRelationships(id);
    calendarIds = linked
      .filter(
        (relationship) => relationship.relationship_type === CALENDAR_ADOPTION_TYPE && relationship.source_id === id,
      )
      .map((relationship) => relationship.target_id);
  } catch {
    // Missing relationships simply leave calendarIds empty.
  }
  return {
    id,
    name: deps.entityName(id),
    start,
    end,
    calendarIds,
  };
}

export async function getMany(eraIds: readonly string[], deps: EraContextDeps): Promise<EraContext[]> {
  const unique = [...new Set(eraIds.filter(Boolean))];
  const missing = unique.filter((id) => !cache.has(id));
  if (missing.length > 0) {
    const loaded = await Promise.all(missing.map((id) => loadOne(id, deps)));
    missing.forEach((id, index) => cache.set(id, loaded[index]!));
  }
  return unique.map((id) => cache.get(id)!);
}
