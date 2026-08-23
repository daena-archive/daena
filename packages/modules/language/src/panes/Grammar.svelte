<script lang="ts">
import { untrack } from "svelte";
import { confirm } from "../confirm.svelte";
import type { EntitySummary, ModuleContext, ModuleRecord } from "../../../../module-api/src/index";
import type { LexemeValue } from "../lexeme";
import { normalizeLexeme } from "../lexeme";
import type { Paradigm } from "../morphology";
import { normalizeParadigm } from "../morphology";
import type { Sample } from "../samples";
import { normalizeSample, sampleTitle } from "../samples";
import {
  GRAMMAR_SECTIONS,
  applyStoredVersion,
  configuredMinimum,
  grammarGlance,
  grammarStatusLabel,
  grammarSystemDescriptor,
  keepDraftAfterConflict,
  openAgreementEditor,
  openAgreementNotUsedEditor,
  openCustomRuleEditor,
  openSystemEditor,
  searchGrammar,
  sectionCardSummary,
  setSystemStatus,
  summarizeSystem,
  systemsForSection,
} from "../grammar.ts";
import { GRAMMAR_STARTER_STEPS, nextStarterSystem, remainingStarterSystems } from "../grammar.ts";
import { emptyGrammarUiState } from "../grammar.ts";
import { loadGrammarIndex } from "../grammar/repository";
import { deleteGrammarRecord } from "../grammar/repository";
import { starterPosition, starterStepLabel } from "../grammar/starter";
import AgreementEditor from "../editor/AgreementEditor.svelte";
import CustomRuleEditor from "../editor/CustomRuleEditor.svelte";
import SystemEditor from "../editor/SystemEditor.svelte";
import { summarizeAgreement } from "../grammar/agreement";
import { isClauseSystem } from "../grammar/clause";
import { isChoiceSystem } from "../grammar/choice";
import { isInventorySystem, referencedCategoryIds } from "../grammar/inventory";
import { isParadigmSystem } from "../grammar/paradigm";
import { isStrategySystem } from "../grammar/strategy";
import {
  deleteGrammarEditor,
  goHome,
  goSection,
  goSystem,
  saveGrammarEditor,
  tryLeaveGrammar,
  type GrammarPaneContext,
} from "../grammar/pane";
import type {
  GrammarAgreementRecord,
  GrammarCustomRuleRecord,
  GrammarEditSession,
  GrammarLink,
  GrammarSearchHit,
  GrammarSectionId,
  GrammarSectionStateRecord,
  GrammarStatus,
  GrammarSystemId,
  GrammarSystemRecord,
  GrammarUiState,
  IndexedGrammar,
} from "../grammar.ts";
import type { ParadigmConfig } from "../grammar/types";

type BreadcrumbItem = { label: string; onclick?: () => void };

let {
  context,
  selectedLanguage,
  active,
  registerLeaveGuard,
  setMutationActive,
  setBreadcrumbExtra,
}: {
  context: ModuleContext;
  selectedLanguage: EntitySummary | null;
  active: boolean;
  registerLeaveGuard: (guard: (() => Promise<boolean> | boolean) | null) => void;
  setMutationActive: (active: boolean) => void;
  setBreadcrumbExtra: (items: BreadcrumbItem[]) => void;
} = $props();

let root: HTMLDivElement | undefined = $state();
let cancelled = $state(false);
let grammarUi: GrammarUiState = $state(emptyGrammarUiState());
let records: ModuleRecord<LexemeValue>[] = $state([]);
let samples: ModuleRecord<Sample>[] = $state([]);
let paradigms: ModuleRecord<Paradigm>[] = $state([]);
let paneLoading = $state(false);
let error = $state("");
let request = $state(0);
let grammarSaving = $state(false);

let lastLoadedLanguage: string | null = null;

$effect(() => {
  const languageId = selectedLanguage?.id ?? null;
  void languageId;
  if (!active) return;
  if (languageId === lastLoadedLanguage) {
    untrack(() => void loadGrammar());
    return;
  }
  lastLoadedLanguage = languageId;
  grammarUi = emptyGrammarUiState();
  untrack(() => void loadGrammar());
});

$effect(() => {
  if (!active) return;
  registerLeaveGuard(() => tryLeaveGrammar(grammarUi, (message) => confirm("Unsaved changes", message)));
  return () => {
    registerLeaveGuard(null);
  };
});

$effect(() => {
  return () => {
    cancelled = true;
  };
});

type AgreementRecord = { id: string; revision: string; value: GrammarAgreementRecord };
type CustomRuleRecord = { id: string; revision: string; value: GrammarCustomRuleRecord };

const windowConfirm = (message: string) => confirm("Confirm", message);

