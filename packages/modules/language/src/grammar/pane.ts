import { grammarSystemDescriptor } from "../grammar.ts";
import type { UUID } from "../../../../module-api/src/index";
import { deleteGrammarRecord, persistGrammarRecord, type GrammarRecordsApi } from "./repository.ts";
import { nextStarterSystem } from "./starter.ts";
import { confirmGrammarLeave, openSystemEditor, type GrammarEditSession, type GrammarUiState } from "./session.ts";
import type { GrammarSectionId, GrammarSystemId } from "./types.ts";

export type GrammarLinkChoices = {
  lexemes: { id: string; lemma: string }[];
  samples: { id: string; title: string }[];
  paradigms: { id: string; name: string }[];
  examples: { lexemeId: string; exampleId: string; lemma: string; text: string }[];
};

export type GrammarPaneContext = {
  languageName?: string;
  ownerId?: UUID;
  records: GrammarRecordsApi;
  confirm: (message: string) => boolean;
  choices: GrammarLinkChoices;
};

export function tryLeaveGrammar(state: GrammarUiState, confirm: (message: string) => boolean) {
  return confirmGrammarLeave(state.editing, confirm);
}

export function goHome(state: GrammarUiState, confirm: (message: string) => boolean) {
  if (!tryLeaveGrammar(state, confirm)) return false;
  state.editing = null;
  state.section = null;
  state.query = "";
  state.starterCurrent = undefined;
  return true;
}

export function goSection(state: GrammarUiState, sectionId: GrammarSectionId, confirm: (message: string) => boolean) {
  if (!tryLeaveGrammar(state, confirm)) return false;
  state.editing = null;
  state.section = sectionId;
  state.query = "";
  return true;
}

function advanceStarter(state: GrammarUiState, current: GrammarSystemId) {
  const next = nextStarterSystem(state.index, current);
  if (!next) {
    state.starterDismissed = true;
    state.starterCurrent = undefined;
    state.editing = null;
    state.section = null;
    state.focusTarget = '[data-grammar-id="section:syntax"]';
    return;
  }
  state.starterCurrent = next;
  state.section = grammarSystemDescriptor(next)?.sectionId ?? state.section;
  state.editing = openSystemEditor(state.index, next);
  state.focusTarget = "#grammar-editor-heading";
}

function grammarFocusSelector(draft: GrammarEditSession["draft"], recordId?: string) {
  if (draft.recordKind === "system") return `[data-grammar-id="system:${draft.systemId}"]`;
  if (draft.recordKind === "agreement") return `[data-grammar-id="agreement:${recordId ?? ""}"]`;
  if (draft.recordKind === "custom-rule") return `[data-grammar-id="rule:${recordId ?? ""}"]`;
  return '[data-grammar-id="section:agreement"]';
}

export function goSystem(state: GrammarUiState, systemId: GrammarSystemId, confirm: (message: string) => boolean) {
  if (!tryLeaveGrammar(state, confirm)) return false;
  state.query = "";
  state.starterCurrent = undefined;
  state.section = grammarSystemDescriptor(systemId)?.sectionId ?? state.section;
  state.editing = openSystemEditor(state.index, systemId);
  state.focusTarget = "#grammar-editor-heading";
  return true;
}

export async function saveGrammarEditor(state: GrammarUiState, ctx: GrammarPaneContext) {
  if (!ctx.ownerId || !state.editing || state.editing.locked) return "This system cannot be edited.";
  const session = state.editing;
  const result = await persistGrammarRecord(ctx.records, ctx.ownerId, session);
  if (result.ok) {
    state.index = result.index;
    if (state.starterCurrent && session.draft.recordKind === "system") {
      advanceStarter(state, session.draft.systemId);
    } else {
      state.editing = null;
      state.section = session.originSection;
      state.focusTarget = grammarFocusSelector(session.draft, result.record?.id ?? session.recordId);
    }
    return "";
  }
  if (result.stale) {
    state.index = result.index;
    state.editing.conflict = true;
    state.editing.validationMessage =
      "This record changed since you opened it. Your draft is kept. Load the stored version or overwrite it.";
    return state.editing.validationMessage;
  }
  state.editing.validationMessage = result.error;
  state.editing.validationFocus = result.issues?.[0]?.path;
  return result.error;
}

export async function deleteGrammarEditor(state: GrammarUiState, ctx: GrammarPaneContext) {
  if (!ctx.ownerId || !state.editing?.recordId || state.editing.locked) return "";
  const session = state.editing;
  const recordId = session.recordId;
  if (!recordId) return "";
  const title =
    session.draft.recordKind === "system"
      ? (grammarSystemDescriptor(session.draft.systemId)?.label ?? "this system")
      : "title" in session.draft
        ? session.draft.title
        : "this record";
  if (!ctx.confirm(`Delete “${title}”?`)) return "";
  const result = await deleteGrammarRecord(ctx.records, ctx.ownerId, {
    recordId,
    revision: session.revision,
  });
  if (result.ok) {
    state.index = result.index;
    state.editing = null;
    state.section = session.originSection;
    state.focusTarget = grammarFocusSelector(session.draft, session.recordId);
    return "";
  }
  if (result.stale) {
    state.index = result.index;
    state.editing.conflict = true;
    state.editing.validationMessage = "This record changed since you opened it. Your draft is kept.";
    return state.editing.validationMessage;
  }
  return result.error;
}
