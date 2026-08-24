<script lang="ts">
import { onMount, tick } from "svelte";
import { X, ArrowRight } from "@lucide/svelte";
import type { FieldDefinition } from "../../packages/module-api/src/index";
import type { Entity, Relationship } from "$lib/project/client";
import {
  calendarDateToParts,
  daysInCalendarMonth,
  formatWithCalendar,
  partsToCalendarDate,
  type CalendarDefinition,
} from "../../packages/modules/timeline/src/calendar";
import {
  formatCalendarDate,
  GREGORIAN_CALENDAR_ID,
  isGregorianCalendarId,
  parseCalendarDate,
  serializeCalendarDate,
  type CalendarDate,
} from "$lib/date";
import CalendarPicker from "$lib/CalendarPicker.svelte";

type Metadata = Record<string, unknown>;
type MetadataField = {
  key: string;
  label: string;
  type: "text" | "number" | "boolean" | "date" | "enum" | "oneof";
  required?: boolean | null;
  options?: string[] | null;
  oneOf?: Array<{ label: string; type: string; options?: string[] | null }>;
};
type RelationshipDefinition = FieldDefinition & { metadataFields?: MetadataField[] };

let {
  relationship,
  definition,
  entities,
  calendarDefinitions = {},
  onSave,
  onClose,
}: {
  relationship: Relationship;
  definition: RelationshipDefinition | null;
  entities: Entity[];
  calendarDefinitions?: Record<string, CalendarDefinition>;
  onSave: (metadata: Metadata) => void | Promise<void>;
  onClose: () => void;
} = $props();

let dialogElement = $state<HTMLDivElement | null>(null);
// The dialog is mounted per relationship, so initialize its editable draft once.
// svelte-ignore state_referenced_locally
let draft = $state<Metadata>(parseMetadata(relationship.metadata));
let fieldErrors = $state<Record<string, string>>({});
let saveError = $state("");
let saving = $state(false);
let lastFocused: Element | null = null;
let dateEditorOpen = $state<Record<string, boolean>>({});
let dateCalendarByField = $state<Record<string, string>>({});

const metadataFields = () => definition?.metadataFields ?? [];
const targetEntity = () => entities.find((entity) => entity.id === relationship.target_id);
const targetName = () => targetEntity()?.name ?? relationship.target_id;
const titleId = () => `relationship-metadata-title-${relationship.id}`;

function parseMetadata(raw: string): Metadata {
  try {
    const parsed: unknown = JSON.parse(raw || "{}");
    return typeof parsed === "object" && parsed !== null && !Array.isArray(parsed) ? (parsed as Metadata) : {};
  } catch {
    return {};
  }
}

function hasValue(value: unknown): boolean {
  return value !== undefined && value !== null && !(typeof value === "string" && value.trim() === "");
}

function valueFor(key: string): unknown {
  return draft[key];
}

function textValue(key: string): string {
  const value = valueFor(key);
  return value === undefined || value === null ? "" : String(value);
}

function setValue(key: string, value: unknown) {
  draft = { ...draft, [key]: value };
  const nextErrors = { ...fieldErrors };
  delete nextErrors[key];
  fieldErrors = nextErrors;
  saveError = "";
}

function isValidDate(value: unknown): boolean {
  return parseCalendarDate(value) !== null;
}

function invalidMessage(field: MetadataField, value: unknown): string {
  if (field.required && !hasValue(value)) return `${field.label} is required.`;
  if (!hasValue(value)) return "";
  if (field.type === "text" && typeof value !== "string") return `${field.label} must be text.`;
  if (field.type === "number" && (typeof value !== "number" || !Number.isFinite(value))) {
    return `${field.label} must be a number.`;
  }
  if (field.type === "boolean" && typeof value !== "boolean") return `${field.label} must be enabled or disabled.`;
  if (field.type === "date" && !isValidDate(value)) return `${field.label} must be a valid date.`;
  if (field.type === "enum" && !field.options?.includes(String(value))) {
    return `${field.label} must use one of the configured options.`;
  }
  if ((field as any).type === "oneof") {
    const opts =
      field.options ??
      ((field as any).oneOf as Array<{ options?: string[] }> | undefined)?.flatMap((v) => v.options ?? []) ??
      [];
    if (!opts.includes(String(value))) return `${field.label} must use one of the configured options.`;
  }
  return "";
}

