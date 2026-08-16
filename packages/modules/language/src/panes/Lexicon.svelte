<script lang="ts">
import type { EntitySummary, ModuleRecord, ModuleRecordQuery } from "../../../../module-api/src/index";
import type { LexemeValue } from "../lexeme";
import { firstGloss, PART_OF_SPEECH_SUGGESTIONS, STATUS_SUGGESTIONS } from "../lexeme";
import type { Paradigm, ParadigmSlot } from "../morphology";
import { clearOverride, pinOverride, previewParadigm } from "../morphology";

let {
  selectedLanguage,
  records,
  paradigms,
  editing,
  editorOpen,
  draft,
  search = $bindable(""),
  statusFilterInput = $bindable(""),
  tagFilterInput = $bindable(""),
  sort = $bindable("lemma"),
  homonymsOnly = $bindable(false),
  page,
  hasNextPage,
  homonymCount,
  lexiconLoading,
  lexiconSaving,
  error,
  addWord,
  openLexiconEditor,
  addHomonym,
  closeLexiconEditor,
  saveLexeme,
  deleteLexeme,
  previousPage,
  nextPage,
  importLexicon,
  exportLexicon,
}: {
  selectedLanguage: EntitySummary | null;
  records: ModuleRecord<LexemeValue>[];
  paradigms: ModuleRecord<Paradigm>[];
  editing: ModuleRecord<LexemeValue> | null;
  editorOpen: boolean;
  draft: LexemeValue;
  search: string;
  statusFilterInput: string;
  tagFilterInput: string;
  sort: ModuleRecordQuery["sort"];
  homonymsOnly: boolean;
  page: number;
  hasNextPage: boolean;
  homonymCount: number;
  lexiconLoading: boolean;
  lexiconSaving: boolean;
  error: string;
  addWord: () => void;
  openLexiconEditor: (record: ModuleRecord<LexemeValue>) => void;
  addHomonym: () => void;
  closeLexiconEditor: () => void;
  saveLexeme: () => Promise<"ok" | "lemma" | "error" | "none">;
  deleteLexeme: () => void;
  previousPage: () => void;
  nextPage: () => void;
  importLexicon: (file: File) => void;
  exportLexicon: () => void;
} = $props();

const statusFilter = $derived(statusFilterInput.trim());
const tagFilter = $derived(tagFilterInput.trim());

let tagsText = $state("");
let fileInput: HTMLInputElement | undefined = $state();
let lemmaInput: HTMLInputElement | undefined = $state();

const activeFilterCount = $derived(
  [search, statusFilter, tagFilter, homonymsOnly ? "homonyms" : ""].filter(Boolean).length,
);
const filtered = $derived(Boolean(search || statusFilter || tagFilter || homonymsOnly));
const attached = $derived(paradigms.find((record) => record.id === draft.paradigmId));
const firstResult = $derived(page * 50 + 1);
const lastResult = $derived(page * 50 + records.length);

$effect(() => {
  if (editorOpen && lemmaInput) lemmaInput.focus();
});

let previousEditorOpen = false;

$effect(() => {
  if (editorOpen && !previousEditorOpen) tagsText = draft.tags.join("\n");
  previousEditorOpen = editorOpen;
});

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

