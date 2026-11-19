<script lang="ts">
import { pluginText, translatePluginText as formatText } from "../i18n.ts";
import { untrack } from "svelte";
import type { EntitySummary, ModuleContext, ModuleRecord, ModuleRecordQuery } from "../../../../module-api/src/index";
import IpaInput from "../IpaInput.svelte";
import type { LexemeValue } from "../lexeme";
import { confirm } from "../confirm.svelte";
import {
  emptyLexeme,
  firstGloss,
  lexiconExport,
  normalizeLexeme,
  parseLexiconImport,
  PART_OF_SPEECH_SUGGESTIONS,
  serializeLexeme,
  STATUS_SUGGESTIONS,
} from "../lexeme";
import type { Paradigm, ParadigmSlot } from "../morphology";
import { clearOverride, normalizeParadigm, overrideTarget, pinOverride, previewParadigm } from "../morphology";

let {
  context,
  selectedLanguage,
  active,
  pendingLexemeId,
  onPendingLexemeHandled,
  registerLeaveGuard,
  setMutationActive,
}: {
  context: ModuleContext;
  selectedLanguage: EntitySummary | null;
  active: boolean;
  pendingLexemeId: string | null;
  onPendingLexemeHandled: () => void;
  registerLeaveGuard: (guard: (() => Promise<boolean> | boolean) | null) => void;
  setMutationActive: (active: boolean) => void;
} = $props();

let cancelled = $state(false);
let records: ModuleRecord<LexemeValue>[] = $state([]);
let paradigms: ModuleRecord<Paradigm>[] = $state([]);
let editing = $state<ModuleRecord<LexemeValue> | null>(null);
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
let homonymRequest = $state(0);
let searchTimer = $state<number | null>(null);
let lexiconLoading = $state(false);
let lexiconSaving = $state(false);
let lexiconImporting = $state(false);
let lexiconExporting = $state(false);
let error = $state("");
let notice = $state("");

let tagsText = $state("");
let fileInput: HTMLInputElement | undefined = $state();
let lemmaInput: HTMLInputElement | undefined = $state();

let lastLoadedLanguage: string | null = null;
let filtersInitialized = false;
let lastHandledPending: string | null = null;

$effect(() => {
  const languageId = selectedLanguage?.id ?? null;
  void languageId;
  if (!active) return;
  if (languageId === lastLoadedLanguage) {
    untrack(() => void loadRecords());
    return;
  }
  lastLoadedLanguage = languageId;
  search = "";
  statusFilterInput = "";
  tagFilterInput = "";
  homonymsOnly = false;
  page = 0;
  editing = null;
  editorOpen = false;
  untrack(() => void loadRecords());
});