// --- Calendar-aware helpers (mirrors inspector) ---
function worldCalendars() {
  return entities.filter((entity) => entity.entity_type === "calendar" && !entity.deleted);
}
function calendarDefinitionForId(calendarId: string | undefined): CalendarDefinition | null {
  if (isGregorianCalendarId(calendarId)) return null;
  return calendarDefinitions[calendarId!] ?? null;
}
function calendarIdForStoredDate(date: Partial<CalendarDate> | null | undefined, fallback: string | undefined): string {
  if (date?.calendar) return date.calendar;
  return fallback || GREGORIAN_CALENDAR_ID;
}
function dateForKey(key: string): CalendarDate | null {
  return parseCalendarDate(draft[key]);
}
function selectedCalendarId(key: string): string {
  return calendarIdForStoredDate(dateForKey(key), dateCalendarByField[key]);
}
function definitionForDateKey(key: string): CalendarDefinition | null {
  return calendarDefinitionForId(selectedCalendarId(key));
}
function dateDraftForKey(key: string): Partial<CalendarDate> | null {
  return (
    dateForKey(key) ?? (dateEditorOpen[key] ? { calendar: selectedCalendarId(key), era: "CE", precision: "day" } : null)
  );
}
function datePartsDraft(key: string) {
  const stored = dateForKey(key);
  const calendar = definitionForDateKey(key);
  if (stored) return calendarDateToParts(stored, calendar);
  return dateEditorOpen[key]
    ? {
        year: undefined as number | undefined,
        month: undefined as number | undefined,
        day: undefined as number | undefined,
        precision: "day" as const,
      }
    : null;
}
function setDateCalendar(key: string, calendarId: string) {
  dateCalendarByField = { ...dateCalendarByField, [key]: calendarId };
  const previous = dateForKey(key);
  if (!previous) {
    dateEditorOpen = { ...dateEditorOpen, [key]: true };
    return;
  }
  const next = serializeCalendarDate({ ...previous, calendar: calendarId });
  setValue(key, next);
}
function openDateEditor(key: string) {
  dateEditorOpen = { ...dateEditorOpen, [key]: true };
  dateCalendarByField = { ...dateCalendarByField, [key]: GREGORIAN_CALENDAR_ID };
  setValue(key, "");
}
function updateDateField(key: string, patch: Partial<CalendarDate>) {
  const calendar = definitionForDateKey(key);
  const calendarId = selectedCalendarId(key);
  const previous = dateForKey(key);
  const currentParts = calendarDateToParts(
    previous ?? { calendar: calendarId, era: "CE", year: 1, precision: "year" },
    calendar,
  ) ?? { year: 1, precision: "year" as const };
  const nextParts = { ...currentParts, ...patch };
  if ((patch as Record<string, unknown>).precision === undefined) {
    const hasMonth = (nextParts as Record<string, unknown>).month !== undefined;
    const hasDay = (nextParts as Record<string, unknown>).day !== undefined;
    if (!hasMonth) {
      nextParts.precision = "year";
      delete (nextParts as Record<string, unknown>).day;
    } else if (!hasDay) {
      nextParts.precision = "month";
    } else {
      if (nextParts.precision !== "hour" && nextParts.precision !== "minute" && nextParts.precision !== "second") {
        nextParts.precision = "day";
      }
    }
  }
  if (patch.precision === "year") {
    delete nextParts.month;
    delete nextParts.day;
  }
  if (patch.precision === "month") {
    delete (nextParts as Record<string, unknown>).day;
    if (nextParts.month === undefined) nextParts.month = 1;
  }
  if (patch.precision === "day") {
    nextParts.month ??= 1;
    nextParts.day ??= 1;
  }
  const stored = partsToCalendarDate(nextParts, calendar);
  stored.calendar = calendarId;
  if (previous) {
    stored.hour = patch.hour ?? previous.hour;
    stored.minute = patch.minute ?? previous.minute;
    stored.second = patch.second ?? previous.second;
  } else if (patch.hour !== undefined) {
    stored.hour = patch.hour;
    stored.minute = patch.minute;
    stored.second = patch.second;
  }
  if (patch.precision === "hour" || patch.precision === "minute" || patch.precision === "second") {
    stored.precision = patch.precision;
  }
  setValue(key, serializeCalendarDate(stored));
}
function updateDatePart(key: string, part: "year" | "month" | "day", raw: string, min: number, max?: number) {
  if (!raw.trim()) {
    if (part === "month") {
      updateDateField(key, { precision: "year" });
    } else if (part === "day") {
      updateDateField(key, { precision: "month" });
    } else if (part === "year") {
      clearDateField(key);
    }
    return;
  }
  const parsed = Math.floor(Number(raw));
  if (!Number.isFinite(parsed)) return;
  updateDateField(key, { [part]: Math.min(max ?? parsed, Math.max(min, parsed)) });
}
function updateDateTime(key: string, raw: string) {
  const [hour, minute, second] = raw.split(":").map(Number);
  if (![hour, minute, second].every(Number.isFinite)) return;
  updateDateField(key, { hour, minute, second, precision: "second" });
}
function calendarTimeValue(date: Partial<CalendarDate>): string {
  if (![date.hour, date.minute, date.second].every((part) => typeof part === "number")) return "";
  return [date.hour, date.minute, date.second].map((part) => String(part).padStart(2, "0")).join(":");
}
function clearDateField(key: string) {
  setValue(key, "");
  dateEditorOpen = { ...dateEditorOpen, [key]: false };
  const next = { ...dateCalendarByField };
  delete next[key];
  dateCalendarByField = next;
}

