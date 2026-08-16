<script lang="ts">
import type { EntitySummary, ModuleRecord } from "../../../../module-api/src/index";
import {
  BACKNESS_SUGGESTIONS,
  consonantChart,
  HEIGHT_SUGGESTIONS,
  MANNER_SUGGESTIONS,
  PHONEME_KINDS,
  PLACE_SUGGESTIONS,
  ROUNDING_SUGGESTIONS,
  VOICING_SUGGESTIONS,
  vowelChart,
  type PhonemeValue,
  type PhonologyNotes,
} from "../phonology";

let {
  selectedLanguage,
  paneLoading,
  error,
  phonemes,
  phonemeEditing,
  phonemeEditorOpen,
  phonemeDraft,
  phonologyRecord,
  phonologyDraft,
  phonologyNotesOpen,
  addPhoneme,
  openPhonemeEditor,
  closePhonemeEditor,
  savePhoneme,
  deletePhoneme,
  savePhonology,
}: {
  selectedLanguage: EntitySummary | null;
  paneLoading: boolean;
  error: string;
  phonemes: ModuleRecord<PhonemeValue>[];
  phonemeEditing: ModuleRecord<PhonemeValue> | null;
  phonemeEditorOpen: boolean;
  phonemeDraft: PhonemeValue;
  phonologyRecord: ModuleRecord<PhonologyNotes> | null;
  phonologyDraft: PhonologyNotes;
  phonologyNotesOpen: boolean;
  addPhoneme: () => void;
  openPhonemeEditor: (record: ModuleRecord<PhonemeValue>) => void;
  closePhonemeEditor: () => void;
  savePhoneme: () => Promise<"ok" | "symbol" | "error" | "none">;
  deletePhoneme: () => void;
  savePhonology: () => Promise<void>;
} = $props();

let symbolInput: HTMLInputElement | undefined = $state();

const phonemeValues = $derived(phonemes.map((record) => record.value));
const consonants = $derived(consonantChart(phonemeValues));
const vowels = $derived(vowelChart(phonemeValues));
const otherSounds = $derived(
  phonemes.filter((record) => record.value.kind === "tone" || record.value.kind === "other"),
);

function chartItems(chart: ReturnType<typeof consonantChart>, row: string, column: string) {
  return chart.cells.find((entry) => entry.row === row && entry.column === column)?.items ?? [];
}

function openFromChart(item: PhonemeValue) {
  const record = phonemes.find((entry) => entry.value.symbol === item.symbol && entry.value.kind === item.kind);
  if (record) openPhonemeEditor(record);
}

function handleSubmit(event: SubmitEvent) {
  event.preventDefault();
  void savePhoneme().then((outcome) => {
    if (outcome === "symbol") symbolInput?.focus();
  });
}

function handleNotesSubmit(event: SubmitEvent) {
  event.preventDefault();
  void savePhonology();
}
</script>

<div class="language-toolbar">
  <div class="language-toolbar-title">
    <p class="language-toolbar-eyebrow">Focused projection</p>
    <h2>Sounds</h2>
    <p class="language-toolbar-subtitle">
      {selectedLanguage
        ? `${selectedLanguage.name} · phoneme inventory and phonology notes`
        : "Select a language to document its sound system."}
    </p>
  </div>
  <div class="language-toolbar-actions">
    <button type="button" class="language-button" disabled={!selectedLanguage} onclick={addPhoneme}>Add sound</button>
  </div>
