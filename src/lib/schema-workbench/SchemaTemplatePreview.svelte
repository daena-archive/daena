<script lang="ts">
import type { EntityTemplate, FieldDefinition } from "$lib/project/client";
import SchemaFieldInput from "./SchemaFieldInput.svelte";
import { defaultFieldValue } from "./model";
import { Eye } from "@lucide/svelte";

export type PreviewField = {
  field: FieldDefinition;
  required: boolean;
};

let {
  template,
  fields = [],
  values = $bindable({}),
  name = $bindable(""),
  documentBody = $bindable(""),
  showDocument = false,
  readOnly = true,
  idPrefix = "template-preview",
}: {
  template: EntityTemplate;
  fields?: PreviewField[];
  values?: Record<string, unknown>;
  name?: string;
  documentBody?: string;
  showDocument?: boolean;
  readOnly?: boolean;
  idPrefix?: string;
} = $props();

const requiredFields = $derived(fields.filter((item) => item.required));
const optionalFields = $derived(fields.filter((item) => !item.required));

function ensureDefaults() {
  const next = { ...values };
  let changed = false;
  for (const item of fields) {
    if (!(item.field.key in next)) {
      next[item.field.key] = defaultFieldValue(item.field);
      changed = true;
    }
  }
  if (changed) values = next;
}

$effect(() => {
  fields;
  ensureDefaults();
});
</script>

<section class="schema-template-preview is-preview" aria-label="Preview create form" data-template-id={template.id}>
  <header class="preview-heading">
    <span class="preview-kicker"
      ><Eye size={12} strokeWidth={1.8} aria-hidden="true" /> Preview — Create form
      <span class="preview-badge">Read-only</span></span>
    <strong>{template.name}</strong>
    {#if template.description}
      <p>{template.description}</p>
    {:else}
      <p>Read-only preview of the create dialog for this template.</p>
    {/if}
  </header>

  <div class="preview-form" aria-disabled={readOnly}>
    <label class="preview-name" for={`${idPrefix}-name`}>
      <span
        >Name {#if true}<b>*</b>{/if}</span>
      <input
        id={`${idPrefix}-name`}
        type="text"
        readonly={readOnly}
        disabled={readOnly}
        bind:value={name}
        placeholder={`e.g. ${template.name}`} />
    </label>

    {#each requiredFields as item}
      <SchemaFieldInput
        field={item.field}
        required={true}
        {readOnly}
        disabled={readOnly}
        {idPrefix}
        value={values[item.field.key]}
        onChange={(next) => {
          values = { ...values, [item.field.key]: next };
        }} />
    {/each}

    {#if optionalFields.length > 0 || showDocument}
      <details class="preview-more" open>
        <summary>
          <strong>More details</strong>
          <small>
            {optionalFields.length} optional {optionalFields.length === 1 ? "field" : "fields"}
            {#if showDocument}
              {optionalFields.length > 0 ? " and an opening note" : "Opening note"}
            {/if}
          </small>
        </summary>
        <div class="preview-more-body">
          {#each optionalFields as item}
            <SchemaFieldInput
              field={item.field}
              required={false}
              {readOnly}
              disabled={readOnly}
              {idPrefix}
              value={values[item.field.key]}
              onChange={(next) => {
                values = { ...values, [item.field.key]: next };
              }} />
          {/each}
          {#if showDocument}
            <label class="preview-name" for={`${idPrefix}-document`}>
              <span>Opening note</span>
              <textarea
                id={`${idPrefix}-document`}
                rows="4"
                readonly={readOnly}
                disabled={readOnly}
                bind:value={documentBody}
                placeholder="Add a first note (optional)"></textarea>
            </label>
          {/if}
        </div>
      </details>
    {/if}

    {#if fields.length === 0 && !showDocument}
      <p class="preview-empty">No fields included yet. Choose fields above to see them here.</p>
    {/if}
  </div>
</section>

<style>
.schema-template-preview {
  display: grid;
  gap: 0.9rem;
  border: 1px dashed var(--theme-warning-border, #d3c0a9);
  border-radius: 11px;
  padding: 0.85rem 0.95rem;
  background: var(--surface-quiet, var(--theme-warning-bg, #fdf8ef));
  box-shadow: inset 0 1px 6px rgba(48, 44, 38, 0.04);
}

.preview-heading {
  display: grid;
  gap: 0.22rem;
}

.preview-kicker {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font:
    700 9px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  letter-spacing: 0.09em;
  text-transform: uppercase;
  color: var(--ink-faint);
}

.preview-badge {
  margin-left: 2px;
  padding: 2px 6px;
  border: 1px solid var(--line-soft);
  border-radius: 999px;
  background: var(--surface-warm);
  color: var(--ink-muted);
  font:
    700 8px Inter,
    sans-serif;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.preview-heading strong {
  font-size: 0.98rem;
  color: var(--ink);
}

.preview-heading p,
.preview-empty {
  margin: 0;
  font-size: 0.82rem;
  color: var(--text-muted, #5c645f);
}

.preview-form {
  display: grid;
  gap: 0.65rem;
}

/* Muted preview inputs — convey read-only, not active form */
.preview-form :global(.schema-field-input) {
  opacity: 1;
}

.preview-name {
  display: grid;
  gap: 0.35rem;
}

.preview-name span {
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--text-muted, #5c645f);
}

.preview-name b {
  color: var(--danger, #a33);
}

.preview-name input,
.preview-name textarea {
  width: 100%;
  min-height: var(--control-min-height, 34px);
  border: 1px solid var(--line-soft, #e8e0d3);
  border-radius: 8px;
  padding: 0.45rem 0.65rem;
  background: var(--surface-subtle, #f7f4ef);
  color: var(--ink-muted);
  font: inherit;
}

/* Override SchemaFieldInput controls when inside preview */
.schema-template-preview :global(.schema-field-input input),
.schema-template-preview :global(.schema-field-input textarea),
.schema-template-preview :global(.schema-field-input select) {
  background: var(--surface-subtle, #f7f4ef) !important;
  border-color: var(--line-soft, #e8e0d3) !important;
  color: var(--ink-muted) !important;
}

.schema-template-preview :global(.schema-field-input .schema-field-label span) {
  color: var(--ink-faint) !important;
}

.preview-more {
  border-top: 1px dashed var(--line-soft);
  padding-top: 0.65rem;
  margin-top: 0.15rem;
}

.preview-more summary {
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
  cursor: pointer;
  list-style: none;
}

.preview-more summary::-webkit-details-marker {
  display: none;
}

.preview-more summary strong {
  font-size: 0.82rem;
  color: var(--ink-soft);
}

.preview-more summary small {
  color: var(--text-muted, #5c645f);
  font-size: 0.76rem;
}

.preview-more-body {
  display: grid;
  gap: 0.65rem;
  margin-top: 0.65rem;
}

.preview-empty {
  padding: 10px 11px;
  border: 1px dashed var(--line-soft);
  border-radius: 8px;
  background: transparent;
  text-align: center;
}
</style>
