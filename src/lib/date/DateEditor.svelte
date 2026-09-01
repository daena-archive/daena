<script lang="ts">
import { X } from "@lucide/svelte";
import CalendarPicker from "$lib/CalendarPicker.svelte";
import {
  formatCalendarDate,
  GREGORIAN_CALENDAR_ID,
  parseCalendarDate,
  serializeCalendarDate,
  type CalendarDate,
} from "$lib/date";
import { patchCalendarDate } from "$lib/date/dateField";
import {
  calendarDateToParts,
  daysInCalendarMonth,
  formatWithCalendar,
  type CalendarDefinition,
} from "../../../packages/modules/timeline/src/calendar";

let {
  label,
  value,
  calendars = [],
  calendar = null as CalendarDefinition | null,
  selectedCalendarId = GREGORIAN_CALENDAR_ID,
  onChange,
  onClear,
  onSelectCalendar,
}: {
  label: string;
  value: unknown;
  calendars?: any[];
  calendar?: CalendarDefinition | null;
  selectedCalendarId?: string;
  onChange: (next: unknown) => void;
  onClear: () => void;
  onSelectCalendar?: (id: string) => void;
} = $props();

let allowNegativeYears = $derived(calendar?.allowNegativeYears ?? false);

let rawDate = $derived(parseCalendarDate(value));
let timeDraftOpen = $state(false);

let showTimeFields = $derived(
  timeDraftOpen || rawDate?.hour !== undefined || rawDate?.minute !== undefined || rawDate?.second !== undefined,
);

function partsFor(d: CalendarDate | null, cal: CalendarDefinition | null) {
  if (!d) return null;
  return calendarDateToParts(d, cal);
}

let parts = $derived(partsFor(rawDate, calendar));
let months: any[] = $derived((calendar as any)?.months ?? []);
// Month/day picks made before a year exists cannot commit yet (patchCalendarDate defers
// them); keep them locally so the inputs hold the user's choice until the year arrives.
let draftParts = $state<{ month?: number; day?: number }>({});
let displayMonth = $derived(parts?.month ?? draftParts.month);
let displayDay = $derived(parts?.day ?? draftParts.day);
let maxDay = $derived(
  daysInCalendarMonth(calendar, parts?.year ?? rawDate?.year ?? 1, displayMonth ?? rawDate?.month ?? 1),
);

// Discard deferred picks whenever the value is replaced from the outside.
$effect(() => {
  value;
  draftParts = {};
});

function calendarTimeValue(d: CalendarDate | null): string {
  if (!d || ![d.hour, d.minute, d.second].every((p) => typeof p === "number")) return "";
  return [d.hour, d.minute, d.second].map((p) => String(p).padStart(2, "0")).join(":");
}

function commitPatch(patch: Partial<CalendarDate> & { calendar?: string }): boolean {
  const next = patchCalendarDate(value, patch, calendar, selectedCalendarId ?? GREGORIAN_CALENDAR_ID);
  if (next === null) return false;
  draftParts = {};
  onChange(next);
  return true;
}

function updatePart(part: "year" | "month" | "day", raw: string, min: number, max?: number) {
  if (!raw.trim()) {
    if (part === "month") {
      if (!commitPatch({ precision: "year" })) draftParts = { ...draftParts, month: undefined };
    } else if (part === "day") {
      if (!commitPatch({ precision: "month" })) draftParts = { ...draftParts, day: undefined };
    } else onClear();
    return;
  }
  const n = Math.floor(Number(raw));
  if (!Number.isFinite(n)) return;
  const v = Math.min(max ?? n, Math.max(min, n));
  if (part === "year") {
    commitPatch({ year: v, ...draftParts });
    return;
  }
  if (!commitPatch({ [part]: v })) draftParts = { ...draftParts, [part]: v };
}

function applyPatch(patch: Partial<CalendarDate> & { calendar?: string }) {
  commitPatch(patch);
}

function updateTime(raw: string) {
  const [h, m, s] = raw.split(":").map(Number);
  if (![h, m, s].every(Number.isFinite)) return;
  applyPatch({ hour: h, minute: m, second: s, precision: "second" } as any);
}
function removeTime() {
  const cur = rawDate;
  if (!cur) {
    timeDraftOpen = false;
    return;
  }
  const next: any = { ...cur };
  delete next.hour;
  delete next.minute;
  delete next.second;
  if (["hour", "minute", "second"].includes(next.precision)) next.precision = "day";
  onChange(serializeCalendarDate(next));
  timeDraftOpen = false;
}
</script>

