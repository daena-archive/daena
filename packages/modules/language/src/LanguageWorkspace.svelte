<script lang="ts">
import { onMount } from "svelte";
import type {
  EntityRecord,
  EntitySummary,
  ModuleContext,
  ModuleManifest,
  ModuleRecord,
  ModuleRecordQuery,
} from "../../../module-api/src/index";
import manifestJson from "../manifest.json";
import type { LexemeValue } from "./lexeme";
import { emptyLexeme, lexiconExport, normalizeLexeme, parseLexiconImport, serializeLexeme } from "./lexeme";
import Lexicon from "./panes/Lexicon.svelte";
import type { OrthographyValue } from "./orthography";
import { emptyOrthography, normalizeOrthography } from "./orthography";
import type { GrammarUiState } from "./grammar";
import { emptyGrammarUiState } from "./grammar";
import { loadGrammarIndex } from "./grammar/repository";
import { tryLeaveGrammar } from "./grammar/pane";
import type { Paradigm, ParadigmSlot } from "./morphology";
import { clearOverride, emptyParadigm, normalizeParadigm, pinOverride, serializeParadigm } from "./morphology";
import {
  emptyPhoneme,
  emptyPhonologyNotes,
  normalizePhoneme,
  normalizePhonologyNotes,
  serializePhoneme,
  serializePhonologyNotes,
  type PhonemeValue,
  type PhonologyNotes,
} from "./phonology";
import type { Sample, SampleKind } from "./samples";
import { emptySample, normalizeSample, sampleTitle, serializeSample } from "./samples";
import { serializeOrthography } from "./orthography";
import Overview from "./panes/Overview.svelte";
import Grammar from "./panes/Grammar.svelte";
import Forms from "./panes/Forms.svelte";
import Samples from "./panes/Samples.svelte";
import Sounds from "./panes/Sounds.svelte";
import Writing from "./panes/Writing.svelte";
import type { FieldDefinition } from "../../../plugin-sdk/src/generated";

type Pane = "overview" | "lexicon" | "sounds" | "writing" | "grammar" | "forms" | "samples";

const PANES: [Pane, string][] = [
  ["overview", "Overview"],
  ["lexicon", "Lexicon"],
  ["sounds", "Sounds"],
  ["writing", "Writing"],
  ["grammar", "Grammar"],
  ["forms", "Forms"],
  ["samples", "Samples"],
];

const manifest = manifestJson as unknown as ModuleManifest;

let { context }: { context: ModuleContext } = $props();

let cancelled = false;
let selectedLanguage: EntitySummary | null = $state(null);
let records: ModuleRecord<LexemeValue>[] = $state([]);
let editing: ModuleRecord<LexemeValue> | null = $state(null);
let editorOpen = $state(false);
let draft: LexemeValue = $state(emptyLexeme());
let search = $state("");
let statusFilterInput = $state("");
let tagFilterInput = $state("");
const statusFilter = $derived(statusFilterInput.trim());
const tagFilter = $derived(tagFilterInput.trim());
let sort: ModuleRecordQuery["sort"] = $state("lemma");
let homonymsOnly = $state(false);
let page = $state(0);
let hasNextPage = $state(false);
let homonymCount = $state(0);
let request = $state(0);
let languageRequest = $state(0);
let searchTimer: number | null = $state(null);
let pane: Pane = $state("overview");
let phonemes: ModuleRecord<PhonemeValue>[] = $state([]);
let phonemeEditing: ModuleRecord<PhonemeValue> | null = $state(null);
let phonemeEditorOpen = $state(false);
let phonemeDraft: PhonemeValue = $state(emptyPhoneme());
let phonologyRecord: ModuleRecord<PhonologyNotes> | null = $state(null);
let phonologyDraft: PhonologyNotes = $state(emptyPhonologyNotes());
let phonologyNotesOpen = $state(false);
let orthographies: ModuleRecord<OrthographyValue>[] = $state([]);
let orthographyEditing: ModuleRecord<OrthographyValue> | null = $state(null);
let orthographyEditorOpen = $state(false);
let orthographyDraft: OrthographyValue = $state(emptyOrthography());
let grammarUi: GrammarUiState = $state(emptyGrammarUiState());
let pendingLexemeId: string | null = $state(null);
let paradigms: ModuleRecord<Paradigm>[] = $state([]);
let paradigmEditing: ModuleRecord<Paradigm> | null = $state(null);
let paradigmEditorOpen = $state(false);
let paradigmDraft: Paradigm = $state(emptyParadigm());
let previewStem = $state("");
let previewLexemeId = $state("");
let samples: ModuleRecord<Sample>[] = $state([]);
let sampleEditing: ModuleRecord<Sample> | null = $state(null);
let sampleEditorOpen = $state(false);
let sampleDraft: Sample = $state(emptySample());
let languageQuery = $state("");
let languageSummaries: EntitySummary[] = $state([]);
let languageListLoaded = $state(false);
let languageLoading = $state(false);
let languageLoadError = $state("");
let creatingLanguage = $state(false);
let languageCreateName = $state("");
let languageCreateError = $state("");
let createBusy = $state(false);
let overviewEntity: EntityRecord | null = $state(null);
let overviewName = $state("");
let overviewFields: Record<string, unknown> = $state({});
let overviewSavedFields: Record<string, unknown> = $state({});
let overviewFieldRevisions: Record<string, string> = $state({});
let overviewDocument = $state("");
let overviewSavedDocument = $state("");
let overviewDocumentRevision = $state("");
let overviewLoading = $state(false);
let overviewSaving = $state(false);
let overviewSavingAutomatically = $state(false);
let overviewDeleting = $state(false);
let overviewDirty = $state(false);
let overviewError = $state("");
let overviewRequest = $state(0);
let overviewAutosaveTimer: number | null = $state(null);
let overviewAutosaveQueued = $state(false);
let paneLoading = $state(false);
let lexiconLoading = $state(false);
let lexiconSaving = $state(false);
let error = $state("");

let paneListEl: HTMLDivElement | undefined = $state();
let createNameInput: HTMLInputElement | undefined = $state();

$effect(() => {
  if (creatingLanguage && createNameInput) createNameInput.focus();
});

onMount(() => {
  languageLoading = true;
  void loadLanguages();
  return () => {
    cancelled = true;
    if (searchTimer !== null) window.clearTimeout(searchTimer);
    if (overviewAutosaveTimer !== null) window.clearTimeout(overviewAutosaveTimer);
  };
});

const overviewFieldDefinitions = $derived(
  manifest.schemas.flatMap((schema) => schema.fields).filter((field) => !field.relationshipType),
);

