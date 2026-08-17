<script lang="ts">
import type { EntitySummary, ModuleRecord } from "../../../../module-api/src/index";
import type { LexemeValue } from "../lexeme";
import {
  emptySample,
  emptyToken,
  groupSamples,
  normalizeSample,
  samplePreviewHtml,
  sampleTitle,
  SAMPLE_KINDS,
  tokenizeSample,
  type Sample,
  type SampleKind,
} from "../samples";

let {
  selectedLanguage,
  paneLoading,
  error,
  records,
  samples,
  sampleEditing,
  sampleEditorOpen,
  sampleDraft,
  addSample,
  openSampleEditor,
  closeSampleEditor,
  saveSample,
  deleteSample,
  openLinkedLexeme,
}: {
  selectedLanguage: EntitySummary | null;
  paneLoading: boolean;
  error: string;
  records: ModuleRecord<LexemeValue>[];
  samples: ModuleRecord<Sample>[];
  sampleEditing: ModuleRecord<Sample> | null;
  sampleEditorOpen: boolean;
  sampleDraft: Sample;
  addSample: (kind?: SampleKind) => void;
  openSampleEditor: (record: ModuleRecord<Sample>) => void;
  closeSampleEditor: () => void;
  saveSample: () => Promise<"ok" | "text" | "error" | "none">;
  deleteSample: () => void;
  openLinkedLexeme: (lexemeId: string) => void;
} = $props();

let titleInput: HTMLInputElement | undefined = $state();
let textInput: HTMLTextAreaElement | undefined = $state();
let previewBox: HTMLDivElement | undefined = $state();

const groups = $derived(groupSamples(samples));
const previewHtml = $derived(samplePreviewHtml(normalizeSample(sampleDraft)));

$effect(() => {
  if (sampleEditorOpen && titleInput) titleInput.focus();
});

$effect(() => {
  previewHtml;
  if (!previewHtml || !previewBox) return;
  for (const control of previewBox.querySelectorAll<HTMLButtonElement>(".sample-ref")) {
    control.onclick = () => {
      const lexemeId = control.dataset.lexemeId;
      if (lexemeId) openLinkedLexeme(lexemeId);
    };
  }
});

function tokenize() {
  sampleDraft.tokens = tokenizeSample(sampleDraft.text, sampleDraft.tokens);
}

function addToken() {
  sampleDraft.tokens.push(emptyToken());
}

function removeToken(index: number) {
  sampleDraft.tokens.splice(index, 1);
}

async function handleSubmit(event: SubmitEvent) {
  event.preventDefault();
  const outcome = await saveSample();
  if (outcome === "text") textInput?.focus();
}
</script>

<div class="language-toolbar">
  <div class="language-toolbar-title">
    <p class="language-toolbar-eyebrow">Focused projection</p>
    <h2>Samples</h2>
    <p class="language-toolbar-subtitle">
      {selectedLanguage
        ? `${selectedLanguage.name} · examples, translations, and interlinear notes`
        : "Select a language to collect examples and usage."}
    </p>
  </div>
  <div>
    <button type="button" class="language-button" disabled={!selectedLanguage} onclick={() => addSample()}
      >Add sample</button>
  </div>
</div>

