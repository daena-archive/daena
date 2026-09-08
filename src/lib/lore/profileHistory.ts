import { compareCalendarDates, parseCalendarDate, type CalendarDate } from "../date.ts";
import {
  parseProfileValue,
  profileValidationErrors,
  type ProfileComponent,
  type ProfileDocument,
  type ProfileValue,
} from "./profile.ts";

export const PROFILE_CHANGE_COLLECTION = "profile-change";
export const PROFILE_CHANGE_SCHEMA_VERSION = 1;
export const PROFILE_CHANGE_RELATIONSHIP = "changes_profile";

export type ProfileChangeRelationshipLike = {
  type?: string;
  sourceId?: string;
  targetId?: string;
};

export function matchingProfileChangeRels<T extends ProfileChangeRelationshipLike>(
  rels: readonly T[],
  eventId: string,
  ownerEntityId: string,
): T[] {
  return rels.filter(
    (relationship) =>
      relationship.type === PROFILE_CHANGE_RELATIONSHIP &&
      relationship.sourceId === eventId &&
      relationship.targetId === ownerEntityId,
  );
}
export const PROFILE_CHANGE_MAX_BYTES = 64 * 1024;

export type ProfilePatch = {
  componentId: string;
  value: ProfileValue;
};

export type ProfileChangeDocument = {
  schemaVersion: number;
  date: CalendarDate;
  eventId?: string;
  reason?: string;
  patches: ProfilePatch[];
};

export type StoredProfileChange = {
  id: string;
  revision: string;
  createdAt: string;
  value: ProfileChangeDocument;
  invalid?: boolean;
  error?: string;
};

export function profileChangeValidationErrors(input: unknown): string[] {
  const errors: string[] = [];
  if (!input || typeof input !== "object" || Array.isArray(input)) return ["Profile change must be an object"];
  const document = input as Record<string, unknown>;
  if (document.schemaVersion !== PROFILE_CHANGE_SCHEMA_VERSION) errors.push("Unsupported Profile change version");
  if (!parseCalendarDate(document.date)) errors.push("Profile change needs a date");
  if (document.eventId !== undefined && (typeof document.eventId !== "string" || !document.eventId.trim())) {
    errors.push("Profile change event must be an id");
  }
  if (document.reason !== undefined && typeof document.reason !== "string") {
    errors.push("Profile change reason must be text");
  }
  if (!Array.isArray(document.patches) || document.patches.length === 0) {
    errors.push("Profile change needs at least one patch");
    return errors;
  }
  try {
    if (new TextEncoder().encode(JSON.stringify(document)).length > PROFILE_CHANGE_MAX_BYTES) {
      errors.push("Profile change exceeds 64 KiB");
    }
  } catch {
    errors.push("Profile change exceeds 64 KiB");
  }
  const ids = new Set<string>();
  document.patches.forEach((raw, index) => {
    const label = `Patch ${index + 1}`;
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
      errors.push(`${label} is invalid`);
      return;
    }
    const patch = raw as Record<string, unknown>;
    if (typeof patch.componentId !== "string" || !patch.componentId.trim()) {
      errors.push(`${label} needs a component`);
    } else if (ids.has(patch.componentId)) errors.push(`Duplicate patch for ${patch.componentId}`);
    else ids.add(patch.componentId);
    if (!parseProfileValue(patch.value, label)) errors.push(`${label} needs a value`);
  });
  return errors;
}

export function parseProfileChange(input: unknown): ProfileChangeDocument {
  const errors = profileChangeValidationErrors(input);
  if (errors.length) throw new Error(errors[0]);
  return readProfileChange(input);
}

function readProfileChange(input: unknown): ProfileChangeDocument {
  const document = input as Record<string, unknown>;
  const date = parseCalendarDate(document.date);
  if (!date) throw new Error("Profile change needs a date");
  const eventId = typeof document.eventId === "string" ? document.eventId.trim() : "";
  const reason = typeof document.reason === "string" ? document.reason.trim() : "";
  return {
    schemaVersion: PROFILE_CHANGE_SCHEMA_VERSION,
    date,
    ...(eventId ? { eventId } : {}),
    ...(reason ? { reason } : {}),
    patches: (document.patches as Record<string, unknown>[]).map((patch) => {
      const value = parseProfileValue(patch.value, "Patch");
      if (!value) throw new Error("Profile change patch is missing a value");
      return { componentId: String(patch.componentId).trim(), value };
    }),
  };
}

export function valuesEqual(left: ProfileValue, right: ProfileValue): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

