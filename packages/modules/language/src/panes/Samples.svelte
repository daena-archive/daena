<script lang="ts">
import { untrack } from "svelte";
import type { EntitySummary, ModuleContext, ModuleRecord } from "../../../../module-api/src/index";
import type { LexemeValue } from "../lexeme";
import { normalizeLexeme } from "../lexeme";
import { confirm } from "../confirm.svelte";
import RichTextEditor from "../../../../../src/lib/editor/RichTextEditor.svelte";
import {
  emptySample,
  emptyToken,
  groupSamples,
  normalizeSample,
  samplePreviewHtml,
  sampleTitle,
  SAMPLE_KINDS,
  serializeSample,
  tokenizeSample,
  type Sample,
  type SampleKind,
} from "../samples";

let {
  context,
  selectedLanguage,
  active,
  openLexeme,
  registerLeaveGuard,
  setMutationActive,
}: {
  context: ModuleContext;
  selectedLanguage: EntitySummary | null;
  active: boolean;
  openLexeme: (lexemeId: string) => void;
  registerLeaveGuard: (guard: (() => Promise<boolean> | boolean) | null) => void;
  setMutationActive: (active: boolean) => void;
} = $props();

let cancelled = $state(false);
let records: ModuleRecord<LexemeValue>[] = $state([]);
let samples: ModuleRecord<Sample>[] = $state([]);
let sampleEditing = $state<ModuleRecord<Sample> | null>(null);
let sampleEditorOpen = $state(false);
let sampleDraft: Sample = $state(emptySample());
let sampleSaving = $state(false);
let paneLoading = $state(false);
let error = $state("");
let request = $state(0);

let titleInput: HTMLInputElement | undefined = $state();
let previewBox: HTMLDivElement | undefined = $state();

let lastLoadedLanguage: string | null = null;

$effect(() => {
  const languageId = selectedLanguage?.id ?? null;
  void languageId;
  if (!active) return;
  if (languageId === lastLoadedLanguage) {
    untrack(() => void loadSamples());
    return;
  }
  lastLoadedLanguage = languageId;
  sampleEditing = null;
  sampleEditorOpen = false;
  sampleDraft = emptySample();
  untrack(() => void loadSamples());
});

$effect(() => {
  return () => {
    cancelled = true;
  };
});

function samplesHasDraft() {
  if (!sampleEditorOpen) return false;
  const baseline = sampleEditing ? normalizeSample(sampleEditing.value) : emptySample();
  return JSON.stringify(serializeSample(normalizeSample(sampleDraft))) !== JSON.stringify(serializeSample(baseline));
}

async function tryLeaveSamples(confirmLeave: (message: string) => Promise<boolean> | boolean) {
  if (!samplesHasDraft()) return true;
  if (sampleSaving) return false;
  const allowed = await confirmLeave("You have unsaved changes to a sample. Discard them?");
  if (allowed) closeSampleEditor();
  return allowed;
}

$effect(() => {
  registerLeaveGuard(() => tryLeaveSamples((message) => confirm("Unsaved changes", message)));
});

const groups = $derived(groupSamples(samples));
const previewHtml = $derived(samplePreviewHtml(normalizeSample(sampleDraft)));

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
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }
}

$effect(() => {
  if (sampleEditorOpen && titleInput) titleInput.focus();
});

$effect(() => {
  previewHtml;
  if (!previewHtml || !previewBox) return;
  for (const control of previewBox.querySelectorAll<HTMLButtonElement>(".sample-ref")) {
    control.onclick = () => {
      const lexemeId = control.dataset.lexemeId;
      if (lexemeId) openLexeme(lexemeId);
    };
  }
});

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
  const ownerLanguageId = selectedLanguage.id;
  const value = normalizeSample(sampleDraft);
  if (!value.text.trim()) {
    error = "Text is required.";
    return "text";
  }
  error = "";
  sampleDraft = value;
  sampleSaving = true;
  setMutationActive(true);
  try {
    const payload = serializeSample(value);
    if (sampleEditing) {
      const updated = await context.records.update("samples", sampleEditing.id, ownerLanguageId, payload, {
        expectedRevision: sampleEditing.revision,
        requestId: crypto.randomUUID(),
      });
      sampleEditing = { ...updated, value: normalizeSample(updated.value) };
    } else {
      const created = await context.records.create("samples", ownerLanguageId, payload, {
        requestId: crypto.randomUUID(),
      });
      sampleEditing = { ...created, value: normalizeSample(created.value) };
    }
    sampleEditorOpen = true;
    sampleDraft = sampleEditing.value;
    sampleSaving = false;
    setMutationActive(false);
    if (ownerLanguageId === selectedLanguage?.id) await loadSamples();
    return "ok";
  } catch (cause) {
    sampleSaving = false;
    setMutationActive(false);
    error = cause instanceof Error ? cause.message : String(cause);
    return "error";
  }
}

