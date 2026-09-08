import { parseCalendarDate } from "$lib/date";
import { project } from "$lib/project/client";
import type { ModuleContext, UUID } from "../../../packages/module-api/src/index";
import {
  emptyProfile,
  parseProfile,
  PROFILE_ALREADY_EXISTS,
  PROFILE_COLLECTION,
  type ProfileDocument,
} from "./profile";
import {
  changesForEvent,
  dateAfter,
  foldProfile,
  latestChangeDate,
  latestUnlinkedChange,
  parseProfileChange,
  patchedProfileErrors,
  matchingProfileChangeRels,
  PROFILE_CHANGE_COLLECTION,
  PROFILE_CHANGE_RELATIONSHIP,
  PROFILE_CHANGE_SCHEMA_VERSION,
  unlinkedChangeAt,
  valuePatches,
  type ProfileChangeDocument,
  type StoredProfileChange,
} from "./profileHistory";

export type StoredProfile = {
  id: UUID;
  revision: string;
  value: ProfileDocument;
  error?: string;
  invalid?: boolean;
};

export const PROFILE_CHANGED_EVENT = "daena:profile-changed";
export const PROFILE_TIMELINE_EVENT = "daena:profile-timeline-changed";

export function notifyProfileChanged(entityId: string) {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent(PROFILE_CHANGED_EVENT, { detail: { entityId } }));
}

export function notifyProfileTimelineChanged(eventId: string) {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent(PROFILE_TIMELINE_EVENT, { detail: { eventId } }));
}

async function listOwnerProfiles(context: ModuleContext, ownerEntityId: string) {
  return context.records.list<unknown>(PROFILE_COLLECTION, ownerEntityId as UUID, {
    limit: 2,
  });
}

async function listAllRecords<T>(context: ModuleContext, collection: string, ownerEntityId: string) {
  const records: Awaited<ReturnType<ModuleContext["records"]["list"]>> = [];
  let offset = 0;
  while (true) {
    const page = await context.records.list<T>(collection, ownerEntityId as UUID, {
      limit: 100,
      offset,
      sort: "updatedAt",
    });
    records.push(...page);
    if (page.length < 100) break;
    offset += page.length;
  }
  return records;
}

export async function loadProfile(context: ModuleContext, ownerEntityId: string): Promise<StoredProfile | null> {
  const records = await listOwnerProfiles(context, ownerEntityId);
  const record = records[0];
  if (!record) return null;
  const duplicate = records.length > 1 ? PROFILE_ALREADY_EXISTS : undefined;
  try {
    return {
      id: record.id,
      revision: record.revision,
      value: parseProfile(record.value),
      error: duplicate,
    };
  } catch (cause) {
    return {
      id: record.id,
      revision: record.revision,
      value: emptyProfile(),
      error: duplicate ?? (cause instanceof Error ? cause.message : String(cause)),
      invalid: true,
    };
  }
}

export async function loadProfileChanges(
  context: ModuleContext,
  ownerEntityId: string,
): Promise<StoredProfileChange[]> {
  const records = await listAllRecords<unknown>(context, PROFILE_CHANGE_COLLECTION, ownerEntityId);
  const changes: StoredProfileChange[] = [];
  for (const record of records) {
    try {
      changes.push({
        id: record.id,
        revision: record.revision,
        createdAt: record.createdAt,
        value: parseProfileChange(record.value),
      });
    } catch (cause) {
      changes.push({
        id: record.id,
        revision: record.revision,
        createdAt: record.createdAt,
        value: {
          schemaVersion: PROFILE_CHANGE_SCHEMA_VERSION,
          date: { calendar: "gregorian", year: 1, era: "CE", precision: "year" },
          patches: [{ componentId: "_invalid", value: { type: "text", value: null } }],
        },
        invalid: true,
        error: cause instanceof Error ? cause.message : String(cause),
      });
    }
  }
  return changes;
}

export async function resolveEventDates(eventIds: readonly string[]): Promise<Map<string, unknown>> {
  const dates = new Map<string, unknown>();
  const unique = [...new Set(eventIds.filter(Boolean))];
  await Promise.all(
    unique.map(async (eventId) => {
      try {
        const entity = await project.getEntity(eventId);
        if (!entity || entity.deleted) return;
        const fields = await project.listFields(eventId);
        const startsAt = fields.find((field) => field.key === "startsAt")?.value;
        dates.set(eventId, startsAt);
      } catch {
        // Missing events drop out of the map so fold skips their changes.
      }
    }),
  );
  return dates;
}

export async function eventDatesForChanges(
  changes: readonly StoredProfileChange[],
  overlay?: { eventId: string; date: unknown },
): Promise<Map<string, unknown>> {
  const dates = await resolveEventDates(changes.map((change) => change.value.eventId ?? ""));
  if (overlay?.eventId) dates.set(overlay.eventId, overlay.date);
  return dates;
}

