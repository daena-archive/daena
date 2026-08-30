<script lang="ts">
import type { EntityRecord, EntitySummary, ModuleContext, ModuleManifest } from "../../../../module-api/src/index";
import type { FieldDefinition } from "../../../../plugin-sdk/src/generated";
import RichTextEditor from "../../../../../src/lib/editor/RichTextEditor.svelte";
import { confirmDialog } from "../../../../../src/lib/dialogs.svelte";
import { archiveConfirmOptions, archivePendingLabel } from "../../../../../src/lib/ui-ux/archive.ts";
import { MUTATION_STATUS } from "../../../../../src/lib/ui-ux/vocabulary.ts";
import { confirm } from "../confirm.svelte";
import manifestJson from "../../manifest.json";

const manifest = manifestJson as unknown as ModuleManifest;

let {
  context,
  selectedLanguage,
  active,
  registerLeaveGuard,
  onLanguageChanged,
  onLanguageArchived,
  openPane,
}: {
  context: ModuleContext;
  selectedLanguage: EntitySummary | null;
  active: boolean;
  registerLeaveGuard: (guard: (() => Promise<boolean> | boolean) | null) => void;
  onLanguageChanged: (language: EntitySummary) => void;
  onLanguageArchived: (languageId: string) => void;
  openPane: (pane: "lexicon" | "sounds" | "writing" | "grammar" | "forms" | "samples") => void;
} = $props();

const overviewFieldDefinitions = manifest.schemas
  .flatMap((schema) => schema.fields)
  .filter((field) => !field.relationshipType);

const FIELD_HINTS: Record<string, string> = {
  nativeName: "Name used by its speakers",
  aliases: "Alternative names, separated by commas",
  status: "e.g. living, ceremonial, extinct",
  family: "Parent family or related language group",
  writingSystem: "Primary script or orthography",
};

const STUDIO_AREAS = [
  { pane: "sounds", title: "Shape the sounds", detail: "Build the phoneme inventory and phonotactics." },
  { pane: "writing", title: "Design the writing", detail: "Map sounds to graphemes and test orthographies." },
  { pane: "lexicon", title: "Coin the words", detail: "Grow vocabulary, senses, pronunciations, and usage." },
  { pane: "grammar", title: "Define the grammar", detail: "Document systems, strategies, and examples." },
  { pane: "forms", title: "Model morphology", detail: "Create paradigms, rules, and generated forms." },
  { pane: "samples", title: "Test it in context", detail: "Analyze sentences and paragraphs interlinearly." },
] as const;

let cancelled = $state(false);
let overviewEntity = $state<EntityRecord | null>(null);
let overviewName = $state("");
let overviewFields: Record<string, unknown> = $state({});
let overviewSavedFields: Record<string, unknown> = $state({});
let overviewFieldRevisions: Record<string, string> = $state({});
let overviewDocument = $state("");
let overviewSavedDocument = $state("");
let overviewDocumentRevision = $state("");
let overviewLoading = $state(false);
let overviewSaving = $state(false);
let overviewSavingAutomatically = $state(false);
let overviewDeleting = $state(false);
let overviewDirty = $state(false);
let overviewError = $state("");
let overviewRequest = $state(0);
let overviewAutosaveTimer = $state<number | null>(null);
let overviewAutosaveQueued = $state(false);

let lastLoadedLanguage: string | null = null;

$effect(() => {
  const languageId = selectedLanguage?.id ?? null;
  void languageId;
  if (!active) return;
  if (languageId === lastLoadedLanguage) return;
  lastLoadedLanguage = languageId;
  void loadOverview();
});

$effect(() => {
  if (!active) return;
  registerLeaveGuard(() => tryLeaveOverview((message) => confirm("Unsaved changes", message)));
  return () => {
    registerLeaveGuard(null);
  };
});

$effect(() => {
  return () => {
    cancelled = true;
    if (overviewAutosaveTimer !== null) window.clearTimeout(overviewAutosaveTimer);
    overviewAutosaveTimer = null;
  };
});

