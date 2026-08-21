import { grammarSystemDescriptor } from "./catalog.ts";
import {
  cloneGrammarRecord,
  emptyAgreementRecord,
  emptyAgreementSectionState,
  emptyCustomRule,
  emptySystemRecord,
  grammarRecordSnapshot,
  normalizeSystemConfig,
} from "./normalize.ts";
import type {
  GrammarDuplicateRecord,
  GrammarRecord,
  GrammarSectionId,
  GrammarSystemId,
  GrammarSystemRecord,
  IndexedGrammar,
  LoadedGrammarRecord,
} from "./types.ts";

export type GrammarEditSession = {
  recordId?: string;
  revision?: string;
  baseline: string;
  draft: GrammarRecord;
  locked: boolean;
  conflict: boolean;
  learnMoreOpen: boolean;
  originSection: GrammarSectionId;
  validationMessage?: string;
  validationFocus?: string;
  duplicates?: GrammarDuplicateRecord[];
};

export type GrammarUiState = {
  index: IndexedGrammar;
  query: string;
  section: GrammarSectionId | null;
  editing: GrammarEditSession | null;
  starterDismissed: boolean;
  starterCurrent?: GrammarSystemId;
  focusTarget?: string;
};

export function emptyGrammarUiState(): GrammarUiState {
  return {
    index: {
      systems: new Map(),
      duplicates: new Map(),
      agreements: [],
      customRules: [],
      sectionStates: new Map(),
      rejected: [],
      diagnostics: [],
    },
    query: "",
    section: null,
    editing: null,
    starterDismissed: false,
  };
}

export function isGrammarDirty(session: GrammarEditSession | null) {
  if (!session || session.locked) return false;
  return grammarRecordSnapshot(session.draft) !== session.baseline;
}

export function confirmGrammarLeave(
  session: GrammarEditSession | null,
  confirm: (message: string) => Promise<boolean>,
) {
  if (!isGrammarDirty(session)) return true;
  return confirm("You have unsaved grammar changes. Leave anyway?");
}

function sessionFromLoaded(
  record: LoadedGrammarRecord,
  originSection: GrammarSectionId,
  locked = false,
): GrammarEditSession {
  const draft = cloneGrammarRecord(record.value);
  return {
    recordId: record.id,
    revision: record.revision,
    baseline: grammarRecordSnapshot(draft),
    draft,
    locked,
    conflict: false,
    learnMoreOpen: false,
    originSection,
  };
}

export function openSystemEditor(index: IndexedGrammar, systemId: GrammarSystemId): GrammarEditSession {
  const descriptor = grammarSystemDescriptor(systemId)!;
  const duplicates = index.duplicates.get(systemId);
  if (duplicates?.length) {
    const label = descriptor.label;
    return {
      draft: emptySystemRecord(systemId),
      baseline: grammarRecordSnapshot(emptySystemRecord(systemId)),
      locked: true,
      conflict: false,
      learnMoreOpen: false,
      originSection: descriptor.sectionId,
      validationMessage: `${label} has duplicate records (${duplicates.length}). Edits are disabled until the conflict is resolved.`,
      duplicates,
    };
  }
  const loaded = index.systems.get(systemId);
  if (loaded) return sessionFromLoaded(loaded, descriptor.sectionId);
  const draft = emptySystemRecord(systemId);
  draft.config = normalizeSystemConfig(systemId, draft.config as Record<string, unknown>, draft.examples);
  return {
    draft,
    baseline: grammarRecordSnapshot(draft),
    locked: false,
    conflict: false,
    learnMoreOpen: false,
    originSection: descriptor.sectionId,
  };
}

export function openCustomRuleEditor(index: IndexedGrammar, recordId?: string): GrammarEditSession {
  const loaded = recordId ? index.customRules.find((item) => item.id === recordId) : undefined;
  if (loaded) return sessionFromLoaded(loaded, "other");
  const draft = emptyCustomRule();
  return {
    draft,
    baseline: grammarRecordSnapshot(draft),
    locked: false,
    conflict: false,
    learnMoreOpen: false,
    originSection: "other",
  };
}

export function openAgreementEditor(index: IndexedGrammar, recordId?: string): GrammarEditSession {
  const loaded = recordId ? index.agreements.find((item) => item.id === recordId) : undefined;
  if (loaded) return sessionFromLoaded(loaded, "agreement");
  const draft = emptyAgreementRecord();
  return {
    draft,
    baseline: grammarRecordSnapshot(draft),
    locked: false,
    conflict: false,
    learnMoreOpen: false,
    originSection: "agreement",
  };
}

export function openAgreementNotUsedEditor(index: IndexedGrammar): GrammarEditSession {
  const loaded = index.sectionStates.get("agreement");
  if (loaded) return sessionFromLoaded(loaded, "agreement");
  const draft = emptyAgreementSectionState();
  return {
    draft,
    baseline: grammarRecordSnapshot(emptyAgreementSectionState()),
    locked: false,
    conflict: false,
    learnMoreOpen: false,
    originSection: "agreement",
  };
}

export function applyStoredVersion(session: GrammarEditSession, stored: LoadedGrammarRecord): GrammarEditSession {
  const draft = cloneGrammarRecord(stored.value);
  return {
    ...session,
    recordId: stored.id,
    revision: stored.revision,
    draft,
    baseline: grammarRecordSnapshot(draft),
    conflict: false,
    validationMessage: undefined,
  };
}

export function keepDraftAfterConflict(session: GrammarEditSession, stored: LoadedGrammarRecord): GrammarEditSession {
  return {
    ...session,
    recordId: stored.id,
    revision: stored.revision,
    conflict: false,
    validationMessage: "Your draft is kept. Saving now overwrites the stored version.",
  };
}

export function setSystemStatus(
  draft: GrammarSystemRecord,
  status: GrammarSystemRecord["status"],
): GrammarSystemRecord {
  if (status === "configured") {
    const config = normalizeSystemConfig(draft.systemId, draft.config as Record<string, unknown>, draft.examples);
    return { ...draft, status, config };
  }
  return { ...draft, status, config: {} };
}
