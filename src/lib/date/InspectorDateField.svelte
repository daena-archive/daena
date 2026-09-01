<script lang="ts">
import DateEditor from "$lib/date/DateEditor.svelte";
import { parseCalendarDate } from "$lib/date";
import type { CalendarDefinition } from "../../../packages/modules/timeline/src/calendar";

let {
  label,
  fieldKey,
  value,
  editorOpen = false,
  editorKey = "",
  calendar = null,
  calendars = [],
  selectedCalendarId,
  required = false,
  onChange,
  onClear,
  onSelectCalendar,
  onOpen,
}: {
  label: string;
  fieldKey: string;
  value: unknown;
  editorOpen?: boolean;
  editorKey?: string;
  calendar?: CalendarDefinition | null;
  calendars?: unknown[];
  selectedCalendarId: string;
  required?: boolean;
  onChange: (next: unknown) => void;
  onClear: () => void;
  onSelectCalendar: (id: string) => void;
  onOpen: () => void;
} = $props();

const hasValue = $derived(Boolean(parseCalendarDate(value)));
const showEditor = $derived(hasValue || editorOpen);
</script>

<div class="property-field">
  <span
    >{label}{#if required}<b>*</b>{/if}</span>
  {#if showEditor}
    {#key editorKey || fieldKey}
      <DateEditor
        {label}
        {value}
        {calendar}
        calendars={calendars as any}
        {selectedCalendarId}
        {onChange}
        {onClear}
        {onSelectCalendar} />
    {/key}
  {:else}
    <button class="date-empty" type="button" onclick={onOpen}>Add a date</button>
  {/if}
</div>

<style>
.property-field {
  display: block;
  margin-top: 14px;
}
.property-field > span {
  display: block;
  margin-bottom: 5px;
  color: var(--ink-soft);
  font-size: 10px;
}
.property-field b {
  margin-left: 3px;
  color: var(--accent);
}
.date-empty {
  width: fit-content;
  padding: 8px 10px;
  border: 1px dashed var(--theme-warning-border, #d3c0a9);
  border-radius: 7px;
  background: transparent;
  color: var(--accent);
  font-size: 10px;
  cursor: pointer;
}
</style>