function clearOverviewAutosave() {
  if (overviewAutosaveTimer !== null) window.clearTimeout(overviewAutosaveTimer);
  overviewAutosaveTimer = null;
  overviewAutosaveQueued = false;
}

async function tryLeaveOverview(confirmLeave: (message: string) => Promise<boolean> | boolean) {
  if (!overviewDirty) {
    clearOverviewAutosave();
    return true;
  }
  if (overviewSaving) return false;
  const saved = await saveOverview(false);
  if (saved) return true;
  const allowed = await confirmLeave("Language details could not be saved. Leave and discard these changes?");
  if (allowed) {
    clearOverviewAutosave();
    overviewDirty = false;
    overviewError = "";
  }
  return allowed;
}

function syncOverviewDirty() {
  const nameDirty = overviewName.trim() !== overviewEntity?.name;
  const fieldsDirty = overviewFieldDefinitions.some(
    (definition) =>
      JSON.stringify(overviewFields[definition.key] ?? "") !==
      JSON.stringify(overviewSavedFields[definition.key] ?? ""),
  );
  overviewDirty = nameDirty || fieldsDirty || overviewDocument !== overviewSavedDocument;
}

function scheduleOverviewAutosave() {
  if (!overviewDirty || !selectedLanguage || !overviewEntity || overviewDeleting) {
    if (!overviewDirty) clearOverviewAutosave();
    return;
  }
  if (overviewSaving) {
    overviewAutosaveQueued = true;
    return;
  }
  if (overviewAutosaveTimer !== null) window.clearTimeout(overviewAutosaveTimer);
  overviewAutosaveTimer = window.setTimeout(() => {
    overviewAutosaveTimer = null;
    void saveOverview(true);
  }, 800);
}

function onOverviewNameInput(value: string) {
  overviewName = value;
  syncOverviewDirty();
  scheduleOverviewAutosave();
}

function onOverviewFieldInput(definition: FieldDefinition, raw: string) {
  overviewFields = {
    ...overviewFields,
    [definition.key]: definition.multiple
      ? raw
          .split(/[,\n]/)
          .map((item) => item.trim())
          .filter(Boolean)
      : raw,
  };
  syncOverviewDirty();
  scheduleOverviewAutosave();
}

function onOverviewDocumentInput(value: string) {
  overviewDocument = value;
  syncOverviewDirty();
  scheduleOverviewAutosave();
}

async function loadOverview() {
  clearOverviewAutosave();
  if (!selectedLanguage) {
    overviewEntity = null;
    overviewLoading = false;
    return;
  }
  const token = ++overviewRequest;
  overviewLoading = true;
  overviewError = "";
  try {
    const [entity, fieldRecords] = await Promise.all([
      context.entities.get(selectedLanguage.id),
      context.fields.listRecords(selectedLanguage.id),
    ]);
    if (cancelled || token !== overviewRequest) return;
    if (!entity) throw new Error("This language is no longer available.");
    const values = Object.fromEntries(fieldRecords.map((record) => [record.key, record.value]));
    for (const definition of overviewFieldDefinitions) {
      if (!(definition.key in values)) values[definition.key] = "";
    }
    const document = entity.documents.find((item) => item.format === "markdown") ?? entity.documents[0];
    overviewEntity = entity;
    overviewName = entity.name;
    overviewFields = values;
    overviewSavedFields = { ...values };
    overviewFieldRevisions = Object.fromEntries(fieldRecords.map((record) => [record.key, record.revision]));
    overviewDocument = document?.body ?? "";
    overviewSavedDocument = overviewDocument;
    overviewDocumentRevision = document?.revision ?? "";
    overviewDirty = false;
    overviewLoading = false;
    overviewError = "";
  } catch (cause) {
    if (cancelled || token !== overviewRequest) return;
    overviewLoading = false;
    overviewError = cause instanceof Error ? cause.message : String(cause);
  }
}

