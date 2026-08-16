<script lang="ts">
import type { EntitySummary, ModuleRecord } from "../../../../module-api/src/index";
import type { LexemeValue } from "../lexeme";
import { PART_OF_SPEECH_SUGGESTIONS } from "../lexeme";
import {
  emptyOperation,
  emptyRule,
  emptySlot,
  normalizeParadigm,
  OPERATION_KINDS,
  PARADIGM_KINDS,
  previewParadigm,
  type MorphOperationKind,
  type Paradigm,
  type ParadigmKind,
  type ParadigmSlot,
} from "../morphology";

let {
  selectedLanguage,
  paneLoading,
  error,
  records,
  paradigms,
  paradigmEditing,
  paradigmEditorOpen,
  paradigmDraft,
  previewStem,
  previewLexemeId,
  addParadigm,
  openParadigmEditor,
  closeParadigmEditor,
  saveParadigm,
  deleteParadigm,
  pinPreviewOverride,
  clearPreviewOverride,
}: {
  selectedLanguage: EntitySummary | null;
  paneLoading: boolean;
  error: string;
  records: ModuleRecord<LexemeValue>[];
  paradigms: ModuleRecord<Paradigm>[];
  paradigmEditing: ModuleRecord<Paradigm> | null;
  paradigmEditorOpen: boolean;
  paradigmDraft: Paradigm;
  previewStem: string;
  previewLexemeId: string;
  addParadigm: () => void;
  openParadigmEditor: (record: ModuleRecord<Paradigm>) => void;
  closeParadigmEditor: () => void;
  saveParadigm: () => Promise<"ok" | "name" | "error" | "none">;
  deleteParadigm: () => void;
  pinPreviewOverride: (record: ModuleRecord<LexemeValue>, slot: ParadigmSlot, form: string) => void;
  clearPreviewOverride: (record: ModuleRecord<LexemeValue>, slot: ParadigmSlot) => void;
} = $props();

let nameInput: HTMLInputElement | undefined = $state();

const slotOptions = $derived(
  paradigmDraft.slots.filter((slot) => slot.label.trim()).map((slot) => ({ id: slot.id, label: slot.label })),
);
const previewLexeme = $derived(records.find((record) => record.id === previewLexemeId));
const previewStemValue = $derived(previewStem || previewLexeme?.value.lemma || "");
const previewParadigmId = $derived(paradigmEditing?.id ?? "");
const previewCells = $derived(
  previewParadigm(
    normalizeParadigm(paradigmDraft),
    previewStemValue,
    previewLexeme?.value.forms ?? [],
    previewParadigmId,
  ),
);

$effect(() => {
  if (paradigmEditorOpen && nameInput) nameInput.focus();
});

function addSlot() {
  paradigmDraft.slots.push(emptySlot());
}

function removeSlot(index: number) {
  const removed = paradigmDraft.slots[index]?.id;
  paradigmDraft.slots.splice(index, 1);
  for (const rule of paradigmDraft.rules) {
    rule.operations = rule.operations.filter((item) => item.slotId !== removed);
  }
}

function addRule() {
  paradigmDraft.rules.push(emptyRule(paradigmDraft.kind));
}

function removeRule(index: number) {
  paradigmDraft.rules.splice(index, 1);
}

function addOperation(ruleIndex: number) {
  paradigmDraft.rules[ruleIndex].operations.push(emptyOperation(paradigmDraft.slots[0]?.id ?? ""));
}

function removeOperation(ruleIndex: number, operationIndex: number) {
  paradigmDraft.rules[ruleIndex].operations.splice(operationIndex, 1);
}

function handleLexemeChange(event: Event & { currentTarget: HTMLSelectElement }) {
  previewLexemeId = event.currentTarget.value;
  const chosen = records.find((record) => record.id === previewLexemeId);
  previewStem = chosen?.value.lemma ?? previewStem;
}

async function handleSubmit(event: SubmitEvent) {
  event.preventDefault();
  const outcome = await saveParadigm();
  if (outcome === "name") nameInput?.focus();
}
</script>

<div class="language-toolbar">
  <div class="language-toolbar-title">
    <p class="language-toolbar-eyebrow">Focused projection</p>
    <h2>Forms</h2>
    <p class="language-toolbar-subtitle">
      {selectedLanguage
        ? `${selectedLanguage.name} · paradigms, rules, and generated forms`
        : "Select a language to document its morphology."}
    </p>
  </div>
  <div class="language-toolbar-actions">
    <button type="button" class="language-button" disabled={!selectedLanguage} onclick={addParadigm}
      >Add paradigm</button>
  </div>
