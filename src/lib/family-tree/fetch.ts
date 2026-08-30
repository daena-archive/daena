import type {
  FieldDefinition,
  ModuleContext,
  ModuleManifest,
  Relationship,
  UUID,
} from "../../../packages/module-api/src/index";
import {
  DEFAULT_SECONDARY_FIELD,
  ENTITY_GET_MANY_LIMIT,
  FIELD_HYDRATE_BATCH,
  INITIAL_ANCESTOR_GENERATIONS,
  INITIAL_DESCENDANT_GENERATIONS,
  HOUSE_TYPE,
  LORE_NAMESPACE,
  MEMBERSHIP_RELATIONSHIP,
  PARENT_RELATIONSHIP,
  PARTNER_RELATIONSHIP,
  PERSON_TYPE,
  RELATIONSHIP_QUERY_ENTITY_LIMIT,
  RELATIONSHIP_QUERY_FETCH_CAP,
  RELATIONSHIP_QUERY_PAGE,
  truncationWarning,
  type BranchDirection,
  type FamilyPerson,
  type FamilyTreeLimits,
  type GenealogyWarning,
  type HouseMemberRecord,
  type HouseMembership,
  type HouseMemberSummary,
} from "./model.ts";
import { personFromRecord } from "./projection.ts";

function membershipMeta(relationship: Relationship): {
  role: string | null;
  customLabel: string | null;
  notes: string | null;
} {
  const metadata = relationship.metadata ?? {};
  const role = typeof metadata.role === "string" ? metadata.role : null;
  const customLabel = typeof metadata.customLabel === "string" ? metadata.customLabel : null;
  const notes = typeof metadata.notes === "string" ? metadata.notes : null;
  return { role, customLabel, notes };
}

/** Count kinship-connected components among house members (isolated people each form a group). */
export function countKinshipFamilyGroups(
  memberIds: Iterable<string>,
  relationships: Array<{ type: string; sourceId: string; targetId: string }>,
): number {
  const members = [...new Set([...memberIds].filter(Boolean))];
  if (members.length === 0) return 0;
  const known = new Set(members);
  const parent = new Map<string, string>(members.map((id) => [id, id]));
  const find = (id: string): string => {
    const next = parent.get(id) ?? id;
    if (next !== id) parent.set(id, find(next));
    return parent.get(id) ?? id;
  };
  const unite = (left: string, right: string) => {
    const a = find(left);
    const b = find(right);
    if (a !== b) parent.set(a, b);
  };
  for (const relationship of relationships) {
    if (relationship.type !== PARENT_RELATIONSHIP && relationship.type !== PARTNER_RELATIONSHIP) continue;
    if (!known.has(relationship.sourceId) || !known.has(relationship.targetId)) continue;
    unite(relationship.sourceId, relationship.targetId);
  }
  return new Set(members.map((id) => find(id))).size;
}

export class NeighborhoodAbortedError extends Error {
  override name = "NeighborhoodAbortedError";
  constructor() {
    super("aborted");
  }
}

export function isNeighborhoodAbort(error: unknown): boolean {
  return error instanceof NeighborhoodAbortedError || (error instanceof DOMException && error.name === "AbortError");
}

function throwIfAborted(signal?: AbortSignal) {
  if (signal?.aborted) throw new NeighborhoodAbortedError();
}