async function saveOverview(automatic = false): Promise<boolean> {
  if (!selectedLanguage || !overviewEntity || overviewSaving || overviewDeleting) return false;
  clearOverviewAutosave();
  const name = overviewName.trim();
  if (!name) {
    overviewError = "Language name is required.";
    return false;
  }
  const entityId = overviewEntity.id;
  const draftFields = { ...overviewFields };
  const draftDocument = overviewDocument;
  overviewSaving = true;
  overviewSavingAutomatically = automatic;
  overviewError = "";
  try {
    if (name !== overviewEntity.name) {
      overviewEntity = await context.entities.update(
        overviewEntity.id,
        { name },
        { expectedRevision: overviewEntity.revision, requestId: crypto.randomUUID() },
      );
    }
    for (const definition of overviewFieldDefinitions) {
      const value = draftFields[definition.key] ?? "";
      if (JSON.stringify(value) === JSON.stringify(overviewSavedFields[definition.key] ?? "")) continue;
      await context.fields.set(overviewEntity.id, definition.key, value, {
        expectedRevision: overviewFieldRevisions[definition.key] ?? "",
        requestId: crypto.randomUUID(),
      });
    }
    if (draftDocument !== overviewSavedDocument) {
      await context.documents.save(
        { entityId: overviewEntity.id, body: draftDocument, format: "markdown" },
        { expectedRevision: overviewDocumentRevision, requestId: crypto.randomUUID() },
      );
    }
    const currentDraftChanged =
      overviewName.trim() !== name ||
      overviewFieldDefinitions.some(
        (definition) =>
          JSON.stringify(overviewFields[definition.key] ?? "") !== JSON.stringify(draftFields[definition.key] ?? ""),
      ) ||
      overviewDocument !== draftDocument;
    const currentDraftName = overviewName;
    const currentDraftFields = { ...overviewFields };
    const currentDraftDocument = overviewDocument;
    const needsFollowUpSave = currentDraftChanged || overviewAutosaveQueued;
    overviewSaving = false;
    overviewSavingAutomatically = false;
    await loadOverview();
    if (selectedLanguage?.id !== entityId) return false;
    if (needsFollowUpSave) {
      overviewName = currentDraftName;
      overviewFields = currentDraftFields;
      overviewDocument = currentDraftDocument;
      overviewDirty = true;
      scheduleOverviewAutosave();
      return false;
    }
    if (overviewEntity) {
      onLanguageChanged({ ...selectedLanguage, name, revision: overviewEntity.revision });
    }
    return true;
  } catch (cause) {
    const currentDraftName = overviewName;
    const currentDraftFields = { ...overviewFields };
    const currentDraftDocument = overviewDocument;
    const saveError = cause instanceof Error ? cause.message : String(cause);
    overviewSaving = false;
    overviewSavingAutomatically = false;
    await loadOverview();
    if (selectedLanguage?.id === entityId && overviewEntity?.id === entityId) {
      overviewName = currentDraftName;
      overviewFields = currentDraftFields;
      overviewDocument = currentDraftDocument;
      syncOverviewDirty();
    }
    overviewError = saveError;
    if (overviewAutosaveQueued) scheduleOverviewAutosave();
    return false;
  }
}

async function archiveOverviewLanguage() {
  if (!selectedLanguage || !overviewEntity || overviewDeleting) return;
  const name = selectedLanguage.name;
  const dirtyNote = overviewDirty ? "Unsaved language details will be discarded." : undefined;
  if (!(await confirmDialog(archiveConfirmOptions(name, { dirtyNote })))) return;
  clearOverviewAutosave();
  overviewDeleting = true;
  overviewError = "";
  try {
    await context.entities.delete(overviewEntity.id, {
      expectedRevision: overviewEntity.revision,
      requestId: crypto.randomUUID(),
    });
    overviewDeleting = false;
    overviewEntity = null;
    overviewDirty = false;
    overviewAutosaveQueued = false;
    onLanguageArchived(selectedLanguage.id);
  } catch (cause) {
    overviewDeleting = false;
    overviewError = cause instanceof Error ? cause.message : String(cause);
  }
}