$effect(() => {
  void search;
  void statusFilterInput;
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

$effect(() => {
  const pending = pendingLexemeId;
  if (!pending) {
    lastHandledPending = null;
    return;
  }
  if (!active || pending === lastHandledPending) return;
  lastHandledPending = pending;
  search = "";
  statusFilterInput = "";
  tagFilterInput = "";
  homonymsOnly = false;
  page = 0;
  editing = null;
  editorOpen = false;
  void openPendingLexeme(pending);
});

$effect(() => {
  if (editorOpen && lemmaInput) lemmaInput.focus();
});

let previousEditorOpen = false;

$effect(() => {
  if (editorOpen && !previousEditorOpen) tagsText = draft.tags.join("\n");
  previousEditorOpen = editorOpen;
});

$effect(() => {
  return () => {
    cancelled = true;
    if (searchTimer !== null) window.clearTimeout(searchTimer);
    searchTimer = null;
  };
});

function lexiconHasDraft() {
  if (!editorOpen) return false;
  const baseline = editing ? normalizeLexeme(editing.value) : emptyLexeme();
  const candidate = normalizeLexeme({ ...draft, tags: tagsText.split(/[\n,]/) });
  return JSON.stringify(serializeLexeme(candidate)) !== JSON.stringify(serializeLexeme(baseline));
}

async function tryLeaveLexicon(confirmLeave: (message: string) => Promise<boolean> | boolean) {
  if (!lexiconHasDraft()) return true;
  if (lexiconSaving || lexiconImporting) return false;
  const allowed = await confirmLeave($pluginText("You have unsaved changes in the word editor. Discard them?"));
  if (allowed) closeLexiconEditor();
  return allowed;
}

$effect(() => {
  registerLeaveGuard(() => tryLeaveLexicon((message) => confirm("Unsaved changes", message)));
});

const activeFilterCount = $derived(
  [search, statusFilter, tagFilter, homonymsOnly ? "homonyms" : ""].filter(Boolean).length,
);
const filtered = $derived(Boolean(search || statusFilter || tagFilter || homonymsOnly));
const attached = $derived(paradigms.find((record) => record.id === draft.paradigmId));
const firstResult = $derived(page * 50 + 1);
const lastResult = $derived(page * 50 + records.length);

async function loadRecords() {
  if (!selectedLanguage) {
    records = [];
    paradigms = [];
    lexiconLoading = false;
    error = "";
    notice = "";
    return;
  }
  const token = ++request;
  lexiconLoading = true;
  error = "";
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
      error = "";
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
    }
  } catch (cause) {
    if (!cancelled && token === request) {
      lexiconLoading = false;
      error = cause instanceof Error ? cause.message : String(cause);
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

async function openPendingLexeme(id: string) {
  const token = request;
  try {
    const target = records.find((record) => record.id === id) ?? (await findLexeme(id, token));
    if (cancelled || token !== request) return;
    if (target) {
      editing = target;
      editorOpen = true;
      draft = normalizeLexeme(target.value);
      tagsText = draft.tags.join("\n");
      error = "";
      void refreshHomonyms(draft.lemma);
    } else {
      error = $pluginText("The linked word could not be found in this language.");
    }
  } catch (cause) {
    if (!cancelled && token === request) error = cause instanceof Error ? cause.message : String(cause);
  } finally {
    if (!cancelled && token === request) onPendingLexemeHandled();
  }
}

async function refreshHomonyms(lemma: string) {
  if (!selectedLanguage || !lemma) {
    homonymRequest += 1;
    homonymCount = 0;
    return;
  }
  const token = ++homonymRequest;
  const ownerLanguageId = selectedLanguage.id;
  const editingId = editing?.id;
  try {
    const matches = await context.records.list<LexemeValue>("lexemes", ownerLanguageId, {
      query: lemma,
      limit: 100,
    });
    if (cancelled || token !== homonymRequest || selectedLanguage?.id !== ownerLanguageId) return;
    homonymCount = matches.filter(
      (record) => record.value.lemma.toLocaleLowerCase() === lemma.toLocaleLowerCase() && record.id !== editingId,
    ).length;
  } catch {
    if (!cancelled && token === homonymRequest) homonymCount = 0;
  }
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
  tagsText = "";
  homonymCount = 0;
  error = "";
  notice = "";
}

function openLexiconEditor(record: ModuleRecord<LexemeValue>) {
  editing = record;
  editorOpen = true;
  draft = normalizeLexeme(record.value);
  tagsText = draft.tags.join("\n");
  error = "";
  notice = "";
  void refreshHomonyms(draft.lemma);
}

async function addHomonym() {
  if (lexiconSaving) return;
  if (
    lexiconHasDraft() &&
    !(await confirm(
      "Start a homonym",
      "Unsaved changes to this word will be discarded. Start a separate entry with the same lemma?",
    ))
  ) {
    return;
  }
  const lemma = draft.lemma;
  editing = null;
  editorOpen = true;
  draft = { ...emptyLexeme(), lemma };
  tagsText = "";
  error = "";
  notice = "";
  void refreshHomonyms(lemma);
}

function closeLexiconEditor() {
  editing = null;
  editorOpen = false;
  draft = emptyLexeme();
  tagsText = "";
  homonymRequest += 1;
  homonymCount = 0;
  error = "";
  notice = "";
}

async function saveLexeme(): Promise<"ok" | "lemma" | "error" | "none"> {
  if (!selectedLanguage || lexiconSaving) return "none";
  const ownerLanguageId = selectedLanguage.id;
  const value = normalizeLexeme(draft);
  if (!value.lemma) {
    error = $pluginText("Lemma is required.");
    return "lemma";
  }
  error = "";
  draft = value;
  lexiconSaving = true;
  setMutationActive(true);
  try {
    const payload = serializeLexeme(value);
    if (editing) {
      const updated = await context.records.update("lexemes", editing.id, ownerLanguageId, payload, {
        expectedRevision: editing.revision,
        requestId: crypto.randomUUID(),
      });
      editing = { ...updated, value: normalizeLexeme(updated.value) };
    } else {
      const created = await context.records.create("lexemes", ownerLanguageId, payload, {
        requestId: crypto.randomUUID(),
      });
      editing = { ...created, value: normalizeLexeme(created.value) };
    }
    lexiconSaving = false;
    setMutationActive(false);
    if (ownerLanguageId === selectedLanguage?.id) await loadRecords();
    closeLexiconEditor();
    return "ok";
  } catch (cause) {
    lexiconSaving = false;
    setMutationActive(false);
    error = cause instanceof Error ? cause.message : String(cause);
    return "error";
  }
}

async function deleteLexeme() {
  if (!selectedLanguage || !editing) return;
  if (!(await confirm("Delete", formatText("Delete “{name}”?", { name: editing.value.lemma })))) return;
  const ownerLanguageId = selectedLanguage.id;
  try {
    setMutationActive(true);
    await context.records.delete("lexemes", editing.id, ownerLanguageId, {
      expectedRevision: editing.revision,
      requestId: crypto.randomUUID(),
    });
    editing = null;
    editorOpen = false;
    draft = emptyLexeme();
    error = "";
    setMutationActive(false);
    if (ownerLanguageId === selectedLanguage?.id) await loadRecords();
  } catch (cause) {
    setMutationActive(false);
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
  if (!selectedLanguage || lexiconExporting) return;
  const ownerLanguageId = selectedLanguage.id;
  const languageName = selectedLanguage.name;
  lexiconExporting = true;
  error = "";
  notice = "";
  setMutationActive(true);
  const values: LexemeValue[] = [];
  try {
    for (let offset = 0; ; offset += 100) {
      const batch = await context.records.list<LexemeValue>("lexemes", ownerLanguageId, {
        limit: 100,
        offset,
        sort: "lemma",
      });
      values.push(...batch.map((record) => normalizeLexeme(record.value)));
      if (batch.length < 100) break;
    }
    const blob = new Blob([lexiconExport(languageName, values)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `${languageName.replace(/\s+/g, "-").toLowerCase()}-lexicon.json`;
    link.click();
    URL.revokeObjectURL(url);
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  } finally {
    lexiconExporting = false;
    setMutationActive(false);
  }
}

async function importLexicon(file: File) {
  if (!selectedLanguage || lexiconImporting) return;
  const ownerLanguageId = selectedLanguage.id;
  let imported = 0;
  let mutationStarted = false;
  try {
    const lexemes = parseLexiconImport(await file.text());
    if (lexemes.length === 0) {
      error = $pluginText("This file does not contain any lexicon entries.");
      return;
    }
    if (
      !(await confirm(
        "Import lexicon",
        `Add ${lexemes.length} ${lexemes.length === 1 ? "word" : "words"} to ${selectedLanguage.name}? Existing entries will not be changed.`,
      ))
    ) {
      return;
    }
    lexiconImporting = true;
    error = "";
    notice = "";
    setMutationActive(true);
    mutationStarted = true;
    for (const value of lexemes) {
      await context.records.create("lexemes", ownerLanguageId, serializeLexeme(value), {
        requestId: crypto.randomUUID(),
      });
      imported += 1;
    }
    if (ownerLanguageId === selectedLanguage?.id) {
      page = 0;
      await loadRecords();
      notice = `Imported ${imported} ${imported === 1 ? "word" : "words"}.`;
    }
  } catch (cause) {
    const message = cause instanceof Error ? cause.message : String(cause);
    error =
      imported > 0
        ? `${message} ${imported} ${imported === 1 ? "word was" : "words were"} imported before the import stopped.`
        : message;
  } finally {
    lexiconImporting = false;
    if (mutationStarted) setMutationActive(false);
  }
}

function handleImportChange() {
  const chosen = fileInput?.files?.[0];
  if (fileInput) fileInput.value = "";
  if (chosen) void importLexicon(chosen);
}
function clearFilters() {
  search = "";
  statusFilterInput = "";
  tagFilterInput = "";
  sort = "lemma";
  homonymsOnly = false;
}

function addPronunciation() {
  draft.pronunciations.push({ id: crypto.randomUUID(), value: "" });
}

function removePronunciation(index: number) {
  draft.pronunciations.splice(index, 1);
}

function addForm() {
  draft.forms.push({ id: crypto.randomUUID(), form: "" });
}

function removeForm(index: number) {
  draft.forms.splice(index, 1);
}

function addSense() {
  draft.senses.push({ id: crypto.randomUUID(), examples: [] });
}

function removeSense(index: number) {
  draft.senses.splice(index, 1);
  if (draft.senses.length === 0) draft.senses.push({ id: crypto.randomUUID(), examples: [] });
}

function addExample(senseIndex: number) {
  draft.senses[senseIndex].examples.push({ id: crypto.randomUUID(), text: "" });
}

function removeExample(senseIndex: number, exampleIndex: number) {
  draft.senses[senseIndex].examples.splice(exampleIndex, 1);
}

function pinSlotOverride(slot: ParadigmSlot, form: string) {
  if (!attached) return;
  draft.forms = pinOverride(draft.forms, attached.id, slot, form);
}

async function clearSlotOverride(slot: ParadigmSlot) {
  if (!attached) return;
  const target = overrideTarget(draft.forms, attached.id, slot);
  if (target?.legacy) {
    if (
      !(await confirm(
        "Clear legacy form",
        `“${target.form}” is an unscoped form matched by label “${slot.label}”. Removing it also deletes a manually authored form. Remove it anyway?`,
      ))
    ) {
      return;
    }
  }
  draft.forms = clearOverride(draft.forms, attached.id, slot);
}

async function handleSubmit(event: SubmitEvent) {
  event.preventDefault();
  draft.tags = tagsText.split(/[\n,]/);
  const outcome = await saveLexeme();
  if (outcome === "lemma") lemmaInput?.focus();
}
</script>

{#if editorOpen}
  <form class="language-editor" onsubmit={handleSubmit}>
    <div class="visually-hidden">
      <datalist id="language-pos">
        {#each PART_OF_SPEECH_SUGGESTIONS as suggestion}
          <option value={suggestion}>{suggestion}</option>
        {/each}
      </datalist>
      <datalist id="language-status">
        {#each STATUS_SUGGESTIONS as suggestion}
          <option value={suggestion}>{suggestion}</option>
        {/each}
      </datalist>
    </div>
    <div class="language-editor-head">
      <h3>{editing ? $pluginText("Edit word") : $pluginText("New word")}</h3>
      <p>{$pluginText("Capture the core meaning first; pronunciation, forms, and notes can grow with the entry.")}</p>
    </div>
    <section class="language-form-section">
      <h3>{$pluginText("Core details")}</h3>
      <div class="language-section-grid">
        <label class="language-field language-field-wide">
          <span>{$pluginText("Lemma")}</span>
          <input name="lemma" bind:this={lemmaInput} bind:value={draft.lemma} />
        </label>
        <label class="language-field">
          <span>{$pluginText("Part of speech (optional)")}</span>
          <input name="partOfSpeech" list="language-pos" bind:value={draft.partOfSpeech} />
        </label>
        <label class="language-field">
          <span>{$pluginText("Status (optional)")}</span>
          <input name="status" list="language-status" bind:value={draft.status} />
        </label>
        <label class="language-field">
          <span>{$pluginText("Tags — comma or line separated (optional)")}</span>
          <textarea name="tags" rows={2} bind:value={tagsText}></textarea>
        </label>
      </div>
    </section>
    <label class="language-field">
      <span>{$pluginText("Paradigm (optional)")}</span>
      <select name="paradigmId" aria-label={$pluginText("Paradigm")} bind:value={draft.paradigmId}>
        <option value={""}>{$pluginText("None")}</option>
        {#each paradigms as record (record.id)}
          <option value={record.id}>{record.value.name || $pluginText("Untitled paradigm")}</option>
        {/each}
      </select>
    </label>
    {#if homonymCount > 0}
      <p class="language-status">
        {homonymCount} {$pluginText("other")} {homonymCount === 1 ? $pluginText("entry shares") : $pluginText("entries share")} {$pluginText("this lemma. Duplicate lemmas are kept as distinct homonyms.")}
      </p>
    {/if}
    <section class="language-group language-form-section">
      <div class="language-group-head">
        <h3>{$pluginText("Pronunciation variants")}</h3>
        <button type="button" class="language-button secondary" onclick={addPronunciation}>{$pluginText("Add")}</button>
      </div>
      {#each draft.pronunciations as pronunciation, index (pronunciation.id)}
        <div class="language-inline">
          <div class="language-inline-fields">
            <IpaInput label={$pluginText("Pronunciation")} name={`pronunciation-${index}`} bind:value={pronunciation.value} />
            <label class="language-field">
              <span>{$pluginText("Note (optional)")}</span>
              <input name={`pronunciation-note-${index}`} bind:value={pronunciation.note} />
            </label>
          </div>
          <button
            type="button"
            class="language-button secondary language-danger"
            onclick={() => removePronunciation(index)}>{$pluginText("Remove")}</button>
        </div>
      {/each}
    </section>
    <section class="language-group language-form-section">
      <div class="language-group-head">
        <h3>{$pluginText("Alternate forms")}</h3>
        <button type="button" class="language-button secondary" onclick={addForm}>{$pluginText("Add")}</button>
      </div>
      {#each draft.forms as form, index (form.id)}
        <div class="language-inline">
          <div class="language-inline-fields">
            <label class="language-field">
              <span>{$pluginText("Form")}</span>
              <input name={`form-${index}`} bind:value={form.form} />
            </label>
            <label class="language-field">
              <span>{$pluginText("Kind (optional)")}</span>
              <input name={`form-kind-${index}`} bind:value={form.kind} />
            </label>
            <IpaInput
              label={$pluginText("Pronunciation (optional)")}
              name={`form-pronunciation-${index}`}
              bind:value={form.pronunciation} />
          </div>
          <button type="button" class="language-button secondary language-danger" onclick={() => removeForm(index)}
            >{$pluginText("Remove")}</button>
        </div>
      {/each}
    </section>
    <section class="language-group language-form-section">
      <div class="language-group-head">
        <h3>{$pluginText("Senses")}</h3>
        <button type="button" class="language-button secondary" onclick={addSense}>{$pluginText("Add")}</button>
      </div>
      {#each draft.senses as sense, index (sense.id)}
        <div class="language-group">
          <div class="language-group-head">
            <h3>{$pluginText("Sense")} {index + 1}</h3>
            <button type="button" class="language-button secondary language-danger" onclick={() => removeSense(index)}
              >{$pluginText("Remove sense")}</button>
          </div>
          <label class="language-field">
            <span>{$pluginText("Gloss (optional)")}</span>
            <input name={`sense-gloss-${index}`} bind:value={sense.gloss} />
          </label>
          <label class="language-field">
            <span>{$pluginText("Definition (optional)")}</span>
            <textarea name={`sense-definition-${index}`} rows={2} bind:value={sense.definition}></textarea>
          </label>
          <label class="language-field">
            <span>{$pluginText("Usage notes (optional)")}</span>
            <textarea name={`sense-usage-${index}`} rows={2} bind:value={sense.usageNotes}></textarea>
          </label>
          {#each sense.examples as example, exampleIndex (example.id)}
            <div class="language-inline">
              <div class="language-inline-fields">
                <label class="language-field">
                  <span>{$pluginText("Example")}</span>
                  <textarea name={`sense-${index}-example-${exampleIndex}`} rows={2} bind:value={example.text}
                  ></textarea>
                </label>
                <label class="language-field">
                  <span>{$pluginText("Translation (optional)")}</span>
                  <textarea
                    name={`sense-${index}-translation-${exampleIndex}`}
                    rows={2}
                    bind:value={example.translation}></textarea>
                </label>
              </div>
              <button
                type="button"
                class="language-button secondary language-danger"
                onclick={() => removeExample(index, exampleIndex)}>{$pluginText("Remove")}</button>
            </div>
          {/each}
          <button type="button" class="language-button secondary" onclick={() => addExample(index)}>{$pluginText("Add example")}</button>
        </div>
      {/each}
    </section>
    <label class="language-field">
      <span>{$pluginText("Etymology (optional)")}</span>
      <textarea name="etymology" bind:value={draft.etymology}></textarea>
    </label>
    <label class="language-field">
      <span>{$pluginText("Source notes (optional)")}</span>
      <textarea name="sourceNotes" bind:value={draft.sourceNotes}></textarea>
    </label>
    <label class="language-field">
      <span>{$pluginText("Notes (optional)")}</span>
      <textarea name="notes" bind:value={draft.notes}></textarea>
    </label>
    {#if attached}
      <section class="language-group language-form-section">
        <h3>{$pluginText("Generated forms preview")}</h3>
        <p class="language-empty" role="status">
          {$pluginText("Generated cells are a preview. Pinning stores an authored override on this word; changing a rule does not delete pinned or other authored forms.")}
        </p>
        <div class="language-chart-wrap">
          <table class="paradigm-preview">
            <thead>
              <tr>
                <th>{$pluginText("Slot")}</th>
                <th>{$pluginText("Form")}</th>
                <th>{$pluginText("Source")}</th>
                <th>{$pluginText("Rule")}</th>
                <th>{$pluginText("Override")}</th>
              </tr>
            </thead>
            <tbody>
              {#each previewParadigm(attached.value, draft.lemma, draft.forms, attached.id) as cell}
                <tr>
                  <th scope="row"
                    >{cell.slot.features ? `${cell.slot.label} (${cell.slot.features})` : cell.slot.label}</th>
                  <td
                    >{cell.form ||
                      "—"}{#if cell.provenance === "authored" && cell.generated && cell.generated !== cell.form}
                      <small> {$pluginText("rule:")} {cell.generated}</small>
                    {/if}</td>
                  <td
                    ><span
                      class="form-provenance"
                      class:is-authored={cell.provenance === "authored"}
                      class:is-missing={cell.provenance === "missing"}
                      >{cell.provenance === "authored"
                        ? $pluginText("authored")
                        : cell.provenance === "generated"
                          ? $pluginText("generated")
                          : $pluginText("no rule")}</span
                    ></td>
                  <td>{cell.ruleName || "—"}</td>
                  <td
                    >{#if cell.provenance === "generated" && cell.form}
                      <button
                        type="button"
                        class="language-button secondary"
                        onclick={() => pinSlotOverride(cell.slot, cell.form)}>{$pluginText("Pin override")}</button>
                    {:else if cell.provenance === "authored"}
                      <button
                        type="button"
                        class="language-button secondary"
                        onclick={() => clearSlotOverride(cell.slot)}>{$pluginText("Clear override")}</button>
                    {/if}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </section>
    {/if}
    {#if error}
      <p class="language-status error" role="alert">{error}</p>
    {/if}
    {#if lexiconSaving}
      <p class="language-empty" role="status">{$pluginText("Saving word…")}</p>
    {/if}
    <div class="language-actions">
      <span>
        {#if editing}
          <button type="button" class="language-button secondary" onclick={() => void addHomonym()}>{$pluginText("Add homonym")}</button>
          <button type="button" class="language-button secondary language-danger" onclick={deleteLexeme}>{$pluginText("Delete")}</button>
        {/if}
      </span>
      <span>
        <button type="button" class="language-button secondary" onclick={closeLexiconEditor}>{$pluginText("Cancel")}</button>
        <button type="submit" class="language-button" disabled={lexiconSaving}
          >{lexiconSaving ? $pluginText("Saving…") : $pluginText("Save word")}</button>
      </span>
    </div>
  </form>
{:else}
  <div class="language-toolbar">
    <div class="language-toolbar-title">
      <h2>{$pluginText("Lexicon")}</h2>
    </div>
    <div class="language-toolbar-actions">
      <input
        type="file"
        accept="application/json,.json"
        class="file-input"
        aria-label={$pluginText("Import lexicon JSON")}
        bind:this={fileInput}
        onchange={handleImportChange} />
      <button
        type="button"
        class="language-button secondary"
        disabled={!selectedLanguage || lexiconExporting}
        onclick={exportLexicon}>{lexiconExporting ? $pluginText("Exporting…") : $pluginText("Export JSON")}</button>
      <button
        type="button"
        class="language-button secondary"
        disabled={!selectedLanguage || lexiconImporting}
        onclick={() => fileInput?.click()}>{lexiconImporting ? $pluginText("Importing…") : $pluginText("Import JSON")}</button>
      <button
        type="button"
        class="language-button"
        data-guide="language-add-word"
        disabled={!selectedLanguage}
        onclick={addWord}>{$pluginText("Add word")}</button>
    </div>
  </div>
  {#if selectedLanguage}
    <div class="language-search-row">
      <label class="language-field language-search-field">
        <span>{$pluginText("Search lemma or meaning")}</span>
        <input
          name="search"
          class="language-search"
          type="search"
          bind:value={search}
          placeholder={$pluginText("Type to search...")} />
      </label>
      {#if filtered}
        <div class="language-filter-badge">
          {$pluginText(activeFilterCount === 1 ? "{count} filter active" : "{count} filters active", { count: activeFilterCount })}
          <button
            type="button"
            class="language-filter-badge-clear"
            onclick={clearFilters}
            aria-label={$pluginText("Clear all filters")}>&times;</button>
        </div>
      {/if}
    </div>
    <details class="language-filter-panel">
      <summary>
        <span>{$pluginText("Filters and sorting")}</span>
        {#if activeFilterCount > 0}
          <span class="language-filter-count">{activeFilterCount}</span>
        {/if}
      </summary>
      <div class="language-filters">
        <label class="language-field">
          <span>{$pluginText("Status")}</span>
          <input
            name="statusFilter"
            list="language-filter-status"
            bind:value={statusFilterInput}
            placeholder={$pluginText("Any status")} />
        </label>
        <label class="language-field">
          <span>{$pluginText("Tag")}</span>
          <input name="tagFilter" bind:value={tagFilterInput} placeholder={$pluginText("Any tag")} />
        </label>
        <label class="language-field">
          <span>{$pluginText("Sort by")}</span>
          <select name="sort" aria-label={$pluginText("Sort lexicon")} bind:value={sort}>
            <option value="lemma">{$pluginText("Lemma")}</option>
            <option value="status">{$pluginText("Status")}</option>
            <option value="updatedAt">{$pluginText("Last updated")}</option>
          </select>
        </label>
        <label class="language-check">
          <input type="checkbox" bind:checked={homonymsOnly} />{$pluginText("Homonyms only")}
        </label>
        <datalist id="language-filter-status">
          {#each STATUS_SUGGESTIONS as suggestion}
            <option value={suggestion}>{suggestion}</option>
          {/each}
        </datalist>
        {#if activeFilterCount > 0}
          <div class="language-filter-actions">
            <button type="button" class="language-button secondary" onclick={clearFilters}>{$pluginText("Clear all filters")}</button>
          </div>
        {/if}
      </div>
    </details>
  {/if}
  {#if notice}
    <p class="language-status success" role="status" aria-live="polite">{notice}</p>
  {/if}
  {#if error}
    <div class="language-empty-card language-error-card">
      <p class="language-status error" role="alert">{error}</p>
      {#if selectedLanguage}
        <button type="button" class="language-button secondary" onclick={() => void loadRecords()}>{$pluginText("Try again")}</button>
      {/if}
    </div>
  {:else if !selectedLanguage}
    <p class="language-empty" role="status">{$pluginText("Select a language to view its lexicon.")}</p>
  {:else if lexiconLoading}
    <p class="language-empty language-loading" role="status">{$pluginText("Loading lexicon…")}</p>
  {:else if records.length === 0}
    <div class="language-empty-card">
      <p class="language-empty" role="status">{filtered ? $pluginText("No words match these filters.") : $pluginText("No words yet.")}</p>
      <div class="language-inline">
        {#if filtered}
          <button type="button" class="language-button secondary" onclick={clearFilters}>{$pluginText("Clear filters")}</button>
        {:else}
          <button type="button" class="language-button" data-guide="language-add-word" onclick={addWord}
            >{$pluginText("Add word")}</button>
        {/if}
      </div>
    </div>
  {:else}
    <p class="language-results" role="status">{$pluginText("Showing")} {firstResult}–{lastResult}{hasNextPage ? "+" : ""} {$pluginText("words")}</p>
    <ul class="lexeme-list">
      {#each records as record (record.id)}
        <li>
          <button
            type="button"
            class="language-item lexeme-row"
            aria-label={$pluginText("Edit {0}", { 0: String(record.value.lemma || "word") })}
            onclick={() => openLexiconEditor(record)}>
            <strong>{record.value.lemma}</strong>
            <small class="lexeme-part">{record.value.partOfSpeech || "—"}</small>
            <span class="lexeme-meaning">{firstGloss(record.value) || $pluginText("No gloss yet")}</span>
            <small class="lexeme-status"
              >{[record.value.status, record.value.tags[0]].filter(Boolean).join(" · ") || "—"}</small>
          </button>
        </li>
      {/each}
    </ul>
    {#if page > 0 || hasNextPage}
      <div class="language-actions">
        <button type="button" class="language-button secondary" disabled={page === 0} onclick={previousPage}
          >{$pluginText("Previous")}</button>
        <button type="button" class="language-button secondary" disabled={!hasNextPage} onclick={nextPage}>{$pluginText("Next")}</button>
      </div>
    {/if}
  {/if}
{/if}

<style>
.language-field-wide {
  grid-column: 1/-1;
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
.language-danger {
  border-color: var(--danger) !important;
  color: var(--danger) !important;
  background: transparent;
}
.language-inline {
  display: flex;
  align-items: end;
  gap: 8px;
  min-width: 0;
}
.language-inline-fields {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: 8px;
  flex: 1;
  min-width: 0;
}
.language-inline > .language-button {
  flex: 0 0 auto;
}
.file-input {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
}
@media (max-width: 760px) {
  .language-filters,
  .lexeme-row,
  .language-item,
  .language-section-grid {
    grid-template-columns: 1fr;
  }
  .language-item span,
  .lexeme-row span,
  .lexeme-row small {
    white-space: normal;
  }
  .lexeme-status {
    justify-self: start;
  }
  .language-inline {
    flex-direction: column;
    align-items: stretch;
  }
}
.lexeme-list {
  display: grid;
  gap: 8px;
  margin: 4px 0 0;
  padding: 0;
  list-style: none;
}
.language-search-row {
  display: flex;
  align-items: flex-end;
  gap: 12px;
  margin-top: 16px;
}
.language-search-field {
  flex: 1;
  min-width: 0;
}
.language-filter-badge {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  border: 1px solid var(--accent);
  border-radius: 8px;
  background: var(--surface-muted);
  color: var(--accent-dark);
  font-size: 12px;
  font-weight: 600;
  white-space: nowrap;
}
.language-filter-badge-clear {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  padding: 0;
  border: none;
  border-radius: 50%;
  background: transparent;
  color: var(--accent-dark);
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
}
.language-filter-badge-clear:hover {
  background: var(--surface);
}
.language-filter-panel {
  margin-top: 10px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-muted);
}
.language-filter-panel summary {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  color: var(--ink-soft);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  list-style-position: inside;
}
.language-filter-panel summary:hover {
  color: var(--ink);
}
.language-filter-count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  border-radius: 999px;
  background: var(--accent);
  color: var(--on-bright-accent);
  font-size: 10px;
  font-weight: 700;
}
.language-filter-panel[open] summary {
  border-bottom: 1px solid var(--line);
}
.language-filter-panel .language-filters {
  margin: 0;
  padding: 12px;
}
.language-filters {
  display: grid;
  grid-template-columns: repeat(3, minmax(110px, 1fr)) minmax(120px, auto);
  gap: 10px 12px;
  align-items: end;
}
.language-filters .language-check {
  padding: 2px 0 0;
  white-space: nowrap;
}
.language-filter-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  grid-column: 1/-1;
  flex-wrap: wrap;
}
.language-search,
.language-filters input,
.language-filters select,
.language-field input,
.language-field textarea,
.language-field select {
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
.language-check {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--ink-soft);
  font-size: 12px;
}
.language-chart-wrap {
  overflow-x: auto;
  margin: 8px 0 4px;
}
.paradigm-preview {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.paradigm-preview th,
.paradigm-preview td {
  border: 1px solid var(--line);
  padding: 8px;
  text-align: left;
  min-width: 52px;
}
.paradigm-preview th {
  background: var(--surface-muted);
  font-weight: 600;
  color: var(--ink-soft);
}
.visually-hidden {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
.paradigm-preview {
  margin: 12px 0;
}
.form-provenance {
  display: inline-block;
  padding: 2px 7px;
  border-radius: 999px;
  background: var(--surface);
  font-size: 10px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--ink-soft);
}
.form-provenance.is-authored {
  color: var(--accent-dark);
  background: var(--theme-success-bg, #eef3ef);
}
.form-provenance.is-missing {
  color: var(--ink-faint);
}
.language-item,
.lexeme-row {
  display: grid;
  grid-template-columns: minmax(0, 1.2fr) auto minmax(0, 1.4fr);
  gap: 8px 12px;
  align-items: baseline;
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--theme-warning-border, #ebe7de);
  border-radius: 10px;
  background: var(--surface);
  color: inherit;
  text-align: left;
  cursor: pointer;
  box-shadow: 0 1px 2px rgba(38, 42, 33, 0.03);
}
.lexeme-row {
  grid-template-columns: minmax(0, 1.05fr) minmax(0, 0.6fr) minmax(0, 1.55fr) minmax(0, 0.7fr);
  padding: 13px 14px;
}
.language-item:hover,
.lexeme-row:hover {
  border-color: var(--theme-warning-border, #e5d8c6);
  background: var(--surface-muted);
}
.language-item strong,
.lexeme-row strong {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.language-item small,
.lexeme-row small {
  color: var(--ink-faint);
}
.language-item span,
.lexeme-row span {
  min-width: 0;
  color: var(--ink-soft);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.lexeme-meaning {
  font-size: 13px;
}
.lexeme-status {
  justify-self: end;
  padding: 3px 7px;
  border-radius: 999px;
  background: var(--surface-muted);
  font-size: 10px;
  letter-spacing: 0.03em;
}
.language-results {
  margin: 14px 0 0;
  color: var(--ink-faint);
  font-size: 11px;
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
.language-item:focus-visible,
.lexeme-row:focus-visible {
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
  color: var(--danger);
}
.language-status.success {
  margin-top: 14px;
  color: var(--accent-dark);
}
.language-error-card {
  border-color: var(--theme-danger-border, #e2b7af);
  background: var(--theme-danger-bg, #fff5f2);
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
@keyframes language-spin {
  to {
    transform: rotate(360deg);
  }
}
@media (prefers-reduced-motion: reduce) {
  .language-loading::before {
    animation: none;
  }
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
.language-editor-head {
  display: grid;
  gap: 4px;
  padding-bottom: 2px;
}
.language-editor-head p {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.55;
}
.language-form-section {
  display: grid;
  gap: 10px;
  min-width: 0;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-muted);
}
.language-section-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px 12px;
}
</style>
