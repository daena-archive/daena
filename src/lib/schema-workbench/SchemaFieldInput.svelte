<script lang="ts">
import type { FieldDefinition } from "$lib/project/client";
import DateEditor from "$lib/date/DateEditor.svelte";
import RelationshipPicker from "$lib/RelationshipPicker.svelte";
import { GREGORIAN_CALENDAR_ID } from "$lib/date";
import type { AsyncEntityResolveFn, AsyncEntitySearchFn, AsyncEntitySearchPage } from "$lib/ui-ux/asyncEntityQuery";

export type SchemaFieldValue = unknown;

let {
  field,
  value = $bindable(),
  required = false,
  disabled = false,
  readOnly = false,
  idPrefix = "schema-field",
  class: className = "",
  search,
  resolveSelected,
  calendars = [],
  calendar = null,
  selectedCalendarId = GREGORIAN_CALENDAR_ID,
  onChange,
  onClearDate,
  onSelectCalendar,
}: {
  field: FieldDefinition;
  value?: SchemaFieldValue;
  required?: boolean;
  disabled?: boolean;
  readOnly?: boolean;
  idPrefix?: string;
  class?: string;
  search?: AsyncEntitySearchFn;
  resolveSelected?: AsyncEntityResolveFn;
  calendars?: unknown[];
  calendar?: unknown;
  selectedCalendarId?: string;
  onChange?: (value: SchemaFieldValue) => void;
  onClearDate?: () => void;
  onSelectCalendar?: (id: string) => void;
} = $props();

const inputId = $derived(`${idPrefix}-${field.key}`);
const locked = $derived(disabled || readOnly);

function emit(next: SchemaFieldValue) {
  value = next;
  onChange?.(next);
}

function updateEnum(event: Event, multiple: boolean) {
  const select = event.currentTarget as HTMLSelectElement;
  if (multiple) {
    emit(
      Array.from(select.selectedOptions)
        .map((option) => option.value)
        .filter(Boolean),
    );
    return;
  }
  emit(select.value);
}

const stubSearch: AsyncEntitySearchFn = async (query): Promise<AsyncEntitySearchPage> => ({
  items: [],
  total: 0,
  offset: query.offset,
  limit: query.limit,
  hasMore: false,
});

const stubResolve: AsyncEntityResolveFn = async (ids) => ids.map((id) => ({ id, name: id, entityType: null }));

const selectedIds = $derived(
  Array.isArray(value) ? (value as string[]) : typeof value === "string" && value ? [value] : [],
);

let dateEditorOpen = $state(false);
const hasDateValue = $derived(value != null && value !== "");
</script>

<div class="schema-field-input {className}" class:is-readonly={readOnly} data-field-key={field.key}>
  <label class="schema-field-label" for={inputId}>
    <span>
      {field.label}
      {#if required}<b aria-hidden="true">*</b>{/if}
    </span>
  </label>

  {#if field.type === "relationship"}
    <RelationshipPicker
      {field}
      search={search ?? stubSearch}
      resolveSelected={resolveSelected ?? stubResolve}
      {selectedIds}
      onChange={(ids) => emit(ids)} />
  {:else if field.type === "text"}
    <textarea
      id={inputId}
      {required}
      disabled={locked}
      readonly={readOnly}
      rows="3"
      value={String(value ?? "")}
      placeholder={`Add ${field.label.toLowerCase()}`}
      oninput={(event) => emit((event.currentTarget as HTMLTextAreaElement).value)}></textarea>
  {:else if field.type === "number"}
    <input
      id={inputId}
      type="number"
      {required}
      disabled={locked}
      readonly={readOnly}
      value={String(value ?? "")}
      placeholder={`Add ${field.label.toLowerCase()}`}
      oninput={(event) => emit((event.currentTarget as HTMLInputElement).value)} />
  {:else if field.type === "boolean"}
    <label class="schema-field-checkbox" for={inputId}>
      <input
        id={inputId}
        type="checkbox"
        {required}
        disabled={locked}
        checked={value === true}
        onchange={(event) => emit((event.currentTarget as HTMLInputElement).checked)} />
      <span>Yes</span>
    </label>
  {:else if field.type === "enum"}
    <select
      id={inputId}
      {required}
      disabled={locked}
      multiple={field.multiple ?? false}
      value={field.multiple ? (Array.isArray(value) ? value : []) : String(value ?? "")}
      onchange={(event) => updateEnum(event, field.multiple ?? false)}>
      <option value="">Choose {field.label.toLowerCase()}</option>
      {#each field.options ?? [] as option}
        <option value={option}>{option}</option>
      {/each}
    </select>
  {:else if field.type === "oneof"}
    <select
      id={inputId}
      {required}
      disabled={locked}
      value={String(value ?? "")}
      onchange={(event) => emit((event.currentTarget as HTMLSelectElement).value)}>
      <option value="">Choose {field.label.toLowerCase()}</option>
      {#each field.options ?? [] as option}
        <option value={option}>{option}</option>
      {/each}
      {#each (field as FieldDefinition & { oneOf?: Array<{ label: string; options?: string[] }> }).oneOf ?? [] as variant}
        {#each variant.options ?? [] as opt}
          <option value={opt}>{variant.label}: {opt}</option>
        {/each}
      {/each}
    </select>
  {:else if field.type === "date"}
    {#if hasDateValue || dateEditorOpen || readOnly}
      <DateEditor
        label={field.label}
        {value}
        calendar={calendar as any}
        calendars={calendars as any}
        {selectedCalendarId}
        onChange={(next) => {
          emit(next);
          dateEditorOpen = true;
        }}
        onClear={() => {
          onClearDate?.();
          emit(null);
          dateEditorOpen = false;
        }}
        onSelectCalendar={(id) => onSelectCalendar?.(id)} />
    {:else}
      <button class="date-empty" type="button" disabled={locked} onclick={() => (dateEditorOpen = true)}>
        Add a date
      </button>
    {/if}
  {:else}
    <input
      id={inputId}
      type="text"
      {required}
      disabled={locked}
      readonly={readOnly}
      value={String(value ?? "")}
      oninput={(event) => emit((event.currentTarget as HTMLInputElement).value)} />
  {/if}
</div>

<style>
.schema-field-input {
  display: grid;
  gap: 0.35rem;
  margin-top: 10px;
}

.schema-field-label span {
  display: inline-flex;
  gap: 0.25rem;
  align-items: baseline;
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--text-muted, #5c645f);
}

.schema-field-label b {
  color: var(--danger, #a33);
}

.schema-field-input :global(textarea),
.schema-field-input :global(input[type="number"]),
.schema-field-input :global(input[type="text"]),
.schema-field-input :global(select) {
  width: 100%;
  min-height: var(--control-min-height);
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 9px 10px;
  background: var(--canvas);
  color: var(--ink);
  font: inherit;
}

.schema-field-input :global(textarea) {
  resize: vertical;
}

.schema-field-checkbox {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  min-height: var(--control-min-height);
}

.date-empty {
  justify-self: start;
  min-height: var(--control-min-height);
  border: 1px dashed var(--line);
  border-radius: 8px;
  padding: 9px 10px;
  background: transparent;
  color: var(--accent);
  font-size: 12px;
  cursor: pointer;
}

.is-readonly :global(textarea),
.is-readonly :global(input),
.is-readonly :global(select) {
  opacity: 0.85;
}
</style>
