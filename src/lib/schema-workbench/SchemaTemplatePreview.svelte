<script lang="ts">
import type { EntityTemplate, FieldDefinition } from "$lib/project/client";
import SchemaFieldInput from "./SchemaFieldInput.svelte";
import { defaultFieldValue } from "./model";

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

<section class="schema-template-preview" aria-label="Preview create form" data-template-id={template.id}>
  <header class="preview-heading">
    <span class="kicker">Preview create form</span>
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
  gap: 0.85rem;
  border: 1px solid var(--border, #d5dbd6);
  border-radius: 12px;
  padding: 0.9rem 1rem;
  background: color-mix(in srgb, var(--surface, #fff) 88%, var(--border, #d5dbd6));
}

.preview-heading {
  display: grid;
  gap: 0.2rem;
}

.kicker {
  font-size: 0.68rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-muted, #5c645f);
}

.preview-heading strong {
  font-size: 0.98rem;
}

.preview-heading p,
.preview-empty {
  margin: 0;
  font-size: 0.82rem;
  color: var(--text-muted, #5c645f);
}

.preview-form {
  display: grid;
  gap: 0.75rem;
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
  border: 1px solid var(--border, #d5dbd6);
  border-radius: 8px;
  padding: 0.45rem 0.65rem;
  background: var(--surface, #fff);
  color: inherit;
  font: inherit;
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

.preview-more summary small {
  color: var(--text-muted, #5c645f);
}

.preview-more-body {
  display: grid;
  gap: 0.75rem;
  margin-top: 0.65rem;
}
</style>