async function queryPaged(
  context: ModuleContext,
  entityIds: string[],
  relationshipTypes: string[],
  direction: "incoming" | "outgoing" | "any",
  collected: Map<string, Relationship>,
  signal?: AbortSignal,
): Promise<{ added: Relationship[]; truncated: boolean; lowerBound: number }> {
  throwIfAborted(signal);
  const unique = [...new Set(entityIds.filter(Boolean))];
  if (unique.length === 0) return { added: [], truncated: false, lowerBound: 0 };
  const added: Relationship[] = [];
  let truncated = false;
  let lowerBound = 0;
  for (let index = 0; index < unique.length; index += RELATIONSHIP_QUERY_ENTITY_LIMIT) {
    const batch = unique.slice(index, index + RELATIONSHIP_QUERY_ENTITY_LIMIT);
    let offset = 0;
    let fetched = 0;
    while (true) {
      throwIfAborted(signal);
      const page = await context.relationships.query({
        entityIds: batch as UUID[],
        relationshipTypes,
        direction,
        offset,
        limit: RELATIONSHIP_QUERY_PAGE,
      });
      throwIfAborted(signal);
      lowerBound = Math.max(lowerBound, page.total);
      for (const item of page.items) {
        if (!collected.has(item.id)) added.push(item);
        collected.set(item.id, item);
      }
      fetched += page.items.length;
      if (!page.hasMore) break;
      if (page.items.length === 0) {
        truncated = true;
        break;
      }
      offset += page.items.length;
      if (fetched >= RELATIONSHIP_QUERY_FETCH_CAP) {
        truncated = true;
        break;
      }
    }
  }
  return { added, truncated, lowerBound };
}

async function collectParentsOfKnownChildren(
  context: ModuleContext,
  collected: Map<string, Relationship>,
  known: Set<string>,
  signal?: AbortSignal,
) {
  const childIds = uniqueIds(
    [...collected.values()]
      .filter((relationship) => relationship.type === PARENT_RELATIONSHIP && known.has(relationship.targetId))
      .map((relationship) => relationship.targetId),
    new Set(),
  );
  if (childIds.length === 0) return { added: [] as Relationship[], truncated: false, lowerBound: 0 };
  return queryPaged(context, childIds, [PARENT_RELATIONSHIP], "incoming", collected, signal);
}

function uniqueIds(values: Iterable<string>, known: Set<string>): string[] {
  const next: string[] = [];
  const seen = new Set<string>();
  for (const id of values) {
    if (!id || known.has(id) || seen.has(id)) continue;
    seen.add(id);
    next.push(id);
  }
  return next;
}

export async function listPersonSecondaryFields(context: ModuleContext): Promise<{ key: string; label: string }[]> {
  const manifests = await context.modules.list();
  const lore = manifests.find((manifest) => manifest.id === "daena.lore") as ModuleManifest | undefined;
  const fields = (lore?.schemas ?? []).flatMap((schema) => schema.fields ?? []) as FieldDefinition[];
  return fields
    .filter((field) => {
      if (field.shared !== true) return false;
      if (field.type !== "text" && field.type !== "enum") return false;
      const types = field.entityTypes ?? [];
      return types.length === 0 || types.includes("person") || types.includes(PERSON_TYPE);
    })
    .map((field) => ({ key: field.key, label: field.label }));
}

