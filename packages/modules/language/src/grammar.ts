export type {
  GrammarAgreementRecord,
  GrammarCustomRuleRecord,
  GrammarExample,
  GrammarIssue,
  GrammarLink,
  GrammarRecord,
  GrammarSearchHit,
  GrammarSectionId,
  GrammarSectionStateRecord,
  GrammarStatus,
  GrammarSystemId,
  GrammarSystemRecord,
  IndexedGrammar,
  NormalizeResult,
} from "./grammar/types.ts";
export { GRAMMAR_SCHEMA_VERSION, GRAMMAR_SYSTEM_IDS } from "./grammar/types.ts";
export {
  GRAMMAR_CATALOG,
  GRAMMAR_SECTIONS,
  assertCatalogComplete,
  grammarSectionDescriptor,
  grammarSystemDescriptor,
  systemsForSection,
} from "./grammar/catalog.ts";
export {
  brokenAgreementFeatures,
  cloneGrammarRecord,
  configuredMinimum,
  emptyAgreementSectionState,
  emptyConfig,
  emptyCustomRule,
  emptySystemRecord,
  grammarRecordSnapshot,
  indexGrammarRecords,
  normalizeGrammarRecord,
  serializeGrammarRecord,
  systemStatus,
  validateGrammarDraft,
} from "./grammar/normalize.ts";
export { grammarGlance, grammarStatusLabel, searchGrammar, sectionCardSummary, summarizeSystem } from "./grammar/summaries.ts";
export { GRAMMAR_VALUE_SCHEMA } from "./grammar/schema.ts";
export {
  applyStoredVersion,
  confirmGrammarLeave,
  emptyGrammarUiState,
  isGrammarDirty,
  keepDraftAfterConflict,
  openAgreementNotUsedEditor,
  openCustomRuleEditor,
  openSystemEditor,
  setSystemStatus,
} from "./grammar/session.ts";
export type { GrammarEditSession, GrammarUiState } from "./grammar/session.ts";
export {
  deleteGrammarRecord,
  isStaleRevisionError,
  loadGrammarIndex,
  paginateRecords,
  persistGrammarRecord,
} from "./grammar/repository.ts";
