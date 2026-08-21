<script lang="ts">
import type { ModuleContext, UUID } from "../../../module-api/src/index";
import {
  CALENDAR_DEFINITION_COLLECTION,
  DATE_FORMAT_PRESETS,
  DATE_FORMAT_GUIDE,
  DEFAULT_DATE_FORMAT,
  YEAR_PRESETS,
  applyYearPreset,
  calendarSummary,
  computedYearLength,
  emptyCalendarDefinition,
  epochSummary,
  formatCalendarParts,
  matchYearPreset,
  normalizeCalendarDefinition,
  previewCalendarParts,
  validateCalendarDefinition,
  type CalendarDefinition,
  type CalendarMonth,
  type CalendarNamedUnit,
  type CalendarSeason,
  type YearPresetId,
} from "./calendar";

let {
  context,
  entityId,
  onsaved,
}: { context: ModuleContext; entityId: UUID; onsaved?: (definition: CalendarDefinition) => void } = $props();

let saved = $state<CalendarDefinition>(emptyCalendarDefinition());
let draft = $state<CalendarDefinition>(emptyCalendarDefinition());
let recordId = $state<UUID | null>(null);
let revision = $state("");
let error = $state("");
let open = $state(false);
let saving = $state(false);
let loadToken = 0;

const issues = $derived(validateCalendarDefinition(draft));
const errors = $derived(issues.filter((issue) => issue.level === "error"));
const yearLength = $derived(computedYearLength(draft));
const activePreset = $derived(matchYearPreset(draft));
const preview = $derived(formatCalendarParts(previewCalendarParts(draft), draft) || "Add months to preview a date");

function newId(prefix: string) {
  return `${prefix}-${crypto.randomUUID().slice(0, 8)}`;
}

async function load() {
  const token = ++loadToken;
  error = "";
  try {
    const records = await context.records.list(CALENDAR_DEFINITION_COLLECTION, entityId, { limit: 1 });
    if (token !== loadToken) return;
    const record = records[0];
    recordId = record?.id ?? null;
    revision = record?.revision ?? "";
    saved = record ? normalizeCalendarDefinition(record.value) : emptyCalendarDefinition();
    if (!open) draft = saved;
  } catch (cause) {
    if (token !== loadToken) return;
    error = cause instanceof Error ? cause.message : String(cause);
  }
}

function payload(definition: CalendarDefinition): CalendarDefinition {
  return JSON.parse(
    JSON.stringify({
      schemaVersion: 1,
      ...(definition.startingYear !== undefined ? { startingYear: definition.startingYear } : {}),
      ...(definition.epoch ? { epoch: definition.epoch } : {}),
      dateFormat: definition.dateFormat?.trim() || DEFAULT_DATE_FORMAT,
      months: definition.months,
      weekdays: definition.weekdays,
      seasons: definition.seasons,
    }),
  ) as CalendarDefinition;
}

async function persist() {
  if (errors.length > 0) return;
  saving = true;
  error = "";
  try {
    const value = payload(draft);
    if (recordId) {
      const updated = await context.records.update(CALENDAR_DEFINITION_COLLECTION, recordId, entityId, value, {
        expectedRevision: revision,
      });
      recordId = updated.id;
      revision = updated.revision;
    } else {
      const created = await context.records.create(CALENDAR_DEFINITION_COLLECTION, entityId, value);
      recordId = created.id;
      revision = created.revision;
    }
    saved = normalizeCalendarDefinition(value);
    draft = saved;
    open = false;
    onsaved?.(saved);
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  } finally {
    saving = false;
  }
}

function openModal() {
  draft = normalizeCalendarDefinition(JSON.parse(JSON.stringify(saved)));
  open = true;
}

function closeModal() {
  draft = saved;
  open = false;
}

function setDraft(next: CalendarDefinition) {
  draft = next;
}

function addMonth() {
  const month: CalendarMonth = { id: newId("month"), name: `Month ${draft.months.length + 1}`, days: 30 };
  setDraft({ ...draft, months: [...draft.months, month] });
}

function addWeekday() {
  const weekday: CalendarNamedUnit = { id: newId("weekday"), name: `Day ${draft.weekdays.length + 1}` };
  setDraft({ ...draft, weekdays: [...draft.weekdays, weekday] });
}