// Import partsToCalendarDate that we missed above - need to adjust imports
// We'll handle by importing directly in script tag; to avoid duplication, we patch import section after.

async function submit() {
  const next = { ...draft };
  const errors: Record<string, string> = {};
  for (const field of metadataFields()) {
    const message = invalidMessage(field, next[field.key]);
    if (message) errors[field.key] = message;
    if (!hasValue(next[field.key])) delete next[field.key];
  }
  if (Object.keys(errors).length > 0) {
    fieldErrors = errors;
    return;
  }
  saving = true;
  saveError = "";
  try {
    await onSave(next);
    onClose();
  } catch (cause) {
    saveError = cause instanceof Error ? cause.message : "Could not save relationship details.";
  } finally {
    saving = false;
  }
}

function focusableElements(): HTMLElement[] {
  return Array.from(
    dialogElement?.querySelectorAll<HTMLElement>(
      'button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ) ?? [],
  );
}

onMount(() => {
  lastFocused = document.activeElement;
  void tick().then(() => focusableElements()[0]?.focus());
  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key !== "Tab") return;
    const focusable = focusableElements();
    if (focusable.length === 0) {
      event.preventDefault();
      dialogElement?.focus();
      return;
    }
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };
  window.addEventListener("keydown", handleKeydown, true);
  return () => {
    window.removeEventListener("keydown", handleKeydown, true);
    if (lastFocused instanceof HTMLElement && lastFocused.isConnected) lastFocused.focus();
  };
});
</script>