export async function loadGenealogyNeighborhood(
  context: ModuleContext,
  rootId: string,
  secondaryField = DEFAULT_SECONDARY_FIELD,
  signal?: AbortSignal,
  generations?: Pick<FamilyTreeLimits, "ancestorGenerations" | "descendantGenerations">,
): Promise<{
  people: FamilyPerson[];
  relationships: Relationship[];
  warnings: GenealogyWarning[];
  truncated: boolean;
  truncationLowerBound: number;
}> {
  const collected = new Map<string, Relationship>();
  const known = new Set<string>([rootId]);
  let truncated = false;
  let truncationLowerBound = 0;

  const recordPage = (page: { truncated: boolean; lowerBound: number }) => {
    truncated = truncated || page.truncated;
    truncationLowerBound = Math.max(truncationLowerBound, page.lowerBound);
  };

  const ancestorGenerations = generations?.ancestorGenerations ?? INITIAL_ANCESTOR_GENERATIONS;
  const descendantGenerations = generations?.descendantGenerations ?? INITIAL_DESCENDANT_GENERATIONS;
  let ancestorFrontier = [rootId];
  for (let generation = 0; generation < ancestorGenerations && ancestorFrontier.length > 0; generation += 1) {
    const page = await queryPaged(context, ancestorFrontier, [PARENT_RELATIONSHIP], "incoming", collected, signal);
    recordPage(page);
    ancestorFrontier = uniqueIds(
      page.added.map((relationship) => relationship.sourceId),
      known,
    );
    for (const id of ancestorFrontier) known.add(id);
  }

  let descendantFrontier = [rootId];
  for (let generation = 0; generation < descendantGenerations && descendantFrontier.length > 0; generation += 1) {
    const page = await queryPaged(context, descendantFrontier, [PARENT_RELATIONSHIP], "outgoing", collected, signal);
    recordPage(page);
    descendantFrontier = uniqueIds(
      page.added.map((relationship) => relationship.targetId),
      known,
    );
    for (const id of descendantFrontier) known.add(id);
  }

  const directParents = [...collected.values()]
    .filter((relationship) => relationship.type === PARENT_RELATIONSHIP && relationship.targetId === rootId)
    .map((relationship) => relationship.sourceId);
  if (directParents.length > 0) {
    const page = await queryPaged(context, directParents, [PARENT_RELATIONSHIP], "outgoing", collected, signal);
    recordPage(page);
    for (const id of uniqueIds(
      page.added.map((relationship) => relationship.targetId),
      known,
    ))
      known.add(id);
  }

  const partnerPage = await queryPaged(context, [...known], [PARTNER_RELATIONSHIP], "any", collected, signal);
  recordPage(partnerPage);
  for (const relationship of partnerPage.added) {
    if (!known.has(relationship.sourceId)) known.add(relationship.sourceId);
    if (!known.has(relationship.targetId)) known.add(relationship.targetId);
  }

  recordPage(await collectParentsOfKnownChildren(context, collected, known, signal));

  const hydrated = await hydratePeople(context, [...known], secondaryField, signal);
  const warnings = [...hydrated.warnings];
  if (truncated) warnings.push(truncationWarning(truncationLowerBound));
  return {
    people: hydrated.people,
    relationships: [...collected.values()],
    warnings,
    truncated,
    truncationLowerBound,
  };
}

export async function hydratePeople(
  context: ModuleContext,
  ids: string[],
  secondaryField = DEFAULT_SECONDARY_FIELD,
  signal?: AbortSignal,
): Promise<{ people: FamilyPerson[]; warnings: GenealogyWarning[] }> {
  const unique = [...new Set(ids.filter(Boolean))];
  const entities = [];
  for (let index = 0; index < unique.length; index += ENTITY_GET_MANY_LIMIT) {
    throwIfAborted(signal);
    const batch = unique.slice(index, index + ENTITY_GET_MANY_LIMIT);
    entities.push(...(await context.entities.getMany(batch as UUID[])));
  }
  const fieldsById = new Map<string, Record<string, unknown>>();
  const warnings: GenealogyWarning[] = [];
  for (let index = 0; index < entities.length; index += FIELD_HYDRATE_BATCH) {
    throwIfAborted(signal);
    const batch = entities.slice(index, index + FIELD_HYDRATE_BATCH);
    const loaded = await Promise.all(
      batch.map(async (entity) => {
        try {
          const records = await context.fields.listShared(entity.id, LORE_NAMESPACE);
          return {
            id: entity.id,
            fields: Object.fromEntries(records.map((record) => [record.key, record.value])),
            warning: null as GenealogyWarning | null,
          };
        } catch {
          return {
            id: entity.id,
            fields: {},
            warning: {
              entityId: entity.id,
              message: "Shared Lore fields could not be read.",
            } satisfies GenealogyWarning,
          };
        }
      }),
    );
    for (const entry of loaded) {
      fieldsById.set(entry.id, entry.fields);
      if (entry.warning) warnings.push(entry.warning);
    }
  }
  const people: FamilyPerson[] = [];
  for (const entity of entities) {
    const person = personFromRecord(entity, fieldsById.get(entity.id) ?? {}, secondaryField);
    if (!person) {
      warnings.push({ entityId: entity.id, message: "Skipped a non-person or deleted entity." });
      continue;
    }
    people.push(person);
  }
  return { people, warnings };
}

