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
export {
  ADJECTIVE_POSITION_OPTIONS,
  ADPOSITION_OPTIONS,
  CHOICE_SYSTEM_IDS,
  POSSESSIVE_POSITION_OPTIONS,
  RELATIVE_CLAUSE_POSITION_OPTIONS,
  WORD_ORDER_INFLUENCE_OPTIONS,
  WORD_ORDER_OPTIONS,
  WORD_ORDER_STRENGTH_OPTIONS,
  applyAdjectivePosition,
  applyAdpositions,
  applyBasicWordOrder,
  applyPossessivePosition,
  applyRelativeClausePosition,
  isChoiceSystem,
} from "./grammar/choice.ts";
export {
  CASE_TEMPLATES,
  INVENTORY_SYSTEM_IDS,
  NUMBER_TEMPLATES,
  addCase,
  addNounClass,
  isInventorySystem,
  moveNumberCategory,
  referencedCategoryIds,
  removeCase,
  removeNumberCategory,
  setNounClassKind,
  toggleNumberMarking,
  toggleNumberTemplate,
  toggleTamTemplate,
  updateNumberCategory,
} from "./grammar/inventory.ts";
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