<div class="relationship-metadata-backdrop" role="presentation" onclick={onClose}>
  <div
    bind:this={dialogElement}
    class="relationship-metadata-dialog"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby={titleId()}
    onclick={(event) => event.stopPropagation()}
    onkeydown={(event) => event.stopPropagation()}>
    <header class="relationship-metadata-header">
      <div>
        <span class="relationship-metadata-kicker">RELATIONSHIP DETAILS</span>
        <h2 id={titleId()}>{relationship.relationship_type}</h2>
        <p>
          <span style="display:inline-flex;vertical-align:middle" aria-hidden="true"
            ><ArrowRight size={12} strokeWidth={1.8} /></span>
          {targetName()}
        </p>
      </div>
      <button
        type="button"
        class="relationship-metadata-close"
        aria-label="Close relationship details"
        onclick={onClose}><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
    </header>

    {#if metadataFields().length === 0}
      <div class="relationship-metadata-empty">
        <strong>No configurable properties for this relationship type</strong>
        <p>The relationship target is <b>{targetName()}</b>. Its metadata is managed outside this module.</p>
      </div>
      <footer class="relationship-metadata-actions">
        <button type="button" class="relationship-metadata-primary" onclick={onClose}>Close</button>
      </footer>
    {:else}
      <form
        class="relationship-metadata-form"
        onsubmit={(event) => {
          event.preventDefault();
          void submit();
        }}>
        {#each metadataFields() as field (field.key)}
          <div class="relationship-metadata-field">
            {#if field.type === "date"}
              <span class="relationship-metadata-field-label"
                >{field.label}{#if field.required}<b aria-hidden="true"> *</b>{/if}</span>
              {#if dateForKey(field.key) || dateEditorOpen[field.key]}
                {@const date = dateDraftForKey(field.key) ?? {
                  calendar: GREGORIAN_CALENDAR_ID,
                  era: "CE",
                  precision: "day",
                }}
                {@const parts = datePartsDraft(field.key)}
                {@const calendar = definitionForDateKey(field.key)}
                {@const months = calendar?.months ?? []}
                <div class="date-editor">
                  <CalendarPicker
                    selectedId={selectedCalendarId(field.key)}
                    calendars={worldCalendars()}
                    onSelect={(id) => setDateCalendar(field.key, id)} />
                  <div class="date-fields">
                    <label for={`relationship-${relationship.id}-${field.key}-year`}
                      >Year<input
                        id={`relationship-${relationship.id}-${field.key}-year`}
                        aria-label={`${field.label} year`}
                        type="number"
                        value={parts?.year ?? date.year ?? ""}
                        onchange={(event) =>
                          updateDatePart(
                            field.key,
                            "year",
                            (event.currentTarget as HTMLInputElement).value,
                            Number.MIN_SAFE_INTEGER,
                          )} /></label
                    >{#if months.length > 0}<label for={`relationship-${relationship.id}-${field.key}-month`}
                        >Month<select
                          id={`relationship-${relationship.id}-${field.key}-month`}
                          aria-label={`${field.label} month`}
                          value={parts?.month ?? ""}
                          onchange={(event) =>
                            updateDatePart(
                              field.key,
                              "month",
                              (event.currentTarget as HTMLSelectElement).value,
                              1,
                              months.length,
                            )}
                          ><option value="">Month</option>{#each months as month, index}<option value={index + 1}
                              >{month.name}</option
                            >{/each}</select
                        ></label
                      >{:else}<label for={`relationship-${relationship.id}-${field.key}-month`}
                        >Month<input
                          id={`relationship-${relationship.id}-${field.key}-month`}
                          aria-label={`${field.label} month`}
                          type="number"
                          min="1"
                          max="12"
                          value={parts?.month ?? date.month ?? ""}
                          onchange={(event) =>
                            updateDatePart(
                              field.key,
                              "month",
                              (event.currentTarget as HTMLInputElement).value,
                              1,
                              12,
                            )} /></label
                      >{/if}<label for={`relationship-${relationship.id}-${field.key}-day`}
                      >Day<input
                        id={`relationship-${relationship.id}-${field.key}-day`}
                        aria-label={`${field.label} day`}
                        type="number"
                        min="1"
                        max={daysInCalendarMonth(
                          calendar,
                          parts?.year ?? date.year ?? 1,
                          parts?.month ?? date.month ?? 1,
                        )}
                        value={parts?.day ?? date.day ?? ""}
                        onchange={(event) =>
                          updateDatePart(
                            field.key,
                            "day",
                            (event.currentTarget as HTMLInputElement).value,
                            1,
                            daysInCalendarMonth(
                              calendar,
                              parts?.year ?? date.year ?? 1,
                              parts?.month ?? date.month ?? 1,
                            ),
                          )} /></label
                    ><label class="date-time-field" for={`relationship-${relationship.id}-${field.key}-time`}
                      >Time<input
                        id={`relationship-${relationship.id}-${field.key}-time`}
                        aria-label={`${field.label} time`}
                        type="time"
                        step="1"
                        value={calendarTimeValue(date)}
                        onchange={(event) =>
                          updateDateTime(field.key, (event.currentTarget as HTMLInputElement).value)} /></label>
                  </div>
                  <small class="date-preview"
                    >{typeof (parts?.year ?? date.year) === "number"
                      ? formatWithCalendar(draft[field.key], calendar) !== "Undated"
                        ? formatWithCalendar(draft[field.key], calendar)
                        : draft[field.key]
                          ? formatCalendarDate(draft[field.key])
                          : "Add a date"
                      : "Add a date"}</small
                  ><button class="date-clear" type="button" onclick={() => clearDateField(field.key)}
                    >Clear date</button>
                </div>
              {:else}<button class="date-empty" type="button" onclick={() => openDateEditor(field.key)}
                  >Add a date</button
                >{/if}
            {:else if field.type === "boolean"}
              <label for={`relationship-${relationship.id}-${field.key}`}>
                <span
                  >{field.label}{#if field.required}<b aria-hidden="true"> *</b>{/if}</span>
                <input
                  id={`relationship-${relationship.id}-${field.key}`}
                  type="checkbox"
                  checked={valueFor(field.key) === true}
                  onchange={(event) => setValue(field.key, (event.currentTarget as HTMLInputElement).checked)} />
              </label>
            {:else if field.type === "number"}
              <label for={`relationship-${relationship.id}-${field.key}`}>
                <span
                  >{field.label}{#if field.required}<b aria-hidden="true"> *</b>{/if}</span>
                <input
                  id={`relationship-${relationship.id}-${field.key}`}
                  type="number"
                  value={textValue(field.key)}
                  oninput={(event) => {
                    const raw = (event.currentTarget as HTMLInputElement).value;
                    setValue(field.key, raw === "" ? "" : Number(raw));
                  }} />
              </label>
            {:else if field.type === "enum"}
              <label for={`relationship-${relationship.id}-${field.key}`}>
                <span
                  >{field.label}{#if field.required}<b aria-hidden="true"> *</b>{/if}</span>
                <select
                  id={`relationship-${relationship.id}-${field.key}`}
                  value={textValue(field.key)}
                  onchange={(event) => setValue(field.key, (event.currentTarget as HTMLSelectElement).value)}>
                  <option value="">Choose {field.label.toLowerCase()}</option>
                  {#each field.options ?? [] as option}<option value={option}>{option}</option>{/each}
                </select>
              </label>
            {:else if (field as any).type === "oneof"}
              <label for={`relationship-${relationship.id}-${field.key}`}>
                <span
                  >{field.label}{#if field.required}<b aria-hidden="true"> *</b>{/if}</span>
                <select
                  id={`relationship-${relationship.id}-${field.key}`}
                  value={textValue(field.key)}
                  onchange={(event) => setValue(field.key, (event.currentTarget as HTMLSelectElement).value)}>
                  <option value="">Choose {field.label.toLowerCase()}</option>
                  {#each field.options ?? [] as option}<option value={option}>{option}</option>{/each}
                  {#each (field as any).oneOf ?? [] as variant}
                    {#each variant.options ?? [] as opt}<option value={opt}>{variant.label}: {opt}</option>{/each}
                  {/each}
                </select>
              </label>
            {:else}
              <label for={`relationship-${relationship.id}-${field.key}`}>
                <span
                  >{field.label}{#if field.required}<b aria-hidden="true"> *</b>{/if}</span>
                <input
                  id={`relationship-${relationship.id}-${field.key}`}
                  type="text"
                  value={textValue(field.key)}
                  placeholder={`Add ${field.label.toLowerCase()}`}
                  oninput={(event) => setValue(field.key, (event.currentTarget as HTMLInputElement).value)} />
              </label>
            {/if}
            {#if fieldErrors[field.key]}<small class="relationship-metadata-error" role="alert"
                >{fieldErrors[field.key]}</small
              >{/if}
          </div>
        {/each}
        {#if saveError}<p class="relationship-metadata-error" role="alert">{saveError}</p>{/if}
        <footer class="relationship-metadata-actions">
          <button type="button" class="relationship-metadata-secondary" onclick={onClose}>Cancel</button>
          <button type="submit" class="relationship-metadata-primary" disabled={saving}
            >{saving ? "Saving…" : "Save details"}</button>
        </footer>
      </form>
    {/if}
  </div>
</div>

<style>
.relationship-metadata-backdrop {
  position: fixed;
  z-index: 85;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 18px;
  background: rgba(37, 37, 31, 0.32);
}
.relationship-metadata-dialog {
  width: min(500px, 100%);
  max-height: min(720px, calc(100vh - 36px));
  overflow-y: auto;
  padding: 22px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
  box-shadow: 0 24px 70px rgba(38, 42, 33, 0.25);
  outline: none;
}
.relationship-metadata-header,
.relationship-metadata-actions {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
.relationship-metadata-kicker {
  display: block;
  color: var(--accent);
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.09em;
  text-transform: uppercase;
}
h2 {
  margin: 4px 0 0;
  color: var(--ink);
  font: 700 21px/1.2 var(--font-display, Georgia, serif);
}
.relationship-metadata-header p {
  margin: 5px 0 0;
  color: var(--ink-soft);
  font-size: 12px;
}
.relationship-metadata-close {
  width: 30px;
  height: 30px;
  flex: none;
  border: 0;
  border-radius: 7px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  font-size: 20px;
  line-height: 1;
  cursor: pointer;
}
.relationship-metadata-close:hover,
.relationship-metadata-close:focus-visible {
  background: var(--theme-warning-bg, #ebe6dd);
  color: var(--ink);
  outline: 2px solid rgba(180, 119, 63, 0.2);
  outline-offset: 1px;
}
.relationship-metadata-form {
  display: grid;
  gap: 13px;
  margin-top: 20px;
}
.relationship-metadata-empty p {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.55;
}
.relationship-metadata-field {
  display: grid;
  gap: 5px;
}
.relationship-metadata-field label {
  display: grid;
  gap: 6px;
  color: var(--ink);
  font-size: 12px;
  font-weight: 650;
}
.relationship-metadata-field-label {
  color: var(--ink-soft);
  font-size: 10px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  font-weight: 650;
}
.relationship-metadata-field-label b {
  color: var(--accent);
}
.relationship-metadata-field label > span {
  color: var(--ink-soft);
  font-size: 10px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.relationship-metadata-field label > span b {
  color: var(--accent);
}
.relationship-metadata-field input:not([type="checkbox"]),
.relationship-metadata-field select {
  width: 100%;
  min-height: 38px;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--canvas);
  color: var(--ink);
  font-size: 13px;
  outline: none;
}
.relationship-metadata-field input:not([type="checkbox"]):focus,
.relationship-metadata-field select:focus {
  border-color: var(--accent-soft);
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.1);
}
.relationship-metadata-field input[type="checkbox"] {
  width: 18px;
  height: 18px;
  accent-color: var(--accent-dark);
}
.relationship-metadata-error {
  color: var(--theme-danger-text, #a1482f);
  font-size: 11px;
  line-height: 1.4;
}
.relationship-metadata-empty {
  margin-top: 22px;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--canvas);
}
.relationship-metadata-empty strong {
  display: block;
  margin-bottom: 5px;
  color: var(--ink);
  font-size: 13px;
}
.relationship-metadata-actions {
  align-items: center;
  justify-content: flex-end;
  margin-top: 20px;
}
.relationship-metadata-secondary,
.relationship-metadata-primary {
  padding: 9px 14px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 650;
  cursor: pointer;
}
.relationship-metadata-secondary {
  border: 1px solid var(--line);
  background: transparent;
  color: var(--ink-soft);
}
.relationship-metadata-primary {
  border: 1px solid var(--accent-dark);
  background: var(--accent-dark);
  color: #fff;
}
.relationship-metadata-primary:disabled {
  cursor: wait;
  opacity: 0.55;
}
.date-editor {
  display: grid;
  gap: 8px;
  padding: 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--theme-warning-bg, #fcf8f1);
}
.date-fields {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr)) minmax(108px, 1.6fr);
  gap: 6px;
}
.date-fields label {
  display: grid;
  gap: 4px;
  color: var(--ink-faint);
  font-size: 9px;
  font-weight: 700;
  text-transform: uppercase;
}
.date-fields input,
.date-fields select {
  min-width: 0;
  width: 100%;
  padding: 8px 6px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--canvas);
  color: var(--ink);
  font-size: 11px;
  outline: none;
}
.date-fields input:focus,
.date-fields select:focus {
  border-color: var(--accent-soft);
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.1);
}
.date-preview {
  color: var(--accent);
  font-size: 10px;
  font-weight: 700;
}
.date-clear,
.date-empty {
  width: fit-content;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--ink-faint);
  font-size: 10px;
  cursor: pointer;
}
.date-empty {
  padding: 8px 10px;
  border: 1px dashed var(--theme-warning-border, #d3c0a9);
  border-radius: 7px;
  color: var(--accent);
}
@media (max-width: 520px) {
  .relationship-metadata-backdrop {
    align-items: end;
    padding: 10px;
  }
  .relationship-metadata-dialog {
    max-height: calc(100vh - 20px);
    padding: 18px;
    border-radius: 12px 12px 8px 8px;
  }
  .date-fields {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
  .date-time-field {
    grid-column: 1 / -1;
  }
}
</style>