let filtersInitialized = false;

$effect(() => {
  void search;
  void statusFilter;
  void tagFilter;
  void sort;
  void homonymsOnly;
  if (!filtersInitialized) {
    filtersInitialized = true;
    return;
  }
  scheduleLoad();
});

function clearOverviewAutosave() {
  if (overviewAutosaveTimer !== null) window.clearTimeout(overviewAutosaveTimer);
  overviewAutosaveTimer = null;
  overviewAutosaveQueued = false;
}

function tryLeaveOverview(confirmLeave: (message: string) => boolean) {
  if (!overviewDirty) {
    clearOverviewAutosave();
    return true;
  }
  const allowed = confirmLeave("You have unsaved language details. Leave without saving?");
  if (allowed) {
    clearOverviewAutosave();
    overviewDirty = false;
    overviewError = "";
  }
  return allowed;
}

function syncOverviewDirty() {
  const nameDirty = overviewName.trim() !== overviewEntity?.name;
  const fieldsDirty = overviewFieldDefinitions.some(
    (definition) =>
      JSON.stringify(overviewFields[definition.key] ?? "") !==
      JSON.stringify(overviewSavedFields[definition.key] ?? ""),
  );
  overviewDirty = nameDirty || fieldsDirty || overviewDocument !== overviewSavedDocument;
}

function scheduleOverviewAutosave() {
  if (!overviewDirty || !selectedLanguage || !overviewEntity || overviewDeleting) {
    if (!overviewDirty) clearOverviewAutosave();
    return;
  }
  if (overviewSaving) {
    overviewAutosaveQueued = true;
    return;
  }
  if (overviewAutosaveTimer !== null) window.clearTimeout(overviewAutosaveTimer);
  overviewAutosaveTimer = window.setTimeout(() => {
    overviewAutosaveTimer = null;
    void saveOverview(true);
  }, 800);
}

function onOverviewNameInput(value: string) {
  overviewName = value;
  syncOverviewDirty();
  scheduleOverviewAutosave();
}

function onOverviewFieldInput(definition: FieldDefinition, raw: string) {
  overviewFields = {
    ...overviewFields,
    [definition.key]: definition.multiple
      ? raw
          .split(/[,\n]/)
          .map((item) => item.trim())
          .filter(Boolean)
      : raw,
  };
  syncOverviewDirty();
  scheduleOverviewAutosave();
}

function onOverviewDocumentInput(value: string) {
  overviewDocument = value;
  syncOverviewDirty();
  scheduleOverviewAutosave();
}

async function saveOverview(automatic = false) {
  if (!selectedLanguage || !overviewEntity || overviewSaving || overviewDeleting) return;
  clearOverviewAutosave();
  const name = overviewName.trim();
  if (!name) {
    overviewError = "Language name is required.";
    return;
  }
  const entityId = overviewEntity.id;
  const draftFields = { ...overviewFields };
  const draftDocument = overviewDocument;
  overviewSaving = true;
  overviewSavingAutomatically = automatic;
  overviewError = "";
  try {
    if (name !== overviewEntity.name) {
      overviewEntity = await context.entities.update(
        overviewEntity.id,
        { name },
        { expectedRevision: overviewEntity.revision, requestId: crypto.randomUUID() },
      );
    }
    for (const definition of overviewFieldDefinitions) {
      const value = draftFields[definition.key] ?? "";
      if (JSON.stringify(value) === JSON.stringify(overviewSavedFields[definition.key] ?? "")) continue;
      await context.fields.set(overviewEntity.id, definition.key, value, {
        expectedRevision: overviewFieldRevisions[definition.key] ?? "",
        requestId: crypto.randomUUID(),
      });
    }
    if (draftDocument !== overviewSavedDocument) {
      await context.documents.save(
        { entityId: overviewEntity.id, body: draftDocument, format: "markdown" },
        { expectedRevision: overviewDocumentRevision, requestId: crypto.randomUUID() },
      );
    }
    const currentDraftChanged =
      overviewName.trim() !== name ||
      overviewFieldDefinitions.some(
        (definition) =>
          JSON.stringify(overviewFields[definition.key] ?? "") !== JSON.stringify(draftFields[definition.key] ?? ""),
      ) ||
      overviewDocument !== draftDocument;
    const currentDraftName = overviewName;
    const currentDraftFields = { ...overviewFields };
    const currentDraftDocument = overviewDocument;
    const needsFollowUpSave = currentDraftChanged || overviewAutosaveQueued;
    overviewSaving = false;
    overviewSavingAutomatically = false;
    await loadOverview();
    if (selectedLanguage?.id !== entityId) return;
    if (needsFollowUpSave) {
      overviewName = currentDraftName;
      overviewFields = currentDraftFields;
      overviewDocument = currentDraftDocument;
      overviewDirty = true;
      scheduleOverviewAutosave();
      return;
    }
    languageSummaries = languageSummaries.map((language) =>
      language.id === selectedLanguage?.id
        ? { ...language, name, revision: overviewEntity?.revision ?? language.revision }
        : language,
    );
    selectedLanguage = selectedLanguage
      ? { ...selectedLanguage, name, revision: overviewEntity?.revision ?? selectedLanguage.revision }
      : selectedLanguage;
  } catch (cause) {
    overviewSaving = false;
    overviewSavingAutomatically = false;
    overviewDirty = true;
    overviewError = cause instanceof Error ? cause.message : String(cause);
    if (overviewAutosaveQueued) scheduleOverviewAutosave();
  }
}

async function archiveOverviewLanguage() {
  if (!selectedLanguage || !overviewEntity || overviewDeleting) return;
  const name = selectedLanguage.name;
  const message = overviewDirty
    ? `Archive “${name}”? Unsaved language details will be discarded.`
    : `Archive “${name}”? It will be removed from the active language list.`;
  if (!window.confirm(message)) return;
  clearOverviewAutosave();
  overviewDeleting = true;
  overviewError = "";
  try {
    await context.entities.delete(overviewEntity.id, {
      expectedRevision: overviewEntity.revision,
      requestId: crypto.randomUUID(),
    });
    languageSummaries = languageSummaries.filter((language) => language.id !== overviewEntity?.id);
    selectedLanguage = languageSummaries[0] ?? null;
    overviewEntity = null;
    overviewName = "";
    overviewFields = {};
    overviewSavedFields = {};
    overviewFieldRevisions = {};
    overviewDocument = "";
    overviewSavedDocument = "";
    overviewDocumentRevision = "";
    overviewDirty = false;
    overviewAutosaveQueued = false;
    overviewDeleting = false;
    overviewLoading = false;
    resetEditors();
    if (selectedLanguage) void loadPane();
  } catch (cause) {
    overviewDeleting = false;
    overviewError = cause instanceof Error ? cause.message : String(cause);
  }
}