export async function loadFoldedProfile(
  context: ModuleContext,
  ownerEntityId: string,
  asOf?: unknown,
): Promise<{ profile: StoredProfile | null; changes: StoredProfileChange[]; folded: ProfileDocument | null }> {
  const profile = await loadProfile(context, ownerEntityId);
  if (!profile || profile.invalid)
    return { profile, changes: [], folded: profile?.invalid ? null : (profile?.value ?? null) };
  const changes = await loadProfileChanges(context, ownerEntityId);
  const eventDates = await resolveEventDates(changes.map((change) => change.value.eventId ?? ""));
  return { profile, changes, folded: foldProfile(profile.value, changes, asOf, eventDates) };
}

export async function createProfile(
  context: ModuleContext,
  ownerEntityId: string,
  value: ProfileDocument = emptyProfile(),
): Promise<StoredProfile> {
  const existing = await listOwnerProfiles(context, ownerEntityId);
  if (existing.length) throw new Error(PROFILE_ALREADY_EXISTS);
  const parsed = parseProfile(value);
  const record = await context.records.create(PROFILE_COLLECTION, ownerEntityId as UUID, parsed);
  const stored = { id: record.id, revision: record.revision, value: parseProfile(record.value) };
  notifyProfileChanged(ownerEntityId);
  return stored;
}

export async function saveProfile(
  context: ModuleContext,
  ownerEntityId: string,
  stored: StoredProfile,
  value: ProfileDocument,
): Promise<StoredProfile> {
  const parsed = parseProfile(value);
  const record = await context.records.update(PROFILE_COLLECTION, stored.id, ownerEntityId as UUID, parsed, {
    expectedRevision: stored.revision,
  });
  const next = { id: record.id, revision: record.revision, value: parseProfile(record.value) };
  notifyProfileChanged(ownerEntityId);
  return next;
}

export async function saveProfileValues(
  context: ModuleContext,
  ownerEntityId: string,
  stored: StoredProfile,
  next: ProfileDocument,
  changes: StoredProfileChange[],
  eventDates: ReadonlyMap<string, unknown>,
  at?: { date: unknown; reason?: string },
): Promise<{ profile: StoredProfile; changes: StoredProfileChange[] }> {
  if (at) {
    return writeUnlinkedChange(context, ownerEntityId, stored, next, changes, eventDates, at);
  }
  if (!changes.some((change) => !change.invalid)) {
    return { profile: await saveProfile(context, ownerEntityId, stored, next), changes };
  }
  const target = latestUnlinkedChange(changes, eventDates);
  if (target) {
    return writeChange(
      context,
      ownerEntityId,
      stored,
      next,
      changes,
      eventDates,
      target,
      { ...target.value, date: target.value.date },
      undefined,
    );
  }
  const folded = foldProfile(stored.value, changes, undefined, eventDates);
  const patches = valuePatches(folded, next);
  if (!patches.length) return { profile: stored, changes };
  const constraint = patchedProfileErrors(folded, patches);
  if (constraint.length) throw new Error(constraint[0]);
  const created = await createProfileChange(context, ownerEntityId, {
    schemaVersion: PROFILE_CHANGE_SCHEMA_VERSION,
    date: dateAfter(latestChangeDate(changes, eventDates)),
    patches,
  });
  return { profile: stored, changes: [...changes, created] };
}

async function writeUnlinkedChange(
  context: ModuleContext,
  ownerEntityId: string,
  stored: StoredProfile,
  next: ProfileDocument,
  changes: StoredProfileChange[],
  eventDates: ReadonlyMap<string, unknown>,
  at: { date: unknown; reason?: string },
): Promise<{ profile: StoredProfile; changes: StoredProfileChange[] }> {
  const date = parseCalendarDate(at.date);
  if (!date) throw new Error("Choose a date for this change");
  const target = unlinkedChangeAt(changes, date);
  const reason = at.reason?.trim() ?? "";
  return writeChange(
    context,
    ownerEntityId,
    stored,
    next,
    changes,
    eventDates,
    target,
    {
      schemaVersion: PROFILE_CHANGE_SCHEMA_VERSION,
      date,
      ...(reason ? { reason } : {}),
      patches: [],
    },
    date,
  );
}

async function writeChange(
  context: ModuleContext,
  ownerEntityId: string,
  stored: StoredProfile,
  next: ProfileDocument,
  changes: StoredProfileChange[],
  eventDates: ReadonlyMap<string, unknown>,
  target: StoredProfileChange | undefined,
  base: ProfileChangeDocument,
  asOf: unknown | undefined,
): Promise<{ profile: StoredProfile; changes: StoredProfileChange[] }> {
  const others = changes.filter((change) => change.id !== target?.id);
  const before = foldProfile(stored.value, others, asOf, eventDates);
  const patches = valuePatches(before, next);
  const nextReason = base.reason?.trim() ?? "";
  const prevReason = target?.value.reason ?? "";
  if (!patches.length) {
    if (target && nextReason !== prevReason) {
      const updated = await updateProfileChange(
        context,
        ownerEntityId,
        target,
        withChangeReason({ ...target.value }, nextReason),
      );
      return { profile: stored, changes: replaceChange(changes, updated) };
    }
    if (target) {
      await deleteProfileChange(context, ownerEntityId, target);
      return { profile: stored, changes: others };
    }
    return { profile: stored, changes };
  }
  const constraint = patchedProfileErrors(before, patches);
  if (constraint.length) throw new Error(constraint[0]);
  const value = withChangeReason({ ...base, patches }, nextReason);
  if (target) {
    const updated = await updateProfileChange(context, ownerEntityId, target, value);
    return { profile: stored, changes: replaceChange(changes, updated) };
  }
  const created = await createProfileChange(context, ownerEntityId, value);
  return { profile: stored, changes: [...changes, created] };
}