export function valuePatches(from: ProfileDocument, to: ProfileDocument): ProfilePatch[] {
  const previous = new Map(from.components.map((component) => [component.id, component]));
  const patches: ProfilePatch[] = [];
  for (const component of to.components) {
    if (component.kind === "derived") continue;
    const before = previous.get(component.id);
    if (!before || before.kind === "derived") continue;
    if (!valuesEqual(before.value, component.value)) {
      patches.push({ componentId: component.id, value: component.value });
    }
  }
  return patches;
}

export function applyPatches(profile: ProfileDocument, patches: readonly ProfilePatch[]): ProfileDocument {
  if (!patches.length) return profile;
  const byId = new Map(patches.map((patch) => [patch.componentId, patch]));
  return {
    ...profile,
    components: profile.components.map((component) => {
      const patch = byId.get(component.id);
      if (!patch || component.kind === "derived") return component;
      return { ...component, value: patch.value };
    }),
  };
}

export function effectiveChangeDate(
  change: ProfileChangeDocument,
  eventDates: ReadonlyMap<string, unknown>,
): CalendarDate | null {
  if (!change.eventId) return change.date;
  if (!eventDates.has(change.eventId)) return null;
  return parseCalendarDate(eventDates.get(change.eventId));
}

export function foldProfile(
  baseline: ProfileDocument,
  changes: readonly StoredProfileChange[],
  asOf: unknown | undefined,
  eventDates: ReadonlyMap<string, unknown> = new Map(),
): ProfileDocument {
  const applicable: { change: StoredProfileChange; date: unknown }[] = [];
  for (const change of changes) {
    if (change.invalid) continue;
    const date = effectiveChangeDate(change.value, eventDates);
    if (date == null) continue;
    if (asOf !== undefined && compareCalendarDates(date, asOf) > 0) continue;
    applicable.push({ change, date });
  }
  applicable.sort((left, right) => {
    const byDate = compareCalendarDates(left.date, right.date);
    if (byDate) return byDate;
    return left.change.createdAt.localeCompare(right.change.createdAt) || left.change.id.localeCompare(right.change.id);
  });
  return applicable.reduce((profile, entry) => applyPatches(profile, entry.change.value.patches), baseline);
}

export function dateAfter(value: unknown): CalendarDate {
  const date = parseCalendarDate(value);
  if (!date) return { calendar: "gregorian", year: 1, era: "CE", precision: "year" };
  if (date.era === "BCE") {
    if (date.year > 1) return { ...date, year: date.year - 1 };
    return { ...date, year: 1, era: "CE" };
  }
  return { ...date, year: date.year + 1 };
}

export function changesForEvent(changes: readonly StoredProfileChange[], eventId: string): StoredProfileChange[] {
  return changes.filter((change) => !change.invalid && change.value.eventId === eventId);
}

export function unlinkedChangeAt(
  changes: readonly StoredProfileChange[],
  date: unknown,
): StoredProfileChange | undefined {
  return changes.find(
    (change) => !change.invalid && !change.value.eventId && compareCalendarDates(change.value.date, date) === 0,
  );
}

export function latestUnlinkedChange(
  changes: readonly StoredProfileChange[],
  eventDates: ReadonlyMap<string, unknown>,
): StoredProfileChange | undefined {
  const ranked = changes
    .filter((change) => !change.invalid)
    .map((change) => ({ change, date: effectiveChangeDate(change.value, eventDates) }))
    .filter((entry): entry is { change: StoredProfileChange; date: CalendarDate } => entry.date != null)
    .sort(
      (left, right) =>
        compareCalendarDates(left.date, right.date) ||
        left.change.createdAt.localeCompare(right.change.createdAt) ||
        left.change.id.localeCompare(right.change.id),
    );
  const last = ranked.at(-1);
  return last && !last.change.value.eventId ? last.change : undefined;
}

export function patchedProfileErrors(profile: ProfileDocument, patches: readonly ProfilePatch[]): string[] {
  return profileValidationErrors(applyPatches(profile, patches));
}

export function latestChangeDate(
  changes: readonly StoredProfileChange[],
  eventDates: ReadonlyMap<string, unknown>,
): unknown | undefined {
  let latest: unknown | undefined;
  for (const change of changes) {
    if (change.invalid) continue;
    const date = effectiveChangeDate(change.value, eventDates);
    if (date == null) continue;
    if (latest === undefined || compareCalendarDates(date, latest) > 0) latest = date;
  }
  return latest;
}

export function patchSummaries(components: readonly ProfileComponent[], patches: readonly ProfilePatch[]): string[] {
  const names = new Map(components.map((component) => [component.id, component.name]));
  return patches.map((patch) => names.get(patch.componentId) ?? patch.componentId);
}