let visibleLanguages = $derived(viewLanguageList(languageQuery, languageSummaries));

function viewLanguageList(query: string, summaries: EntitySummary[]) {
  const needle = query.trim().toLocaleLowerCase();
  return needle ? summaries.filter((language) => language.name.toLocaleLowerCase().includes(needle)) : summaries;
}

async function loadLanguages() {
  const token = ++languageRequest;
  try {
    const languages = await context.entities.list({ type: "language", limit: 500 });
    if (cancelled || token !== languageRequest) return;
    languageSummaries = languages;
    languageListLoaded = true;
    languageLoading = false;
    languageLoadError = "";
    let shouldLoadPane = false;
    if (!selectedLanguage && languages.length) {
      selectedLanguage = languages.find((language) => language.id === context.focusEntityId) ?? languages[0];
      shouldLoadPane = true;
    }
    if (shouldLoadPane) void loadPane();
  } catch (cause) {
    if (cancelled || token !== languageRequest) return;
    languageLoading = false;
    languageLoadError = cause instanceof Error ? cause.message : String(cause);
  }
}

async function loadOverview() {
  clearOverviewAutosave();
  paneLoading = false;
  if (!selectedLanguage) {
    overviewEntity = null;
    overviewLoading = false;
    return;
  }
  const token = ++overviewRequest;
  overviewLoading = true;
  overviewError = "";
  try {
    const [entity, fieldRecords] = await Promise.all([
      context.entities.get(selectedLanguage.id),
      context.fields.listRecords(selectedLanguage.id),
    ]);
    if (cancelled || token !== overviewRequest) return;
    if (!entity) throw new Error("This language is no longer available.");
    const values = Object.fromEntries(fieldRecords.map((record) => [record.key, record.value]));
    for (const definition of overviewFieldDefinitions) {
      if (!(definition.key in values)) values[definition.key] = "";
    }
    const document = entity.documents.find((item) => item.format === "markdown") ?? entity.documents[0];
    overviewEntity = entity;
    overviewName = entity.name;
    overviewFields = values;
    overviewSavedFields = { ...values };
    overviewFieldRevisions = Object.fromEntries(fieldRecords.map((record) => [record.key, record.revision]));
    overviewDocument = document?.body ?? "";
    overviewSavedDocument = overviewDocument;
    overviewDocumentRevision = document?.revision ?? "";
    overviewDirty = false;
    overviewLoading = false;
    overviewError = "";
  } catch (cause) {
    if (cancelled || token !== overviewRequest) return;
    overviewLoading = false;
    overviewError = cause instanceof Error ? cause.message : String(cause);
  }
}

async function loadRecords() {
  if (!selectedLanguage) {
    records = [];
    paradigms = [];
    lexiconLoading = false;
    return;
  }
  const token = ++request;
  lexiconLoading = true;
  try {
    const [result, paradigmList] = await Promise.all([
      context.records.list<LexemeValue>("lexemes", selectedLanguage.id, {
        query: search || undefined,
        status: statusFilter || undefined,
        tag: tagFilter || undefined,
        sort,
        homonymsOnly: homonymsOnly || undefined,
        limit: 51,
        offset: page * 50,
      }),
      context.records.list<Paradigm>("paradigms", selectedLanguage.id, { limit: 100, sort: "name" }),
    ]);
    if (!cancelled && token === request) {
      lexiconLoading = false;
      hasNextPage = result.length > 50;
      records = result.slice(0, 50).map((record) => ({
        ...record,
        value: normalizeLexeme(record.value),
      }));
      paradigms = paradigmList.map((record) => ({ ...record, value: normalizeParadigm(record.value) }));
      if (editing) {
        const current = records.find((record) => record.id === editing?.id);
        if (current) editing = current;
      }
      if (pendingLexemeId) {
        const target =
          records.find((record) => record.id === pendingLexemeId) ??
          (editing?.id === pendingLexemeId ? editing : null) ??
          (await findLexeme(pendingLexemeId, token));
        if (cancelled || token !== request) return;
        if (target) {
          editing = target;
          editorOpen = true;
          draft = normalizeLexeme(target.value);
          pendingLexemeId = null;
        } else {
          pendingLexemeId = null;
        }
      }
    }
  } catch (cause) {
    if (!cancelled && token === request) {
      lexiconLoading = false;
    }
  }
}

async function findLexeme(id: string, token: number) {
  if (!selectedLanguage) return null;
  for (let offset = 0; offset < 2000; offset += 100) {
    const batch = await context.records.list<LexemeValue>("lexemes", selectedLanguage.id, {
      limit: 100,
      offset,
      sort: "lemma",
    });
    if (cancelled || token !== request) return null;
    const found = batch.find((record) => record.id === id);
    if (found) return { ...found, value: normalizeLexeme(found.value) };
    if (batch.length < 100) break;
  }
  return null;
}

async function refreshHomonyms(lemma: string) {
  if (!selectedLanguage || !lemma) {
    homonymCount = 0;
    return;
  }
  const matches = await context.records.list<LexemeValue>("lexemes", selectedLanguage.id, {
    query: lemma,
    limit: 100,
  });
  homonymCount = matches.filter(
    (record) => record.value.lemma.toLocaleLowerCase() === lemma.toLocaleLowerCase() && record.id !== editing?.id,
  ).length;
}

function scheduleLoad() {
  page = 0;
  if (searchTimer !== null) window.clearTimeout(searchTimer);
  searchTimer = window.setTimeout(() => void loadRecords(), 180);
}

function addWord() {
  editing = null;
  editorOpen = true;
  draft = emptyLexeme();
  homonymCount = 0;
}

function openLexiconEditor(record: ModuleRecord<LexemeValue>) {
  editing = record;
  editorOpen = true;
  draft = normalizeLexeme(record.value);
  void refreshHomonyms(draft.lemma);
}

function addHomonym() {
  const lemma = draft.lemma;
  editing = null;
  editorOpen = true;
  draft = { ...emptyLexeme(), lemma };
  void refreshHomonyms(lemma);
}

function closeLexiconEditor() {
  editing = null;
  editorOpen = false;
  draft = emptyLexeme();
  error = "";
}