function fieldValue(definition: FieldDefinition) {
  const value = overviewFields[definition.key];
  return Array.isArray(value) ? value.join("\n") : String(value ?? "");
}

let status = $derived.by(() => {
  if (!selectedLanguage) return { state: null, text: "Select a language" };
  if (overviewLoading || !overviewEntity) return { state: null, text: "Loading language details…" };
  const hasError = Boolean(overviewError);
  return {
    state: hasError ? "error" : overviewDeleting || overviewSaving ? "saving" : overviewDirty ? "dirty" : "saved",
    text: hasError
      ? "Changes need attention"
      : overviewDeleting
        ? MUTATION_STATUS.working
        : overviewSaving
          ? MUTATION_STATUS.saving
          : overviewDirty
            ? "Changes save automatically"
            : MUTATION_STATUS.saved,
  };
});
</script>

<div class="language-toolbar">
  <div class="language-toolbar-title">
    <p class="language-toolbar-eyebrow">Unified language workspace</p>
    <h2>Overview</h2>
    <p class="language-toolbar-subtitle">
      {selectedLanguage
        ? `${selectedLanguage.name} · identity, properties, and canonical notes`
        : "Select a language to begin."}
    </p>
  </div>
  <div class="language-toolbar-actions">
    {#if selectedLanguage && (overviewDirty || overviewError)}
      <button
        type="button"
        class="language-button secondary"
        disabled={overviewSaving || overviewDeleting}
        onclick={() => void saveOverview(false)}>{overviewError ? "Try saving again" : "Save now"}</button>
    {/if}
    <span class="language-overview-status" role="status" aria-live="polite" data-state={status.state}
      >{status.text}</span>
  </div>
</div>
{#if !selectedLanguage}
  <div class="language-empty-card">
    <p class="language-empty" role="status">Select a language, or create one from the list.</p>
  </div>
{:else if overviewLoading || !overviewEntity}
  <p class="language-empty language-loading" role="status" aria-live="polite">Loading language details…</p>
{:else}
  <form class="language-overview" onsubmit={(event) => event.preventDefault()}>
    <section class="language-overview-identity">
      <div class="language-overview-identity-header">
        <div>
          <h3>Language identity</h3>
          <p>Keep the name and short identity details close while you build the language.</p>
        </div>
        <button
          type="button"
          class="language-button secondary language-danger language-overview-archive-btn"
          disabled={overviewSaving || overviewDeleting}
          onclick={archiveOverviewLanguage}>{archivePendingLabel(overviewDeleting)}</button>
      </div>
      <label class="language-field">
        <span>Language name</span>
        <input
          name="overviewName"
          autocomplete="off"
          value={overviewName}
          oninput={(event) => onOverviewNameInput(event.currentTarget.value)} />
      </label>
    </section>

    <section class="language-overview-section">
      <div class="language-overview-section-header">
        <div>
          <h3>Properties</h3>
          <p>A few useful anchors for how this language belongs in the world.</p>
        </div>
      </div>
      <div class="language-overview-fields">
        {#each overviewFieldDefinitions as definition (definition.key)}
          <label class="language-field">
            <span>{definition.label}</span>
            {#if definition.multiple}
              <textarea
                name={`overview-${definition.key}`}
                rows={2}
                placeholder={FIELD_HINTS[definition.key] ?? ""}
                value={fieldValue(definition)}
                oninput={(event) => onOverviewFieldInput(definition, event.currentTarget.value)}></textarea>
            {:else}
              <input
                name={`overview-${definition.key}`}
                placeholder={FIELD_HINTS[definition.key] ?? ""}
                value={fieldValue(definition)}
                oninput={(event) => onOverviewFieldInput(definition, event.currentTarget.value)} />
            {/if}
          </label>
        {/each}
      </div>
    </section>

    <section class="language-overview-section">
      <div class="language-overview-section-header">
        <div>
          <h3>Crafting workbench</h3>
          <p>Move between the parts of the language in any order. Each area stays scoped to this language.</p>
        </div>
      </div>
      <div class="language-studio-areas">
        {#each STUDIO_AREAS as area (area.pane)}
          <button type="button" class="language-studio-area" onclick={() => openPane(area.pane)}>
            <strong>{area.title}</strong>
            <span>{area.detail}</span>
          </button>
        {/each}
      </div>
    </section>

    <section class="language-overview-section">
      <div class="language-overview-section-header">
        <div>
          <h3>Canonical notes</h3>
          <p>Describe what makes this language itself. These notes stay with the language as the projection grows.</p>
        </div>
      </div>
      <div class="language-overview-editor">
        <RichTextEditor value={overviewDocument} onChange={onOverviewDocumentInput} />
      </div>
    </section>

    {#if overviewError}
      <p class="language-status error" role="alert">{overviewError}</p>
    {/if}
  </form>
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
.language-overview-status {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 32px;
  padding: 8px 12px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  font-size: 12px;
  font-weight: 600;
  white-space: nowrap;
}
.language-overview-status::before {
  content: "";
  width: 7px;
  height: 7px;
  flex: 0 0 7px;
  border-radius: 50%;
  background: currentColor;
}
.language-overview-status[data-state="saved"] {
  border-color: var(--theme-success-border, #c6d8cb);
  background: var(--theme-success-bg, #eef3ef);
  color: var(--accent-dark);
}
.language-overview-status[data-state="saving"] {
  border-color: var(--theme-warning-border, #d8c3a5);
  color: var(--accent-dark);
}
.language-overview-status[data-state="saving"]::before {
  animation: language-pulse 1.2s ease-in-out infinite;
}
.language-overview-status[data-state="error"] {
  border-color: var(--theme-danger-border, #e2b7af);
  background: var(--theme-danger-bg, #fff5f2);
  color: var(--danger);
}
.language-overview {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 16px;
  margin-top: 18px;
  min-width: 0;
  min-height: 0;
}
.language-overview-identity {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 20px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface-muted);
}
.language-overview-identity-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
.language-overview-identity-header h3 {
  font-size: 20px;
  margin: 0;
}
.language-overview-identity-header p {
  margin: 6px 0 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.55;
}
.language-overview-archive-btn {
  flex-shrink: 0;
}
.language-overview-section {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 20px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.language-overview-section-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
.language-overview-section-header h3 {
  font-size: 17px;
  margin: 0;
}
.language-overview-section-header p {
  margin: 6px 0 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.55;
}
.language-overview-fields {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}
.language-studio-areas {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}
.language-studio-area {
  display: grid;
  gap: 5px;
  min-width: 0;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface-muted);
  color: var(--ink);
  text-align: left;
  cursor: pointer;
}
.language-studio-area:hover {
  border-color: var(--theme-warning-border, #d8c3a5);
  background: var(--surface);
}
.language-studio-area:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
.language-studio-area strong {
  color: var(--accent-dark);
  font-size: 13px;
}
.language-studio-area span {
  color: var(--ink-soft);
  font-size: 11px;
  line-height: 1.5;
}
.language-overview-editor {
  min-height: 16rem;
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
.language-button:focus-visible {
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
@keyframes language-pulse {
  50% {
    opacity: 0.35;
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
.language-danger {
  border-color: var(--danger) !important;
  color: var(--danger) !important;
  background: transparent;
}
@media (max-width: 760px) {
  .language-overview-identity-header {
    flex-direction: column;
    gap: 12px;
  }
  .language-overview-archive-btn {
    width: 100%;
  }
  .language-overview-fields {
    grid-template-columns: 1fr;
  }
  .language-studio-areas {
    grid-template-columns: 1fr;
  }
}
</style>