</div>
{#if !selectedLanguage}
  <div class="language-empty-card">
    <p class="language-empty" role="status">Select a language to document its sounds.</p>
  </div>
{:else if paneLoading}
  <p class="language-empty language-loading" role="status">Loading sound inventory…</p>
{:else if phonemeEditorOpen}
  <form class="language-editor" onsubmit={handleSubmit}>
    <datalist id="language-place">
      {#each PLACE_SUGGESTIONS as suggestion}
        <option value={suggestion}>{suggestion}</option>
      {/each}
    </datalist>
    <datalist id="language-manner">
      {#each MANNER_SUGGESTIONS as suggestion}
        <option value={suggestion}>{suggestion}</option>
      {/each}
    </datalist>
    <datalist id="language-voice">
      {#each VOICING_SUGGESTIONS as suggestion}
        <option value={suggestion}>{suggestion}</option>
      {/each}
    </datalist>
    <datalist id="language-height">
      {#each HEIGHT_SUGGESTIONS as suggestion}
        <option value={suggestion}>{suggestion}</option>
      {/each}
    </datalist>
    <datalist id="language-backness">
      {#each BACKNESS_SUGGESTIONS as suggestion}
        <option value={suggestion}>{suggestion}</option>
      {/each}
    </datalist>
    <datalist id="language-rounding">
      {#each ROUNDING_SUGGESTIONS as suggestion}
        <option value={suggestion}>{suggestion}</option>
      {/each}
    </datalist>
    <label class="language-field">
      <span>Symbol</span>
      <input name="symbol" bind:this={symbolInput} bind:value={phonemeDraft.symbol} />
    </label>
    <label class="language-field">
      <span>IPA (optional)</span>
      <input name="ipa" bind:value={phonemeDraft.ipa} />
    </label>
    <label class="language-field">
      <span>Kind</span>
      <select name="kind" aria-label="Sound kind" bind:value={phonemeDraft.kind}>
        {#each PHONEME_KINDS as kind (kind)}
          <option value={kind}>{kind}</option>
        {/each}
      </select>
    </label>
    <label class="language-field">
      <span>Place (optional)</span>
      <input name="place" list="language-place" bind:value={phonemeDraft.place} />
    </label>
    <label class="language-field">
      <span>Manner (optional)</span>
      <input name="manner" list="language-manner" bind:value={phonemeDraft.manner} />
    </label>
    <label class="language-field">
      <span>Voicing (optional)</span>
      <input name="voicing" list="language-voice" bind:value={phonemeDraft.voicing} />
    </label>
    <label class="language-field">
      <span>Height (optional)</span>
      <input name="height" list="language-height" bind:value={phonemeDraft.height} />
    </label>
    <label class="language-field">
      <span>Backness (optional)</span>
      <input name="backness" list="language-backness" bind:value={phonemeDraft.backness} />
    </label>
    <label class="language-field">
      <span>Rounding (optional)</span>
      <input name="rounding" list="language-rounding" bind:value={phonemeDraft.rounding} />
    </label>
    <label class="language-field">
      <span>Example (optional)</span>
      <input name="example" bind:value={phonemeDraft.example} />
    </label>
    <label class="language-field">
      <span>Notes (optional)</span>
      <textarea name="notes" bind:value={phonemeDraft.notes}></textarea>
    </label>
    {#if error}
      <p class="language-status error" role="alert">{error}</p>
    {/if}
    <div class="language-actions">
      <span>
        {#if phonemeEditing}
          <button type="button" class="language-button secondary language-danger" onclick={deletePhoneme}
            >Delete</button>
        {/if}
      </span>
      <span>
        <button type="button" class="language-button secondary" onclick={closePhonemeEditor}>Cancel</button>
        <button type="submit" class="language-button">Save sound</button>
      </span>
    </div>
  </form>
{:else}
  <details
    class="language-sounds-notes"
    open={phonologyNotesOpen}
    ontoggle={(event) => (phonologyNotesOpen = event.currentTarget.open)}>
    <summary>
      <span class="language-sounds-notes-title">
        <strong>Phonology notes</strong>
        <span>Optional sound-pattern notes</span>
      </span>
      <span class="language-sounds-notes-meta">{phonologyRecord ? "Saved" : "Optional"}</span>
    </summary>
    <div class="language-sounds-notes-body">
      <p>Capture the sound patterns that sit behind the inventory and charts.</p>
      <form class="language-editor language-pane-form language-sounds-notes-content" onsubmit={handleNotesSubmit}>
        <label class="language-field">
          <span>Syllable structure (optional)</span>
          <textarea name="syllableStructure" rows={2} bind:value={phonologyDraft.syllableStructure}></textarea>
        </label>
        <label class="language-field">
          <span>Stress (optional)</span>
          <textarea name="stress" rows={2} bind:value={phonologyDraft.stress}></textarea>
        </label>
        <label class="language-field">
          <span>Tone (optional)</span>
          <textarea name="tone" rows={2} bind:value={phonologyDraft.tone}></textarea>
        </label>
        <label class="language-field">
          <span>Phonotactics (optional)</span>
          <textarea name="phonotactics" rows={2} bind:value={phonologyDraft.phonotactics}></textarea>
        </label>
        <label class="language-field">
          <span>Notes (optional)</span>
          <textarea name="notes" rows={2} bind:value={phonologyDraft.notes}></textarea>
        </label>
        <button type="submit" class="language-button">Save sound notes</button>
      </form>
    </div>
  </details>
  <section class="language-group language-sounds-chart">
    <div class="language-sounds-chart-heading">
      <h3>Consonants</h3>
    </div>
    {#if consonants.columns.length === 0}
      <p class="language-empty" role="status">Add place and manner to position consonants here.</p>
    {:else}
      <div class="language-chart-wrap">
        <table class="language-chart">
          <thead>
            <tr>
              <th></th>
              {#each consonants.columns as column (column)}
                <th scope="col">{column}</th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each consonants.rows as rowLabel (rowLabel)}
              <tr>
                <th scope="row">{rowLabel}</th>
                {#each consonants.columns as column (column)}
                  {@const items = chartItems(consonants, rowLabel, column)}
                  <td class:is-empty={items.length === 0}>
                    {#if items.length === 0}
                      ·
                    {:else}
                      {#each items as item}
                        <button
                          type="button"
                          class="language-button secondary"
                          title={item.ipa ? `${item.symbol} (${item.ipa})` : item.symbol}
                          onclick={() => openFromChart(item)}>{item.symbol}</button>
                      {/each}
                    {/if}
                  </td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
    {#if consonants.unplaced.length}
      <p class="language-empty">Unplaced: {consonants.unplaced.map((item) => item.symbol).join(", ")}</p>
    {/if}
  </section>
  <section class="language-group language-sounds-chart">
    <div class="language-sounds-chart-heading">
      <h3>Vowels</h3>
    </div>
    {#if vowels.columns.length === 0}
      <p class="language-empty" role="status">Add height and backness to position vowels here.</p>
    {:else}
      <div class="language-chart-wrap">
        <table class="language-chart">
          <thead>
            <tr>
              <th></th>
              {#each vowels.columns as column (column)}
                <th scope="col">{column}</th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each vowels.rows as rowLabel (rowLabel)}
              <tr>
                <th scope="row">{rowLabel}</th>
                {#each vowels.columns as column (column)}
                  {@const items = chartItems(vowels, rowLabel, column)}
                  <td class:is-empty={items.length === 0}>
                    {#if items.length === 0}
                      ·
                    {:else}
                      {#each items as item}
                        <button
                          type="button"
                          class="language-button secondary"
                          title={item.ipa ? `${item.symbol} (${item.ipa})` : item.symbol}
                          onclick={() => openFromChart(item)}>{item.symbol}</button>
                      {/each}
                    {/if}
                  </td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
    {#if vowels.unplaced.length}
      <p class="language-empty">Unplaced: {vowels.unplaced.map((item) => item.symbol).join(", ")}</p>
    {/if}
  </section>
  {#if otherSounds.length > 0}
    <p class="language-empty" role="status">
      Other sounds: {otherSounds.map((record) => record.value.symbol).join(", ")}
    </p>
  {/if}
  {#if error}
    <p class="language-status error" role="alert">{error}</p>
  {:else if phonemes.length === 0}
    <div class="language-empty-card">
      <p class="language-empty" role="status">
        No sounds yet. Add consonants and vowels; charts stay empty until place, manner, height, or backness is filled
        in.
      </p>
      <div class="language-inline">
        <button type="button" class="language-button secondary" onclick={addPhoneme}>Add first sound</button>
      </div>
    </div>
  {:else}
    <section class="language-pane-section">
      <h3>Sound inventory</h3>
      <p>{phonemes.length} sound{phonemes.length === 1 ? "" : "s"} · select one to edit its features.</p>
      <ul class="lexeme-list">
        {#each phonemes as record (record.id)}
          <li>
            <button
              type="button"
              class="language-item"
              aria-label={`Edit sound ${record.value.symbol}`}
              onclick={() => openPhonemeEditor(record)}>
              <strong>{record.value.symbol}</strong>
              <small>{record.value.kind}</small>
              <span
                >{record.value.ipa ||
                  [record.value.place, record.value.manner, record.value.height, record.value.backness]
                    .filter(Boolean)
                    .join(" · ") ||
                  "No features yet"}</span>
            </button>
          </li>
        {/each}
      </ul>
    </section>
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
.language-sounds-notes {
  display: block;
  margin-top: 8px;
  padding: 0;
  border: 0;
  border-top: 1px solid var(--line);
  border-radius: 0;
  background: transparent;
  overflow: visible;
}
.language-sounds-notes summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 11px 0;
  color: var(--ink);
  cursor: pointer;
  list-style: none;
}
.language-sounds-notes summary::-webkit-details-marker {
  display: none;
}
.language-sounds-notes summary::after {
  content: "⌄";
  color: var(--ink-faint);
  font-size: 15px;
  line-height: 1;
  transition: transform 0.16s ease;
}
.language-sounds-notes[open] summary {
  border-bottom: 1px solid var(--line);
}
.language-sounds-notes[open] summary::after {
  transform: rotate(180deg);
}
.language-sounds-notes-title {
  display: flex;
  align-items: baseline;
  gap: 9px;
  min-width: 0;
}
.language-sounds-notes-title strong {
  font-family: var(--font-display);
  font-size: 15px;
  font-weight: 500;
}
.language-sounds-notes-title span,
.language-sounds-notes-meta {
  color: var(--ink-faint);
  font-size: 11px;
}
.language-sounds-notes-meta {
  white-space: nowrap;
}
.language-sounds-notes-body {
  display: grid;
  gap: 10px;
  padding: 12px 0 14px;
}
.language-sounds-notes-body > p {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.55;
}
.language-sounds-notes-body .language-sounds-notes-content {
  display: grid;
  gap: 10px;
  margin-top: 0;
  padding: 0;
}
.language-group.language-sounds-chart {
  margin-top: 4px;
  padding: 14px 0 0;
  border: 0;
  border-top: 1px solid var(--line);
  border-radius: 0;
  background: transparent;
}
.language-sounds-chart-heading {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}
.language-sounds-chart-heading h3 {
  font-family: var(--font-sans);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--ink-soft);
}
.language-sounds-chart .language-empty {
  margin: 0;
  color: var(--ink-faint);
  font-size: 12px;
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
.language-field {
  display: grid;
  gap: 6px;
  min-width: 0;
  color: var(--ink-soft);
  font-size: 11px;
  letter-spacing: 0.01em;
}
.language-chart-wrap {
  overflow-x: auto;
  margin: 8px 0 4px;
}
.language-chart {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.language-chart th,
.language-chart td {
  border: 1px solid var(--line);
  padding: 8px;
  text-align: center;
  min-width: 52px;
}
.language-chart th {
  background: var(--surface-muted);
  font-weight: 600;
  color: var(--ink-soft);
}
.language-chart button {
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  cursor: pointer;
}
.language-chart .is-empty {
  color: var(--ink-faint);
}
.lexeme-list {
  display: grid;
  gap: 8px;
  margin: 4px 0 0;
  padding: 0;
  list-style: none;
}
.language-item {
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
.language-item:hover {
  border-color: #e5d8c6;
  background: var(--surface-muted);
}
.language-item strong {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.language-item small {
  color: var(--ink-faint);
}
.language-item span {
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
.language-inline {
  display: flex;
  align-items: end;
  gap: 8px;
  min-width: 0;
}
.language-inline > .language-button {
  flex: 0 0 auto;
}
@media (max-width: 760px) {
  .language-item span {
    white-space: normal;
  }
  .language-inline {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