function addSeason() {
  const season: CalendarSeason = {
    id: newId("season"),
    name: `Season ${draft.seasons.length + 1}`,
    startMonth: 1,
    startDay: 1,
    endMonth: Math.max(1, draft.months.length),
    endDay: 1,
  };
  setDraft({ ...draft, seasons: [...draft.seasons, season] });
}

function move<T>(list: T[], index: number, delta: number): T[] {
  const next = index + delta;
  if (next < 0 || next >= list.length) return list;
  const copy = [...list];
  const [item] = copy.splice(index, 1);
  copy.splice(next, 0, item);
  return copy;
}

function integerField(raw: string, fallback: number, min = 1) {
  const parsed = Number(raw);
  return Number.isInteger(parsed) ? Math.max(min, parsed) : fallback;
}

function setEpoch(part: "year" | "month" | "day", raw: string) {
  const epoch = draft.epoch ?? { year: 1, month: 1, day: 1 };
  const next = { year: epoch.year, month: epoch.month ?? 1, day: epoch.day ?? 1 };
  next[part] = integerField(raw, next[part], part === "year" ? Number.MIN_SAFE_INTEGER : 1);
  setDraft({ ...draft, epoch: next });
}

$effect(() => {
  void entityId;
  open = false;
  void load();
});

$effect(() => {
  if (!open) return;
  const onKey = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      closeModal();
    }
  };
  window.addEventListener("keydown", onKey, true);
  return () => window.removeEventListener("keydown", onKey, true);
});
</script>