async function deleteSample() {
  if (!selectedLanguage || !sampleEditing) return;
  if (!(await confirm("Delete", `Delete “${sampleTitle(sampleEditing.value)}”?`))) return;
  const ownerLanguageId = selectedLanguage.id;
  error = "";
  try {
    setMutationActive(true);
    await context.records.delete("samples", sampleEditing.id, ownerLanguageId, {
      expectedRevision: sampleEditing.revision,
      requestId: crypto.randomUUID(),
    });
    sampleEditing = null;
    sampleEditorOpen = false;
    sampleDraft = emptySample();
    setMutationActive(false);
    if (ownerLanguageId === selectedLanguage?.id) await loadSamples();
  } catch (cause) {
    setMutationActive(false);
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

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
  await saveSample();
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
      <RichTextEditor value={sampleDraft.text} onChange={(v) => (sampleDraft.text = v)} />
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
    <section class="language-form-section">
      <div class="language-form-section-header">
        <div>
          <h3>Interlinear tokens</h3>
          <p>
            Tokenize splits the sample on whitespace. Matching surface forms keep their glosses, grammar tags, and
            lexeme links.
          </p>
        </div>
        <div class="samples-token-actions">
          <button type="button" class="language-button secondary" onclick={tokenize}>Tokenize text</button>
          <button type="button" class="language-button secondary" onclick={addToken}>Add token</button>
        </div>
      </div>
      {#if sampleDraft.tokens.length === 0}
        <div class="language-empty-card">
          <p class="language-empty" role="status">
            No tokens defined. Use "Tokenize text" to split on whitespace, or add tokens manually.
          </p>
        </div>
      {:else}
        <div class="samples-tokens-table">
          <div class="samples-tokens-header">
            <span class="samples-tokens-col">#</span>
            <span class="samples-tokens-col">Form</span>
            <span class="samples-tokens-col">Gloss</span>
            <span class="samples-tokens-col">Grammar</span>
            <span class="samples-tokens-col">Lexeme</span>
            <span class="samples-tokens-col samples-tokens-actions"></span>
          </div>
          {#each sampleDraft.tokens as token, index (token.id)}
            <div class="samples-token-row">
              <span class="samples-tokens-col samples-token-number">{index + 1}</span>
              <label class="samples-tokens-col">
                <span class="visually-hidden">Form</span>
                <input name={`token-text-${index}`} bind:value={token.text} placeholder="Word" />
              </label>
              <label class="samples-tokens-col">
                <span class="visually-hidden">Gloss</span>
                <input name={`token-gloss-${index}`} bind:value={token.gloss} placeholder="Translation" />
              </label>
              <label class="samples-tokens-col">
                <span class="visually-hidden">Grammar</span>
                <input name={`token-grammar-${index}`} bind:value={token.grammar} placeholder="e.g. N, V" />
              </label>
              <label class="samples-tokens-col">
                <span class="visually-hidden">Lexeme</span>
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
              <span class="samples-tokens-col samples-tokens-actions">
                <button
                  type="button"
                  class="samples-token-remove"
                  onclick={() => removeToken(index)}
                  aria-label="Remove token">&times;</button>
              </span>
            </div>
          {/each}
        </div>
      {/if}
    </section>
    <section class="language-form-section">
      <div class="language-form-section-header">
        <div>
          <h3>Readable preview</h3>
          <p>See how the sample will look when rendered with interlinear glosses.</p>
        </div>
      </div>
      <div class="samples-preview" bind:this={previewBox}>
        {#if previewHtml}
          {@html previewHtml}
        {:else}
          <div class="language-empty-card">
            <p class="language-empty" role="status">Add text or tokens to see the rendered sample.</p>
          </div>
        {/if}
      </div>
    </section>
    {#if error}
      <p class="language-status error" role="alert">{error}</p>
    {/if}
    <div class="language-actions">
      <span>
        {#if sampleEditing}
          <button
            type="button"
            class="language-button secondary language-danger"
            onclick={deleteSample}
            disabled={sampleSaving}>Delete</button>
        {/if}
      </span>
      <span>
        <button type="button" class="language-button secondary" onclick={closeSampleEditor} disabled={sampleSaving}
          >Cancel</button>
        <button type="submit" class="language-button" disabled={sampleSaving}
          >{sampleSaving ? "Saving…" : "Save sample"}</button>
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
.language-form-section-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
.language-form-section-header h3 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}
.language-form-section-header p {
  margin: 4px 0 0;
  color: var(--ink-soft);
  font-size: 12px;
}
.samples-token-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}
.samples-tokens-table {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--line);
  border-radius: 10px;
  overflow: hidden;
}
.samples-tokens-header {
  display: grid;
  grid-template-columns: 32px 1fr 1fr 100px 120px 40px;
  gap: 8px;
  padding: 8px 12px;
  background: var(--surface-muted);
  border-bottom: 1px solid var(--line);
  font-size: 11px;
  font-weight: 600;
  color: var(--ink-soft);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.samples-token-row {
  display: grid;
  grid-template-columns: 32px 1fr 1fr 100px 120px 40px;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--line);
  align-items: center;
}
.samples-token-row:last-child {
  border-bottom: none;
}
.samples-token-row:hover {
  background: var(--surface-muted);
}
.samples-token-number {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: var(--surface-muted);
  color: var(--ink-soft);
  font-size: 11px;
  font-weight: 600;
}
.samples-tokens-col {
  display: flex;
  align-items: center;
  min-width: 0;
}
.samples-tokens-col input,
.samples-tokens-col select {
  box-sizing: border-box;
  width: 100%;
  min-width: 0;
  padding: 6px 8px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--surface);
  color: var(--ink);
  font: inherit;
  font-size: 12px;
}
.samples-tokens-col input:focus,
.samples-tokens-col select:focus {
  outline: 2px solid var(--accent);
  outline-offset: -1px;
}
.samples-tokens-actions {
  justify-content: center;
}
.samples-token-remove {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-faint);
  cursor: pointer;
  font-size: 14px;
}
.samples-token-remove:hover {
  background: var(--surface-muted);
  color: #a14f42;
}
.samples-preview {
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface);
  min-height: 100px;
}
.samples-preview :global(.sample-source) {
  font-size: 16px;
  line-height: 1.6;
  margin-bottom: 12px;
}
.samples-preview :global(.sample-interlinear) {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(60px, 1fr));
  gap: 8px;
  margin-bottom: 12px;
}
.samples-preview :global(.sample-token) {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 4px;
  text-align: center;
}
.samples-preview :global(.sample-token .surface) {
  font-weight: 600;
  font-size: 14px;
}
.samples-preview :global(.sample-token .gloss),
.samples-preview :global(.sample-token .grammar) {
  font-size: 11px;
  color: var(--ink-soft);
}
.samples-preview :global(.sample-transliteration) {
  font-style: italic;
  color: var(--ink-soft);
  margin-bottom: 8px;
}
.samples-preview :global(.sample-translation) {
  font-size: 14px;
  margin-top: 8px;
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
@media (max-width: 760px) {
  .samples-tokens-header {
    display: none;
  }
  .samples-token-row {
    grid-template-columns: 1fr;
    gap: 6px;
    padding: 12px;
  }
  .samples-token-number {
    display: none;
  }
  .samples-tokens-actions {
    justify-content: flex-start;
  }
}
</style>