{#if paneLoading}
  <p class="language-empty language-loading" role="status" aria-live="polite">Loading samples…</p>
{:else if sampleEditorOpen}
  <form class="language-editor" onsubmit={handleSubmit}>
    <label class="language-field">
      <span>Title (optional)</span>
      <input name="title" bind:this={titleInput} bind:value={sampleDraft.title} />
    </label>
    <label class="language-field">
      <span>Kind</span>
      <select name="kind" aria-label="Sample kind" bind:value={sampleDraft.kind}>
        {#each SAMPLE_KINDS as item (item.id)}
          <option value={item.id}>{item.label}</option>
        {/each}
      </select>
    </label>
    <label class="language-field">
      <span>Text</span>
      <textarea
        name="text"
        bind:this={textInput}
        rows={sampleDraft.kind === "paragraph" ? 6 : 3}
        bind:value={sampleDraft.text}></textarea>
    </label>
    <label class="language-field">
      <span>Transliteration (optional)</span>
      <textarea name="transliteration" rows={2} bind:value={sampleDraft.transliteration}></textarea>
    </label>
    <label class="language-field">
      <span>Translation (optional)</span>
      <textarea name="translation" rows={2} bind:value={sampleDraft.translation}></textarea>
    </label>
    <label class="language-field">
      <span>Notes (optional)</span>
      <textarea name="notes" rows={2} bind:value={sampleDraft.notes}></textarea>
    </label>
    <section class="language-group">
      <div class="language-group-head">
        <h3>Interlinear tokens</h3>
        <span>
          <button type="button" class="language-button secondary" onclick={tokenize}>Tokenize text</button>
          <button type="button" class="language-button secondary" onclick={addToken}>Add</button>
        </span>
      </div>
      <p class="language-empty" role="status">
        Tokenize splits the sample on whitespace. Matching surface forms keep their glosses, grammar tags, and lexeme
        links.
      </p>
      {#each sampleDraft.tokens as token, index (token.id)}
        <div class="language-inline">
          <div class="language-inline-fields">
            <label class="language-field">
              <span>Form</span>
              <input name={`token-text-${index}`} bind:value={token.text} />
            </label>
            <label class="language-field">
              <span>Gloss</span>
              <input name={`token-gloss-${index}`} bind:value={token.gloss} />
            </label>
            <label class="language-field">
              <span>Grammar</span>
              <input name={`token-grammar-${index}`} bind:value={token.grammar} />
            </label>
            <label class="language-field">
              <span>Lexeme</span>
              <select
                name={`token-lexeme-${index}`}
                aria-label={`Lexeme for token ${index + 1}`}
                bind:value={token.lexemeId}>
                <option value={""}>None</option>
                {#each records as record (record.id)}
                  <option value={record.id}>{record.value.lemma}</option>
                {/each}
              </select>
            </label>
          </div>
          <button type="button" class="language-button secondary language-danger" onclick={() => removeToken(index)}
            >Remove</button>
        </div>
      {/each}
    </section>
    <div class="sample-block">
      <h3>Readable preview</h3>
      <div bind:this={previewBox}>
        {#if previewHtml}
          {@html previewHtml}
        {:else}
          <p class="language-empty" role="status">Add text or tokens to see the rendered sample.</p>
        {/if}
      </div>
    </div>
    {#if error}
      <p class="language-status error" role="alert">{error}</p>
    {/if}
    <div class="language-actions">
      <span>
        {#if sampleEditing}
          <button type="button" class="language-button secondary language-danger" onclick={deleteSample}>Delete</button>
        {/if}
      </span>
      <span>
        <button type="button" class="language-button secondary" onclick={closeSampleEditor}>Cancel</button>
        <button type="submit" class="language-button">Save sample</button>
      </span>
    </div>
  </form>
{:else if error}
  <p class="language-status error" role="alert">{error}</p>
{:else if !selectedLanguage}
  <div class="language-empty-card">
    <p class="language-empty" role="status">Select a language to collect sample sentences and paragraphs.</p>
  </div>
{:else}
  <p class="language-pane-summary">
    {samples.length} sample{samples.length === 1 ? "" : "s"} · grouped by kind for quick browsing.
  </p>
  <div class="language-panes">
    {#each groups as group (group.id)}
      <section class="language-group">
        <div class="language-group-head">
          <h3>{group.label}</h3>
          <button type="button" class="language-button secondary" onclick={() => addSample(group.id)}
            >Add {group.label.toLowerCase()}</button>
        </div>
        {#if group.samples.length === 0}
          <p class="language-empty" role="status">No {group.label.toLowerCase()} yet.</p>
        {:else}
          <ul class="lexeme-list">
            {#each group.samples as record (record.id)}
              <li>
                <button
                  type="button"
                  class="language-item"
                  aria-label={`Edit sample ${sampleTitle(record.value)}`}
                  onclick={() => openSampleEditor(record)}>
                  <strong>{sampleTitle(record.value)}</strong>
                  <span>{record.value.translation || record.value.text.trim().split("\n")[0] || "No text yet"}</span>
                  <small>{record.value.tokens.length} token{record.value.tokens.length === 1 ? "" : "s"}</small>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/each}
  </div>
{/if}

<style>
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
.language-editor {
  display: grid;
  gap: 16px;
  margin-top: 16px;
  min-width: 0;
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
.language-group {
  display: grid;
  gap: 10px;
  min-width: 0;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-muted);
}
.language-group-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  min-width: 0;
}
.language-group-head h3 {
  margin: 0;
  font-family: var(--font-display);
  font-weight: 500;
}
.language-inline {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: end;
  gap: 10px;
}
.language-inline .language-inline-fields {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}
.language-inline .language-inline-fields .language-field {
  flex: 1 1 200px;
}
.language-inline > .language-button {
  margin-bottom: 1px;
  flex: 0 0 auto;
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
.language-item:focus-visible {
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
.language-pane-summary {
  margin: 16px 0 4px;
  color: var(--ink-faint);
  font-size: 11px;
}
.language-panes {
  display: grid;
  gap: 12px;
}
.lexeme-list {
  display: grid;
  gap: 8px;
  padding: 0;
  margin: 0;
  list-style: none;
}
.language-item {
  display: grid;
  grid-template-columns: 1.2fr auto 1.4fr;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface);
  color: var(--ink);
  font: inherit;
  cursor: pointer;
}
.language-item strong {
  font-weight: 600;
  color: var(--accent-dark);
}
.language-item small {
  justify-self: end;
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.language-item span {
  color: var(--ink-soft);
  font-size: 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
  text-align: left;
}
.sample-block {
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface-muted);
  font-size: 13px;
  line-height: 1.55;
}
.sample-block h3 {
  margin: 0 0 8px;
  font-family: var(--font-display);
  font-weight: 500;
}
:global(.sample-interlinear) {
  display: flex;
  flex-wrap: wrap;
  gap: 10px 18px;
  margin: 10px 0;
}
:global(.sample-token) {
  display: grid;
  gap: 2px;
  justify-items: center;
  text-align: center;
  padding: 6px 8px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
}
:global(.sample-token .surface),
:global(.sample-ref) {
  font-weight: 600;
}
:global(.sample-token .gloss),
:global(.sample-token .grammar),
:global(.sample-transliteration) {
  color: var(--ink-soft);
  font-size: 11px;
}
:global(.sample-translation) {
  margin: 8px 0 0;
  font-style: italic;
}
:global(.sample-source) {
  margin: 4px 0 0;
  color: var(--ink-soft);
  font-size: 11px;
}
:global(.sample-ref) {
  padding: 0;
  border: 0;
  border-bottom: 1px dotted var(--accent-dark);
  background: transparent;
  color: var(--accent-dark);
  font: inherit;
  cursor: pointer;
}
:global(.sample-ref:focus-visible) {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
</style>
