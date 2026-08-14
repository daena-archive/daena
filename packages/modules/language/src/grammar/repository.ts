import type { ModuleContext, ModuleRecord, ModuleRecordQuery } from "../../../../module-api/src/index";
import {
  indexGrammarRecords,
  serializeGrammarRecord,
  validateGrammarDraft,
  brokenAgreementFeatures,
} from "./normalize.ts";
import type { GrammarRecord, IndexedGrammar } from "./types.ts";

export type GrammarRecordsApi = Pick<ModuleContext["records"], "list" | "create" | "update" | "delete">;

export function isStaleRevisionError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  return /revision conflict/i.test(message);
}

export async function paginateRecords<T>(
  records: GrammarRecordsApi,
  collection: string,
  ownerEntityId: string,
  sort: ModuleRecordQuery["sort"] = "updatedAt",
) {
  const collected: ModuleRecord<T>[] = [];
  let offset = 0;
  for (;;) {
    const page = await records.list<T>(collection, ownerEntityId, { limit: 100, offset, sort });
    collected.push(...page);
    if (page.length < 100) break;
    offset += 100;
  }
  return collected;
}

export async function loadGrammarIndex(records: GrammarRecordsApi, ownerEntityId: string) {
  const collected = await paginateRecords<GrammarRecord>(records, "grammar", ownerEntityId);
  const index = indexGrammarRecords(collected);
  index.diagnostics.push(...brokenAgreementFeatures(index));
  return { records: collected, index };
}

export type PersistOk = { ok: true; record: ModuleRecord<GrammarRecord> | null; index: IndexedGrammar };
export type PersistStale = {
  ok: false;
  stale: true;
  stored?: ModuleRecord<GrammarRecord>;
  index: IndexedGrammar;
};
export type PersistErr = { ok: false; stale?: false; error: string; issues?: { message: string; path?: string }[] };
export type PersistResult = PersistOk | PersistStale | PersistErr;

async function reload(records: GrammarRecordsApi, ownerEntityId: string) {
  return loadGrammarIndex(records, ownerEntityId);
}

export async function persistGrammarRecord(
  api: GrammarRecordsApi,
  ownerEntityId: string,
  session: { recordId?: string; revision?: string; draft: GrammarRecord },
): Promise<PersistResult> {
  const issues = validateGrammarDraft(session.draft);
  if (issues.length) return { ok: false, error: issues[0].message, issues };

  const shouldDelete =
    session.draft.recordKind === "system" && session.draft.status === "unconfigured" && session.recordId;
  const shouldSkip =
    session.draft.recordKind === "system" && session.draft.status === "unconfigured" && !session.recordId;

  try {
    if (shouldSkip) {
      const loaded = await reload(api, ownerEntityId);
      return { ok: true, record: null, index: loaded.index };
    }
    if (shouldDelete && session.recordId) {
      await api.delete("grammar", session.recordId, ownerEntityId, {
        expectedRevision: session.revision,
        requestId: crypto.randomUUID(),
      });
      const loaded = await reload(api, ownerEntityId);
      return { ok: true, record: null, index: loaded.index };
    }
    const payload = serializeGrammarRecord(session.draft) as GrammarRecord;
    let saved: ModuleRecord<GrammarRecord>;
    if (session.recordId) {
      saved = await api.update("grammar", session.recordId, ownerEntityId, payload, {
        expectedRevision: session.revision,
        requestId: crypto.randomUUID(),
      });
    } else {
      saved = await api.create("grammar", ownerEntityId, payload, { requestId: crypto.randomUUID() });
    }
    if (session.draft.recordKind === "agreement") {
      const afterSave = await reload(api, ownerEntityId);
      const unused = afterSave.index.sectionStates.get("agreement");
      if (unused) {
        await api.delete("grammar", unused.id, ownerEntityId, {
          expectedRevision: unused.revision,
          requestId: crypto.randomUUID(),
        });
      }
    }
    const loaded = await reload(api, ownerEntityId);
    const current = loaded.records.find((item) => item.id === saved.id) ?? saved;
    return { ok: true, record: current, index: loaded.index };
  } catch (cause) {
    if (isStaleRevisionError(cause) && session.recordId) {
      const loaded = await reload(api, ownerEntityId);
      return {
        ok: false,
        stale: true,
        stored: loaded.records.find((item) => item.id === session.recordId),
        index: loaded.index,
      };
    }
    return { ok: false, error: cause instanceof Error ? cause.message : String(cause) };
  }
}

export async function deleteGrammarRecord(
  api: GrammarRecordsApi,
  ownerEntityId: string,
  session: { recordId: string; revision?: string },
): Promise<PersistResult> {
  try {
    await api.delete("grammar", session.recordId, ownerEntityId, {
      expectedRevision: session.revision,
      requestId: crypto.randomUUID(),
    });
    const loaded = await reload(api, ownerEntityId);
    return { ok: true, record: null, index: loaded.index };
  } catch (cause) {
    if (isStaleRevisionError(cause)) {
      const loaded = await reload(api, ownerEntityId);
      return {
        ok: false,
        stale: true,
        stored: loaded.records.find((item) => item.id === session.recordId),
        index: loaded.index,
      };
    }
    return { ok: false, error: cause instanceof Error ? cause.message : String(cause) };
  }
}
