<script lang="ts">
import type { EntitySummary, ModuleRecord } from "../../../../module-api/src/index";
import { STATUS_SUGGESTIONS } from "../lexeme";
import type { OrthographyValue } from "../orthography";
import type { PhonemeValue } from "../phonology";

let {
  selectedLanguage,
  paneLoading,
  error,
  phonemes,
  orthographies,
  orthographyEditing,
  orthographyEditorOpen,
  orthographyDraft,
  addOrthography,
  openOrthographyEditor,
  closeOrthographyEditor,
  saveOrthography,
  deleteOrthography,
}: {
  selectedLanguage: EntitySummary | null;
  paneLoading: boolean;
  error: string;
  phonemes: ModuleRecord<PhonemeValue>[];
  orthographies: ModuleRecord<OrthographyValue>[];
  orthographyEditing: ModuleRecord<OrthographyValue> | null;
  orthographyEditorOpen: boolean;
  orthographyDraft: OrthographyValue;
  addOrthography: () => void;
  openOrthographyEditor: (record: ModuleRecord<OrthographyValue>) => void;
  closeOrthographyEditor: () => void;
  saveOrthography: () => Promise<"ok" | "name" | "error" | "none">;
  deleteOrthography: () => void;
} = $props();

let nameInput: HTMLInputElement | undefined = $state();
let soundsText = $state<string[]>([]);

$effect(() => {
  if (orthographyEditorOpen) soundsText = orthographyDraft.mappings.map((mapping) => mapping.sounds.join(" "));
});

function addMapping() {
  orthographyDraft.mappings.push({ id: crypto.randomUUID(), grapheme: "", sounds: [] });
}

function removeMapping(index: number) {
  orthographyDraft.mappings.splice(index, 1);
}

function handleSubmit(event: SubmitEvent) {
  event.preventDefault();
  orthographyDraft.mappings.forEach((mapping, index) => {
    mapping.sounds = (soundsText[index] ?? "").split(/[\s,]+/);
  });
  void saveOrthography().then((outcome) => {
    if (outcome === "name") nameInput?.focus();
  });
}
</script>

<div class="language-toolbar">
  <div class="language-toolbar-title">
    <p class="language-toolbar-eyebrow">Focused projection</p>
    <h2>Writing</h2>
    <p class="language-toolbar-subtitle">
      {selectedLanguage
        ? `${selectedLanguage.name} · scripts, graphemes, and sound mappings`
        : "Select a language to document its writing systems."}
    </p>
  </div>
  <div class="language-toolbar-actions">
    <button type="button" class="language-button" disabled={!selectedLanguage} onclick={addOrthography}
      >Add writing system</button>
  </div>