export async function loadExpansionLayer(
  context: ModuleContext,
  personId: string,
  direction: BranchDirection,
  collected: Map<string, Relationship>,
  knownPeople: Set<string>,
  secondaryField = DEFAULT_SECONDARY_FIELD,
  signal?: AbortSignal,
): Promise<{
  people: FamilyPerson[];
  relationships: Relationship[];
  warnings: GenealogyWarning[];
  truncated: boolean;
  truncationLowerBound: number;
}> {
  let truncated = false;
  let truncationLowerBound = 0;
  const recordPage = (page: { truncated: boolean; lowerBound: number }) => {
    truncated = truncated || page.truncated;
    truncationLowerBound = Math.max(truncationLowerBound, page.lowerBound);
  };
  const discovered = new Set<string>();
  if (direction === "parents") {
    const page = await queryPaged(context, [personId], [PARENT_RELATIONSHIP], "incoming", collected, signal);
    recordPage(page);
    for (const relationship of page.added) discovered.add(relationship.sourceId);
  } else if (direction === "children") {
    const page = await queryPaged(context, [personId], [PARENT_RELATIONSHIP], "outgoing", collected, signal);
    recordPage(page);
    for (const relationship of page.added) discovered.add(relationship.targetId);
    recordPage(
      await collectParentsOfKnownChildren(
        context,
        collected,
        new Set([personId, ...knownPeople, ...discovered]),
        signal,
      ),
    );
  } else if (direction === "siblings") {
    const parents = await queryPaged(context, [personId], [PARENT_RELATIONSHIP], "incoming", collected, signal);
    recordPage(parents);
    const parentIds = uniqueIds(
      [...collected.values()]
        .filter((relationship) => relationship.type === PARENT_RELATIONSHIP && relationship.targetId === personId)
        .map((relationship) => relationship.sourceId),
      new Set(),
    );
    if (parentIds.length > 0) {
      const children = await queryPaged(context, parentIds, [PARENT_RELATIONSHIP], "outgoing", collected, signal);
      recordPage(children);
      for (const relationship of children.added) {
        if (relationship.targetId !== personId) discovered.add(relationship.targetId);
      }
    }
  } else {
    const page = await queryPaged(context, [personId], [PARTNER_RELATIONSHIP], "any", collected, signal);
    recordPage(page);
    for (const relationship of page.added) {
      discovered.add(relationship.sourceId === personId ? relationship.targetId : relationship.sourceId);
    }
  }
  const newIds = uniqueIds(discovered, knownPeople);
  if (newIds.length > 0) {
    const incident = await queryPaged(
      context,
      newIds,
      [PARENT_RELATIONSHIP, PARTNER_RELATIONSHIP],
      "any",
      collected,
      signal,
    );
    recordPage(incident);
  }
  const hydrateIds = [...new Set([personId, ...discovered])];
  const hydrated = await hydratePeople(context, hydrateIds, secondaryField, signal);
  const warnings = [...hydrated.warnings];
  if (truncated) warnings.push(truncationWarning(truncationLowerBound));
  return {
    people: hydrated.people,
    relationships: [...collected.values()],
    warnings,
    truncated,
    truncationLowerBound,
  };
}

export async function listHouses(
  context: ModuleContext,
  options?: { text?: string; offset?: number; limit?: number; signal?: AbortSignal },
): Promise<{ items: { id: string; name: string }[]; total: number; offset: number; limit: number; hasMore: boolean }> {
  const signal = options?.signal;
  throwIfAborted(signal);
  const offset = Math.max(0, options?.offset ?? 0);
  const limit = Math.min(200, Math.max(1, options?.limit ?? 40));
  const page = await context.entities.query({
    type: HOUSE_TYPE,
    text: options?.text?.trim() || undefined,
    sortField: "name",
    sortDirection: "asc",
    offset,
    limit,
    includeDeleted: false,
  });
  throwIfAborted(signal);
  const items = page.items.filter((entity) => !entity.deleted).map((entity) => ({ id: entity.id, name: entity.name }));
  return {
    items,
    total: page.total,
    offset: page.offset,
    limit: page.limit,
    hasMore: page.hasMore,
  };
}

export async function houseMemberCounts(
  context: ModuleContext,
  houseIds: string[],
  signal?: AbortSignal,
): Promise<Map<string, number>> {
  const summaries = await houseMemberSummaries(context, houseIds, signal);
  return new Map([...summaries.entries()].map(([id, summary]) => [id, summary.memberCount]));
}

