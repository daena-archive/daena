<script lang="ts">
import { untrack } from "svelte";
import type { EntitySummary, ModuleContext, ModuleRecord } from "../../../../module-api/src/index";
import type { LexemeValue } from "../lexeme";
import { confirm } from "../confirm.svelte";
import { normalizeLexeme, PART_OF_SPEECH_SUGGESTIONS, serializeLexeme } from "../lexeme";
import {
  clearOverride,
  emptyOperation,
  emptyParadigm,
  emptyRule,
  emptySlot,
  normalizeParadigm,
  OPERATION_KINDS,
  overrideTarget,
  PARADIGM_KINDS,
  pinOverride,
  previewParadigm,
  serializeParadigm,
  type MorphOperationKind,
  type Paradigm,
  type ParadigmKind,
  type ParadigmSlot,
} from "../morphology";

let {
  context,
  selectedLanguage,
  active,
  registerLeaveGuard,
  setMutationActive,
}: {
  context: ModuleContext;
  selectedLanguage: EntitySummary | null;
  active: boolean;
  registerLeaveGuard: (guard: (() => Promise<boolean> | boolean) | null) => void;
  setMutationActive: (active: boolean) => void;
} = $props();

let cancelled = $state(false);
let records: ModuleRecord<LexemeValue>[] = $state([]);
let paradigms: ModuleRecord<Paradigm>[] = $state([]);
let paradigmEditing = $state<ModuleRecord<Paradigm> | null>(null);
let paradigmEditorOpen = $state(false);
let paradigmDraft: Paradigm = $state(emptyParadigm());
let paradigmSaving = $state(false);
let previewStem = $state("");
let previewLexemeId = $state("");
let paneLoading = $state(false);
let error = $state("");
let request = $state(0);

let nameInput: HTMLInputElement | undefined = $state();

let lastLoadedLanguage: string | null = null;

$effect(() => {
  const languageId = selectedLanguage?.id ?? null;
  void languageId;
  if (!active) return;
  if (languageId === lastLoadedLanguage) {
    untrack(() => void loadForms());
    return;
  }
  lastLoadedLanguage = languageId;
  paradigmEditing = null;
  paradigmEditorOpen = false;
  paradigmDraft = emptyParadigm();
  previewStem = "";
  previewLexemeId = "";
  untrack(() => void loadForms());
});

$effect(() => {
  return () => {
    cancelled = true;
  };
});

function formsHasDraft() {
  if (!paradigmEditorOpen) return false;
  const baseline = paradigmEditing ? normalizeParadigm(paradigmEditing.value) : emptyParadigm();
  return (
    JSON.stringify(serializeParadigm(normalizeParadigm(paradigmDraft))) !== JSON.stringify(serializeParadigm(baseline))
  );
}

async function tryLeaveForms(confirmLeave: (message: string) => Promise<boolean> | boolean) {
  if (!formsHasDraft()) return true;
  if (paradigmSaving) return false;
  const allowed = await confirmLeave("You have unsaved changes to a paradigm. Discard them?");
  if (allowed) closeParadigmEditor();
  return allowed;
}

$effect(() => {
  registerLeaveGuard(() => tryLeaveForms((message) => confirm("Unsaved changes", message)));
});

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