function clearSlotOverride(slot: ParadigmSlot) {
  if (!attached) return;
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
      <h3>{editing ? "Edit word" : "New word"}</h3>
      <p>Capture the core meaning first; pronunciation, forms, and notes can grow with the entry.</p>
    </div>
    <section class="language-form-section">
      <h3>Core details</h3>
      <div class="language-section-grid">
        <label class="language-field language-field-wide">
          <span>Lemma</span>
          <input name="lemma" bind:this={lemmaInput} bind:value={draft.lemma} />
        </label>
        <label class="language-field">
          <span>Part of speech (optional)</span>
          <input name="partOfSpeech" list="language-pos" bind:value={draft.partOfSpeech} />
        </label>
        <label class="language-field">
          <span>Status (optional)</span>
          <input name="status" list="language-status" bind:value={draft.status} />
        </label>
        <label class="language-field">
          <span>Tags — comma or line separated (optional)</span>
          <textarea name="tags" rows={2} bind:value={tagsText}></textarea>
        </label>
      </div>
    </section>
    <label class="language-field">
      <span>Paradigm (optional)</span>
      <select name="paradigmId" aria-label="Paradigm" bind:value={draft.paradigmId}>
        <option value={""}>None</option>
        {#each paradigms as record (record.id)}
          <option value={record.id}>{record.value.name || "Untitled paradigm"}</option>
        {/each}
      </select>
    </label>
    {#if homonymCount > 0}
      <p class="language-status">
        {homonymCount} other {homonymCount === 1 ? "entry shares" : "entries share"} this lemma. Duplicate lemmas are kept
        as distinct homonyms.
      </p>
    {/if}
    <section class="language-group language-form-section">
      <div class="language-group-head">
        <h3>Pronunciation variants</h3>
        <button type="button" class="language-button secondary" onclick={addPronunciation}>Add</button>
      </div>
      {#each draft.pronunciations as pronunciation, index (pronunciation.id)}
        <div class="language-inline">
          <div class="language-inline-fields">
            <label class="language-field">
              <span>Pronunciation</span>
              <input name={`pronunciation-${index}`} bind:value={pronunciation.value} />
            </label>
            <label class="language-field">
              <span>Note (optional)</span>
              <input name={`pronunciation-note-${index}`} bind:value={pronunciation.note} />
            </label>
          </div>
          <button
            type="button"
            class="language-button secondary language-danger"
            onclick={() => removePronunciation(index)}>Remove</button>
        </div>
      {/each}
    </section>
    <section class="language-group language-form-section">
      <div class="language-group-head">
        <h3>Alternate forms</h3>
        <button type="button" class="language-button secondary" onclick={addForm}>Add</button>
      </div>
      {#each draft.forms as form, index (form.id)}
        <div class="language-inline">
          <div class="language-inline-fields">
            <label class="language-field">
              <span>Form</span>
              <input name={`form-${index}`} bind:value={form.form} />
            </label>
            <label class="language-field">
              <span>Kind (optional)</span>
              <input name={`form-kind-${index}`} bind:value={form.kind} />
            </label>
            <label class="language-field">
              <span>Pronunciation (optional)</span>
              <input name={`form-pronunciation-${index}`} bind:value={form.pronunciation} />
            </label>
          </div>
          <button type="button" class="language-button secondary language-danger" onclick={() => removeForm(index)}
            >Remove</button>
        </div>
      {/each}
    </section>
    <section class="language-group language-form-section">
      <div class="language-group-head">
        <h3>Senses</h3>
        <button type="button" class="language-button secondary" onclick={addSense}>Add</button>
      </div>
      {#each draft.senses as sense, index (sense.id)}
        <div class="language-group">
          <div class="language-group-head">
            <h3>Sense {index + 1}</h3>
            <button type="button" class="language-button secondary language-danger" onclick={() => removeSense(index)}
              >Remove sense</button>
          </div>
          <label class="language-field">
            <span>Gloss (optional)</span>
            <input name={`sense-gloss-${index}`} bind:value={sense.gloss} />
          </label>
          <label class="language-field">
            <span>Definition (optional)</span>
            <textarea name={`sense-definition-${index}`} rows={2} bind:value={sense.definition}></textarea>
          </label>
          <label class="language-field">
            <span>Usage notes (optional)</span>
            <textarea name={`sense-usage-${index}`} rows={2} bind:value={sense.usageNotes}></textarea>
          </label>
          {#each sense.examples as example, exampleIndex (example.id)}
            <div class="language-inline">
              <div class="language-inline-fields">
                <label class="language-field">
                  <span>Example</span>
                  <textarea name={`sense-${index}-example-${exampleIndex}`} rows={2} bind:value={example.text}
                  ></textarea>
                </label>
                <label class="language-field">
                  <span>Translation (optional)</span>
                  <textarea
                    name={`sense-${index}-translation-${exampleIndex}`}
                    rows={2}
                    bind:value={example.translation}></textarea>
                </label>
              </div>
              <button
                type="button"
                class="language-button secondary language-danger"
                onclick={() => removeExample(index, exampleIndex)}>Remove</button>
            </div>
          {/each}
          <button type="button" class="language-button secondary" onclick={() => addExample(index)}>Add example</button>
        </div>
      {/each}
    </section>
    <label class="language-field">
      <span>Etymology (optional)</span>
      <textarea name="etymology" bind:value={draft.etymology}></textarea>
    </label>
    <label class="language-field">
      <span>Source notes (optional)</span>
      <textarea name="sourceNotes" bind:value={draft.sourceNotes}></textarea>
    </label>
    <label class="language-field">
      <span>Notes (optional)</span>
      <textarea name="notes" bind:value={draft.notes}></textarea>
    </label>
    {#if attached}
      <section class="language-group language-form-section">
        <h3>Generated forms preview</h3>
        <p class="language-empty" role="status">
          Generated cells are a preview. Pinning stores an authored override on this word; changing a rule does not
          delete pinned or other authored forms.
        </p>
        <div class="language-chart-wrap">
          <table class="paradigm-preview">
            <thead>
              <tr>
                <th>Slot</th>
                <th>Form</th>
                <th>Source</th>
                <th>Rule</th>
                <th>Override</th>
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
                      <small> rule: {cell.generated}</small>
                    {/if}</td>
                  <td
                    ><span
                      class="form-provenance"
                      class:is-authored={cell.provenance === "authored"}
                      class:is-missing={cell.provenance === "missing"}
                      >{cell.provenance === "authored"
                        ? "authored"
                        : cell.provenance === "generated"
                          ? "generated"
                          : "no rule"}</span
                    ></td>
                  <td>{cell.ruleName || "—"}</td>
                  <td
                    >{#if cell.provenance === "generated" && cell.form}
                      <button
                        type="button"
                        class="language-button secondary"
                        onclick={() => pinSlotOverride(cell.slot, cell.form)}>Pin override</button>
                    {:else if cell.provenance === "authored"}
                      <button
                        type="button"
                        class="language-button secondary"
                        onclick={() => clearSlotOverride(cell.slot)}>Clear override</button>
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
      <p class="language-empty" role="status">Saving word…</p>
    {/if}
    <div class="language-actions">
      <span>
        {#if editing}
          <button type="button" class="language-button secondary" onclick={addHomonym}>Add homonym</button>
          <button type="button" class="language-button secondary language-danger" onclick={deleteLexeme}>Delete</button>
        {/if}
      </span>
      <span>
        <button type="button" class="language-button secondary" onclick={closeLexiconEditor}>Cancel</button>
        <button type="submit" class="language-button" disabled={lexiconSaving}
          >{lexiconSaving ? "Saving…" : "Save word"}</button>
      </span>
    </div>
  </form>
{:else}
  <div class="language-toolbar">
    <div class="language-toolbar-title">
      <p class="language-toolbar-eyebrow">Focused projection</p>
      <h2>Lexicon</h2>
      <p class="language-toolbar-subtitle">
        {selectedLanguage
          ? `${selectedLanguage.name} · words, meanings, and usage`
          : "Select a language to begin building its lexicon."}
      </p>
    </div>
    <div class="language-toolbar-actions">
      <input
        type="file"
        accept="application/json,.json"
        class="file-input"
        aria-label="Import lexicon JSON"
        bind:this={fileInput}
        onchange={handleImportChange} />
      <button type="button" class="language-button secondary" disabled={!selectedLanguage} onclick={exportLexicon}
        >Export JSON</button>
      <button
        type="button"
        class="language-button secondary"
        disabled={!selectedLanguage}
        onclick={() => fileInput?.click()}>Import JSON</button>
      <button type="button" class="language-button" disabled={!selectedLanguage} onclick={addWord}>Add word</button>
    </div>
  </div>
  {#if selectedLanguage}
    <div class="language-search-row">
      <label class="language-field">
        <span>Search lemma or meaning</span>
        <input name="search" class="language-search" type="search" bind:value={search} />
      </label>
    </div>
    <details class="language-filter-panel" open={activeFilterCount > 0}>
      <summary>{activeFilterCount ? `Filters · ${activeFilterCount} active` : "Filters and sorting"}</summary>
      <div class="language-filters">
        <label class="language-field">
          <span>Status</span>
          <input name="statusFilter" list="language-filter-status" bind:value={statusFilterInput} />
        </label>
        <label class="language-field">
          <span>Tag</span>
          <input name="tagFilter" bind:value={tagFilterInput} />
        </label>
        <label class="language-field">
          <span>Sort</span>
          <select name="sort" aria-label="Sort lexicon" bind:value={sort}>
            <option value="lemma">Sort by lemma</option>
            <option value="status">Sort by status</option>
            <option value="updatedAt">Sort by updated</option>
          </select>
        </label>
        <label class="language-check">
          <input type="checkbox" bind:checked={homonymsOnly} /> Homonyms only
        </label>
        <datalist id="language-filter-status">
          {#each STATUS_SUGGESTIONS as suggestion}
            <option value={suggestion}>{suggestion}</option>
          {/each}
        </datalist>
        <div class="language-filter-actions">
          <span class="language-status">Use filters to narrow the working set.</span>
          <button
            type="button"
            class="language-button secondary"
            disabled={activeFilterCount === 0}
            onclick={clearFilters}>Clear filters</button>
        </div>
      </div>
    </details>
  {/if}
  {#if error}
    <p class="language-status error" role="alert">{error}</p>
  {:else if !selectedLanguage}
    <p class="language-empty" role="status">Select a language to view its lexicon.</p>
  {:else if lexiconLoading}
    <p class="language-empty language-loading" role="status">Loading lexicon…</p>
  {:else if records.length === 0}
    <div class="language-empty-card">
      <p class="language-empty" role="status">{filtered ? "No words match these filters." : "No words yet."}</p>
      <div class="language-inline">
        {#if filtered}
          <button type="button" class="language-button secondary" onclick={clearFilters}>Clear filters</button>
        {:else}
          <button type="button" class="language-button" onclick={addWord}>Add word</button>
        {/if}
      </div>
    </div>
  {:else}
    <p class="language-results" role="status">Showing {firstResult}–{lastResult}{hasNextPage ? "+" : ""} words</p>
    <ul class="lexeme-list">
      {#each records as record (record.id)}
        <li>
          <button
            type="button"
            class="language-item lexeme-row"
            aria-label={`Edit ${record.value.lemma || "word"}`}
            onclick={() => openLexiconEditor(record)}>
            <strong>{record.value.lemma}</strong>
            <small class="lexeme-part">{record.value.partOfSpeech || "—"}</small>
            <span class="lexeme-meaning">{firstGloss(record.value) || "No gloss yet"}</span>
            <small class="lexeme-status"
              >{[record.value.status, record.value.tags[0]].filter(Boolean).join(" · ") || "—"}</small>
          </button>
        </li>
      {/each}
    </ul>
    {#if page > 0 || hasNextPage}
      <div class="language-actions">
        <button type="button" class="language-button secondary" disabled={page === 0} onclick={previousPage}
          >Previous</button>
        <button type="button" class="language-button secondary" disabled={!hasNextPage} onclick={nextPage}>Next</button>
      </div>
    {/if}
  {/if}
{/if}

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
.language-field-wide {
  grid-column: 1/-1;
}
.language-field {
  display: grid;
  gap: 6px;
  min-width: 0;
  color: var(--ink-soft);
  font-size: 11px;
  letter-spacing: 0.01em;
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
  border-color: #a14f42 !important;
  color: #a14f42 !important;
  background: transparent;
}
.language-group {
  display: grid;
  gap: 10px;
  min-width: 0;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface-muted);
}
.language-group .language-group {
  background: var(--surface);
}
.language-group-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
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
.language-filter-panel {
  margin-top: 10px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-muted);
}
.language-filter-panel summary {
  padding: 10px 12px;
  color: var(--accent-dark);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  list-style-position: inside;
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
  grid-template-columns: repeat(3, minmax(110px, 1fr));
  gap: 10px 12px;
  align-items: end;
}
.language-filters .language-check {
  grid-column: 1/-1;
  padding: 2px 0 0;
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
  background: #eef3ef;
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
  border: 1px solid #ebe7de;
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
  border-color: #e5d8c6;
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
  color: #a14f42;
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
