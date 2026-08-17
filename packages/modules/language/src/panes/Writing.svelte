<script lang="ts">
import { untrack } from "svelte";
import type { EntitySummary, ModuleContext, ModuleRecord } from "../../../../module-api/src/index";
import { confirm } from "../confirm.svelte";
import { STATUS_SUGGESTIONS } from "../lexeme";
import type { OrthographyValue } from "../orthography";
import { emptyOrthography, normalizeOrthography, serializeOrthography } from "../orthography";
import type { PhonemeValue } from "../phonology";
import { normalizePhoneme } from "../phonology";

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
let phonemes: ModuleRecord<PhonemeValue>[] = $state([]);
let orthographies: ModuleRecord<OrthographyValue>[] = $state([]);
let orthographyEditing = $state<ModuleRecord<OrthographyValue> | null>(null);
let orthographyEditorOpen = $state(false);
let orthographyDraft: OrthographyValue = $state(emptyOrthography());
let orthographySaving = $state(false);
let paneLoading = $state(false);
let error = $state("");
let request = $state(0);

let nameInput: HTMLInputElement | undefined = $state();
let soundsText = $state<string[]>([]);

let lastLoadedLanguage: string | null = null;

$effect(() => {
  const languageId = selectedLanguage?.id ?? null;
  void languageId;
  if (!active) return;
  if (languageId === lastLoadedLanguage) {
    untrack(() => void loadWriting());
    return;
  }
  lastLoadedLanguage = languageId;
  orthographyEditing = null;
  orthographyEditorOpen = false;
  orthographyDraft = emptyOrthography();
  untrack(() => void loadWriting());
});

$effect(() => {
  return () => {
    cancelled = true;
  };
});

function writingHasDraft() {
  if (!orthographyEditorOpen) return false;
  const baseline = orthographyEditing ? normalizeOrthography(orthographyEditing.value) : emptyOrthography();
  return (
    JSON.stringify(serializeOrthography(normalizeOrthography(orthographyDraft))) !==
    JSON.stringify(serializeOrthography(baseline))
  );
}

async function tryLeaveWriting(confirmLeave: (message: string) => Promise<boolean> | boolean) {
  if (!writingHasDraft()) return true;
  if (orthographySaving) return false;
  const allowed = await confirmLeave("You have unsaved changes to a writing system. Discard them?");
  if (allowed) closeOrthographyEditor();
  return allowed;
}

$effect(() => {
  registerLeaveGuard(() => tryLeaveWriting((message) => confirm("Unsaved changes", message)));
});

$effect(() => {
  if (orthographyEditorOpen) soundsText = orthographyDraft.mappings.map((mapping) => mapping.sounds.join(" "));
});

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
  const ownerLanguageId = selectedLanguage.id;
  orthographyDraft = normalizeOrthography(orthographyDraft);
  if (!orthographyDraft.name) {
    error = "Writing system name is required.";
    return "name";
  }
  error = "";
  orthographySaving = true;
  setMutationActive(true);
  try {
    const payload = serializeOrthography(orthographyDraft);
    if (orthographyEditing) {
      const updated = await context.records.update(
        "orthographies",
        orthographyEditing.id,
        ownerLanguageId,
        payload,
        { expectedRevision: orthographyEditing.revision, requestId: crypto.randomUUID() },
      );
      orthographyEditing = { ...updated, value: normalizeOrthography(updated.value) };
    } else {
      const created = await context.records.create("orthographies", ownerLanguageId, payload, {
        requestId: crypto.randomUUID(),
      });
      orthographyEditing = { ...created, value: normalizeOrthography(created.value) };
    }
    orthographyEditorOpen = true;
    orthographyDraft = orthographyEditing.value;
    orthographySaving = false;
    setMutationActive(false);
    if (ownerLanguageId === selectedLanguage?.id) await loadWriting();
    return "ok";
  } catch (cause) {
    orthographySaving = false;
    setMutationActive(false);
    error = cause instanceof Error ? cause.message : String(cause);
    return "error";
  }
}

async function deleteOrthography() {
  if (!selectedLanguage || !orthographyEditing) return;
  if (!await confirm("Delete", `Delete “${orthographyEditing.value.name}”?`)) return;
  const ownerLanguageId = selectedLanguage.id;
  error = "";
  try {
    setMutationActive(true);
    await context.records.delete("orthographies", orthographyEditing.id, ownerLanguageId, {
      expectedRevision: orthographyEditing.revision,
      requestId: crypto.randomUUID(),
    });
    orthographyEditing = null;
    orthographyEditorOpen = false;
    orthographyDraft = emptyOrthography();
    setMutationActive(false);
    if (ownerLanguageId === selectedLanguage?.id) await loadWriting();
  } catch (cause) {
    setMutationActive(false);
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

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
      <input name="status" list="language-status" bind:value={orthographyDraft.status} />
    </label>
    <label class="language-field">
      <span>Notes (optional)</span>
      <textarea name="notes" bind:value={orthographyDraft.notes}></textarea>
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
              <input name={`environment-${index}`} bind:value={mapping.environment} />
            </label>
            <label class="language-field">
              <span>Notes (optional)</span>
              <input name={`mapping-notes-${index}`} bind:value={mapping.notes} />
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
            disabled={orthographySaving}
            >Delete</button>
        {/if}
      </span>
      <span>
        <button type="button" class="language-button secondary" onclick={closeOrthographyEditor}
          disabled={orthographySaving}
          >Cancel</button>
        <button type="submit" class="language-button" disabled={orthographySaving}
          >{orthographySaving ? "Saving…" : "Save writing system"}</button>
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
.language-field {
  display: grid;
  gap: 6px;
  min-width: 0;
  color: var(--ink-soft);
  font-size: 11px;
  letter-spacing: 0.01em;
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
  .language-item span {
    white-space: normal;
  }
  .language-inline {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