<div class="date-editor">
  {#if calendars.length > 0}
    <div class="date-editor-head">
      <CalendarPicker
        selectedId={selectedCalendarId}
        calendars={calendars as any}
        onSelect={(id) => {
          if (onSelectCalendar) onSelectCalendar(id);
          else applyPatch({ calendar: id } as any);
        }} />
    </div>
  {/if}
  <div class="date-fields" class:has-custom-months={months.length > 0}>
    <label
      >Year<input
        type="number"
        aria-label={`${label} year`}
        value={parts?.year ?? (rawDate as any)?.year ?? ""}
        onchange={(e) =>
          updatePart(
            "year",
            (e.currentTarget as HTMLInputElement).value,
            allowNegativeYears ? Number.MIN_SAFE_INTEGER : 1,
          )} /></label>
    {#if months.length > 0}
      <label
        >Month<select
          aria-label={`${label} month`}
          value={displayMonth ?? ""}
          onchange={(e) => updatePart("month", (e.currentTarget as HTMLSelectElement).value, 1, months.length)}
          ><option value="">Month</option>{#each months as month, i}<option value={i + 1}>{month.name}</option
            >{/each}</select
        ></label>
    {:else}
      <label
        >Month<input
          type="number"
          aria-label={`${label} month`}
          min="1"
          max="12"
          value={displayMonth ?? (rawDate as any)?.month ?? ""}
          onchange={(e) => updatePart("month", (e.currentTarget as HTMLInputElement).value, 1, 12)} /></label>
    {/if}
    <label
      >Day<input
        type="number"
        aria-label={`${label} day`}
        min="1"
        max={maxDay}
        value={displayDay ?? (rawDate as any)?.day ?? ""}
        onchange={(e) => updatePart("day", (e.currentTarget as HTMLInputElement).value, 1, maxDay)} /></label>
  </div>
  {#if showTimeFields}
    {@const maxHour = 23}
    <div class="date-time-row">
      <div class="date-time-fields">
        <label
          >Hour<input
            type="number"
            aria-label={`${label} hour`}
            min="0"
            max={maxHour}
            value={rawDate?.hour ?? ""}
            placeholder="HH"
            onchange={(e) => {
              const v = (e.currentTarget as HTMLInputElement).value;
              if (!v.trim()) {
                const cur = rawDate as any;
                if (cur) {
                  const next: any = { ...cur };
                  delete next.hour;
                  delete next.minute;
                  delete next.second;
                  if (["hour", "minute", "second"].includes(next.precision)) next.precision = "day";
                  onChange(serializeCalendarDate(next));
                }
              } else {
                const h = Math.max(0, Math.min(maxHour, Math.floor(Number(v))));
                applyPatch({ hour: h, precision: "hour" } as any);
              }
            }} /></label>
        <span class="date-time-sep">:</span>
        <label
          >Minute<input
            type="number"
            aria-label={`${label} minute`}
            min="0"
            max="59"
            value={rawDate?.minute ?? ""}
            placeholder="MM"
            onchange={(e) => {
              const v = (e.currentTarget as HTMLInputElement).value;
              if (!v.trim() && rawDate?.hour === undefined) return;
              const m = v.trim() ? Math.max(0, Math.min(59, Math.floor(Number(v)))) : 0;
              applyPatch({ hour: rawDate?.hour ?? 0, minute: m, precision: "minute" } as any);
            }} /></label>
        <span class="date-time-sep">:</span>
        <label
          >Second<input
            type="number"
            aria-label={`${label} second`}
            min="0"
            max="59"
            value={rawDate?.second ?? ""}
            placeholder="SS"
            onchange={(e) => {
              const v = (e.currentTarget as HTMLInputElement).value;
              const s = v.trim() ? Math.max(0, Math.min(59, Math.floor(Number(v)))) : 0;
              applyPatch({
                hour: rawDate?.hour ?? 0,
                minute: rawDate?.minute ?? 0,
                second: s,
                precision: "second",
              } as any);
            }} /></label>
      </div>
      <button type="button" class="date-time-remove" aria-label="Remove time" onclick={removeTime}
        ><X size={14} strokeWidth={1.8} aria-hidden="true" /></button>
    </div>
  {:else}
    <button
      type="button"
      class="date-time-add"
      disabled={rawDate === null}
      title={rawDate === null ? "Enter a year first" : undefined}
      onclick={() => {
        timeDraftOpen = true;
      }}>Add time</button>
  {/if}
  <div class="date-editor-footer">
    <small class="date-preview"
      >{rawDate ? (calendar ? formatWithCalendar(value, calendar) : formatCalendarDate(rawDate)) : ""}</small>
    <button type="button" class="date-clear" onclick={onClear}>Clear date</button>
  </div>
</div>

<style>
.date-editor {
  display: grid;
  gap: 8px;
  min-width: 0;
  max-width: 100%;
  padding: 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--theme-warning-bg, #fcf8f1);
  overflow: visible;
}
.date-editor-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  min-width: 0;
}
.date-editor-head :global(.calendar-picker) {
  flex: 1 1 0;
  min-width: 0;
}
.date-editor-head :global(.calendar-picker-trigger) {
  width: 100%;
}
.date-fields {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 4px;
  min-width: 0;
}
.date-fields.has-custom-months {
  grid-template-columns: minmax(0, 1fr) minmax(0, 1.6fr) minmax(0, 0.8fr);
}
.date-fields label {
  display: grid;
  gap: 4px;
  min-width: 0;
  color: var(--ink-faint);
  font-size: 9px;
  font-weight: 700;
  text-transform: uppercase;
  overflow: hidden;
}
.date-fields input,
.date-fields select {
  min-width: 0;
  width: 100%;
  max-width: 100%;
  height: 36px;
  min-height: 36px;
  box-sizing: border-box;
  padding: 8px 6px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--canvas);
  color: var(--ink);
  font-size: 11px;
  line-height: 1.2;
}
.date-fields select {
  appearance: none;
  -webkit-appearance: none;
  padding-right: 28px;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%2377766d' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
  background-size: 12px;
  cursor: pointer;
}
.date-time-row {
  display: flex;
  align-items: end;
  gap: 8px;
}
.date-time-fields {
  flex: 1;
  display: grid;
  grid-template-columns: 1fr auto 1fr auto 1fr;
  gap: 4px;
  align-items: end;
  min-width: 0;
}
.date-time-fields label {
  display: grid;
  gap: 4px;
  min-width: 0;
  color: var(--ink-faint);
  font-size: 9px;
  font-weight: 700;
  text-transform: uppercase;
  overflow: hidden;
}
.date-time-fields input {
  min-width: 0;
  width: 100%;
  height: 36px;
  box-sizing: border-box;
  padding: 8px 6px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--canvas);
  color: var(--ink);
  font-size: 11px;
  text-align: center;
}
.date-time-sep {
  padding-bottom: 9px;
  color: var(--ink-faint);
  font-weight: 700;
  font-size: 11px;
}
.date-time-row label {
  flex: 1;
  display: grid;
  gap: 4px;
  color: var(--ink-faint);
  font-size: 9px;
  font-weight: 700;
  text-transform: uppercase;
}
.date-time-row input {
  padding: 8px 6px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--canvas);
  color: var(--ink);
  font-size: 11px;
}
.date-time-add {
  padding: 6px 10px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--surface);
  color: var(--ink-soft);
  font-size: 10px;
  cursor: pointer;
}
.date-time-add:hover {
  border-color: var(--accent-soft);
  background: var(--surface-muted);
}
.date-time-add:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}
.date-time-remove {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  padding: 0;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--surface);
  color: var(--ink-faint);
  cursor: pointer;
  flex: none;
}
.date-time-remove:hover {
  border-color: var(--line-strong);
  background: var(--surface-muted);
  color: var(--ink);
}
.date-preview {
  color: var(--accent);
  font-size: 10px;
  font-weight: 700;
}
.date-editor-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding-top: 2px;
}
.date-editor-footer .date-preview {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.date-clear {
  flex: none;
  width: fit-content;
  border: 0;
  background: transparent;
  color: var(--ink-faint);
  font-size: 10px;
  cursor: pointer;
}
@media (max-width: 520px) {
  .date-fields {
    grid-template-columns: 1fr;
  }
  .date-time-row {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
