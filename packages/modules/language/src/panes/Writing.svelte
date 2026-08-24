<script lang="ts">
import { untrack } from "svelte";
import type { EntitySummary, ModuleContext, ModuleRecord } from "../../../../module-api/src/index";
import IpaInput from "../IpaInput.svelte";
import SoundSequenceEditor from "../SoundSequenceEditor.svelte";
import { confirm } from "../confirm.svelte";
import {
  emptyOrthography,
  emptyOrthographyMapping,
  emptyOrthographySample,
  mappingFromPhoneme,
  normalizeOrthography,
  orthographyCoverage,
  representedPhonemeIds,
  serializeOrthography,
  validateOrthography,
  type CharacterGroup,
  type OrthographyValue,
  type PhonemeOption,
} from "../orthography";
import type { PhonemeValue } from "../phonology";
import { normalizePhoneme } from "../phonology";

type EditorSection = "basics" | "characters" | "samples";

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
let editorSection = $state<EditorSection>("basics");
let orthographySaving = $state(false);
let paneLoading = $state(false);
let addFromSoundsOpen = $state(false);
let selectedBulkIds = $state<string[]>([]);
let error = $state("");
let request = $state(0);

let nameInput: HTMLInputElement | undefined = $state();
let lastLoadedLanguage: string | null = null;

const phonemeOptions: PhonemeOption[] = $derived(phonemes.map((record) => ({ id: record.id, ...record.value })));
const coverage = $derived(
  orthographyCoverage(
    orthographyDraft,
    phonemeOptions.map((phoneme) => phoneme.id),
  ),
);
const representedIds = $derived(representedPhonemeIds(orthographyDraft));
const unmappedPhonemes = $derived(
  coverage.unmapped
    .map((phonemeId) => phonemeOptions.find((phoneme) => phoneme.id === phonemeId))
    .filter((item): item is PhonemeOption => !!item),
);

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
  addFromSoundsOpen = false;
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