<section class="calendar-summary" aria-label="Calendar structure">
  <div class="calendar-summary-head">
    <h3>Calendar</h3>
    <button type="button" onclick={openModal}>Configure calendar</button>
  </div>
  <p>{calendarSummary(saved)}</p>
  <p>{epochSummary(saved)}</p>
  {#if error && !open}<p class="calendar-error" role="alert">{error}</p>{/if}
</section>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="calendar-backdrop" role="presentation" onclick={closeModal}>
    <div
      class="calendar-modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="calendar-modal-title"
      tabindex="-1"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => {
        if (event.key === "Escape") closeModal();
      }}>
      <div class="calendar-modal-heading">
        <div>
          <span>TIMELINE</span>
          <strong id="calendar-modal-title">Configure calendar</strong>
          <p>Pick a year shape, then edit months, epoch, and how dates should read.</p>
        </div>
        <button type="button" class="calendar-close" aria-label="Close calendar configuration" onclick={closeModal}
          >×</button>
      </div>
      <div class="calendar-modal-body">
        {#if error}<p class="calendar-error" role="alert">{error}</p>{/if}
        {#each issues as issue}
          <p class={issue.level === "error" ? "calendar-error" : "calendar-note"}>{issue.message}</p>
        {/each}

        <section>
          <div class="calendar-group-head">
            <h4>Year</h4>
            <small>{yearLength ? `${yearLength} days` : "Add months to compute the year length"}</small>
          </div>
          <div class="calendar-presets">
            {#each YEAR_PRESETS as preset}
              <button
                type="button"
                class:selected={activePreset === preset.id}
                onclick={() => setDraft(applyYearPreset(preset.id as YearPresetId, draft))}>
                <strong>{preset.name}</strong>
                <small>{preset.description}</small>
              </button>
            {/each}
          </div>
          <div class="calendar-group-head">
            <h4>Months</h4>
            <button type="button" onclick={addMonth}>Add month</button>
          </div>
          {#if draft.months.length === 0}
            <p class="calendar-note">No months yet. Choose a preset or add months to define the year.</p>
          {:else}
            <ol>
              {#each draft.months as month, index (month.id)}
                <li>
                  <input
                    aria-label={`Month ${index + 1} name`}
                    value={month.name}
                    oninput={(event) => {
                      const months = draft.months.map((item, itemIndex) =>
                        itemIndex === index ? { ...item, name: event.currentTarget.value } : item,
                      );
                      setDraft({ ...draft, months });
                    }} />
                  <input
                    aria-label={`${month.name} days`}
                    type="number"
                    min="1"
                    value={month.days}
                    onchange={(event) => {
                      const days = integerField(event.currentTarget.value, month.days);
                      const months = draft.months.map((item, itemIndex) =>
                        itemIndex === index ? { ...item, days } : item,
                      );
                      setDraft({ ...draft, months });
                    }} />
                  <span>days</span>
                  <button
                    type="button"
                    disabled={index === 0}
                    onclick={() => setDraft({ ...draft, months: move(draft.months, index, -1) })}>Up</button>
                  <button
                    type="button"
                    disabled={index === draft.months.length - 1}
                    onclick={() => setDraft({ ...draft, months: move(draft.months, index, 1) })}>Down</button>
                  <button
                    type="button"
                    onclick={() =>
                      setDraft({ ...draft, months: draft.months.filter((_, itemIndex) => itemIndex !== index) })}
                    >Remove</button>
                </li>
              {/each}
            </ol>
          {/if}
        </section>

        <section>
          <h4>Epoch</h4>
          <p class="calendar-note">
            Year numbering and the Gregorian timeline day that counts as the first day of that year.
          </p>
          <div class="calendar-grid">
            <label>
              First year number
              <input
                type="number"
                value={draft.startingYear ?? 1}
                onchange={(event) =>
                  setDraft({
                    ...draft,
                    startingYear: integerField(event.currentTarget.value, 1, Number.MIN_SAFE_INTEGER),
                  })} />
            </label>
            <label>
              Gregorian year
              <input
                type="number"
                value={draft.epoch?.year ?? 1}
                onchange={(event) => setEpoch("year", event.currentTarget.value)} />
            </label>
            <label>
              Month
              <input
                type="number"
                min="1"
                max="12"
                value={draft.epoch?.month ?? 1}
                onchange={(event) => setEpoch("month", event.currentTarget.value)} />
            </label>
            <label>
              Day
              <input
                type="number"
                min="1"
                max="31"
                value={draft.epoch?.day ?? 1}
                onchange={(event) => setEpoch("day", event.currentTarget.value)} />
            </label>
          </div>
        </section>

        <section>
          <div class="calendar-group-head">
            <h4>Date display</h4>
            <div class="calendar-format-help">
              <button
                type="button"
                class="calendar-help"
                aria-label="Date format guide"
                aria-describedby="calendar-format-help">
                ?
              </button>
              <div id="calendar-format-help" role="tooltip" class="calendar-help-box">
                <p>Write a pattern with these tokens. Spaces, slashes, dashes, and commas stay as you type them.</p>
                <p>
                  If a date has no month, day, weekday, or season, that token is dropped and leftover punctuation is
                  cleaned up.
                </p>
                <dl>
                  {#each DATE_FORMAT_GUIDE as item}
                    <div>
                      <dt><code>{item.token}</code></dt>
                      <dd>{item.meaning}</dd>
                    </div>
                  {/each}
                </dl>
              </div>
            </div>
          </div>
          <div class="calendar-presets">
            {#each DATE_FORMAT_PRESETS as preset}
              <button
                type="button"
                class:selected={(draft.dateFormat || DEFAULT_DATE_FORMAT) === preset.pattern}
                onclick={() => setDraft({ ...draft, dateFormat: preset.pattern })}>
                <strong>{preset.label}</strong>
              </button>
            {/each}
          </div>
          <label class="calendar-format">
            Format
            <input
              value={draft.dateFormat ?? DEFAULT_DATE_FORMAT}
              placeholder="D MMMM YYYY"
              oninput={(event) => setDraft({ ...draft, dateFormat: event.currentTarget.value })} />
          </label>
          <p class="calendar-preview"><span>Preview</span> {preview}</p>
        </section>

        <section>
          <div class="calendar-group-head">
            <h4>Weekdays</h4>
            <button type="button" onclick={addWeekday}>Add weekday</button>
          </div>
          {#if draft.weekdays.length === 0}
            <p class="calendar-note">No week. Dates omit weekday names.</p>
          {:else}
            <ol>
              {#each draft.weekdays as weekday, index (weekday.id)}
                <li>
                  <input
                    aria-label={`Weekday ${index + 1} name`}
                    value={weekday.name}
                    oninput={(event) => {
                      const weekdays = draft.weekdays.map((item, itemIndex) =>
                        itemIndex === index ? { ...item, name: event.currentTarget.value } : item,
                      );
                      setDraft({ ...draft, weekdays });
                    }} />
                  <button
                    type="button"
                    disabled={index === 0}
                    onclick={() => setDraft({ ...draft, weekdays: move(draft.weekdays, index, -1) })}>Up</button>
                  <button
                    type="button"
                    disabled={index === draft.weekdays.length - 1}
                    onclick={() => setDraft({ ...draft, weekdays: move(draft.weekdays, index, 1) })}>Down</button>
                  <button
                    type="button"
                    onclick={() =>
                      setDraft({ ...draft, weekdays: draft.weekdays.filter((_, itemIndex) => itemIndex !== index) })}
                    >Remove</button>
                </li>
              {/each}
            </ol>
          {/if}
        </section>

        <section>
          <div class="calendar-group-head">
            <h4>Seasons</h4>
            <button type="button" onclick={addSeason}>Add season</button>
          </div>
          {#if draft.seasons.length === 0}
            <p class="calendar-note">No seasons. They can span months independently.</p>
          {:else}
            <ol>
              {#each draft.seasons as season, index (season.id)}
                <li class="calendar-season">
                  <input
                    aria-label={`Season ${index + 1} name`}
                    value={season.name}
                    oninput={(event) => {
                      const seasons = draft.seasons.map((item, itemIndex) =>
                        itemIndex === index ? { ...item, name: event.currentTarget.value } : item,
                      );
                      setDraft({ ...draft, seasons });
                    }} />
                  <label
                    >From month<input
                      type="number"
                      min="1"
                      value={season.startMonth}
                      onchange={(event) => {
                        const startMonth = integerField(event.currentTarget.value, season.startMonth);
                        setDraft({
                          ...draft,
                          seasons: draft.seasons.map((item, itemIndex) =>
                            itemIndex === index ? { ...item, startMonth } : item,
                          ),
                        });
                      }} /></label>
                  <label
                    >Day<input
                      type="number"
                      min="1"
                      value={season.startDay}
                      onchange={(event) => {
                        const startDay = integerField(event.currentTarget.value, season.startDay);
                        setDraft({
                          ...draft,
                          seasons: draft.seasons.map((item, itemIndex) =>
                            itemIndex === index ? { ...item, startDay } : item,
                          ),
                        });
                      }} /></label>
                  <label
                    >To month<input
                      type="number"
                      min="1"
                      value={season.endMonth}
                      onchange={(event) => {
                        const endMonth = integerField(event.currentTarget.value, season.endMonth);
                        setDraft({
                          ...draft,
                          seasons: draft.seasons.map((item, itemIndex) =>
                            itemIndex === index ? { ...item, endMonth } : item,
                          ),
                        });
                      }} /></label>
                  <label
                    >Day<input
                      type="number"
                      min="1"
                      value={season.endDay}
                      onchange={(event) => {
                        const endDay = integerField(event.currentTarget.value, season.endDay);
                        setDraft({
                          ...draft,
                          seasons: draft.seasons.map((item, itemIndex) =>
                            itemIndex === index ? { ...item, endDay } : item,
                          ),
                        });
                      }} /></label>
                  <button
                    type="button"
                    onclick={() =>
                      setDraft({ ...draft, seasons: draft.seasons.filter((_, itemIndex) => itemIndex !== index) })}
                    >Remove</button>
                </li>
              {/each}
            </ol>
          {/if}
        </section>
      </div>
      <div class="calendar-modal-actions">
        <button type="button" onclick={closeModal}>Cancel</button>
        <button type="button" class="primary" disabled={saving || errors.length > 0} onclick={() => void persist()}>
          {saving ? "Saving…" : "Save"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
.calendar-summary,
.calendar-modal-body {
  display: grid;
  gap: 10px;
}
.calendar-summary {
  padding: 4px 0 8px;
}
.calendar-summary-head,
.calendar-group-head,
.calendar-modal-heading,
.calendar-modal-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.calendar-summary h3,
.calendar-modal-body h4 {
  margin: 0;
  color: #302c26;
  font:
    650 12px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.calendar-summary p,
.calendar-note,
.calendar-modal-heading p {
  margin: 0;
  color: #8f897e;
  font:
    12px/1.45 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.calendar-error {
  margin: 0;
  color: #a14f42;
  font:
    12px/1.45 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.calendar-backdrop {
  position: fixed;
  inset: 0;
  z-index: 40;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(37, 37, 31, 0.28);
}
.calendar-modal {
  display: flex;
  flex-direction: column;
  width: min(860px, 100%);
  max-height: min(760px, calc(100vh - 32px));
  overflow: hidden;
  border: 1px solid #e3d9ca;
  border-radius: 14px;
  background: #fffefa;
  box-shadow: 0 22px 70px rgba(37, 37, 31, 0.2);
}
.calendar-modal-heading {
  align-items: flex-start;
  padding: 22px 24px 16px;
  border-bottom: 1px solid #efe7db;
}
.calendar-modal-heading span {
  color: #8f897e;
  font:
    700 10px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  letter-spacing: 0.06em;
}
.calendar-modal-heading strong {
  display: block;
  margin-top: 4px;
  font:
    500 24px/1.15 Georgia,
    serif;
}
.calendar-modal-body {
  gap: 18px;
  overflow: auto;
  padding: 18px 24px;
}
.calendar-modal-body section {
  display: grid;
  gap: 10px;
}
.calendar-presets {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 8px;
}
.calendar-presets button {
  display: grid;
  gap: 4px;
  padding: 10px;
  text-align: left;
}
.calendar-presets button.selected,
.calendar-presets button:hover {
  border-color: #b4773f;
}
.calendar-presets small {
  color: #8f897e;
  font-weight: 500;
}
.calendar-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
}
.calendar-grid label,
.calendar-format,
.calendar-season label {
  display: grid;
  gap: 4px;
  color: #62594e;
  font:
    600 10px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.calendar-format-help {
  position: relative;
}
.calendar-help {
  width: 28px;
  height: 28px;
  padding: 0;
  border-radius: 999px;
  font:
    650 14px Georgia,
    serif;
  line-height: 1;
}
.calendar-help-box {
  display: none;
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  z-index: 5;
  width: min(340px, calc(100vw - 80px));
  padding: 12px 14px;
  border: 1px solid #e3d9ca;
  border-radius: 10px;
  background: #fffefa;
  box-shadow: 0 12px 32px rgba(37, 37, 31, 0.16);
  color: #302c26;
  font:
    12px/1.45 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.calendar-format-help:hover .calendar-help-box,
.calendar-format-help:focus-within .calendar-help-box {
  display: grid;
  gap: 8px;
}
.calendar-help-box p {
  margin: 0;
  color: #62594e;
}
.calendar-help-box dl {
  display: grid;
  gap: 6px;
  margin: 0;
}
.calendar-help-box dl div {
  display: grid;
  grid-template-columns: 4.5rem 1fr;
  gap: 8px;
  align-items: start;
}
.calendar-help-box dt {
  margin: 0;
}
.calendar-help-box code {
  color: #b4773f;
  font:
    650 12px ui-monospace,
    SFMono-Regular,
    Menlo,
    monospace;
}
.calendar-help-box dd {
  margin: 0;
  color: #302c26;
}
.calendar-preview {
  margin: 0;
  padding: 10px 12px;
  border: 1px solid #efe7db;
  border-radius: 8px;
  background: #fcf8f1;
  color: #302c26;
  font:
    500 13px Georgia,
    serif;
}
.calendar-preview span {
  display: block;
  margin-bottom: 4px;
  color: #8f897e;
  font:
    700 10px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.calendar-modal-body ol {
  display: grid;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.calendar-modal-body li {
  display: grid;
  grid-template-columns: minmax(0, 1.4fr) 72px auto auto auto auto;
  gap: 6px;
  align-items: center;
}
.calendar-season {
  grid-template-columns: minmax(0, 1fr);
}
.calendar-modal input,
.calendar-modal button,
.calendar-summary button {
  border: 1px solid #d9cdbd;
  border-radius: 7px;
  background: #fffefa;
  color: #55351f;
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.calendar-modal input {
  min-width: 0;
  padding: 6px 8px;
}
.calendar-modal button,
.calendar-summary button {
  padding: 6px 9px;
  cursor: pointer;
}
.calendar-modal button:disabled {
  opacity: 0.45;
  cursor: default;
}
.calendar-close {
  width: 30px;
  height: 30px;
  border: 0;
  background: #f4eee3;
  font-size: 20px;
}
.calendar-modal-actions {
  justify-content: flex-end;
  padding: 14px 24px 18px;
  border-top: 1px solid #efe7db;
}
.calendar-modal-actions .primary {
  border-color: #365342;
  background: #365342;
  color: #fff;
}
@media (max-width: 720px) {
  .calendar-grid,
  .calendar-modal-body li {
    grid-template-columns: 1fr;
  }
}
</style>