</div>
{#if paneLoading}
  <p class="language-empty language-loading" role="status">Loading paradigms…</p>
{:else if paradigmEditorOpen}
  <form class="language-editor" onsubmit={handleSubmit}>
    <datalist id="language-pos">
      {#each PART_OF_SPEECH_SUGGESTIONS as suggestion}
        <option value={suggestion}>{suggestion}</option>
      {/each}
    </datalist>
    <label class="language-field">
      <span>Name</span>
      <input name="name" bind:this={nameInput} bind:value={paradigmDraft.name} />
    </label>
    <label class="language-field">
      <span>Kind</span>
      <select
        name="kind"
        aria-label="Paradigm kind"
        value={paradigmDraft.kind}
        onchange={(event) => (paradigmDraft.kind = event.currentTarget.value as ParadigmKind)}>
        {#each PARADIGM_KINDS as item (item.id)}
          <option value={item.id}>{item.label}</option>
        {/each}
      </select>
    </label>
    <label class="language-field">
      <span>Part of speech (optional)</span>
      <input
        name="partOfSpeech"
        list="language-pos"
        value={paradigmDraft.partOfSpeech ?? ""}
        oninput={(event) => (paradigmDraft.partOfSpeech = event.currentTarget.value)} />
    </label>
    <label class="language-field">
      <span>Notes (optional)</span>
      <textarea
        name="notes"
        value={paradigmDraft.notes ?? ""}
        oninput={(event) => (paradigmDraft.notes = event.currentTarget.value)}></textarea>
    </label>
    <section class="language-group">
      <div class="language-group-head">
        <h3>Slots</h3>
        <button type="button" class="language-button secondary" onclick={addSlot}>Add</button>
      </div>
      {#if paradigmDraft.slots.length === 0}
        <p class="language-empty" role="status">Add cells such as 1sg, plural, or comparative.</p>
      {/if}
      {#each paradigmDraft.slots as slot, index (slot.id)}
        <div class="language-inline">
          <div class="language-inline-fields">
            <label class="language-field">
              <span>Slot label</span>
              <input name={`slot-label-${index}`} bind:value={slot.label} />
            </label>
            <label class="language-field">
              <span>Features (optional)</span>
              <input
                name={`slot-features-${index}`}
                value={slot.features ?? ""}
                oninput={(event) => (slot.features = event.currentTarget.value || undefined)} />
            </label>
          </div>
          <button type="button" class="language-button secondary language-danger" onclick={() => removeSlot(index)}
            >Remove</button>
        </div>
      {/each}
    </section>
    <section class="language-group">
      <div class="language-group-head">
        <h3>Rules</h3>
        <button type="button" class="language-button secondary" onclick={addRule}>Add</button>
      </div>
      {#if paradigmDraft.rules.length === 0}
        <p class="language-empty" role="status">
          Add an inflection or derivation rule. More specific suffix matches win.
        </p>
      {/if}
      {#each paradigmDraft.rules as rule, index (rule.id)}
        <section class="language-group">
          <div class="language-group-head">
            <h3>{rule.name || `Rule ${index + 1}`}</h3>
            <button type="button" class="language-button secondary language-danger" onclick={() => removeRule(index)}
              >Remove</button>
          </div>
          <label class="language-field">
            <span>Rule name</span>
            <input name={`rule-name-${index}`} bind:value={rule.name} />
          </label>
          <label class="language-field">
            <span>Kind</span>
            <select
              name={`rule-kind-${index}`}
              aria-label="Rule kind"
              value={rule.kind}
              onchange={(event) => (rule.kind = (event.currentTarget.value || paradigmDraft.kind) as ParadigmKind)}>
              {#each PARADIGM_KINDS as item (item.id)}
                <option value={item.id}>{item.label}</option>
              {/each}
            </select>
          </label>
          <label class="language-field">
            <span>Match lemma ending (optional)</span>
            <input
              name={`rule-match-${index}`}
              value={rule.match ?? ""}
              oninput={(event) => (rule.match = event.currentTarget.value || undefined)} />
          </label>
          <label class="language-field">
            <span>Notes (optional)</span>
            <textarea
              name={`rule-notes-${index}`}
              rows={2}
              value={rule.notes ?? ""}
              oninput={(event) => (rule.notes = event.currentTarget.value || undefined)}></textarea>
          </label>
          {#each rule.operations as operation, operationIndex (operation.id)}
            <div class="language-inline">
              <div class="language-inline-fields">
                <label class="language-field">
                  <span>Slot</span>
                  <select
                    name={`op-slot-${index}-${operationIndex}`}
                    aria-label="Operation slot"
                    value={operation.slotId}
                    onchange={(event) => (operation.slotId = event.currentTarget.value)}>
                    {#each slotOptions as option (option.id)}
                      <option value={option.id}>{option.label}</option>
                    {/each}
                  </select>
                </label>
                <label class="language-field">
                  <span>Operation</span>
                  <select
                    name={`op-kind-${index}-${operationIndex}`}
                    aria-label="Operation kind"
                    value={operation.op}
                    onchange={(event) =>
                      (operation.op = (event.currentTarget.value || "suffix") as MorphOperationKind)}>
                    {#each OPERATION_KINDS as item (item.id)}
                      <option value={item.id}>{item.label}</option>
                    {/each}
                  </select>
                </label>
                <label class="language-field">
                  <span>Replace from (optional)</span>
                  <input
                    name={`op-from-${index}-${operationIndex}`}
                    value={operation.from ?? ""}
                    oninput={(event) => (operation.from = event.currentTarget.value || undefined)} />
                </label>
                <label class="language-field">
                  <span>Affix or replacement (optional)</span>
                  <input
                    name={`op-value-${index}-${operationIndex}`}
                    value={operation.value ?? ""}
                    oninput={(event) => (operation.value = event.currentTarget.value || undefined)} />
                </label>
              </div>
              <button
                type="button"
                class="language-button secondary language-danger"
                onclick={() => removeOperation(index, operationIndex)}>Remove</button>
            </div>
          {/each}
          <button type="button" class="language-button secondary" onclick={() => addOperation(index)}
            >Add operation</button>
        </section>
      {/each}
    </section>
    <section class="language-group">
      <h3>Generated preview</h3>
      <p class="language-empty" role="status">
        This table is computed from the current rules. Saving a rule never rewrites authored word forms.
      </p>
      <label class="language-field">
        <span>Preview lexeme (optional)</span>
        <select
          name="previewLexemeId"
          aria-label="Preview lexeme"
          value={previewLexemeId}
          onchange={handleLexemeChange}>
          <option value="">Type a stem</option>
          {#each records as record (record.id)}
            <option value={record.id}>{record.value.lemma}</option>
          {/each}
        </select>
      </label>
      <label class="language-field">
        <span>Stem</span>
        <input
          name="previewStem"
          value={previewStemValue}
          onchange={(event) => (previewStem = event.currentTarget.value)} />
      </label>
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
            {#each previewCells as cell (cell.slot.id)}
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
                  >{#if previewLexeme && previewParadigmId && cell.form && cell.provenance === "generated"}
                    <button
                      type="button"
                      class="language-button secondary"
                      onclick={() => void pinPreviewOverride(previewLexeme, cell.slot, cell.form)}>Pin override</button>
                  {:else if previewLexeme && previewParadigmId && cell.provenance === "authored"}
                    <button
                      type="button"
                      class="language-button secondary"
                      onclick={() => void clearPreviewOverride(previewLexeme, cell.slot)}>Clear override</button>
                  {/if}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>
    {#if error}
      <p class="language-status error" role="alert">{error}</p>
    {/if}
    <div class="language-actions">
      <span>
        {#if paradigmEditing}
          <button type="button" class="language-button secondary language-danger" onclick={deleteParadigm}
            >Delete</button>
        {/if}
      </span>
      <span>
        <button type="button" class="language-button secondary" onclick={closeParadigmEditor}>Cancel</button>
        <button type="submit" class="language-button">Save paradigm</button>
      </span>
    </div>
  </form>
{:else if error}
  <p class="language-status error" role="alert">{error}</p>
{:else if !selectedLanguage}
  <div class="language-empty-card">
    <p class="language-empty" role="status">Select a language to document its paradigms.</p>
  </div>
{:else if paradigms.length === 0}
  <div class="language-empty-card">
    <p class="language-empty" role="status">
      No paradigms yet. Add an inflection or derivation table, then preview generated forms.
    </p>
    <div class="language-inline">
      <button type="button" class="language-button secondary" onclick={addParadigm}>Add first paradigm</button>
    </div>
  </div>
{:else}
  <section class="language-pane-section">
    <h3>Paradigm library</h3>
    <p>{paradigms.length} paradigm{paradigms.length === 1 ? "" : "s"} · select one to edit rules or preview forms.</p>
    <ul class="lexeme-list">
      {#each paradigms as record (record.id)}
        <li>
          <button
            type="button"
            class="language-item"
            aria-label={`Edit paradigm ${record.value.name}`}
            onclick={() => openParadigmEditor(record)}>
            <strong>{record.value.name}</strong>
            <small>{record.value.kind}</small>
            <span
              >{record.value.slots.length} slot{record.value.slots.length === 1 ? "" : "s"} · {record.value.rules
                .length}
              rule{record.value.rules.length === 1 ? "" : "s"}</span>
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
.language-chart,
.paradigm-preview {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.language-chart th,
.language-chart td,
.paradigm-preview th,
.paradigm-preview td {
  border: 1px solid var(--line);
  padding: 8px;
  text-align: center;
  min-width: 52px;
}
.paradigm-preview th,
.paradigm-preview td {
  text-align: left;
}
.language-chart th,
.paradigm-preview th {
  background: var(--surface-muted);
  font-weight: 600;
  color: var(--ink-soft);
}
.language-chart-wrap {
  overflow-x: auto;
  margin: 8px 0 4px;
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