</div>
{#if paneLoading}
  <p class="language-empty language-loading" role="status">Loading writing systems…</p>
{:else if orthographyEditorOpen}
  <form class="language-editor" onsubmit={handleSubmit}>
    <datalist id="language-status">
      {#each STATUS_SUGGESTIONS as suggestion}
        <option value={suggestion}>{suggestion}</option>
      {/each}
    </datalist>
    <datalist id="language-sounds">
      {#each phonemes as record (record.id)}
        <option value={record.value.symbol}>{record.value.symbol}</option>
      {/each}
    </datalist>
    <label class="language-field">
      <span>Name</span>
      <input name="name" bind:this={nameInput} bind:value={orthographyDraft.name} />
    </label>
    <label class="language-field">
      <span>Status (optional)</span>
      <input
        name="status"
        list="language-status"
        value={orthographyDraft.status ?? ""}
        oninput={(event) => (orthographyDraft.status = event.currentTarget.value)} />
    </label>
    <label class="language-field">
      <span>Notes (optional)</span>
      <textarea
        name="notes"
        value={orthographyDraft.notes ?? ""}
        oninput={(event) => (orthographyDraft.notes = event.currentTarget.value)}></textarea>
    </label>
    <section class="language-group">
      <div class="language-group-head">
        <h3>Grapheme to sound</h3>
        <button type="button" class="language-button secondary" onclick={addMapping}>Add</button>
      </div>
      {#each orthographyDraft.mappings as mapping, index (mapping.id)}
        <div class="language-inline">
          <div class="language-inline-fields">
            <label class="language-field">
              <span>Grapheme</span>
              <input name={`grapheme-${index}`} bind:value={mapping.grapheme} />
            </label>
            <label class="language-field">
              <span>Sounds</span>
              <input name={`sounds-${index}`} list="language-sounds" bind:value={soundsText[index]} />
            </label>
            <label class="language-field">
              <span>Environment (optional)</span>
              <input
                name={`environment-${index}`}
                value={mapping.environment ?? ""}
                oninput={(event) => (mapping.environment = event.currentTarget.value)} />
            </label>
            <label class="language-field">
              <span>Notes (optional)</span>
              <input
                name={`mapping-notes-${index}`}
                value={mapping.notes ?? ""}
                oninput={(event) => (mapping.notes = event.currentTarget.value)} />
            </label>
          </div>
          <button type="button" class="language-button secondary language-danger" onclick={() => removeMapping(index)}
            >Remove</button>
        </div>
      {/each}
    </section>
    {#if error}
      <p class="language-status error" role="alert">{error}</p>
    {/if}
    <div class="language-actions">
      <span>
        {#if orthographyEditing}
          <button type="button" class="language-button secondary language-danger" onclick={deleteOrthography}
            >Delete</button>
        {/if}
      </span>
      <span>
        <button type="button" class="language-button secondary" onclick={closeOrthographyEditor}>Cancel</button>
        <button type="submit" class="language-button">Save writing system</button>
      </span>
    </div>
  </form>
{:else if error}
  <p class="language-status error" role="alert">{error}</p>
{:else if !selectedLanguage}
  <div class="language-empty-card">
    <p class="language-empty" role="status">Select a language to document its writing systems.</p>
  </div>
{:else if orthographies.length === 0}
  <div class="language-empty-card">
    <p class="language-empty" role="status">No writing systems yet. Add one and map graphemes to sounds.</p>
    <div class="language-inline">
      <button type="button" class="language-button secondary" onclick={addOrthography}>Add first writing system</button>
    </div>
  </div>
{:else}
  <section class="language-pane-section">
    <h3>Writing systems</h3>
    <p>{orthographies.length} system{orthographies.length === 1 ? "" : "s"} · select one to edit its mappings.</p>
    <ul class="lexeme-list">
      {#each orthographies as record (record.id)}
        <li>
          <button
            type="button"
            class="language-item"
            aria-label={`Edit writing system ${record.value.name}`}
            onclick={() => openOrthographyEditor(record)}>
            <strong>{record.value.name}</strong>
            <small>{record.value.status || "—"}</small>
            <span>{record.value.mappings.length} mapping{record.value.mappings.length === 1 ? "" : "s"}</span>
          </button>
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
.language-sidebar-kicker,
.language-toolbar-eyebrow {
  margin: 0 0 5px;
  color: var(--accent);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}
.language-sidebar-intro,
.language-toolbar-subtitle {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.55;
}
.language-panel h2,
.language-panel h3 {
  margin: 0;
  font-family: var(--font-display);
  font-weight: 500;
}
.language-panel h2 {
  font-size: 24px;
  line-height: 1.15;
}
.language-panel h3 {
  font-size: 16px;
  line-height: 1.3;
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
.language-pane-section {
  display: grid;
  gap: 10px;
  margin-top: 16px;
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface-muted);
}
.language-pane-section > p {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.55;
}
.language-pane-section .lexeme-list {
  margin-top: 2px;
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
.language-field {
  display: grid;
  gap: 6px;
  min-width: 0;
  color: var(--ink-soft);
  font-size: 11px;
  letter-spacing: 0.01em;
}
.language-list,
.lexeme-list {
  display: grid;
  gap: 8px;
  margin: 4px 0 0;
  padding: 0;
  list-style: none;
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
.language-list button:focus-visible,
.language-item:focus-visible,
.lexeme-row:focus-visible,
.grammar-card:focus-visible,
.grammar-system:focus-visible,
.sample-ref:focus-visible,
.grammar-choice:focus-within,
.grammar-status input:focus-visible,
.grammar-checks input:focus-visible,
.grammar-learn summary:focus-visible {
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
@media (max-width: 760px) {
  .language-item span,
  .lexeme-row span,
  .lexeme-row small {
    white-space: normal;
  }
  .language-inline {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
