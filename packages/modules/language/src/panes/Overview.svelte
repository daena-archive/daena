<script lang="ts">
import type { EntityRecord, EntitySummary } from "../../../../module-api/src/index";
import type { FieldDefinition } from "../../../../plugin-sdk/src/generated";

let {
  selectedLanguage,
  error,
  overviewEntity,
  overviewLoading,
  overviewName,
  overviewFields,
  overviewDocument,
  overviewDirty,
  overviewSaving,
  overviewSavingAutomatically,
  overviewDeleting,
  overviewError,
  overviewFieldDefinitions,
  onOverviewNameInput,
  onOverviewFieldInput,
  onOverviewDocumentInput,
  archiveOverviewLanguage,
}: {
  selectedLanguage: EntitySummary | null;
  error: string;
  overviewEntity: EntityRecord | null;
  overviewLoading: boolean;
  overviewName: string;
  overviewFields: Record<string, unknown>;
  overviewDocument: string;
  overviewDirty: boolean;
  overviewSaving: boolean;
  overviewSavingAutomatically: boolean;
  overviewDeleting: boolean;
  overviewError: string;
  overviewFieldDefinitions: () => FieldDefinition[];
  onOverviewNameInput: (value: string) => void;
  onOverviewFieldInput: (definition: FieldDefinition, raw: string) => void;
  onOverviewDocumentInput: (value: string) => void;
  archiveOverviewLanguage: () => void;
} = $props();

function fieldValue(definition: FieldDefinition) {
  const value = overviewFields[definition.key];
  return Array.isArray(value) ? value.join("\n") : String(value ?? "");
}

let status = $derived.by(() => {
  if (!selectedLanguage) return { state: null, text: "Select a language" };
  if (overviewLoading || !overviewEntity) return { state: null, text: "Loading language details…" };
  const hasError = Boolean(overviewError || error);
  return {
    state: hasError ? "error" : overviewDeleting || overviewSaving ? "saving" : overviewDirty ? "dirty" : "saved",
    text: hasError
      ? "Changes need attention"
      : overviewDeleting
        ? "Archiving language…"
        : overviewSaving
          ? overviewSavingAutomatically
            ? "Saving automatically…"
            : "Saving language details…"
          : overviewDirty
            ? "Changes save automatically"
            : "All changes saved",
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
  <span class="language-overview-status" role="status" aria-live="polite" data-state={status.state}>{status.text}</span>
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
      <div>
        <h3>Language identity</h3>
        <p>Keep the name and short identity details close while you build the language.</p>
        <label class="language-field">
          <span>Language name</span>
          <input
            name="overviewName"
            autocomplete="off"
            value={overviewName}
            oninput={(event) => onOverviewNameInput(event.currentTarget.value)} />
        </label>
      </div>
      <div class="language-overview-identity-meta">
        <span>Workspace status</span>
        <strong>{overviewDirty ? "Draft changes" : "Ready to build"}</strong>
      </div>
    </section>

    <section class="language-overview-section">
      <h3>Properties</h3>
      <p>A few useful anchors for how this language belongs in the world.</p>
      <div class="language-overview-fields">
        {#each overviewFieldDefinitions() as definition (definition.key)}
          {#if definition.multiple}
            <label class="language-field">
              <span>{definition.label}</span>
              <textarea
                name={`overview-${definition.key}`}
                rows={2}
                value={fieldValue(definition)}
                oninput={(event) => onOverviewFieldInput(definition, event.currentTarget.value)} />
            </label>
          {:else}
            <label class="language-field">
              <span>{definition.label}</span>
              <input
                name={`overview-${definition.key}`}
                value={fieldValue(definition)}
                oninput={(event) => onOverviewFieldInput(definition, event.currentTarget.value)} />
            </label>
          {/if}
        {/each}
      </div>
    </section>

    <section class="language-overview-section">
      <h3>Canonical notes</h3>
      <p>Describe what makes this language itself. These notes stay with the language as the projection grows.</p>
      <textarea
        class="language-overview-document"
        name="overviewDocument"
        rows={12}
        value={overviewDocument}
        oninput={(event) => onOverviewDocumentInput(event.currentTarget.value)} />
    </section>

    {#if overviewError || error}
      <p class="language-status error" role="alert">{overviewError || error}</p>
    {/if}
    <div class="language-overview-actions">
      <span class="language-overview-danger">
        <button
          type="button"
          class="language-button secondary language-danger"
          disabled={overviewSaving || overviewDeleting}
          onclick={archiveOverviewLanguage}>{overviewDeleting ? "Archiving…" : "Archive language"}</button>
      </span>
    </div>
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
  border-color: #c6d8cb;
  background: #eef3ef;
  color: var(--accent-dark);
}
.language-overview-status[data-state="saving"] {
  border-color: #d8c3a5;
  color: var(--accent-dark);
}
.language-overview-status[data-state="saving"]::before {
  animation: language-pulse 1.2s ease-in-out infinite;
}
.language-overview-status[data-state="error"] {
  border-color: #e2b7af;
  background: #fff5f2;
  color: #a14f42;
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
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(180px, 0.55fr);
  gap: 16px;
  padding: 18px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface-muted);
}
.language-overview-identity h3 {
  font-size: 20px;
}
.language-overview-identity p {
  margin: 5px 0 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.55;
}
.language-overview-identity-meta {
  display: grid;
  align-content: center;
  justify-items: end;
  gap: 4px;
  color: var(--ink-soft);
  font-size: 12px;
  text-align: right;
}
.language-overview-identity-meta strong {
  color: var(--accent-dark);
  font-size: 13px;
}
.language-overview-section {
  display: grid;
  gap: 12px;
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.language-overview-section h3 {
  font-size: 17px;
}
.language-overview-section > p {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.55;
}
.language-overview-fields {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}
.language-overview-document {
  min-height: 16rem;
  resize: vertical;
  line-height: 1.6;
}
.language-overview-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 12px;
  flex-wrap: wrap;
  margin: auto -20px -24px;
  padding: 12px 20px 24px;
  border-top: 1px solid var(--line);
  background: var(--surface);
  box-shadow: 0 -8px 16px -16px rgba(38, 42, 33, 0.4);
}
.language-field {
  display: grid;
  gap: 6px;
  min-width: 0;
  color: var(--ink-soft);
  font-size: 11px;
  letter-spacing: 0.01em;
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
  border-color: #a14f42 !important;
  color: #a14f42 !important;
  background: transparent;
}
@media (max-width: 760px) {
  .language-overview-identity,
  .language-overview-fields {
    grid-template-columns: 1fr;
  }
  .language-overview-identity-meta {
    justify-items: start;
    text-align: left;
  }
}
</style>