async function loadForms() {
  if (!selectedLanguage) {
    paradigms = [];
    records = [];
    paneLoading = false;
    error = "";
    return;
  }
  const token = ++request;
  paneLoading = true;
  error = "";
  try {
    const [tables, lexemes] = await Promise.all([
      context.records.list<Paradigm>("paradigms", selectedLanguage.id, { limit: 100, sort: "name" }),
      context.records.list<LexemeValue>("lexemes", selectedLanguage.id, { limit: 500, sort: "lemma" }),
    ]);
    if (!cancelled && token === request) {
      paneLoading = false;
      error = "";
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
      error = cause instanceof Error ? cause.message : String(cause);
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
  const ownerLanguageId = selectedLanguage.id;
  const blankSlot = paradigmDraft.slots.findIndex((slot) => !slot.label.trim());
  if (blankSlot >= 0) {
    error = `Slot ${blankSlot + 1} needs a label before this paradigm can be saved.`;
    return "error";
  }
  const slotIds = new Set(paradigmDraft.slots.map((slot) => slot.id));
  const incompleteOperation = paradigmDraft.rules.some((rule) =>
    rule.operations.some((operation) => !operation.slotId || !slotIds.has(operation.slotId)),
  );
  if (incompleteOperation) {
    error = "Every operation must target an existing slot.";
    return "error";
  }
  const value = normalizeParadigm(paradigmDraft);
  if (!value.name) {
    error = "Name is required.";
    return "name";
  }
  error = "";
  paradigmDraft = value;
  paradigmSaving = true;
  setMutationActive(true);
  try {
    const payload = serializeParadigm(value);
    if (paradigmEditing) {
      const updated = await context.records.update("paradigms", paradigmEditing.id, ownerLanguageId, payload, {
        expectedRevision: paradigmEditing.revision,
        requestId: crypto.randomUUID(),
      });
      paradigmEditing = { ...updated, value: normalizeParadigm(updated.value) };
    } else {
      const created = await context.records.create("paradigms", ownerLanguageId, payload, {
        requestId: crypto.randomUUID(),
      });
      paradigmEditing = { ...created, value: normalizeParadigm(created.value) };
    }
    paradigmEditorOpen = true;
    paradigmDraft = paradigmEditing.value;
    paradigmSaving = false;
    setMutationActive(false);
    if (ownerLanguageId === selectedLanguage?.id) await loadForms();
    return "ok";
  } catch (cause) {
    paradigmSaving = false;
    setMutationActive(false);
    error = cause instanceof Error ? cause.message : String(cause);
    return "error";
  }
}

async function deleteParadigm() {
  if (!selectedLanguage || !paradigmEditing) return;
  if (!(await confirm("Delete", `Delete “${paradigmEditing.value.name}”?`))) return;
  const ownerLanguageId = selectedLanguage.id;
  error = "";
  try {
    setMutationActive(true);
    await context.records.delete("paradigms", paradigmEditing.id, ownerLanguageId, {
      expectedRevision: paradigmEditing.revision,
      requestId: crypto.randomUUID(),
    });
    paradigmEditing = null;
    paradigmEditorOpen = false;
    paradigmDraft = emptyParadigm();
    setMutationActive(false);
    if (ownerLanguageId === selectedLanguage?.id) await loadForms();
  } catch (cause) {
    setMutationActive(false);
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

async function persistLexemeForms(record: ModuleRecord<LexemeValue>, forms: LexemeValue["forms"]) {
  if (!selectedLanguage) return;
  const ownerLanguageId = selectedLanguage.id;
  const value = normalizeLexeme({ ...record.value, forms });
  setMutationActive(true);
  try {
    const updated = await context.records.update("lexemes", record.id, ownerLanguageId, serializeLexeme(value), {
      expectedRevision: record.revision,
      requestId: crypto.randomUUID(),
    });
    const next = { ...updated, value: normalizeLexeme(updated.value) };
    records = records.map((item) => (item.id === next.id ? next : item));
    if (ownerLanguageId !== selectedLanguage?.id) throw new Error("Language changed while updating lexeme.");
    return;
  } finally {
    setMutationActive(false);
  }
}

async function pinPreviewOverride(record: ModuleRecord<LexemeValue>, slot: ParadigmSlot, form: string) {
  const paradigmId = paradigmEditing?.id;
  if (!paradigmId) return;
  error = "";
  try {
    await persistLexemeForms(record, pinOverride(record.value.forms, paradigmId, slot, form));
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

async function clearPreviewOverride(record: ModuleRecord<LexemeValue>, slot: ParadigmSlot) {
  const paradigmId = paradigmEditing?.id;
  if (!paradigmId) return;
  const target = overrideTarget(record.value.forms, paradigmId, slot);
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
  error = "";
  try {
    await persistLexemeForms(record, clearOverride(record.value.forms, paradigmId, slot));
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

$effect(() => {
  if (paradigmEditorOpen && nameInput) nameInput.focus();
});

function addSlot() {
  paradigmDraft.slots.push(emptySlot());
}

async function removeSlot(index: number) {
  const slot = paradigmDraft.slots[index];
  if (!slot) return;
  const operationCount = paradigmDraft.rules.reduce(
    (count, rule) => count + rule.operations.filter((item) => item.slotId === slot.id).length,
    0,
  );
  if (
    operationCount > 0 &&
    !(await confirm(
      "Remove slot",
      `Remove “${slot.label || `Slot ${index + 1}`}” and ${operationCount} linked ${operationCount === 1 ? "operation" : "operations"}?`,
    ))
  ) {
    return;
  }
  const removed = slot.id;
  paradigmDraft.slots.splice(index, 1);
  for (const rule of paradigmDraft.rules) {
    rule.operations = rule.operations.filter((item) => item.slotId !== removed);
  }
}

function addRule() {
  paradigmDraft.rules.push(emptyRule(paradigmDraft.kind));
}

async function removeRule(index: number) {
  const rule = paradigmDraft.rules[index];
  if (!rule) return;
  if (!(await confirm("Remove rule", `Remove “${rule.name || `Rule ${index + 1}`}” from this paradigm?`))) return;
  paradigmDraft.rules.splice(index, 1);
}

function addOperation(ruleIndex: number) {
  const firstSlot = slotOptions[0];
  if (!firstSlot) {
    error = "Add and label a slot before adding rule operations.";
    return;
  }
  error = "";
  paradigmDraft.rules[ruleIndex].operations.push(emptyOperation(firstSlot.id));
}

function removeOperation(ruleIndex: number, operationIndex: number) {
  paradigmDraft.rules[ruleIndex].operations.splice(operationIndex, 1);
}

function handleLexemeChange() {
  const chosen = records.find((record) => record.id === previewLexemeId);
  previewStem = chosen?.value.lemma ?? "";
}

async function handleSubmit(event: SubmitEvent) {
  event.preventDefault();
  const outcome = await saveParadigm();
  if (outcome === "name") nameInput?.focus();
}
</script>

<div class="language-toolbar">
  <div class="language-toolbar-title">
    <p class="language-toolbar-eyebrow">Language crafting studio</p>
    <h2>Morphology</h2>
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
      <select name="kind" aria-label="Paradigm kind" bind:value={paradigmDraft.kind}>
        {#each PARADIGM_KINDS as item (item.id)}
          <option value={item.id}>{item.label}</option>
        {/each}
      </select>
    </label>
    <label class="language-field">
      <span>Part of speech (optional)</span>
      <input name="partOfSpeech" list="language-pos" bind:value={paradigmDraft.partOfSpeech} />
    </label>
    <label class="language-field">
      <span>Notes (optional)</span>
      <textarea name="notes" bind:value={paradigmDraft.notes}></textarea>
    </label>
    <section class="language-form-section">
      <div class="language-form-section-header">
        <div>
          <h3>Slots</h3>
          <p>Define the cells in your paradigm table (e.g., 1sg, plural, comparative).</p>
        </div>
        <button type="button" class="language-button secondary" onclick={addSlot}>Add slot</button>
      </div>
      {#if paradigmDraft.slots.length === 0}
        <div class="language-empty-card">
          <p class="language-empty" role="status">
            No slots defined yet. Add cells such as 1sg, plural, or comparative.
          </p>
        </div>
      {:else}
        <div class="forms-slots-grid">
          {#each paradigmDraft.slots as slot, index (slot.id)}
            <div class="forms-slot-card">
              <div class="forms-slot-header">
                <span class="forms-slot-number">{index + 1}</span>
                <button
                  type="button"
                  class="forms-slot-remove"
                  onclick={() => void removeSlot(index)}
                  aria-label="Remove slot">&times;</button>
              </div>
              <label class="language-field">
                <span>Label</span>
                <input name={`slot-label-${index}`} bind:value={slot.label} placeholder="e.g. 1sg" />
              </label>
              <label class="language-field">
                <span>Features (optional)</span>
                <input
                  name={`slot-features-${index}`}
                  bind:value={slot.features}
                  placeholder="e.g. person=1, number=sg" />
              </label>
            </div>
          {/each}
        </div>
      {/if}
    </section>
    <section class="language-form-section">
      <div class="language-form-section-header">
        <div>
          <h3>Rules</h3>
          <p>Define how forms are generated from the stem. More specific suffix matches win.</p>
        </div>
        <button type="button" class="language-button secondary" onclick={addRule}>Add rule</button>
      </div>
      {#if paradigmDraft.rules.length === 0}
        <div class="language-empty-card">
          <p class="language-empty" role="status">No rules defined yet. Add inflection or derivation rules.</p>
        </div>
      {:else}
        <div class="forms-rules-list">
          {#each paradigmDraft.rules as rule, index (rule.id)}
            <details class="forms-rule-item" open={index === 0}>
              <summary>
                <span class="forms-rule-name">{rule.name || `Rule ${index + 1}`}</span>
                <span class="forms-rule-kind"
                  >{PARADIGM_KINDS.find((k) => k.id === rule.kind)?.label ?? rule.kind}</span>
                <button
                  type="button"
                  class="forms-rule-remove"
                  onclick={(e) => {
                    e.preventDefault();
                    void removeRule(index);
                  }}
                  aria-label="Remove rule">&times;</button>
              </summary>
              <div class="forms-rule-content">
                <div class="language-section-grid">
                  <label class="language-field">
                    <span>Rule name</span>
                    <input name={`rule-name-${index}`} bind:value={rule.name} placeholder="e.g. Regular plural" />
                  </label>
                  <label class="language-field">
                    <span>Kind</span>
                    <select name={`rule-kind-${index}`} aria-label="Rule kind" bind:value={rule.kind}>
                      {#each PARADIGM_KINDS as item (item.id)}
                        <option value={item.id}>{item.label}</option>
                      {/each}
                    </select>
                  </label>
                </div>
                <label class="language-field">
                  <span>Match lemma ending (optional)</span>
                  <input
                    name={`rule-match-${index}`}
                    bind:value={rule.match}
                    placeholder="e.g. -ar (matches verbs ending in -ar)" />
                </label>
                <label class="language-field">
                  <span>Notes (optional)</span>
                  <textarea
                    name={`rule-notes-${index}`}
                    rows={2}
                    bind:value={rule.notes}
                    placeholder="Explain when this rule applies"></textarea>
                </label>
                <div class="forms-operations">
                  <div class="forms-operations-header">
                    <h4>Operations</h4>
                    <button
                      type="button"
                      class="language-button secondary"
                      disabled={slotOptions.length === 0}
                      title={slotOptions.length === 0 ? "Add and label a slot first" : undefined}
                      onclick={() => addOperation(index)}>Add operation</button>
                  </div>
                  {#if rule.operations.length === 0}
                    <p class="language-empty" role="status">
                      No operations defined. Add suffix, prefix, or replacement rules.
                    </p>
                  {:else}
                    <div class="forms-operations-list">
                      {#each rule.operations as operation, operationIndex (operation.id)}
                        <div class="forms-operation-item">
                          <div class="forms-operation-fields">
                            <label class="language-field">
                              <span>Slot</span>
                              <select
                                name={`op-slot-${index}-${operationIndex}`}
                                aria-label="Operation slot"
                                bind:value={operation.slotId}>
                                <option value="">Select slot...</option>
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
                                bind:value={operation.op}>
                                {#each OPERATION_KINDS as item (item.id)}
                                  <option value={item.id}>{item.label}</option>
                                {/each}
                              </select>
                            </label>
                            {#if operation.op === "replace-suffix"}
                              <label class="language-field">
                                <span>Replace from</span>
                                <input
                                  name={`op-from-${index}-${operationIndex}`}
                                  bind:value={operation.from}
                                  placeholder="e.g. -ar" />
                              </label>
                            {/if}
                            <label class="language-field">
                              <span>{operation.op === "replace-suffix" ? "Replace with" : "Affix"}</span>
                              <input
                                name={`op-value-${index}-${operationIndex}`}
                                bind:value={operation.value}
                                placeholder={operation.op === "prefix"
                                  ? "e.g. un-"
                                  : operation.op === "suffix"
                                    ? "e.g. -ed"
                                    : "e.g. -ó"} />
                            </label>
                          </div>
                          <button
                            type="button"
                            class="forms-operation-remove"
                            onclick={() => removeOperation(index, operationIndex)}
                            aria-label="Remove operation">&times;</button>
                        </div>
                      {/each}
                    </div>
                  {/if}
                </div>
              </div>
            </details>
          {/each}
        </div>
      {/if}
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
          bind:value={previewLexemeId}
          onchange={handleLexemeChange}>
          <option value={""}>Type a stem</option>
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
          oninput={(event) => (previewStem = event.currentTarget.value)} />
      </label>
      {#if paradigmDraft.slots.length === 0}
        <div class="language-empty-card">
          <p class="language-empty" role="status">Add a slot to preview generated forms.</p>
        </div>
      {:else}<div class="language-chart-wrap">
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
                        onclick={() => void pinPreviewOverride(previewLexeme, cell.slot, cell.form)}
                        >Pin override</button>
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
        </div>{/if}
    </section>
    {#if error}
      <p class="language-status error" role="alert">{error}</p>
    {/if}
    <div class="language-actions">
      <span>
        {#if paradigmEditing}
          <button
            type="button"
            class="language-button secondary language-danger"
            onclick={deleteParadigm}
            disabled={paradigmSaving}>Delete</button>
        {/if}
      </span>
      <span>
        <button type="button" class="language-button secondary" onclick={closeParadigmEditor} disabled={paradigmSaving}
          >Cancel</button>
        <button type="submit" class="language-button" disabled={paradigmSaving}
          >{paradigmSaving ? "Saving…" : "Save paradigm"}</button>
      </span>
    </div>
  </form>
{:else if error}
  <div class="language-empty-card language-error-card">
    <p class="language-status error" role="alert">{error}</p>
    {#if selectedLanguage}
      <button type="button" class="language-button secondary" onclick={() => void loadForms()}>Try again</button>
    {/if}
  </div>
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
.paradigm-preview {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
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
  background: var(--theme-success-bg, #eef3ef);
}
.form-provenance.is-missing {
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
  border: 1px solid var(--theme-warning-border, #ebe7de);
  border-radius: 10px;
  background: var(--surface);
  color: inherit;
  text-align: left;
  cursor: pointer;
  box-shadow: 0 1px 2px rgba(38, 42, 33, 0.03);
}
.language-item:hover {
  border-color: var(--theme-warning-border, #e5d8c6);
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
  color: var(--danger);
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
.forms-slots-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 12px;
}
.forms-slot-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface);
}
.forms-slot-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.forms-slot-number {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: var(--accent);
  color: var(--on-bright-accent);
  font-size: 12px;
  font-weight: 600;
}
.forms-slot-remove {
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
  font-size: 16px;
}
.forms-slot-remove:hover {
  background: var(--surface-muted);
  color: var(--danger);
}
.forms-rules-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.forms-rule-item {
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface);
  overflow: hidden;
}
.forms-rule-item summary {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px;
  cursor: pointer;
  list-style: none;
}
.forms-rule-item summary::-webkit-details-marker {
  display: none;
}
.forms-rule-item summary::before {
  content: "▸";
  color: var(--ink-faint);
  font-size: 12px;
  transition: transform 0.15s ease;
}
.forms-rule-item[open] summary::before {
  transform: rotate(90deg);
}
.forms-rule-name {
  flex: 1;
  font-weight: 600;
  font-size: 14px;
}
.forms-rule-kind {
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  font-size: 11px;
}
.forms-rule-remove {
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
  font-size: 16px;
}
.forms-rule-remove:hover {
  background: var(--surface-muted);
  color: var(--danger);
}
.forms-rule-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 12px;
  border-top: 1px solid var(--line);
}
.forms-operations {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.forms-operations-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.forms-operations-header h4 {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
}
.forms-operations-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.forms-operation-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface-muted);
}
.forms-operation-fields {
  flex: 1;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}
.forms-operation-remove {
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
  font-size: 16px;
  flex-shrink: 0;
}
.forms-operation-remove:hover {
  background: var(--surface-muted);
  color: var(--danger);
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
@media (max-width: 760px) {
  .forms-slots-grid {
    grid-template-columns: 1fr;
  }
  .forms-operation-fields {
    grid-template-columns: 1fr;
  }
}
</style>
