import type { Entity, EntityListQuery, EntityPage } from "$lib/project/client";
import type { ModuleContext, UUID } from "../../../packages/module-api/src/index";
import { normalizeCalendarDefinition, type CalendarDefinition } from "../../../packages/modules/timeline/src/calendar";

type CalendarProjectClient = {
  queryEntities: (query?: EntityListQuery) => Promise<EntityPage>;
};

let loadedRoot = "";
let definitions: Record<string, CalendarDefinition> = {};
let entities: Entity[] = [];
let loadPromise: Promise<void> | null = null;
// Bumped on every invalidate and on every load start so a stale in-flight load can never
// overwrite newer cache state (e.g. after a project switch).
let loadGeneration = 0;

export function snapshot(): {
  definitions: Record<string, CalendarDefinition>;
  entities: Entity[];
} {
  return { definitions: { ...definitions }, entities: [...entities] };
}

export function getDefinition(calendarId: string): CalendarDefinition | null {
  return definitions[calendarId] ?? null;
}

export function setDefinition(calendarId: string, definition: CalendarDefinition) {
  definitions = { ...definitions, [calendarId]: definition };
}

export function invalidate(calendarId?: string) {
  loadGeneration += 1;
  loadPromise = null;
  if (calendarId) {
    const next = { ...definitions };
    delete next[calendarId];
    definitions = next;
    entities = entities.filter((entity) => entity.id !== calendarId);
    return;
  }
  loadedRoot = "";
  definitions = {};
  entities = [];
}

export async function ensureLoaded(
  root: string,
  client: CalendarProjectClient,
  context: ModuleContext,
  onEntity?: (entity: Entity) => void,
  force = false,
): Promise<{ definitions: Record<string, CalendarDefinition>; entities: Entity[] }> {
  if (!root) {
    invalidate();
    return snapshot();
  }
  if (!force && loadedRoot === root && !loadPromise) return snapshot();
  if (loadPromise) await loadPromise.catch(() => {});
  if (!force && loadedRoot === root) return snapshot();
  const generation = ++loadGeneration;
  const operation = (async () => {
    const next: Record<string, CalendarDefinition> = {};
    const calendars: Entity[] = [];
    let offset = 0;
    const pageSize = 200;
    while (true) {
      const page = await client.queryEntities({
        entityTypes: ["daena.timeline:calendar"],
        sortField: "name",
        sortDirection: "asc",
        limit: pageSize,
        offset,
      });
      calendars.push(...page.items.filter((entity) => !entity.deleted));
      // Guard against a misbehaving backend claiming has_more with an empty page.
      if (!page.has_more || page.items.length === 0) break;
      offset += page.items.length;
    }
    await Promise.all(
      calendars.map(async (calendar) => {
        const records = await context.records.list("calendar-definition", calendar.id as UUID, { limit: 1 });
        next[calendar.id] = records[0]
          ? normalizeCalendarDefinition(records[0].value)
          : normalizeCalendarDefinition({});
      }),
    );
    // Apply atomically, and only if nothing invalidated or restarted the load meanwhile.
    if (generation !== loadGeneration) return;
    definitions = next;
    entities = calendars;
    loadedRoot = root;
    for (const calendar of calendars) onEntity?.(calendar);
  })();
  loadPromise = operation;
  try {
    await operation;
  } catch {
    // A missing record capability or empty project still leaves Gregorian as the default.
  } finally {
    if (loadPromise === operation) loadPromise = null;
  }
  return snapshot();
}