async function loadGrammar() {
  if (!selectedLanguage) {
    grammarUi.index = emptyGrammarUiState().index;
    records = [];
    samples = [];
    paradigms = [];
    paneLoading = false;
    error = "";
    return;
  }
  const token = ++request;
  paneLoading = true;
  error = "";
  try {
    const loaded = await loadGrammarIndex(context.records, selectedLanguage.id);
    const [lexemes, sampleRecords, paradigmRecords] = await Promise.all([
      context.records.list<LexemeValue>("lexemes", selectedLanguage.id, { limit: 500, sort: "lemma" }),
      context.records.list<Sample>("samples", selectedLanguage.id, { limit: 100, sort: "title" }),
      context.records.list<Paradigm>("paradigms", selectedLanguage.id, { limit: 100, sort: "name" }),
    ]);
    if (!cancelled && token === request) {
      paneLoading = false;
      error = "";
      grammarUi.index = loaded.index;
      records = lexemes.map((record) => ({ ...record, value: normalizeLexeme(record.value) }));
      samples = sampleRecords.map((record) => ({ ...record, value: normalizeSample(record.value) }));
      paradigms = paradigmRecords.map((record) => ({ ...record, value: normalizeParadigm(record.value) }));
    }
  } catch (cause) {
    if (!cancelled && token === request) {
      paneLoading = false;
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }
}

const session = $derived(grammarUi.editing);
const section = $derived(GRAMMAR_SECTIONS.find((item) => item.id === grammarUi.section));
const hits = $derived(searchGrammar(grammarUi.query, grammarUi.index));
const starterRemaining = $derived(remainingStarterSystems(grammarUi.index));
const glanceRows = $derived(grammarGlance(grammarUi.index));
const agreementRecords = $derived(
  grammarUi.index.agreements.flatMap((item): AgreementRecord[] =>
    item.value.recordKind === "agreement" ? [{ ...item, value: item.value }] : [],
  ),
);
const customRules = $derived(
  grammarUi.index.customRules.flatMap((item): CustomRuleRecord[] =>
    item.value.recordKind === "custom-rule" ? [{ ...item, value: item.value }] : [],
  ),
);
const unusedAgreementState = $derived.by(() => {
  const loaded = grammarUi.index.sectionStates.get("agreement");
  if (!loaded || loaded.value.recordKind !== "section-state") return undefined;
  return { ...loaded, value: loaded.value } as { id: string; revision: string; value: GrammarSectionStateRecord };
});
const agreementChoices = $derived(
  grammarUi.index.agreements.flatMap((item) =>
    item.value.recordKind === "agreement" ? [{ id: item.id, title: item.value.title }] : [],
  ),
);
const negativeVerbSummary = $derived.by(() => {
  const value = grammarUi.index.systems.get("verbs.negative-forms")?.value;
  return value?.recordKind === "system" ? summarizeSystem("verbs.negative-forms", value) : undefined;
});
const relativePositionSummary = $derived.by(() => {
  const value = grammarUi.index.systems.get("syntax.relative-clause-position")?.value;
  return value?.recordKind === "system" ? summarizeSystem("syntax.relative-clause-position", value) : undefined;
});
const pronounAxes = $derived.by(() => {
  const value = grammarUi.index.systems.get("pronouns.personal")?.value;
  return value?.recordKind === "system" ? (value.config as ParadigmConfig).axes : undefined;
});

const ctx: GrammarPaneContext = $derived({
  languageName: selectedLanguage?.name,
  ownerId: selectedLanguage?.id,
  records: context.records,
  confirm: windowConfirm,
  choices: {
    lexemes: records.map((record) => ({ id: record.id, lemma: record.value.lemma })),
    samples: samples.map((record) => ({ id: record.id, title: sampleTitle(record.value) })),
    paradigms: paradigms.map((record) => ({ id: record.id, name: record.value.name })),
    examples: records.flatMap((record) =>
      record.value.senses.flatMap((sense) =>
        sense.examples.map((example) => ({
          lexemeId: record.id,
          exampleId: example.id,
          lemma: record.value.lemma,
          text: example.text,
        })),
      ),
    ),
  },
});

const editorTitle = $derived.by(() => {
  const current = session;
  if (!current) return "Grammar";
  const value = current.draft;
  if (value.recordKind === "system") return grammarSystemDescriptor(value.systemId)?.label ?? "System";
  if (value.recordKind === "agreement") return current.recordId ? value.title || "Agreement" : "New agreement system";
  if (value.recordKind === "custom-rule") return current.recordId ? "Custom rule" : "New custom rule";
  return "Agreement";
});

$effect(() => {
  if (!active) return;
  const crumbs: BreadcrumbItem[] = [];
  if (session) {
    if (section) {
      crumbs.push({
        label: section.label,
        onclick: async () => {
          if (!(await tryLeaveGrammar(grammarUi, windowConfirm))) return;
          grammarUi.editing = null;
          grammarUi.starterCurrent = undefined;
        },
      });
    }
    crumbs.push({ label: editorTitle });
  } else if (section) {
    crumbs.push({ label: section.label });
  }
  setBreadcrumbExtra(crumbs);
  return () => setBreadcrumbExtra([]);
});

const stored = $derived.by(() => {
  if (!session?.conflict || !session.recordId) return undefined;
  return [
    ...grammarUi.index.systems.values(),
    ...grammarUi.index.customRules,
    ...grammarUi.index.agreements,
    ...grammarUi.index.sectionStates.values(),
  ].find((item) => item.id === session.recordId);
});

let previousSession: GrammarEditSession | null = null;

$effect(() => {
  const current = grammarUi.editing;
  const explicit = grammarUi.focusTarget;
  const target = explicit ?? (current && current !== previousSession ? "#grammar-editor-heading" : undefined);
  previousSession = current;
  if (!target || !root) return;
  untrack(() => {
    grammarUi.focusTarget = undefined;
  });
  root.querySelector<HTMLElement>(target)?.focus();
});

function grammarFocusSelector(draftValue: GrammarEditSession["draft"], recordId?: string) {
  if (draftValue.recordKind === "system") return `[data-grammar-id="system:${draftValue.systemId}"]`;
  if (draftValue.recordKind === "agreement") return `[data-grammar-id="agreement:${recordId ?? ""}"]`;
  if (draftValue.recordKind === "custom-rule") return `[data-grammar-id="rule:${recordId ?? ""}"]`;
  return '[data-grammar-id="section:agreement"]';
}

async function leaveEditor() {
  const current = session;
  if (grammarSaving) return false;
  if (!current || !(await tryLeaveGrammar(grammarUi, windowConfirm))) return false;
  const origin = current.originSection;
  const focus = grammarFocusSelector(current.draft, current.recordId);
  grammarUi.editing = null;
  grammarUi.starterCurrent = undefined;
  grammarUi.section = origin;
  grammarUi.focusTarget = focus;
  return true;
}

async function handleAllSections() {
  await goHome(grammarUi, windowConfirm);
}

async function handleStatusChange(status: GrammarStatus) {
  const current = session;
  const value = current?.draft;
  if (!current || !value || value.recordKind !== "system" || current.locked) return;
  if (value.status === "configured" && status !== "configured") {
    if (!(await windowConfirm("Reset this system's configuration? Unsaved settings in this editor will be cleared.")))
      return;
  }
  current.draft = setSystemStatus(value, status);
}

function advanceStarter(current: GrammarSystemId) {
  const next = nextStarterSystem(grammarUi.index, current);
  if (!next) {
    grammarUi.starterDismissed = true;
    grammarUi.starterCurrent = undefined;
    grammarUi.editing = null;
    grammarUi.section = null;
    grammarUi.focusTarget = '[data-grammar-id="section:syntax"]';
    return;
  }
  grammarUi.starterCurrent = next;
  grammarUi.section = grammarSystemDescriptor(next)?.sectionId ?? grammarUi.section;
  grammarUi.editing = openSystemEditor(grammarUi.index, next);
  grammarUi.focusTarget = "#grammar-editor-heading";
}

async function handleSkip() {
  if (!(await tryLeaveGrammar(grammarUi, windowConfirm))) return;
  if (grammarUi.starterCurrent) advanceStarter(grammarUi.starterCurrent);
}

async function handleExitStarter() {
  if (!(await tryLeaveGrammar(grammarUi, windowConfirm))) return;
  grammarUi.starterDismissed = true;
  grammarUi.starterCurrent = undefined;
  grammarUi.editing = null;
}

async function handleStartStarter() {
  if (!(await tryLeaveGrammar(grammarUi, windowConfirm))) return;
  const next = nextStarterSystem(grammarUi.index);
  if (!next) {
    grammarUi.starterDismissed = true;
    return;
  }
  grammarUi.query = "";
  grammarUi.starterCurrent = next;
  grammarUi.section = grammarSystemDescriptor(next)?.sectionId ?? null;
  grammarUi.editing = openSystemEditor(grammarUi.index, next);
  grammarUi.focusTarget = "#grammar-editor-heading";
}

async function handleDismissStarter() {
  if (!(await tryLeaveGrammar(grammarUi, windowConfirm))) return;
  grammarUi.starterDismissed = true;
  grammarUi.starterCurrent = undefined;
  grammarUi.editing = null;
}

async function handleStartAgreement() {
  if (!(await tryLeaveGrammar(grammarUi, windowConfirm))) return;
  if (grammarUi.index.sectionStates.get("agreement")) {
    if (
      !(await windowConfirm(
        "This section is marked not used. Create an agreement system anyway? Saving it will clear the not-used marker.",
      ))
    ) {
      return;
    }
  }
  grammarUi.section = "agreement";
  grammarUi.query = "";
  grammarUi.starterCurrent = undefined;
  grammarUi.editing = openAgreementEditor(grammarUi.index);
  grammarUi.focusTarget = "#grammar-editor-heading";
}

async function handleOpenAgreementNotUsed() {
  if (!(await tryLeaveGrammar(grammarUi, windowConfirm))) return;
  grammarUi.editing = openAgreementNotUsedEditor(grammarUi.index);
}

async function handleAddCustomRule() {
  if (!(await tryLeaveGrammar(grammarUi, windowConfirm))) return;
  grammarUi.editing = openCustomRuleEditor(grammarUi.index);
}

async function handleSearchHit(hit: GrammarSearchHit) {
  if (hit.kind === "system" && hit.systemId) {
    await goSystem(grammarUi, hit.systemId, windowConfirm);
  } else if (hit.kind === "custom-rule") {
    if (!(await tryLeaveGrammar(grammarUi, windowConfirm))) return;
    grammarUi.section = "other";
    grammarUi.query = "";
    grammarUi.editing = openCustomRuleEditor(grammarUi.index, hit.recordId);
  } else if (hit.kind === "agreement" && hit.recordId) {
    if (!(await tryLeaveGrammar(grammarUi, windowConfirm))) return;
    grammarUi.section = "agreement";
    grammarUi.query = "";
    grammarUi.editing = openAgreementEditor(grammarUi.index, hit.recordId);
  } else {
    await goSection(grammarUi, hit.sectionId, windowConfirm);
  }
}

async function handleSubmit(event: SubmitEvent) {
  event.preventDefault();
  if (grammarSaving || !session || session.locked) return;
  const ownerLanguageId = selectedLanguage?.id;
  if (!ownerLanguageId) return;
  grammarSaving = true;
  setMutationActive(true);
  try {
    const message = await saveGrammarEditor(grammarUi, ctx);
    if (message && grammarUi.editing?.validationFocus) {
      grammarUi.focusTarget = `[name="${CSS.escape(grammarUi.editing.validationFocus)}"]`;
    } else if (message) {
      grammarUi.focusTarget = "#grammar-editor-heading";
    }
  } finally {
    grammarSaving = false;
    setMutationActive(false);
  }
}

async function handleDelete() {
  if (grammarSaving || !session || session.locked) return;
  const ownerLanguageId = selectedLanguage?.id;
  if (!ownerLanguageId) return;
  grammarSaving = true;
  setMutationActive(true);
  try {
    const message = await deleteGrammarEditor(grammarUi, ctx);
    if (message && grammarUi.editing) grammarUi.editing.validationMessage = message;
  } finally {
    grammarSaving = false;
    setMutationActive(false);
  }
}

async function resolveDuplicate(recordId: string, revision: string) {
  if (grammarSaving || !session || !session.duplicates) return;
  const ownerLanguageId = selectedLanguage?.id;
  if (!ownerLanguageId) return;
  if (session.draft.recordKind !== "system") return;
  const systemId = session.draft.systemId;
  if (
    !(await windowConfirm(
      `Remove duplicate record ${recordId.slice(0, 8)}…? The other records for this system are kept.`,
    ))
  )
    return;
  grammarSaving = true;
  setMutationActive(true);
  try {
    const result = await deleteGrammarRecord(ctx.records, ownerLanguageId, { recordId, revision });
    if (result.ok) {
      grammarUi.index = result.index;
      grammarUi.editing = openSystemEditor(result.index, systemId);
      error = "";
    } else if (result.stale) {
      const reopened = openSystemEditor(result.index, systemId);
      grammarUi.index = result.index;
      grammarUi.editing = reopened;
      error = reopened.validationMessage ?? "This record changed; review the duplicates and try again.";
    } else {
      error = result.error;
    }
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  } finally {
    grammarSaving = false;
    setMutationActive(false);
  }
}

function handleLoadStored() {
  if (!stored || !session) return;
  grammarUi.editing = applyStoredVersion(session, stored);
}

function handleKeepDraft() {
  if (!stored || !session) return;
  grammarUi.editing = keepDraftAfterConflict(session, stored);
}

function recordWithExtras(value: GrammarUiState["editing"]) {
  if (!value) return null;
  const draft = value.draft;
  if (draft.recordKind !== "system" && draft.recordKind !== "custom-rule" && draft.recordKind !== "agreement") {
    return null;
  }
  return draft;
}

function addExample() {
  const draft = recordWithExtras(session);
  if (!draft) return;
  draft.examples = [...draft.examples, { id: crypto.randomUUID(), text: "" }];
}

function removeExample(index: number) {
  const draft = recordWithExtras(session);
  if (!draft) return;
  draft.examples = draft.examples.filter((_, item) => item !== index);
}

function handleAddLink(event: Event & { currentTarget: HTMLSelectElement }) {
  const select = event.currentTarget;
  if (!select.value) return;
  const draft = recordWithExtras(session);
  if (!draft) {
    select.value = "";
    return;
  }
  const parsed = JSON.parse(select.value) as GrammarLink;
  const duplicate = draft.links.some(
    (link) => link.kind === parsed.kind && link.targetId === parsed.targetId && link.secondaryId === parsed.secondaryId,
  );
  if (!duplicate) draft.links = [...draft.links, { ...parsed, id: crypto.randomUUID() }];
  select.value = "";
}

function removeLink(index: number) {
  const draft = recordWithExtras(session);
  if (!draft) return;
  draft.links = draft.links.filter((_, item) => item !== index);
}
</script>

<div bind:this={root}>
  <div class="language-toolbar">
    <div class="language-toolbar-title">
      <p class="language-toolbar-eyebrow">Language crafting studio</p>
      <h2>Grammar</h2>
      <p class="language-toolbar-subtitle">
        {selectedLanguage
          ? `${selectedLanguage.name} · systems, examples, and usage patterns`
          : "Select a language to document its grammar."}
      </p>
    </div>
    {#if grammarUi.section || grammarUi.editing}
      <div class="language-toolbar-actions">
        <button type="button" class="language-button secondary" onclick={handleAllSections}>All sections</button>
      </div>
    {/if}
  </div>
  {#if error}
    <div class="language-empty-card language-error-card">
      <p class="language-status error" role="alert">{error}</p>
      {#if selectedLanguage}
        <button type="button" class="language-button secondary" onclick={() => void loadGrammar()}>Try again</button>
      {/if}
    </div>
  {:else if !selectedLanguage}
    <div class="language-empty-card">
      <p class="language-empty" role="status">Select a language to document its grammar.</p>
    </div>
  {:else if paneLoading}
    <p class="language-empty language-loading" aria-live="polite" role="status">Loading grammar systems…</p>
  {:else if session}
    <div class="language-toolbar">
      <button type="button" class="language-button secondary" onclick={leaveEditor}>Back</button>
      {#if grammarUi.starterCurrent && session.draft.recordKind === "system"}
        {@const position = starterPosition(grammarUi.starterCurrent)}
        <span>Starter {position.current} of {position.total}</span>
        <button type="button" class="language-button secondary" onclick={handleSkip}>Skip</button>
        <button type="button" class="language-button secondary" onclick={handleExitStarter}>Exit starter</button>
      {/if}
    </div>
    <form class="language-editor" onsubmit={handleSubmit}>
      <h2 id="grammar-editor-heading" tabindex="-1">{editorTitle}</h2>
      {#if session.draft.recordKind === "system"}
        {@const systemDraft = session.draft}
        <p class="language-empty" role="status">{grammarSystemDescriptor(systemDraft.systemId)?.hint}</p>
        {#if systemDraft.status === "not-used"}
          <label class="language-field">
            <span>Why it is not used (optional)</span>
            <textarea
              rows="3"
              placeholder="Noun roles are primarily expressed through word order and adpositions."
              value={systemDraft.notes}
              disabled={session.locked}
              oninput={(event) => (systemDraft.notes = event.currentTarget.value)}></textarea>
          </label>
        {:else}
          {#if configuredMinimum(systemDraft.systemId, systemDraft.config)}
            <p class="language-empty" role="status">{summarizeSystem(systemDraft.systemId, systemDraft)}</p>
          {/if}
          <SystemEditor
            draft={systemDraft}
            locked={session.locked}
            lexemes={ctx.choices.lexemes}
            referencedIds={referencedCategoryIds(grammarUi.index, systemDraft.systemId)}
            confirm={windowConfirm}
            agreements={agreementChoices}
            {negativeVerbSummary}
            {relativePositionSummary}
            {pronounAxes} />
          <label class="language-field">
            <span>Notes</span>
            <textarea
              rows="4"
              value={systemDraft.notes}
              disabled={session.locked}
              oninput={(event) => (systemDraft.notes = event.currentTarget.value)}></textarea>
          </label>
        {/if}
        {#if systemDraft.status === "not-used"}
          <div class="language-inline">
            <button
              type="button"
              class="language-button"
              disabled={session.locked}
              onclick={() => handleStatusChange("configured")}>Configure this section</button>
          </div>
        {:else}
          <div class="language-inline">
            <button
              type="button"
              class="language-button secondary language-danger"
              disabled={session.locked}
              onclick={() => handleStatusChange("not-used")}>Mark as not used</button>
          </div>
        {/if}
      {:else if session.draft.recordKind === "agreement"}
        {@const agreementDraft = session.draft}
        <AgreementEditor draft={agreementDraft} locked={session.locked} index={grammarUi.index} />
        <label class="language-field">
          <span>Notes</span>
          <textarea
            rows="4"
            value={agreementDraft.notes}
            disabled={session.locked}
            oninput={(event) => (agreementDraft.notes = event.currentTarget.value)}></textarea>
        </label>
      {:else if session.draft.recordKind === "custom-rule"}
        {@const customRuleDraft = session.draft}
        <CustomRuleEditor draft={customRuleDraft} locked={session.locked} />
      {:else if session.draft.recordKind === "section-state"}
        {@const sectionDraft = session.draft}
        <p class="language-empty" role="status">
          If your language does not use agreement, you can mark this section as not used.
        </p>
        <label class="language-field">
          <span>Note (optional)</span>
          <textarea
            rows="3"
            value={sectionDraft.note ?? ""}
            disabled={session.locked}
            oninput={(event) => (sectionDraft.note = event.currentTarget.value)}></textarea>
        </label>
      {/if}
      {#if session.draft.recordKind === "system" || session.draft.recordKind === "custom-rule" || session.draft.recordKind === "agreement"}
        {@const recordDraft = session.draft}
        <section class="language-group">
          <h3>Examples</h3>
          <p class="language-empty" role="status">Add a sentence, and optionally a translation, gloss, or notes.</p>
          {#each recordDraft.examples as example, index (example.id)}
            <div class="grammar-example">
              <label class="language-field">
                <span>Example</span>
                <textarea
                  rows="2"
                  placeholder="Nar bel tor."
                  value={example.text}
                  disabled={session.locked}
                  oninput={(event) => (example.text = event.currentTarget.value)}></textarea>
              </label>
              <label class="language-field">
                <span>Translation (optional)</span>
                <input
                  placeholder="I eat bread."
                  value={example.translation ?? ""}
                  disabled={session.locked}
                  oninput={(event) => (example.translation = event.currentTarget.value)} />
              </label>
              <label class="language-field">
                <span>Gloss (optional)</span>
                <input
                  placeholder="1sg bread eat"
                  value={example.gloss ?? ""}
                  disabled={session.locked}
                  oninput={(event) => (example.gloss = event.currentTarget.value)} />
              </label>
              <label class="language-field">
                <span>Notes (optional)</span>
                <textarea
                  rows="2"
                  value={example.notes ?? ""}
                  disabled={session.locked}
                  oninput={(event) => (example.notes = event.currentTarget.value)}></textarea>
              </label>
              {#if !session.locked}
                <button
                  type="button"
                  class="language-button secondary language-danger"
                  onclick={() => removeExample(index)}>Remove example</button>
              {/if}
            </div>
          {/each}
          {#if !session.locked}
            <button type="button" class="language-button secondary" onclick={addExample}>Add example</button>
          {/if}
        </section>
        <section class="language-group">
          <h3>Links</h3>
          {#each recordDraft.links as link, index (link.id)}
            <div class="language-inline">
              <span>{link.kind}: {link.label || link.targetId}</span>
              {#if !session.locked}
                <button
                  type="button"
                  class="language-button secondary language-danger"
                  onclick={() => removeLink(index)}>Remove</button>
              {/if}
            </div>
          {/each}
          {#if !session.locked}
            <select aria-label="Link a record" onchange={handleAddLink}>
              <option value="">Link a word, sample, or paradigm…</option>
              {#each ctx.choices.lexemes as lexeme (lexeme.id)}
                <option value={JSON.stringify({ kind: "lexeme", targetId: lexeme.id, label: lexeme.lemma })}
                  >Word:
                  {lexeme.lemma}</option>
              {/each}
              {#each ctx.choices.examples as example (example.lexemeId + example.exampleId)}
                <option
                  value={JSON.stringify({
                    kind: "lexeme-example",
                    targetId: example.lexemeId,
                    secondaryId: example.exampleId,
                    label: example.text,
                  })}>Example: {example.lemma} — {example.text}</option>
              {/each}
              {#each ctx.choices.samples as sample (sample.id)}
                <option value={JSON.stringify({ kind: "sample", targetId: sample.id, label: sample.title })}
                  >Sample:
                  {sample.title}</option>
              {/each}
              {#each ctx.choices.paradigms as paradigm (paradigm.id)}
                <option value={JSON.stringify({ kind: "paradigm", targetId: paradigm.id, label: paradigm.name })}
                  >Paradigm:
                  {paradigm.name}</option>
              {/each}
            </select>
          {/if}
        </section>
      {/if}
      {#if error || session.validationMessage}
        <p class="language-status error" role="alert">{session.validationMessage || error}</p>
      {/if}
      {#if session.duplicates && session.duplicates.length > 1}
        <section class="language-group" aria-label="Resolve duplicate records">
          <h3>Resolve duplicate records</h3>
          <p class="language-empty" role="status">
            Keep one record for {editorTitle} and remove the duplicates below. Edits stay locked until a single record remains.
          </p>
          <ul class="grammar-duplicate-list">
            {#each session.duplicates as record (record.id)}
              <li>
                <span class="grammar-duplicate-id">{record.id}</span>
                <button
                  type="button"
                  class="language-button secondary language-danger"
                  disabled={grammarSaving}
                  onclick={() => void resolveDuplicate(record.id, record.revision)}>
                  {grammarSaving ? "Removing…" : "Remove this record"}
                </button>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
      {#if session.conflict && session.recordId && stored}
        <div class="language-inline">
          <button type="button" class="language-button secondary" onclick={handleLoadStored}
            >Load stored version</button>
          <button type="button" class="language-button secondary" onclick={handleKeepDraft}>Keep my draft</button>
        </div>
      {/if}
      <div class="language-actions">
        <span>
          {#if session.recordId && !session.locked}
            <button
              type="button"
              class="language-button secondary language-danger"
              aria-label={`Delete ${editorTitle}`}
              disabled={grammarSaving}
              onclick={handleDelete}>Delete</button>
          {/if}
        </span>
        <span>
          <button type="button" class="language-button secondary" disabled={grammarSaving} onclick={leaveEditor}
            >Cancel</button>
          <button type="submit" class="language-button" disabled={session.locked || grammarSaving}
            >{grammarSaving ? "Saving…" : "Save"}</button>
        </span>
      </div>
    </form>
  {:else}
    <div class="language-search-row">
      <label class="language-field">
        <span>Search grammar systems</span>
        <input
          name="grammar-search"
          placeholder="Search grammar systems…"
          aria-label="Search grammar systems"
          value={grammarUi.query}
          oninput={(event) => (grammarUi.query = event.currentTarget.value)} />
      </label>
    </div>
    <div class="grammar-home">
      {#each grammarUi.index.diagnostics as diagnostic}
        <div class="grammar-diagnostic" role="group" aria-label="Grammar diagnostic">
          <p class="language-status error" role="alert">{diagnostic.message}</p>
          {#if diagnostic.systemId}
            <button
              type="button"
              class="language-button secondary"
              onclick={async () => {
                await goSystem(grammarUi, diagnostic.systemId!, windowConfirm);
              }}
              >Open
              {grammarSystemDescriptor(diagnostic.systemId)?.label ?? diagnostic.systemId}</button>
          {:else if diagnostic.recordIds[0] && grammarUi.index.agreements.some((item) => item.id === diagnostic.recordIds[0])}
            <button
              type="button"
              class="language-button secondary"
              onclick={async () => {
                if (!(await tryLeaveGrammar(grammarUi, windowConfirm))) return;
                grammarUi.section = "agreement";
                grammarUi.editing = openAgreementEditor(grammarUi.index, diagnostic.recordIds[0]);
                grammarUi.focusTarget = "#grammar-editor-heading";
              }}>Open agreement</button>
          {/if}
        </div>
      {/each}
      {#if grammarUi.query.trim()}
        <div class="grammar-systems">
          {#if hits.length === 0}
            <p class="language-empty" role="status">No matching grammar systems.</p>
          {/if}
          {#each hits as hit}
            <button type="button" class="grammar-system" onclick={() => handleSearchHit(hit)}>
              <strong>{hit.label}</strong>
              <span
                >{GRAMMAR_SECTIONS.find((entry) => entry.id === hit.sectionId)?.label ?? ""} ·
                {hit.status ? grammarStatusLabel(hit.status) : hit.summary}</span>
            </button>
          {/each}
        </div>
      {:else if !grammarUi.section}
        <p class="language-empty" role="status">
          Define how sentences and words behave in this language. You do not need to configure every system.
        </p>
        {#if !grammarUi.starterDismissed && starterRemaining.length > 0}
          <section class="language-empty-card" data-grammar-id="starter">
            <p class="language-empty" role="status">Start your grammar</p>
            <p class="language-empty" role="status">
              Choose a few foundational systems now. Everything can be changed later.
            </p>
            <ol class="grammar-starter-list">
              {#each starterRemaining as systemId (systemId)}
                <li>{starterStepLabel(systemId)}</li>
              {/each}
            </ol>
            <div class="language-inline">
              <button type="button" class="language-button" data-grammar-id="starter-start" onclick={handleStartStarter}
                >{starterRemaining.length === GRAMMAR_STARTER_STEPS.length ? "Start" : "Continue starter"}</button>
              <button type="button" class="language-button secondary" onclick={handleDismissStarter}
                >I'll configure grammar manually</button>
            </div>
          </section>
        {/if}
        <div class="grammar-cards">
          {#each GRAMMAR_SECTIONS as entry (entry.id)}
            {@const summary = sectionCardSummary(grammarUi.index, entry.id)}
            {@const notUsed = summary.notUsed ? ` · ${summary.notUsed} not used` : ""}
            {@const progress = summary.total > 0 ? Math.round((summary.configured / summary.total) * 100) : 0}
            <button
              type="button"
              class="grammar-card"
              data-grammar-id={`section:${entry.id}`}
              aria-label={`${summary.label}: ${summary.detail}${notUsed}`}
              onclick={async () => {
                await goSection(grammarUi, entry.id, windowConfirm);
              }}>
              <div class="grammar-card-header">
                <strong>{summary.label}</strong>
                {#if summary.total > 0}
                  <span class="grammar-card-progress">{progress}%</span>
                {/if}
              </div>
              <span class="grammar-card-detail">{summary.detail}{notUsed}</span>
              {#if summary.total > 0}
                <div class="grammar-progress-bar">
                  <div class="grammar-progress-fill" style="width: {progress}%"></div>
                </div>
              {/if}
            </button>
          {/each}
        </div>
        <dl class="grammar-glance" aria-label="At a glance">
          {#each glanceRows as row}
            <dt>{row.label}</dt>
            <dd>{row.value}</dd>
          {/each}
        </dl>
      {:else}
        {@const currentSection = section!}
        <h3>{currentSection.label}</h3>
        <p class="language-empty" role="status">{currentSection.orientation}</p>
        {#if currentSection.id === "agreement"}
          {#if unusedAgreementState}
            <p class="language-empty" role="status">Not used</p>
            {#if unusedAgreementState.value.note}
              <p class="language-empty" role="status">{unusedAgreementState.value.note}</p>
            {/if}
            <div class="language-inline">
              <button type="button" class="language-button" onclick={handleStartAgreement}>Add agreement system</button>
              <button type="button" class="language-button secondary" onclick={handleOpenAgreementNotUsed}>Edit</button>
            </div>
          {:else if agreementRecords.length === 0}
            <div class="language-empty-card">
              <p class="language-empty" role="status">{currentSection.emptyBody}</p>
              <div class="language-inline">
                <button type="button" class="language-button" onclick={handleStartAgreement}
                  >Add agreement system</button>
                <button type="button" class="language-button secondary" onclick={handleOpenAgreementNotUsed}
                  >Mark as not used</button>
              </div>
            </div>
          {:else}
            <button type="button" class="language-button" onclick={handleStartAgreement}>Add agreement system</button>
            <div class="grammar-systems">
              {#each agreementRecords as record (record.id)}
                <button
                  type="button"
                  class="grammar-system"
                  data-grammar-id={`agreement:${record.id}`}
                  aria-label={`${record.value.title}: ${summarizeAgreement(record.value)}`}
                  onclick={async () => {
                    if (!(await tryLeaveGrammar(grammarUi, windowConfirm))) return;
                    grammarUi.editing = openAgreementEditor(grammarUi.index, record.id);
                    grammarUi.focusTarget = "#grammar-editor-heading";
                  }}>
                  <strong>{record.value.title}</strong>
                  <span>{summarizeAgreement(record.value)}</span>
                </button>
              {/each}
            </div>
          {/if}
        {:else if currentSection.id === "other"}
          {#if customRules.length === 0}
            <div class="language-empty-card">
              <p class="language-empty" role="status">{currentSection.emptyBody}</p>
              <div class="language-inline">
                <button type="button" class="language-button" onclick={handleAddCustomRule}>Add a custom rule</button>
              </div>
            </div>
          {:else}
            <button type="button" class="language-button" onclick={handleAddCustomRule}>Add a custom rule</button>
            <div class="grammar-systems">
              {#each customRules as record (record.id)}
                <button
                  type="button"
                  class="grammar-system"
                  data-grammar-id={`rule:${record.id}`}
                  aria-label={record.value.title}
                  onclick={async () => {
                    if (!(await tryLeaveGrammar(grammarUi, windowConfirm))) return;
                    grammarUi.editing = openCustomRuleEditor(grammarUi.index, record.id);
                    grammarUi.focusTarget = "#grammar-editor-heading";
                  }}>
                  <strong>{record.value.title}</strong>
                  <span>{record.value.tags.join(", ") || record.value.body.split("\n")[0] || "Custom rule"}</span>
                </button>
              {/each}
            </div>
          {/if}
        {:else}
          {@const listed = systemsForSection(currentSection.id)}
          {#if listed.every((system) => !grammarUi.index.systems.has(system.id) && !grammarUi.index.duplicates.has(system.id))}
            {@const first = listed[0]}
            <div class="language-empty-card">
              <p class="language-empty" role="status">{currentSection.emptyBody}</p>
              {#if first}
                <div class="language-inline">
                  <button
                    type="button"
                    class="language-button"
                    onclick={async () => {
                      await goSystem(grammarUi, first.id, windowConfirm);
                    }}>{first.emptyAction}</button>
                </div>
              {/if}
            </div>
          {/if}
          <div class="grammar-systems">
            {#each listed as system (system.id)}
              {@const record = grammarUi.index.systems.get(system.id)?.value}
              {@const duplicate = grammarUi.index.duplicates.has(system.id)}
              {@const summary = duplicate
                ? "Duplicate records — edits disabled"
                : record?.recordKind === "system"
                  ? summarizeSystem(system.id, record)
                  : grammarStatusLabel("unconfigured")}
              <button
                type="button"
                class="grammar-system"
                data-grammar-id={`system:${system.id}`}
                aria-label={`${system.label}: ${summary}`}
                onclick={async () => {
                  await goSystem(grammarUi, system.id, windowConfirm);
                }}>
                <strong>{system.label}</strong>
                <span>{summary}</span>
              </button>
            {/each}
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
.language-toolbar-eyebrow {
  margin: 0 0 5px;
  color: var(--accent);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}
.language-toolbar-subtitle {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.55;
}
.language-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}
.language-toolbar-title {
  display: grid;
  gap: 3px;
}
.language-toolbar-title h2 {
  margin: 0;
}
.language-toolbar-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.language-search-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 10px;
  margin-top: 16px;
}
.language-field {
  display: grid;
  gap: 6px;
  min-width: 0;
  color: var(--ink-soft);
  font-size: 11px;
  letter-spacing: 0.01em;
}
.language-field input,
.language-field textarea {
  box-sizing: border-box;
  width: 100%;
  min-width: 0;
  padding: 9px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font: inherit;
}
.language-field textarea {
  min-height: 4.5em;
  resize: vertical;
}
.grammar-card:focus-visible,
.grammar-system:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
.language-empty,
.language-status {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.6;
}
.language-status.error {
  color: #a14f42;
}
.language-error-card {
  border-color: #e2b7af;
  background: #fff5f2;
}
.language-loading {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--ink-soft);
}
.language-loading::before {
  content: "";
  width: 11px;
  height: 11px;
  flex: 0 0 11px;
  border: 2px solid var(--line);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: language-spin 0.75s linear infinite;
}
.language-empty-card {
  display: grid;
  gap: 12px;
  justify-items: start;
  margin: 18px 0;
  padding: 20px;
  border: 1px dashed var(--line);
  border-radius: 12px;
  background: var(--surface-muted);
}
.language-editor {
  display: grid;
  gap: 16px;
  margin-top: 16px;
  min-width: 0;
}
.language-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  flex-wrap: wrap;
  margin: 0 -20px -24px;
  padding: 12px 20px 24px;
  border-top: 1px solid var(--line);
  background: var(--surface);
  box-shadow: 0 -8px 16px -16px rgba(38, 42, 33, 0.4);
}
.language-actions span {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.language-inline {
  display: flex;
  align-items: end;
  gap: 8px;
  min-width: 0;
}
.grammar-home {
  display: grid;
  gap: 16px;
  margin-top: 14px;
}
.grammar-cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 10px;
}
.grammar-card,
.grammar-system {
  display: grid;
  gap: 6px;
  width: 100%;
  padding: 12px;
  border: 1px solid #ebe7de;
  border-radius: 10px;
  background: var(--surface);
  color: inherit;
  text-align: left;
  cursor: pointer;
}
.grammar-card:hover,
.grammar-system:hover {
  border-color: #e5d8c6;
  background: var(--surface-muted);
}
.grammar-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.grammar-card-header strong,
.grammar-system strong {
  font-size: 14px;
}
.grammar-card-progress {
  font-size: 11px;
  font-weight: 600;
  color: var(--accent-dark);
}
.grammar-card-detail,
.grammar-system span,
.grammar-glance dd {
  color: var(--ink-soft);
  font-size: 12px;
}
.grammar-progress-bar {
  height: 4px;
  background: var(--line);
  border-radius: 2px;
  overflow: hidden;
}
.grammar-progress-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 2px;
  transition: width 0.3s ease;
}
.grammar-glance {
  display: grid;
  grid-template-columns: minmax(8rem, 12rem) minmax(0, 1fr);
  gap: 6px 14px;
  margin: 0;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface-muted);
}
.grammar-glance dt {
  margin: 0;
  color: var(--ink-faint);
  font-size: 11px;
}
.grammar-glance dd {
  margin: 0;
}
.grammar-systems {
  display: grid;
  gap: 8px;
}
.grammar-starter-list {
  margin: 0;
  padding-left: 1.2em;
}
.grammar-diagnostic {
  display: grid;
  gap: 8px;
  justify-items: start;
}
.grammar-example {
  display: grid;
  gap: 8px;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface-muted);
}
.grammar-duplicate-list {
  display: grid;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.grammar-duplicate-list li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
}
.grammar-duplicate-id {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 11px;
  color: var(--ink-soft);
}
@keyframes language-spin {
  to {
    transform: rotate(360deg);
  }
}
@keyframes language-pulse {
  50% {
    opacity: 0.35;
  }
}
@media (prefers-reduced-motion: reduce) {
  .language-loading::before {
    animation: none;
  }
}
@media (max-width: 760px) {
  .language-inline {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