export async function houseMemberSummaries(
  context: ModuleContext,
  houseIds: string[],
  signal?: AbortSignal,
): Promise<Map<string, HouseMemberSummary>> {
  const unique = [...new Set(houseIds.filter(Boolean))];
  const summaries = new Map<string, HouseMemberSummary>(
    unique.map((id) => [id, { houseId: id, memberCount: 0, headName: null, heirName: null }]),
  );
  if (unique.length === 0) return summaries;
  const collected = new Map<string, Relationship>();
  await queryPaged(context, unique, [MEMBERSHIP_RELATIONSHIP], "incoming", collected, signal);
  const peopleByHouse = new Map<string, Set<string>>();
  const leadershipIds = new Set<string>();
  const headByHouse = new Map<string, string>();
  const heirByHouse = new Map<string, string>();
  for (const relationship of collected.values()) {
    if (relationship.type !== MEMBERSHIP_RELATIONSHIP || !unique.includes(relationship.targetId)) continue;
    const members = peopleByHouse.get(relationship.targetId) ?? new Set<string>();
    members.add(relationship.sourceId);
    peopleByHouse.set(relationship.targetId, members);
    const { role } = membershipMeta(relationship);
    if (role === "head" && !headByHouse.has(relationship.targetId)) {
      headByHouse.set(relationship.targetId, relationship.sourceId);
      leadershipIds.add(relationship.sourceId);
    }
    if (role === "heir" && !heirByHouse.has(relationship.targetId)) {
      heirByHouse.set(relationship.targetId, relationship.sourceId);
      leadershipIds.add(relationship.sourceId);
    }
  }
  const names = new Map<string, string>();
  const leaderList = [...leadershipIds];
  for (let index = 0; index < leaderList.length; index += ENTITY_GET_MANY_LIMIT) {
    throwIfAborted(signal);
    const batch = leaderList.slice(index, index + ENTITY_GET_MANY_LIMIT);
    for (const entity of await context.entities.getMany(batch as UUID[])) {
      if (!entity.deleted) names.set(entity.id, entity.name);
    }
  }
  for (const id of unique) {
    const headId = headByHouse.get(id);
    const heirId = heirByHouse.get(id);
    summaries.set(id, {
      houseId: id,
      memberCount: peopleByHouse.get(id)?.size ?? 0,
      headName: headId ? (names.get(headId) ?? null) : null,
      heirName: heirId ? (names.get(heirId) ?? null) : null,
    });
  }
  return summaries;
}

export function formatHouseMemberSummary(
  summary: HouseMemberSummary | undefined | null,
  options?: { pending?: boolean },
): string {
  if (!summary) return options?.pending ? "Loading…" : "0 members";
  const count = summary.memberCount;
  const parts = [`${count} ${count === 1 ? "member" : "members"}`];
  if (summary.headName) parts.push(`Head ${summary.headName}`);
  if (summary.heirName) parts.push(`Heir ${summary.heirName}`);
  return parts.join(" · ");
}

export async function listHouseMembers(
  context: ModuleContext,
  houseId: string,
  signal?: AbortSignal,
): Promise<{ id: string; name: string }[]> {
  const records = await listHouseMemberRecords(context, houseId, signal);
  return records.map((record) => ({ id: record.personId, name: record.personName }));
}

export async function listHouseMemberRecords(
  context: ModuleContext,
  houseId: string,
  signal?: AbortSignal,
): Promise<HouseMemberRecord[]> {
  if (!houseId) return [];
  const collected = new Map<string, Relationship>();
  await queryPaged(context, [houseId], [MEMBERSHIP_RELATIONSHIP], "incoming", collected, signal);
  const memberships = [...collected.values()].filter(
    (relationship) => relationship.type === MEMBERSHIP_RELATIONSHIP && relationship.targetId === houseId,
  );
  const personIds = [...new Set(memberships.map((relationship) => relationship.sourceId))];
  if (personIds.length === 0) return [];
  const names = new Map<string, string>();
  for (let index = 0; index < personIds.length; index += ENTITY_GET_MANY_LIMIT) {
    throwIfAborted(signal);
    const batch = personIds.slice(index, index + ENTITY_GET_MANY_LIMIT);
    for (const entity of await context.entities.getMany(batch as UUID[])) {
      if (!entity.deleted && entity.type === PERSON_TYPE) names.set(entity.id, entity.name);
    }
  }
  return memberships
    .flatMap((relationship) => {
      const personName = names.get(relationship.sourceId);
      if (!personName) return [];
      const meta = membershipMeta(relationship);
      return [
        {
          id: relationship.id,
          revision: relationship.revision,
          personId: relationship.sourceId,
          personName,
          houseId,
          role: meta.role,
          customLabel: meta.customLabel,
          notes: meta.notes,
        } satisfies HouseMemberRecord,
      ];
    })
    .sort(
      (left, right) => left.personName.localeCompare(right.personName) || left.personId.localeCompare(right.personId),
    );
}