async function loadWriting() {
  if (!selectedLanguage) {
    orthographies = [];
    phonemes = [];
    paneLoading = false;
    return;
  }
  const token = ++request;
  paneLoading = true;
  error = "";
  try {
    const [systems, inventory] = await Promise.all([
      context.records.list<OrthographyValue>("orthographies", selectedLanguage.id, { limit: 100, sort: "name" }),
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
  editorSection = "basics";
  error = "";
}

function openOrthographyEditor(record: ModuleRecord<OrthographyValue>) {
  orthographyEditing = record;
  orthographyEditorOpen = true;
  orthographyDraft = normalizeOrthography(record.value);
  editorSection = "characters";
  error = "";
}

function closeOrthographyEditor() {
  orthographyEditing = null;
  orthographyEditorOpen = false;
  orthographyDraft = emptyOrthography();
  addFromSoundsOpen = false;
  selectedBulkIds = [];
  error = "";
}

async function saveOrthography(): Promise<"ok" | "name" | "error" | "none"> {
  if (!selectedLanguage) return "none";
  const ownerLanguageId = selectedLanguage.id;
  orthographyDraft = normalizeOrthography(orthographyDraft);
  const validationError = validateOrthography(orthographyDraft);
  if (validationError) {
    error = validationError;
    if (validationError.includes("name")) editorSection = "basics";
    if (validationError.includes("character")) editorSection = "characters";
    if (validationError.includes("sample")) editorSection = "samples";
    return validationError.includes("name") ? "name" : "error";
  }
  error = "";
  orthographySaving = true;
  setMutationActive(true);
  try {
    const payload = serializeOrthography(orthographyDraft);
    if (orthographyEditing) {
      const updated = await context.records.update("orthographies", orthographyEditing.id, ownerLanguageId, payload, {
        expectedRevision: orthographyEditing.revision,
        requestId: crypto.randomUUID(),
      });
      orthographyEditing = { ...updated, value: normalizeOrthography(updated.value) };
    } else {
      const created = await context.records.create("orthographies", ownerLanguageId, payload, {
        requestId: crypto.randomUUID(),
      });
      orthographyEditing = { ...created, value: normalizeOrthography(created.value) };
    }
    orthographyDraft = normalizeOrthography(orthographyEditing.value);
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
  if (
    !(await confirm("Delete writing system", `Delete “${orthographyEditing.value.name}” and its mappings and samples?`))
  )
    return;
  const ownerLanguageId = selectedLanguage.id;
  error = "";
  try {
    setMutationActive(true);
    await context.records.delete("orthographies", orthographyEditing.id, ownerLanguageId, {
      expectedRevision: orthographyEditing.revision,
      requestId: crypto.randomUUID(),
    });
    setMutationActive(false);
    closeOrthographyEditor();
    if (ownerLanguageId === selectedLanguage?.id) await loadWriting();
  } catch (cause) {
    setMutationActive(false);
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

function addMapping(group: CharacterGroup = "ungrouped") {
  orthographyDraft.mappings = [...orthographyDraft.mappings, emptyOrthographyMapping(group)];
}

function addMappingForPhoneme(phoneme: PhonemeOption) {
  orthographyDraft.mappings = [...orthographyDraft.mappings, mappingFromPhoneme(phoneme)];
  editorSection = "characters";
}

function removeMapping(index: number) {
  orthographyDraft.mappings = orthographyDraft.mappings.filter((_, itemIndex) => itemIndex !== index);
}

function moveMapping(index: number, offset: -1 | 1) {
  const destination = index + offset;
  if (destination < 0 || destination >= orthographyDraft.mappings.length) return;
  const next = [...orthographyDraft.mappings];
  [next[index], next[destination]] = [next[destination], next[index]];
  orthographyDraft.mappings = next;
}

function addSample() {
  orthographyDraft.samples = [...orthographyDraft.samples, emptyOrthographySample()];
}

function removeSample(index: number) {
  orthographyDraft.samples = orthographyDraft.samples.filter((_, itemIndex) => itemIndex !== index);
}

function moveSample(index: number, offset: -1 | 1) {
  const destination = index + offset;
  if (destination < 0 || destination >= orthographyDraft.samples.length) return;
  const next = [...orthographyDraft.samples];
  [next[index], next[destination]] = [next[destination], next[index]];
  orthographyDraft.samples = next;
}

function openAddFromSounds() {
  selectedBulkIds = unmappedPhonemes.map((phoneme) => phoneme.id);
  addFromSoundsOpen = true;
}

function toggleBulkSound(phonemeId: string, checked: boolean) {
  selectedBulkIds = checked
    ? [...selectedBulkIds.filter((id) => id !== phonemeId), phonemeId]
    : selectedBulkIds.filter((id) => id !== phonemeId);
}

function addSelectedSounds() {
  const selected = new Set(selectedBulkIds);
  orthographyDraft.mappings = [
    ...orthographyDraft.mappings,
    ...phonemeOptions.filter((phoneme) => selected.has(phoneme.id)).map(mappingFromPhoneme),
  ];
  addFromSoundsOpen = false;
  selectedBulkIds = [];
  editorSection = "characters";
}

function handleSubmit(event: SubmitEvent) {
  event.preventDefault();
  void saveOrthography().then((outcome) => {
    if (outcome === "name") nameInput?.focus();
  });
}

function handleWindowKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && addFromSoundsOpen) {
    event.preventDefault();
    addFromSoundsOpen = false;
  }
}

function soundLabel(phoneme: PhonemeOption) {
  return phoneme.ipa || phoneme.symbol;
}
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<div class="language-toolbar">
  <div class="language-toolbar-title">
    <p class="language-toolbar-eyebrow">Focused projection</p>
    <h2>Writing</h2>
    <p class="language-toolbar-subtitle">
      {selectedLanguage
        ? `${selectedLanguage.name} · writing systems, sound mappings, and samples`
        : "Select a language to document how it is written."}
    </p>
  </div>
  <div class="language-toolbar-actions">
    {#if !orthographyEditorOpen}
      <button type="button" class="language-button" disabled={!selectedLanguage} onclick={addOrthography}
        >Add writing system</button>
    {/if}
  </div>
</div>

{#if paneLoading}
  <p class="language-empty language-loading" role="status">Loading writing systems…</p>
{:else if orthographyEditorOpen}
  <form class="language-editor" onsubmit={handleSubmit}>
    <nav class="writing-tabs" aria-label="Writing system sections">
      <button type="button" class:active={editorSection === "basics"} onclick={() => (editorSection = "basics")}
        >Basics</button>
      <button
        type="button"
        class:active={editorSection === "characters"}
        onclick={() => (editorSection = "characters")}>
        Characters <span>{orthographyDraft.mappings.length}</span>
      </button>
      <button type="button" class:active={editorSection === "samples"} onclick={() => (editorSection = "samples")}>
        Samples <span>{orthographyDraft.samples.length}</span>
      </button>
    </nav>

    {#if editorSection === "basics"}
      <section class="writing-section">
        <div class="writing-section-head">
          <div>
            <h3>Basics</h3>
            <p>Name the writing system and record its direction and purpose.</p>
          </div>
        </div>
        <div class="writing-basics-grid">
          <label class="language-field">
            <span>Name</span>
            <input
              name="name"
              bind:this={nameInput}
              bind:value={orthographyDraft.name}
              required
              placeholder="e.g. Common Script" />
          </label>
          <label class="language-field">
            <span>Writing direction (optional)</span>
            <select name="direction" bind:value={orthographyDraft.direction}>
              <option value="ltr">Left to right</option>
              <option value="rtl">Right to left</option>
              <option value="vertical">Vertical</option>
              <option value="unspecified">Other / unspecified</option>
            </select>
          </label>
        </div>
        <label class="language-field">
          <span>Description / notes (optional)</span>
          <textarea
            name="description"
            rows={5}
            bind:value={orthographyDraft.description}
            placeholder="How and where is this writing system used?"></textarea>
        </label>
      </section>
    {:else if editorSection === "characters"}
      <section class="writing-section">
        <div class="writing-section-head">
          <div>
            <h3>Characters</h3>
            <p>Map Unicode written forms to existing Sounds or explicit ad-hoc IPA values.</p>
          </div>
          <div class="writing-section-actions">
            <button type="button" class="language-button secondary" onclick={() => addMapping()}>Add character</button>
            <button
              type="button"
              class="language-button"
              onclick={openAddFromSounds}
              disabled={phonemeOptions.length === 0}
              title={phonemeOptions.length === 0
                ? "Define Sounds first, or add characters manually."
                : "Create blank mappings from the sound inventory"}>
              Add from Sounds
            </button>
          </div>
        </div>

        {#if phonemeOptions.length === 0}
          <div class="writing-notice">
            <strong>No sounds have been defined for this language yet.</strong>
            <span>You can still add characters manually and use ad-hoc IPA values.</span>
          </div>
        {:else}
          <div class="coverage" aria-label={`${coverage.represented} of ${coverage.total} sounds represented`}>
            <div>
              <strong>{coverage.represented} of {coverage.total} sounds represented</strong>
              <span
                >{coverage.unmapped.length === 0
                  ? "All defined sounds are represented."
                  : `${coverage.unmapped.length} sounds are not represented yet.`}</span>
            </div>
            <div class="coverage-bar" aria-hidden="true">
              <span style={`width: ${coverage.total ? (coverage.represented / coverage.total) * 100 : 0}%`}></span>
            </div>
          </div>
          {#if unmappedPhonemes.length > 0}
            <div class="unmapped">
              <span>Unmapped sounds</span>
              <div>
                {#each unmappedPhonemes as phoneme (phoneme.id)}
                  <button
                    type="button"
                    title={`Create a mapping for ${soundLabel(phoneme)}`}
                    onclick={() => addMappingForPhoneme(phoneme)}>
                    /{soundLabel(phoneme)}/ <small>+</small>
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        {/if}

        {#if orthographyDraft.mappings.length === 0}
          <div class="writing-empty-card">
            <p>No characters yet.</p>
            <span>Add them manually or start from the sounds already defined for this language.</span>
            <div>
              <button type="button" class="language-button secondary" onclick={() => addMapping()}
                >Add character</button>
              {#if phonemeOptions.length > 0}
                <button type="button" class="language-button" onclick={openAddFromSounds}>Add from Sounds</button>
              {/if}
            </div>
          </div>
        {:else}
          <div class="character-table-wrap">
            <div class="character-table" role="list" aria-label="Character-to-sound mappings">
              {#each orthographyDraft.mappings as mapping, index (mapping.id)}
                <article class="character-card" role="listitem">
                  <header class="character-card-head">
                    <div class="character-card-title">
                      <span class="character-index" aria-hidden="true">{index + 1}</span>
                      <div>
                        <strong>{mapping.writtenForm || `Character ${index + 1}`}</strong>
                        <span>{mapping.group === "ungrouped" ? "Ungrouped" : mapping.group}</span>
                      </div>
                    </div>
                    <div
                      class="row-action-buttons"
                      aria-label={`Actions for ${mapping.writtenForm || `character ${index + 1}`}`}>
                      <button
                        type="button"
                        title="Move earlier"
                        onclick={() => moveMapping(index, -1)}
                        disabled={index === 0}
                        aria-label={`Move ${mapping.writtenForm || "mapping"} earlier`}>↑</button>
                      <button
                        type="button"
                        title="Move later"
                        onclick={() => moveMapping(index, 1)}
                        disabled={index === orthographyDraft.mappings.length - 1}
                        aria-label={`Move ${mapping.writtenForm || "mapping"} later`}>↓</button>
                      <button
                        type="button"
                        class="remove"
                        onclick={() => removeMapping(index)}
                        aria-label={`Remove ${mapping.writtenForm || "mapping"}`}>Remove</button>
                    </div>
                  </header>
                  <div class="character-card-grid">
                    <section class="character-identity" aria-label="Character identity">
                      <label class="language-field">
                        <span>Written form</span>
                        <input
                          id={`written-form-${mapping.id}`}
                          bind:value={mapping.writtenForm}
                          required
                          placeholder="e.g. sh" />
                      </label>
                      <label class="mapping-group">
                        <span>Group</span>
                        <select bind:value={mapping.group}>
                          <option value="ungrouped">Ungrouped</option>
                          <option value="vowels">Vowels</option>
                          <option value="consonants">Consonants</option>
                          <option value="other">Other</option>
                        </select>
                      </label>
                    </section>
                    <section class="character-sounds" aria-label="Mapped sounds">
                      <SoundSequenceEditor
                        bind:sounds={mapping.sounds}
                        phonemes={phonemeOptions}
                        label="Mapped sound sequence" />
                    </section>
                    <section class="character-details" aria-label="Character details">
                      <label class="language-field">
                        <span>Romanization (optional)</span>
                        <input
                          id={`romanization-${mapping.id}`}
                          bind:value={mapping.romanization}
                          placeholder="e.g. sh" />
                      </label>
                      <label class="language-field">
                        <span>Notes (optional)</span>
                        <textarea
                          id={`mapping-notes-${mapping.id}`}
                          rows={3}
                          bind:value={mapping.notes}
                          placeholder="Usage, variants, or contextual notes"></textarea>
                      </label>
                    </section>
                  </div>
                </article>
              {/each}
            </div>
          </div>
        {/if}
      </section>
    {:else}
      <section class="writing-section">
        <div class="writing-section-head">
          <div>
            <h3>Samples</h3>
            <p>Save examples that belong specifically to this writing system.</p>
          </div>
          <button type="button" class="language-button" onclick={addSample}>Add sample</button>
        </div>
        {#if orthographyDraft.samples.length === 0}
          <div class="writing-empty-card">
            <p>No samples yet.</p>
            <span>Add a word, phrase, or sentence to show the writing system in use.</span>
            <button type="button" class="language-button secondary" onclick={addSample}>Add sample</button>
          </div>
        {:else}
          <div class="sample-list">
            {#each orthographyDraft.samples as sample, index (sample.id)}
              <article class="sample-card">
                <div class="sample-head">
                  <strong>Sample {index + 1}</strong>
                  <div>
                    <button
                      type="button"
                      onclick={() => moveSample(index, -1)}
                      disabled={index === 0}
                      aria-label={`Move sample ${index + 1} earlier`}>↑</button>
                    <button
                      type="button"
                      onclick={() => moveSample(index, 1)}
                      disabled={index === orthographyDraft.samples.length - 1}
                      aria-label={`Move sample ${index + 1} later`}>↓</button>
                    <button type="button" class="remove" onclick={() => removeSample(index)}>Remove</button>
                  </div>
                </div>
                <label class="language-field">
                  <span>Written text</span>
                  <textarea rows={3} bind:value={sample.writtenText} required placeholder="Text in this writing system"
                  ></textarea>
                </label>
                <IpaInput
                  label="Pronunciation (optional)"
                  bind:value={sample.pronunciation}
                  multiline
                  rows={2}
                  placeholder="e.g. ʃara ven tal" />
                <div class="sample-grid">
                  <label class="language-field">
                    <span>Translation (optional)</span>
                    <textarea rows={2} bind:value={sample.translation}></textarea>
                  </label>
                  <label class="language-field">
                    <span>Notes (optional)</span>
                    <textarea rows={2} bind:value={sample.notes}></textarea>
                  </label>
                </div>
              </article>
            {/each}
          </div>
        {/if}
      </section>
    {/if}

    {#if error}
      <p class="language-status error" role="alert">{error}</p>
    {/if}
    <div class="language-actions">
      <span>
        {#if orthographyEditing}
          <button
            type="button"
            class="language-button secondary language-danger"
            onclick={deleteOrthography}
            disabled={orthographySaving}>Delete writing system</button>
        {/if}
      </span>
      <span>
        <button
          type="button"
          class="language-button secondary"
          onclick={closeOrthographyEditor}
          disabled={orthographySaving}>Cancel</button>
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
    <p class="language-empty" role="status">No writing systems yet.</p>
    <span>Create a writing system to describe how this language is written.</span>
    <button type="button" class="language-button" onclick={addOrthography}>Create writing system</button>
  </div>
{:else}
  <section class="language-pane-section">
    <h3>Writing systems</h3>
    <p>
      {orthographies.length} system{orthographies.length === 1 ? "" : "s"} · each has its own character mappings and samples.
    </p>
    <ul class="writing-system-list">
      {#each orthographies as record (record.id)}
        {@const systemCoverage = orthographyCoverage(
          record.value,
          phonemeOptions.map((phoneme) => phoneme.id),
        )}
        <li>
          <button
            type="button"
            class="language-item"
            aria-label={`Edit writing system ${record.value.name}`}
            onclick={() => openOrthographyEditor(record)}>
            <strong>{record.value.name}</strong>
            <small
              >{record.value.direction === "ltr"
                ? "Left to right"
                : record.value.direction === "rtl"
                  ? "Right to left"
                  : record.value.direction === "vertical"
                    ? "Vertical"
                    : "Direction unspecified"}</small>
            <span
              >{record.value.mappings.length} character{record.value.mappings.length === 1 ? "" : "s"} · {systemCoverage.represented}
              of {systemCoverage.total} sounds · {record.value.samples.length} sample{record.value.samples.length === 1
                ? ""
                : "s"}</span>
          </button>
        </li>
      {/each}
    </ul>
  </section>
{/if}

{#if addFromSoundsOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="bulk-backdrop" role="presentation" onclick={() => (addFromSoundsOpen = false)}>
    <div
      class="bulk-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="bulk-title"
      tabindex="-1"
      onclick={(event) => event.stopPropagation()}>
      <header>
        <div>
          <h3 id="bulk-title">Add from Sounds</h3>
          <p>
            Unmapped sounds are selected by default. Selecting an already represented sound intentionally creates
            another blank spelling.
          </p>
        </div>
        <button type="button" class="language-button secondary" onclick={() => (addFromSoundsOpen = false)}
          >Close</button>
      </header>
      <div class="bulk-list">
        {#each phonemeOptions as phoneme (phoneme.id)}
          <label class:represented={representedIds.has(phoneme.id)}>
            <input
              type="checkbox"
              checked={selectedBulkIds.includes(phoneme.id)}
              onchange={(event) => toggleBulkSound(phoneme.id, event.currentTarget.checked)} />
            <strong>/{soundLabel(phoneme)}/</strong>
            <span>{phoneme.kind}</span>
            <small>{representedIds.has(phoneme.id) ? "Already represented" : "Unmapped"}</small>
          </label>
        {/each}
      </div>
      <footer>
        <span>{selectedBulkIds.length} selected</span>
        <button
          type="button"
          class="language-button"
          onclick={addSelectedSounds}
          disabled={selectedBulkIds.length === 0}>Add selected</button>
      </footer>
    </div>
  </div>
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
.language-toolbar-actions,
.writing-section-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.language-editor {
  display: grid;
  gap: 16px;
  margin-top: 16px;
  min-width: 0;
}
.writing-tabs {
  display: flex;
  gap: 5px;
  padding: 4px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface-muted);
}
.writing-tabs button {
  flex: 1;
  padding: 8px 10px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--ink-soft);
  cursor: pointer;
}
.writing-tabs button.active {
  background: var(--surface);
  color: var(--ink);
  box-shadow: 0 1px 3px rgba(30, 34, 27, 0.08);
}
.writing-tabs span {
  margin-left: 5px;
  color: var(--ink-faint);
  font-size: 10px;
}
.writing-section,
.language-pane-section {
  display: grid;
  gap: 14px;
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface-muted);
}
.writing-section-head {
  display: flex;
  justify-content: space-between;
  align-items: start;
  gap: 12px;
  flex-wrap: wrap;
}
.writing-section-head h3,
.language-pane-section h3 {
  margin: 0;
}
.writing-section-head p,
.language-pane-section > p {
  margin: 3px 0 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.55;
}
.writing-basics-grid,
.sample-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}
.language-field {
  display: grid;
  gap: 6px;
  min-width: 0;
  color: var(--ink-soft);
  font-size: 11px;
}
.language-field input,
.language-field textarea,
.language-field select,
.character-table input,
.character-table textarea,
.mapping-group select {
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
.language-field textarea,
.character-table textarea {
  resize: vertical;
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
  background: var(--surface);
}
.language-button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  filter: none;
}
.language-danger {
  border-color: var(--danger) !important;
  color: var(--danger) !important;
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
.language-loading {
  margin-top: 16px;
}
.language-empty-card,
.writing-empty-card {
  display: grid;
  gap: 10px;
  justify-items: start;
  margin: 18px 0;
  padding: 20px;
  border: 1px dashed var(--line);
  border-radius: 12px;
  background: var(--surface-muted);
}
.language-empty-card span,
.writing-empty-card span {
  color: var(--ink-soft);
  font-size: 12px;
}
.writing-empty-card {
  margin: 0;
}
.writing-empty-card p {
  margin: 0;
  font-weight: 650;
}
.writing-empty-card div {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.writing-notice {
  display: grid;
  gap: 3px;
  padding: 12px;
  border: 1px dashed var(--line);
  border-radius: 10px;
  background: var(--surface);
  font-size: 12px;
}
.writing-notice span {
  color: var(--ink-soft);
}
.coverage {
  display: grid;
  grid-template-columns: minmax(180px, auto) minmax(160px, 1fr);
  gap: 18px;
  align-items: center;
  padding: 11px 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface);
}
.coverage > div:first-child {
  display: grid;
  gap: 2px;
}
.coverage strong {
  font-size: 12px;
}
.coverage span {
  color: var(--ink-soft);
  font-size: 10px;
}
.coverage-bar {
  height: 7px;
  overflow: hidden;
  border-radius: 999px;
  background: var(--line);
}
.coverage-bar span {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--accent);
}
.unmapped {
  display: grid;
  gap: 6px;
}
.unmapped > span {
  color: var(--ink-soft);
  font-size: 11px;
}
.unmapped > div {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}
.unmapped button {
  padding: 5px 8px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--surface);
  color: var(--ink);
  cursor: pointer;
}
.unmapped small {
  color: var(--accent-dark);
}
.character-table-wrap {
  min-width: 0;
}
.character-table {
  display: grid;
  gap: 12px;
  min-width: 0;
}
.character-card {
  min-width: 0;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
  box-shadow: 0 4px 16px rgba(38, 42, 33, 0.045);
  transition:
    border-color 0.15s ease,
    box-shadow 0.15s ease;
}
.character-card:hover,
.character-card:focus-within {
  border-color: var(--theme-warning-border, #d8c3a5);
  box-shadow: 0 8px 24px rgba(38, 42, 33, 0.075);
}
.character-card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px 10px 14px;
  border-bottom: 1px solid var(--line);
  background: color-mix(in srgb, var(--surface-muted) 72%, var(--surface));
}
.character-card-title {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}
.character-card-title > div {
  display: grid;
  gap: 2px;
  min-width: 0;
}
.character-card-title strong {
  overflow: hidden;
  color: var(--ink);
  font-size: 14px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.character-card-title div > span {
  color: var(--ink-faint);
  font-size: 10px;
  letter-spacing: 0.05em;
  text-transform: capitalize;
}
.character-index {
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  flex: 0 0 26px;
  border-radius: 8px;
  background: var(--surface);
  color: var(--accent-dark);
  font-size: 11px;
  font-weight: 700;
  box-shadow: inset 0 0 0 1px var(--line);
}
.character-card-grid {
  display: grid;
  grid-template-columns: minmax(150px, 0.58fr) minmax(340px, 1.5fr) minmax(220px, 0.85fr);
  min-width: 0;
}
.character-card-grid > section {
  display: grid;
  align-content: start;
  gap: 8px;
  min-width: 0;
  padding: 14px;
}
.character-card-grid > section + section {
  border-left: 1px solid var(--line);
}
.character-identity > .language-field input {
  font-size: 15px;
  font-weight: 600;
}
.character-details textarea {
  min-height: 88px;
}
.mapping-group {
  display: grid;
  gap: 5px;
  margin-top: 9px;
}
.mapping-group span {
  color: var(--ink-faint);
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.mapping-group select {
  padding: 9px 34px 9px 10px;
  font-size: 11px;
}
.row-action-buttons {
  display: flex;
  align-items: center;
  gap: 5px;
  flex: 0 0 auto;
}
.row-action-buttons button,
.sample-head button {
  margin: 0;
  min-height: 30px;
  padding: 5px 8px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--surface);
  color: var(--ink-soft);
  cursor: pointer;
}
.row-action-buttons button:hover,
.sample-head button:hover {
  border-color: var(--theme-warning-border, #d8c3a5);
  background: var(--surface-muted);
  color: var(--ink);
}
.row-action-buttons button.remove,
.sample-head button.remove {
  border-color: var(--theme-danger-border, #e2b7af);
  color: var(--danger);
}
.row-action-buttons button.remove:hover,
.sample-head button.remove:hover {
  background: var(--theme-danger-bg, #fff5f2);
}
.row-action-buttons button:disabled,
.sample-head button:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}
.sample-list {
  display: grid;
  gap: 10px;
}
.sample-card {
  display: grid;
  gap: 10px;
  padding: 13px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface);
}
.sample-head {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  align-items: center;
}
.writing-system-list {
  display: grid;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.language-item {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 5px 12px;
  width: 100%;
  padding: 11px 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface);
  color: inherit;
  text-align: left;
  cursor: pointer;
}
.language-item strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.language-item small {
  color: var(--ink-faint);
}
.language-item span {
  grid-column: 1 / -1;
  color: var(--ink-soft);
  font-size: 11px;
}
.bulk-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1100;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(24, 28, 22, 0.54);
  backdrop-filter: blur(3px);
}
.bulk-dialog {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr) auto;
  gap: 14px;
  width: min(680px, 100%);
  max-height: min(720px, calc(100vh - 40px));
  padding: 18px;
  border: 1px solid var(--line);
  border-radius: 16px;
  background: var(--surface);
  box-shadow: 0 24px 70px rgba(20, 22, 18, 0.3);
}
.bulk-dialog header,
.bulk-dialog footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
}
.bulk-dialog h3,
.bulk-dialog p {
  margin: 0;
}
.bulk-dialog p {
  margin-top: 4px;
  color: var(--ink-soft);
  font-size: 11px;
  line-height: 1.5;
}
.bulk-list {
  display: grid;
  gap: 6px;
  min-height: 0;
  overflow: auto;
}
.bulk-list label {
  display: grid;
  grid-template-columns: auto minmax(70px, 0.5fr) minmax(70px, 0.5fr) minmax(110px, 1fr);
  gap: 9px;
  align-items: center;
  padding: 9px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface-muted);
  cursor: pointer;
}
.bulk-list label.represented {
  opacity: 0.68;
}
.bulk-list span,
.bulk-list small,
.bulk-dialog footer span {
  color: var(--ink-soft);
  font-size: 10px;
}
button:focus-visible,
input:focus-visible,
textarea:focus-visible,
select:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
@media (max-width: 1180px) {
  .character-card-grid {
    grid-template-columns: minmax(180px, 0.62fr) minmax(360px, 1.38fr);
  }
  .character-details {
    grid-column: 1 / -1;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    border-top: 1px solid var(--line);
    border-left: 0 !important;
  }
}
@media (max-width: 720px) {
  .writing-basics-grid,
  .sample-grid,
  .coverage {
    grid-template-columns: 1fr;
  }
  .writing-tabs {
    overflow-x: auto;
  }
  .writing-tabs button {
    min-width: 110px;
  }
  .character-card-head {
    align-items: flex-start;
  }
  .character-card-grid,
  .character-details {
    grid-template-columns: 1fr;
  }
  .character-card-grid > section + section {
    border-top: 1px solid var(--line);
    border-left: 0;
  }
  .character-details {
    grid-column: auto;
  }
  .bulk-backdrop {
    padding: 0;
  }
  .bulk-dialog {
    width: 100%;
    height: 100%;
    max-height: none;
    border-radius: 0;
  }
  .bulk-list label {
    grid-template-columns: auto 1fr auto;
  }
  .bulk-list label span {
    display: none;
  }
}
</style>