function replaceChange(changes: StoredProfileChange[], next: StoredProfileChange): StoredProfileChange[] {
  return changes.map((change) => (change.id === next.id ? next : change));
}

export async function upsertEventProfileChange(
  context: ModuleContext,
  ownerEntityId: string,
  stored: StoredProfile,
  next: ProfileDocument,
  changes: StoredProfileChange[],
  eventDates: ReadonlyMap<string, unknown>,
  eventId: string,
  date: unknown,
  reason?: string,
): Promise<StoredProfileChange[]> {
  const parsed = parseCalendarDate(date);
  if (!parsed) throw new Error("Set Starts on this event to date Profile changes.");
  const matches = changesForEvent(changes, eventId);
  const [target, ...dupes] = matches;
  for (const dupe of dupes) await deleteProfileChange(context, ownerEntityId, dupe);
  const remaining = changes.filter((change) => !dupes.some((dupe) => dupe.id === change.id));
  const trimmed = reason?.trim() ?? "";
  const result = await writeChange(
    context,
    ownerEntityId,
    stored,
    next,
    remaining,
    eventDates,
    target,
    {
      schemaVersion: PROFILE_CHANGE_SCHEMA_VERSION,
      date: parsed,
      eventId,
      ...(trimmed ? { reason: trimmed } : {}),
      patches: [],
    },
    parsed,
  );
  return result.changes;
}

function withChangeReason(document: ProfileChangeDocument, reason: string): ProfileChangeDocument {
  if (!reason) {
    const { reason: _ignored, ...rest } = document;
    return rest;
  }
  return { ...document, reason };
}

export async function unlinkEventProfileChange(
  timeline: ModuleContext,
  ownerEntityId: string,
  eventId: string,
): Promise<void> {
  const fromEvent = matchingProfileChangeRels(
    await timeline.relationships.list(eventId as UUID),
    eventId,
    ownerEntityId,
  );
  const rels = fromEvent.length
    ? fromEvent
    : matchingProfileChangeRels(await timeline.relationships.list(ownerEntityId as UUID), eventId, ownerEntityId);
  for (const relationship of rels) {
    await timeline.relationships.delete(relationship.id as UUID, PROFILE_CHANGE_RELATIONSHIP, {
      expectedRevision: relationship.revision,
    });
  }
}

export async function createProfileChange(
  context: ModuleContext,
  ownerEntityId: string,
  value: ProfileChangeDocument,
): Promise<StoredProfileChange> {
  const parsed = parseProfileChange(value);
  const record = await context.records.create(PROFILE_CHANGE_COLLECTION, ownerEntityId as UUID, parsed);
  const stored = {
    id: record.id,
    revision: record.revision,
    createdAt: record.createdAt,
    value: parseProfileChange(record.value),
  };
  notifyProfileChanged(ownerEntityId);
  return stored;
}

export async function updateProfileChange(
  context: ModuleContext,
  ownerEntityId: string,
  stored: StoredProfileChange,
  value: ProfileChangeDocument,
): Promise<StoredProfileChange> {
  const parsed = parseProfileChange(value);
  const record = await context.records.update(
    PROFILE_CHANGE_COLLECTION,
    stored.id as UUID,
    ownerEntityId as UUID,
    parsed,
    { expectedRevision: stored.revision },
  );
  const next = {
    id: record.id,
    revision: record.revision,
    createdAt: record.createdAt,
    value: parseProfileChange(record.value),
  };
  notifyProfileChanged(ownerEntityId);
  return next;
}

export async function deleteProfileChange(
  context: ModuleContext,
  ownerEntityId: string,
  stored: StoredProfileChange,
): Promise<void> {
  await context.records.delete(PROFILE_CHANGE_COLLECTION, stored.id as UUID, ownerEntityId as UUID, {
    expectedRevision: stored.revision,
  });
  notifyProfileChanged(ownerEntityId);
}

export async function deleteProfile(
  context: ModuleContext,
  ownerEntityId: string,
  stored: StoredProfile,
): Promise<void> {
  const records = await listAllRecords<unknown>(context, PROFILE_CHANGE_COLLECTION, ownerEntityId);
  for (const record of records) {
    await context.records.delete(PROFILE_CHANGE_COLLECTION, record.id as UUID, ownerEntityId as UUID, {
      expectedRevision: record.revision,
    });
  }
  await context.records.delete(PROFILE_COLLECTION, stored.id, ownerEntityId as UUID, {
    expectedRevision: stored.revision,
  });
  notifyProfileChanged(ownerEntityId);
}