export async function loadHouseNeighborhood(
  context: ModuleContext,
  houseId: string,
  secondaryField = DEFAULT_SECONDARY_FIELD,
  signal?: AbortSignal,
): Promise<{
  people: FamilyPerson[];
  relationships: Relationship[];
  warnings: GenealogyWarning[];
  truncated: boolean;
  truncationLowerBound: number;
}> {
  const members = await listHouseMembers(context, houseId, signal);
  const memberIds = members.map((member) => member.id);
  const known = new Set(memberIds);
  const collected = new Map<string, Relationship>();
  let truncated = false;
  let truncationLowerBound = 0;
  const recordPage = (page: { truncated: boolean; lowerBound: number }) => {
    truncated = truncated || page.truncated;
    truncationLowerBound = Math.max(truncationLowerBound, page.lowerBound);
  };
  if (memberIds.length > 0) {
    recordPage(
      await queryPaged(context, memberIds, [PARENT_RELATIONSHIP, PARTNER_RELATIONSHIP], "any", collected, signal),
    );
  }
  const relationships = [...collected.values()].filter((relationship) => {
    if (relationship.type !== PARENT_RELATIONSHIP && relationship.type !== PARTNER_RELATIONSHIP) return false;
    return known.has(relationship.sourceId) && known.has(relationship.targetId);
  });
  const hydrated = await hydratePeople(context, memberIds, secondaryField, signal);
  const warnings = [...hydrated.warnings];
  if (truncated) warnings.push(truncationWarning(truncationLowerBound));
  return {
    people: hydrated.people,
    relationships,
    warnings,
    truncated,
    truncationLowerBound,
  };
}

export async function loadHouseMemberships(
  context: ModuleContext,
  personIds: string[],
  signal?: AbortSignal,
): Promise<HouseMembership[]> {
  const uniquePeople = [...new Set(personIds.filter(Boolean))];
  if (uniquePeople.length === 0) return [];
  const collected = new Map<string, Relationship>();
  await queryPaged(context, uniquePeople, [MEMBERSHIP_RELATIONSHIP], "outgoing", collected, signal);
  const memberships = [...collected.values()].filter(
    (relationship) => relationship.type === MEMBERSHIP_RELATIONSHIP && uniquePeople.includes(relationship.sourceId),
  );
  const houseIds = [...new Set(memberships.map((relationship) => relationship.targetId))];
  const houses = new Map<string, string>();
  for (let index = 0; index < houseIds.length; index += ENTITY_GET_MANY_LIMIT) {
    throwIfAborted(signal);
    const batch = houseIds.slice(index, index + ENTITY_GET_MANY_LIMIT);
    for (const entity of await context.entities.getMany(batch as UUID[])) {
      if (!entity.deleted && entity.type === HOUSE_TYPE) houses.set(entity.id, entity.name);
    }
  }
  return memberships.flatMap((relationship) => {
    const houseName = houses.get(relationship.targetId);
    if (!houseName) return [];
    const meta = membershipMeta(relationship);
    return [
      {
        id: relationship.id,
        revision: relationship.revision,
        personId: relationship.sourceId,
        houseId: relationship.targetId,
        houseName,
        role: meta.role,
        customLabel: meta.customLabel,
        notes: meta.notes,
      } satisfies HouseMembership,
    ];
  });
}