async function saveLexeme() {
  if (!selectedLanguage || lexiconSaving) return "none";
  const value = normalizeLexeme(draft);
  if (!value.lemma) {
    error = "Lemma is required.";
    return "lemma";
  }
  error = "";
  draft = value;
  lexiconSaving = true;
  try {
    const payload = serializeLexeme(value);
    if (editing) {
      const updated = await context.records.update("lexemes", editing.id, selectedLanguage.id, payload, {
        expectedRevision: editing.revision,
        requestId: crypto.randomUUID(),
      });
      editing = { ...updated, value: normalizeLexeme(updated.value) };
    } else {
      const created = await context.records.create("lexemes", selectedLanguage.id, payload, {
        requestId: crypto.randomUUID(),
      });
      editing = { ...created, value: normalizeLexeme(created.value) };
    }
    editorOpen = true;
    draft = editing.value;
    lexiconSaving = false;
    await loadRecords();
    await refreshHomonyms(draft.lemma);
    return "ok";
  } catch (cause) {
    lexiconSaving = false;
    error = cause instanceof Error ? cause.message : String(cause);
    return "error";
  }
}

async function deleteLexeme() {
  if (!selectedLanguage || !editing) return;
  if (!window.confirm(`Delete “${editing.value.lemma}”?`)) return;
  try {
    await context.records.delete("lexemes", editing.id, selectedLanguage.id, {
      expectedRevision: editing.revision,
      requestId: crypto.randomUUID(),
    });
    editing = null;
    editorOpen = false;
    draft = emptyLexeme();
    error = "";
    await loadRecords();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

function previousPage() {
  page = Math.max(0, page - 1);
  void loadRecords();
}

function nextPage() {
  page += 1;
  void loadRecords();
}

async function exportLexicon() {
  if (!selectedLanguage) return;
  const values: LexemeValue[] = [];
  for (let offset = 0; ; offset += 100) {
    const batch = await context.records.list<LexemeValue>("lexemes", selectedLanguage.id, {
      limit: 100,
      offset,
      sort: "lemma",
    });
    values.push(...batch.map((record) => normalizeLexeme(record.value)));
    if (batch.length < 100) break;
  }
  const blob = new Blob([lexiconExport(selectedLanguage.name, values)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = `${selectedLanguage.name.replace(/\s+/g, "-").toLowerCase()}-lexicon.json`;
  link.click();
  URL.revokeObjectURL(url);
}

async function importLexicon(file: File) {
  if (!selectedLanguage) return;
  try {
    const lexemes = parseLexiconImport(await file.text());
    for (const value of lexemes) {
      await context.records.create("lexemes", selectedLanguage.id, serializeLexeme(value), {
        requestId: crypto.randomUUID(),
      });
    }
    page = 0;
    await loadPane();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

function resetEditors() {
  editing = null;
  editorOpen = false;
  draft = emptyLexeme();
  phonemeEditing = null;
  phonemeEditorOpen = false;
  phonemeDraft = emptyPhoneme();
  phonologyNotesOpen = false;
  orthographyEditing = null;
  orthographyEditorOpen = false;
  orthographyDraft = emptyOrthography();
  grammarUi = emptyGrammarUiState();
  paradigmEditing = null;
  paradigmEditorOpen = false;
  paradigmDraft = emptyParadigm();
  previewStem = "";
  previewLexemeId = "";
  sampleEditing = null;
  sampleEditorOpen = false;
  sampleDraft = emptySample();
  lexiconSaving = false;
}

async function loadPane() {
  if (pane === "overview") return loadOverview();
  if (pane === "sounds") return loadSounds();
  if (pane === "writing") return loadWriting();
  if (pane === "grammar") return loadGrammar();
  if (pane === "forms") return loadForms();
  if (pane === "samples") return loadSamples();
  return loadRecords();
}

async function loadSounds() {
  if (!selectedLanguage) {
    phonemes = [];
    phonologyRecord = null;
    phonologyDraft = emptyPhonologyNotes();
    phonologyNotesOpen = false;
    paneLoading = false;
    return;
  }
  const token = ++request;
  paneLoading = true;
  try {
    const [inventory, notes] = await Promise.all([
      context.records.list<PhonemeValue>("phonemes", selectedLanguage.id, { limit: 100, sort: "symbol" }),
      context.records.list<PhonologyNotes>("phonology", selectedLanguage.id, { limit: 1 }),
    ]);
    if (!cancelled && token === request) {
      paneLoading = false;
      phonemes = inventory.map((record) => ({ ...record, value: normalizePhoneme(record.value) }));
      phonologyRecord = notes[0] ? { ...notes[0], value: normalizePhonologyNotes(notes[0].value) } : null;
      phonologyDraft = phonologyRecord?.value ?? emptyPhonologyNotes();
      if (phonemeEditing) {
        const current = phonemes.find((record) => record.id === phonemeEditing?.id);
        if (current) phonemeEditing = current;
      }
    }
  } catch (cause) {
    if (!cancelled && token === request) {
      paneLoading = false;
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }
}

async function loadWriting() {
  if (!selectedLanguage) {
    orthographies = [];
    paneLoading = false;
    return;
  }
  const token = ++request;
  paneLoading = true;
  try {
    const [systems, inventory] = await Promise.all([
      context.records.list<OrthographyValue>("orthographies", selectedLanguage.id, {
        limit: 100,
        sort: "name",
      }),
      context.records.list<PhonemeValue>("phonemes", selectedLanguage.id, { limit: 100, sort: "symbol" }),
    ]);
    if (!cancelled && token === request) {
      paneLoading = false;
      orthographies = systems.map((record) => ({ ...record, value: normalizeOrthography(record.value) }));
      phonemes = inventory.map((record) => ({ ...record, value: normalizePhoneme(record.value) }));
      if (orthographyEditing) {
        const current = orthographies.find((record) => record.id === orthographyEditing?.id);
        if (current) orthographyEditing = current;
      }
    }
  } catch (cause) {
    if (!cancelled && token === request) {
      paneLoading = false;
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }
}

function addPhoneme() {
  phonemeEditing = null;
  phonemeEditorOpen = true;
  phonemeDraft = emptyPhoneme();
}

function openPhonemeEditor(record: ModuleRecord<PhonemeValue>) {
  phonemeEditing = record;
  phonemeEditorOpen = true;
  phonemeDraft = normalizePhoneme(record.value);
}

function closePhonemeEditor() {
  phonemeEditing = null;
  phonemeEditorOpen = false;
  phonemeDraft = emptyPhoneme();
  error = "";
}

async function savePhoneme(): Promise<"ok" | "symbol" | "error" | "none"> {
  if (!selectedLanguage) return "none";
  phonemeDraft = normalizePhoneme(phonemeDraft);
  if (!phonemeDraft.symbol) {
    error = "Symbol is required. IPA is optional.";
    return "symbol";
  }
  error = "";
  try {
    const payload = serializePhoneme(phonemeDraft);
    if (phonemeEditing) {
      const updated = await context.records.update("phonemes", phonemeEditing.id, selectedLanguage.id, payload, {
        expectedRevision: phonemeEditing.revision,
        requestId: crypto.randomUUID(),
      });
      phonemeEditing = { ...updated, value: normalizePhoneme(updated.value) };
    } else {
      const created = await context.records.create("phonemes", selectedLanguage.id, payload, {
        requestId: crypto.randomUUID(),
      });
      phonemeEditing = { ...created, value: normalizePhoneme(created.value) };
    }
    phonemeEditorOpen = true;
    phonemeDraft = phonemeEditing.value;
    await loadSounds();
    return "ok";
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
    return "error";
  }
}

async function deletePhoneme() {
  if (!selectedLanguage || !phonemeEditing) return;
  if (!window.confirm(`Delete “${phonemeEditing.value.symbol}”?`)) return;
  error = "";
  try {
    await context.records.delete("phonemes", phonemeEditing.id, selectedLanguage.id, {
      expectedRevision: phonemeEditing.revision,
      requestId: crypto.randomUUID(),
    });
    phonemeEditing = null;
    phonemeEditorOpen = false;
    phonemeDraft = emptyPhoneme();
    await loadSounds();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

async function savePhonology() {
  if (!selectedLanguage) return;
  error = "";
  try {
    phonologyDraft = normalizePhonologyNotes(phonologyDraft);
    const payload = serializePhonologyNotes(phonologyDraft);
    if (phonologyRecord) {
      const updated = await context.records.update("phonology", phonologyRecord.id, selectedLanguage.id, payload, {
        expectedRevision: phonologyRecord.revision,
        requestId: crypto.randomUUID(),
      });
      phonologyRecord = { ...updated, value: normalizePhonologyNotes(updated.value) };
    } else {
      const created = await context.records.create("phonology", selectedLanguage.id, payload, {
        requestId: crypto.randomUUID(),
      });
      phonologyRecord = { ...created, value: normalizePhonologyNotes(created.value) };
    }
    phonologyDraft = phonologyRecord.value;
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

function addOrthography() {
  orthographyEditing = null;
  orthographyEditorOpen = true;
  orthographyDraft = emptyOrthography();
}

function openOrthographyEditor(record: ModuleRecord<OrthographyValue>) {
  orthographyEditing = record;
  orthographyEditorOpen = true;
  orthographyDraft = normalizeOrthography(record.value);
}

function closeOrthographyEditor() {
  orthographyEditing = null;
  orthographyEditorOpen = false;
  orthographyDraft = emptyOrthography();
  error = "";
}

async function saveOrthography(): Promise<"ok" | "name" | "error" | "none"> {
  if (!selectedLanguage) return "none";
  orthographyDraft = normalizeOrthography(orthographyDraft);
  if (!orthographyDraft.name) {
    error = "Writing system name is required.";
    return "name";
  }
  error = "";
  try {
    const payload = serializeOrthography(orthographyDraft);
    if (orthographyEditing) {
      const updated = await context.records.update(
        "orthographies",
        orthographyEditing.id,
        selectedLanguage.id,
        payload,
        { expectedRevision: orthographyEditing.revision, requestId: crypto.randomUUID() },
      );
      orthographyEditing = { ...updated, value: normalizeOrthography(updated.value) };
    } else {
      const created = await context.records.create("orthographies", selectedLanguage.id, payload, {
        requestId: crypto.randomUUID(),
      });
      orthographyEditing = { ...created, value: normalizeOrthography(created.value) };
    }
    orthographyEditorOpen = true;
    orthographyDraft = orthographyEditing.value;
    await loadWriting();
    return "ok";
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
    return "error";
  }
}

async function deleteOrthography() {
  if (!selectedLanguage || !orthographyEditing) return;
  if (!window.confirm(`Delete “${orthographyEditing.value.name}”?`)) return;
  error = "";
  try {
    await context.records.delete("orthographies", orthographyEditing.id, selectedLanguage.id, {
      expectedRevision: orthographyEditing.revision,
      requestId: crypto.randomUUID(),
    });
    orthographyEditing = null;
    orthographyEditorOpen = false;
    orthographyDraft = emptyOrthography();
    await loadWriting();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

async function loadGrammar() {
  if (!selectedLanguage) {
    grammarUi.index = emptyGrammarUiState().index;
    records = [];
    paneLoading = false;
    return;
  }
  const token = ++request;
  paneLoading = true;
  try {
    const loaded = await loadGrammarIndex(context.records, selectedLanguage.id);
    const [lexemes, sampleRecords, paradigmRecords] = await Promise.all([
      context.records.list<LexemeValue>("lexemes", selectedLanguage.id, { limit: 500, sort: "lemma" }),
      context.records.list<Sample>("samples", selectedLanguage.id, { limit: 100, sort: "title" }),
      context.records.list<Paradigm>("paradigms", selectedLanguage.id, { limit: 100, sort: "name" }),
    ]);
    if (!cancelled && token === request) {
      paneLoading = false;
      grammarUi.index = loaded.index;
      records = lexemes.map((record) => ({ ...record, value: normalizeLexeme(record.value) }));
      samples = sampleRecords.map((record) => ({ ...record, value: normalizeSample(record.value) }));
      paradigms = paradigmRecords.map((record) => ({ ...record, value: normalizeParadigm(record.value) }));
    }
  } catch (cause) {
    if (!cancelled && token === request) {
      paneLoading = false;
    }
  }
}

function addParadigm() {
  paradigmEditing = null;
  paradigmEditorOpen = true;
  paradigmDraft = emptyParadigm();
  previewStem = "";
  previewLexemeId = "";
}

function openParadigmEditor(record: ModuleRecord<Paradigm>) {
  paradigmEditing = record;
  paradigmEditorOpen = true;
  paradigmDraft = normalizeParadigm(record.value);
  previewStem = "";
  previewLexemeId = "";
}

function closeParadigmEditor() {
  paradigmEditing = null;
  paradigmEditorOpen = false;
  paradigmDraft = emptyParadigm();
  error = "";
}

async function saveParadigm(): Promise<"ok" | "name" | "error" | "none"> {
  if (!selectedLanguage) return "none";
  const value = normalizeParadigm(paradigmDraft);
  if (!value.name) {
    error = "Name is required.";
    return "name";
  }
  error = "";
  paradigmDraft = value;
  try {
    const payload = serializeParadigm(value);
    if (paradigmEditing) {
      const updated = await context.records.update("paradigms", paradigmEditing.id, selectedLanguage.id, payload, {
        expectedRevision: paradigmEditing.revision,
        requestId: crypto.randomUUID(),
      });
      paradigmEditing = { ...updated, value: normalizeParadigm(updated.value) };
    } else {
      const created = await context.records.create("paradigms", selectedLanguage.id, payload, {
        requestId: crypto.randomUUID(),
      });
      paradigmEditing = { ...created, value: normalizeParadigm(created.value) };
    }
    paradigmEditorOpen = true;
    paradigmDraft = paradigmEditing.value;
    await loadForms();
    return "ok";
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
    return "error";
  }
}

async function deleteParadigm() {
  if (!selectedLanguage || !paradigmEditing) return;
  if (!window.confirm(`Delete “${paradigmEditing.value.name}”?`)) return;
  error = "";
  try {
    await context.records.delete("paradigms", paradigmEditing.id, selectedLanguage.id, {
      expectedRevision: paradigmEditing.revision,
      requestId: crypto.randomUUID(),
    });
    paradigmEditing = null;
    paradigmEditorOpen = false;
    paradigmDraft = emptyParadigm();
    await loadForms();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

async function persistLexemeForms(record: ModuleRecord<LexemeValue>, forms: LexemeValue["forms"]) {
  if (!selectedLanguage) return;
  const value = normalizeLexeme({ ...record.value, forms });
  const updated = await context.records.update("lexemes", record.id, selectedLanguage.id, serializeLexeme(value), {
    expectedRevision: record.revision,
    requestId: crypto.randomUUID(),
  });
  const next = { ...updated, value: normalizeLexeme(updated.value) };
  records = records.map((item) => (item.id === next.id ? next : item));
}

async function pinPreviewOverride(record: ModuleRecord<LexemeValue>, slot: ParadigmSlot, form: string) {
  const paradigmId = paradigmEditing?.id;
  if (!paradigmId) return;
  try {
    await persistLexemeForms(record, pinOverride(record.value.forms, paradigmId, slot, form));
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

async function clearPreviewOverride(record: ModuleRecord<LexemeValue>, slot: ParadigmSlot) {
  const paradigmId = paradigmEditing?.id;
  if (!paradigmId) return;
  try {
    await persistLexemeForms(record, clearOverride(record.value.forms, paradigmId, slot));
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

function openLinkedLexeme(lexemeId: string) {
  const target = records.find((record) => record.id === lexemeId);
  pendingLexemeId = lexemeId;
  pane = "lexicon";
  sampleEditorOpen = false;
  paradigmEditorOpen = false;
  search = "";
  statusFilterInput = "";
  tagFilterInput = "";
  homonymsOnly = false;
  page = 0;
  if (target) {
    editing = target;
    editorOpen = true;
    draft = normalizeLexeme(target.value);
  }
  void loadRecords();
}

function addSample(kind: SampleKind = "sentence") {
  sampleEditing = null;
  sampleEditorOpen = true;
  sampleDraft = emptySample(kind);
}

function openSampleEditor(record: ModuleRecord<Sample>) {
  sampleEditing = record;
  sampleEditorOpen = true;
  sampleDraft = normalizeSample(record.value);
}

function closeSampleEditor() {
  sampleEditing = null;
  sampleEditorOpen = false;
  sampleDraft = emptySample();
  error = "";
}

async function saveSample(): Promise<"ok" | "text" | "error" | "none"> {
  if (!selectedLanguage) return "none";
  const value = normalizeSample(sampleDraft);
  if (!value.text.trim()) {
    error = "Text is required.";
    return "text";
  }
  error = "";
  sampleDraft = value;
  try {
    const payload = serializeSample(value);
    if (sampleEditing) {
      const updated = await context.records.update("samples", sampleEditing.id, selectedLanguage.id, payload, {
        expectedRevision: sampleEditing.revision,
        requestId: crypto.randomUUID(),
      });
      sampleEditing = { ...updated, value: normalizeSample(updated.value) };
    } else {
      const created = await context.records.create("samples", selectedLanguage.id, payload, {
        requestId: crypto.randomUUID(),
      });
      sampleEditing = { ...created, value: normalizeSample(created.value) };
    }
    sampleEditorOpen = true;
    sampleDraft = sampleEditing.value;
    await loadSamples();
    return "ok";
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
    return "error";
  }
}

async function deleteSample() {
  if (!selectedLanguage || !sampleEditing) return;
  if (!window.confirm(`Delete “${sampleTitle(sampleEditing.value)}”?`)) return;
  error = "";
  try {
    await context.records.delete("samples", sampleEditing.id, selectedLanguage.id, {
      expectedRevision: sampleEditing.revision,
      requestId: crypto.randomUUID(),
    });
    sampleEditing = null;
    sampleEditorOpen = false;
    sampleDraft = emptySample();
    await loadSamples();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

async function loadForms() {
  if (!selectedLanguage) {
    paradigms = [];
    records = [];
    paneLoading = false;
    return;
  }
  const token = ++request;
  paneLoading = true;
  try {
    const [tables, lexemes] = await Promise.all([
      context.records.list<Paradigm>("paradigms", selectedLanguage.id, { limit: 100, sort: "name" }),
      context.records.list<LexemeValue>("lexemes", selectedLanguage.id, { limit: 500, sort: "lemma" }),
    ]);
    if (!cancelled && token === request) {
      paneLoading = false;
      paradigms = tables.map((record) => ({ ...record, value: normalizeParadigm(record.value) }));
      records = lexemes.map((record) => ({ ...record, value: normalizeLexeme(record.value) }));
      if (paradigmEditing) {
        const current = paradigms.find((record) => record.id === paradigmEditing?.id);
        if (current) paradigmEditing = current;
      }
    }
  } catch (cause) {
    if (!cancelled && token === request) {
      paneLoading = false;
    }
  }
}

async function loadSamples() {
  if (!selectedLanguage) {
    samples = [];
    records = [];
    paneLoading = false;
    return;
  }
  const token = ++request;
  paneLoading = true;
  try {
    const [items, lexemes] = await Promise.all([
      context.records.list<Sample>("samples", selectedLanguage.id, { limit: 100, sort: "title" }),
      context.records.list<LexemeValue>("lexemes", selectedLanguage.id, { limit: 500, sort: "lemma" }),
    ]);
    if (!cancelled && token === request) {
      paneLoading = false;
      samples = items.map((record) => ({ ...record, value: normalizeSample(record.value) }));
      records = lexemes.map((record) => ({ ...record, value: normalizeLexeme(record.value) }));
      if (sampleEditing) {
        const current = samples.find((record) => record.id === sampleEditing?.id);
        if (current) sampleEditing = current;
      }
    }
  } catch (cause) {
    if (!cancelled && token === request) {
      paneLoading = false;
    }
  }
}

function openLanguage(language: EntitySummary) {
  if (!tryLeaveOverview((message) => window.confirm(message))) return;
  if (!tryLeaveGrammar(grammarUi, (message) => window.confirm(message))) return;
  selectedLanguage = language;
  resetEditors();
  error = "";
  search = "";
  statusFilterInput = "";
  tagFilterInput = "";
  sort = "lemma";
  homonymsOnly = false;
  page = 0;
  void loadPane();
}

function switchPane(id: Pane) {
  if (pane === id) return;
  if (!tryLeaveOverview((message) => window.confirm(message))) return;
  if (!tryLeaveGrammar(grammarUi, (message) => window.confirm(message))) return;
  pane = id;
  error = "";
  void loadPane();
}

function roveTabs(event: KeyboardEvent, index: number) {
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight" && event.key !== "Home" && event.key !== "End") {
    return;
  }
  event.preventDefault();
  const tabs = paneListEl?.querySelectorAll<HTMLButtonElement>("button") ?? [];
  const next =
    event.key === "Home"
      ? 0
      : event.key === "End"
        ? tabs.length - 1
        : (index + (event.key === "ArrowRight" ? 1 : -1) + tabs.length) % tabs.length;
  tabs[next]?.focus();
  tabs[next]?.click();
}

function openCreateForm() {
  creatingLanguage = true;
  languageCreateName = "";
  languageCreateError = "";
}

function cancelCreateLanguage() {
  creatingLanguage = false;
  languageCreateName = "";
  languageCreateError = "";
}

async function submitCreateLanguage(event: SubmitEvent) {
  event.preventDefault();
  languageCreateName = languageCreateName.trim();
  if (!languageCreateName) {
    languageCreateError = "Language name is required.";
    createNameInput?.focus();
    return;
  }
  createBusy = true;
  try {
    const created = await context.entities.create({ name: languageCreateName, type: "language" });
    languageSummaries = [created, ...languageSummaries.filter((language) => language.id !== created.id)];
    languageListLoaded = true;
    languageLoading = false;
    selectedLanguage = created;
    creatingLanguage = false;
    languageCreateName = "";
    languageCreateError = "";
    createBusy = false;
    resetEditors();
    error = "";
    search = "";
    statusFilterInput = "";
    tagFilterInput = "";
    page = 0;
    void loadPane();
  } catch (cause) {
    languageCreateError = cause instanceof Error ? cause.message : String(cause);
    createBusy = false;
    createNameInput?.focus();
  }
}
</script>

<section class="language-workspace" class:language-workspace-embedded={context.embedded}>
  {#if !context.embedded}
    <aside class="language-panel language-sidebar" aria-busy={languageLoading}>
      <div class="language-sidebar-head">
        <div>
          <p class="language-sidebar-kicker">Language studio</p>
          <h2>Languages</h2>
        </div>
        <button type="button" class="language-button secondary" onclick={openCreateForm}>Create language</button>
      </div>
      <p class="language-sidebar-intro">Choose a language to shape its words, sounds, writing, and grammar.</p>
      <label class="language-field">
        <span>Filter languages</span>
        <input name="languageQuery" type="search" bind:value={languageQuery} />
      </label>
      {#if creatingLanguage}
        <form class="language-create" onsubmit={submitCreateLanguage}>
          <label class="language-field">
            <span>Language name</span>
            <input
              name="languageCreateName"
              autocomplete="off"
              bind:this={createNameInput}
              bind:value={languageCreateName}
              oninput={() => (languageCreateError = "")} />
          </label>
          {#if languageCreateError}
            <p class="language-status error" role="alert">{languageCreateError}</p>
          {/if}
          <div class="language-create-actions">
            <button type="button" class="language-button secondary" onclick={cancelCreateLanguage}>Cancel</button>
            <button type="submit" class="language-button" disabled={createBusy}
              >{createBusy ? "Creating…" : "Create"}</button>
          </div>
        </form>
      {/if}
      <ul class="language-list">
        {#if languageLoading}
          <li><p class="language-empty" role="status">Loading languages…</p></li>
        {:else if languageLoadError}
          <li><p class="language-status error" role="alert">{languageLoadError}</p></li>
        {:else if languageSummaries.length === 0}
          <li><p class="language-empty" role="status">No languages yet. Create one to start.</p></li>
        {:else if visibleLanguages.length === 0}
          <li><p class="language-empty" role="status">No languages match that filter.</p></li>
        {:else}
          {#each visibleLanguages as language (language.id)}
            <li>
              <button
                type="button"
                aria-current={selectedLanguage?.id === language.id ? "page" : undefined}
                onclick={() => openLanguage(language)}>
                <span class="language-list-name">{language.name}</span>
                <span class="language-list-meta"
                  >{selectedLanguage?.id === language.id ? "Selected language" : "Open language"}</span>
              </button>
            </li>
          {/each}
        {/if}
      </ul>
    </aside>
  {/if}
  <div
    id="language-pane"
    class="language-panel language-main"
    role="tabpanel"
    aria-labelledby={`language-tab-${pane}`}
    aria-busy={pane === "overview" ? overviewLoading : pane === "lexicon" ? lexiconLoading : paneLoading}>
    <div bind:this={paneListEl} class="language-tabs" role="tablist" aria-label="Language workspace">
      {#each PANES as [id, label], index (id)}
        <button
          type="button"
          role="tab"
          id={`language-tab-${id}`}
          aria-controls="language-pane"
          aria-selected={pane === id}
          tabindex={pane === id ? 0 : -1}
          onclick={() => switchPane(id)}
          onkeydown={(event) => roveTabs(event, index)}>{label}</button>
      {/each}
    </div>
    <div class="language-pane" hidden={pane !== "overview"}>
      <Overview
        {selectedLanguage}
        {error}
        {overviewEntity}
        {overviewLoading}
        {overviewName}
        {overviewFields}
        {overviewDocument}
        {overviewDirty}
        {overviewSaving}
        {overviewSavingAutomatically}
        {overviewDeleting}
        {overviewError}
        {overviewFieldDefinitions}
        {onOverviewNameInput}
        {onOverviewFieldInput}
        {onOverviewDocumentInput}
        {archiveOverviewLanguage} />
    </div>
    <div class="language-pane" hidden={pane !== "sounds"}>
      <Sounds
        {selectedLanguage}
        {paneLoading}
        {error}
        {phonemes}
        {phonemeEditing}
        {phonemeEditorOpen}
        {phonemeDraft}
        {phonologyRecord}
        {phonologyDraft}
        {phonologyNotesOpen}
        {addPhoneme}
        {openPhonemeEditor}
        {closePhonemeEditor}
        {savePhoneme}
        {deletePhoneme}
        {savePhonology} />
    </div>
    <div class="language-pane" hidden={pane !== "writing"}>
      <Writing
        {selectedLanguage}
        {paneLoading}
        {error}
        {phonemes}
        {orthographies}
        {orthographyEditing}
        {orthographyEditorOpen}
        {orthographyDraft}
        {addOrthography}
        {openOrthographyEditor}
        {closeOrthographyEditor}
        {saveOrthography}
        {deleteOrthography} />
    </div>
    <div class="language-pane" hidden={pane !== "grammar"}>
      <Grammar {context} {selectedLanguage} {paneLoading} {error} {grammarUi} {records} {samples} {paradigms} />
    </div>
    <div class="language-pane" hidden={pane !== "forms"}>
      <Forms
        {selectedLanguage}
        {paneLoading}
        {error}
        {records}
        {paradigms}
        {paradigmEditing}
        {paradigmEditorOpen}
        {paradigmDraft}
        bind:previewStem
        bind:previewLexemeId
        {addParadigm}
        {openParadigmEditor}
        {closeParadigmEditor}
        {saveParadigm}
        {deleteParadigm}
        {pinPreviewOverride}
        {clearPreviewOverride} />
    </div>
    <div class="language-pane" hidden={pane !== "samples"}>
      <Samples
        {selectedLanguage}
        {paneLoading}
        {error}
        {records}
        {samples}
        {sampleEditing}
        {sampleEditorOpen}
        {sampleDraft}
        {addSample}
        {openSampleEditor}
        {closeSampleEditor}
        {saveSample}
        {deleteSample}
        {openLinkedLexeme} />
    </div>
    <div class="language-pane" hidden={pane !== "lexicon"}>
      <Lexicon
        {selectedLanguage}
        {records}
        {paradigms}
        {editing}
        {editorOpen}
        {draft}
        bind:search
        bind:statusFilterInput
        bind:tagFilterInput
        bind:sort
        bind:homonymsOnly
        {page}
        {hasNextPage}
        {homonymCount}
        {lexiconLoading}
        {lexiconSaving}
        {error}
        {addWord}
        {openLexiconEditor}
        {addHomonym}
        {closeLexiconEditor}
        {saveLexeme}
        {deleteLexeme}
        {previousPage}
        {nextPage}
        {importLexicon}
        {exportLexicon} />
    </div>
  </div>
</section>

<style>
.language-workspace {
  display: grid;
  grid-template-columns: minmax(220px, 260px) minmax(0, 1fr);
  gap: 18px;
  height: 100%;
  min-height: 0;
  color: var(--ink);
}
.language-workspace-embedded {
  grid-template-columns: minmax(0, 1fr);
  height: auto;
}
.language-panel {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: auto;
  border: 1px solid var(--line);
  border-radius: 16px;
  background: var(--surface);
  padding: 22px 20px 24px;
  box-shadow: var(--shadow-sm, 0 2px 8px rgba(38, 42, 33, 0.05));
}
.language-sidebar {
  gap: 14px;
}
.language-sidebar-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
.language-sidebar-kicker {
  margin: 0 0 5px;
  color: var(--accent);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}
.language-sidebar-intro {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.55;
}
.language-sidebar-intro {
  margin-top: -5px;
}
.language-field {
  display: grid;
  gap: 6px;
  min-width: 0;
  color: var(--ink-soft);
  font-size: 11px;
  letter-spacing: 0.01em;
}
@media (max-width: 760px) {
  .language-workspace {
    display: flex;
    flex-direction: column;
    overflow: auto;
  }
  .language-sidebar {
    max-height: none;
  }
  .language-main {
    min-height: 34rem;
  }
  .language-tabs {
    flex-wrap: nowrap;
    overflow-x: auto;
    overscroll-behavior-inline: contain;
    padding-bottom: 10px;
    scrollbar-width: thin;
  }
  .language-tabs button {
    flex: 0 0 auto;
  }
}
.language-panel h2 {
  margin: 0;
  font-family: var(--font-display);
  font-weight: 500;
  font-size: 24px;
  line-height: 1.15;
}
.language-list {
  display: grid;
  gap: 8px;
  margin: 4px 0 0;
  padding: 0;
  list-style: none;
}
.language-list button {
  display: grid;
  gap: 3px;
  width: 100%;
  padding: 11px 12px;
  border: 1px solid #ebe7de;
  border-radius: 10px;
  background: var(--surface);
  color: inherit;
  text-align: left;
  cursor: pointer;
  box-shadow: 0 1px 2px rgba(38, 42, 33, 0.03);
}
.language-list button:hover {
  border-color: #e5d8c6;
  background: var(--surface-muted);
}
.language-list button[aria-current="page"] {
  border-color: #d8c3a5;
  background: var(--surface-muted);
  box-shadow:
    inset 3px 0 var(--accent),
    0 1px 2px rgba(38, 42, 33, 0.03);
  color: var(--ink);
}
.language-list-name {
  font-weight: 600;
}
.language-list-meta {
  color: var(--ink-faint);
  font-size: 11px;
}
.language-create {
  display: grid;
  gap: 10px;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-muted);
}
.language-create-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  flex-wrap: wrap;
}
.language-field input {
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
.language-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin: 0 0 8px;
  padding: 0 0 12px;
  background: var(--surface);
}
.language-tabs button {
  padding: 7px 12px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: transparent;
  color: var(--ink-soft);
  cursor: pointer;
}
.language-tabs button:hover {
  border-color: #d8c3a5;
  color: var(--ink);
  background: var(--surface-muted);
}
.language-tabs button[aria-selected="true"] {
  border-color: var(--accent-dark);
  background: var(--surface-muted);
  color: var(--accent-dark);
}
.language-button {
  padding: 8px 12px;
  border: 1px solid var(--accent-dark);
  border-radius: 8px;
  background: var(--accent-dark);
  color: #fff;
  cursor: pointer;
}
.language-button:hover {
  filter: brightness(1.06);
}
.language-button.secondary {
  background: transparent;
  color: var(--accent-dark);
}
.language-button.secondary:hover {
  background: var(--surface-muted);
}
.language-button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  filter: none;
}
.language-button:focus-visible,
.language-tabs button:focus-visible,
.language-list button:focus-visible {
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
</style>
