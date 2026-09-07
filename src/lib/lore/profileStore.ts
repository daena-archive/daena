import type { ModuleContext, UUID } from "../../../packages/module-api/src/index";
import {
  emptyProfile,
  parseProfile,
  PROFILE_ALREADY_EXISTS,
  PROFILE_COLLECTION,
  type ProfileDocument,
} from "./profile";

export type StoredProfile = {
  id: UUID;
  revision: string;
  value: ProfileDocument;
  error?: string;
  invalid?: boolean;
};

export const PROFILE_CHANGED_EVENT = "daena:profile-changed";

export function notifyProfileChanged(entityId: string) {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent(PROFILE_CHANGED_EVENT, { detail: { entityId } }));
}

async function listOwnerProfiles(context: ModuleContext, ownerEntityId: string) {
  return context.records.list<unknown>(PROFILE_COLLECTION, ownerEntityId as UUID, {
    limit: 2,
  });
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

export async function deleteProfile(
  context: ModuleContext,
  ownerEntityId: string,
  stored: StoredProfile,
): Promise<void> {
  await context.records.delete(PROFILE_COLLECTION, stored.id, ownerEntityId as UUID, {
    expectedRevision: stored.revision,
  });
  notifyProfileChanged(ownerEntityId);
}
