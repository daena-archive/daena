<script lang="ts">
import { Search } from "@lucide/svelte";
import { GREGORIAN_CALENDAR_ID } from "$lib/date";
import type { Entity } from "$lib/project/client";

let {
  selectedId,
  calendars,
  onSelect,
  label = "Calendar",
}: {
  selectedId: string;
  calendars: Entity[];
  onSelect: (id: string) => void;
  label?: string;
} = $props();

let query = $state("");
let open = $state(false);
let rootEl: HTMLDivElement | null = $state(null);

$effect(() => {
  if (!open) return;
  const handler = (event: MouseEvent) => {
    const target = event.target as Node | null;
    const el = rootEl as unknown as HTMLElement | null;
    if (el && target && !el.contains(target)) open = false;
  };
  document.addEventListener("pointerdown", handler, true);
  return () => document.removeEventListener("pointerdown", handler, true);
});

function isGregorian(id: string) {
  return id === GREGORIAN_CALENDAR_ID;
}

function selectedName() {
  if (isGregorian(selectedId)) return "Default";
  const found = calendars.find((c) => c.id === selectedId);
  return found ? found.name : selectedId;
}

function candidates() {
  const q = query.trim().toLowerCase();
  const all: Array<{ id: string; name: string; sub?: string }> = [
    { id: GREGORIAN_CALENDAR_ID, name: "Default", sub: "Default calendar" },
    ...calendars
      .filter((e) => !e.deleted)
      .map((e) => ({ id: e.id, name: e.name, sub: e.entity_type ?? "daena.timeline:calendar" })),
  ];
  if (!q) return all;
  return all.filter((c) => `${c.name} ${c.sub ?? ""}`.toLowerCase().includes(q));
}

function select(id: string) {
  onSelect(id);
  query = "";
  open = false;
}
</script>

<div
  bind:this={rootEl}
  class="calendar-picker"
  onfocusout={(event) => {
    const next = event.relatedTarget as Node | null;
    const root = event.currentTarget as HTMLElement;
    if (next && root.contains(next)) return;
    window.setTimeout(() => {
      if (!root.contains(document.activeElement)) open = false;
    }, 0);
  }}>
  <label class="calendar-picker-label" for="calendar-picker-trigger">{label}</label>
  <button
    id="calendar-picker-trigger"
    type="button"
    class="calendar-picker-trigger"
    aria-haspopup="listbox"
    aria-expanded={open}
    onclick={() => (open = !open)}>
    <span class="calendar-picker-value">{selectedName()}</span>
    <span class="calendar-picker-chevron" aria-hidden="true">{open ? "▴" : "▾"}</span>
  </button>

  {#if open}
    <div class="calendar-picker-menu" role="listbox" aria-label="{label} options">
      <div class="calendar-picker-search">
        <span aria-hidden="true"><Search size={14} strokeWidth={1.8} aria-hidden="true" /></span>
        <input
          type="text"
          placeholder="Search calendars…"
          aria-label="Search calendars"
          value={query}
          onfocus={() => (open = true)}
          oninput={(e) => {
            query = (e.currentTarget as HTMLInputElement).value;
            open = true;
          }}
          onkeydown={(e) => {
            if (e.key === "Escape") {
              e.preventDefault();
              open = false;
              (document.getElementById("calendar-picker-trigger") as HTMLElement | null)?.focus();
            }
          }} />
      </div>
      <div class="calendar-picker-options">
        {#each candidates() as cal}
          <button
            type="button"
            role="option"
            aria-selected={cal.id === selectedId}
            class:selected={cal.id === selectedId}
            onpointerdown={(e) => {
              e.preventDefault();
              select(cal.id);
            }}
            onkeydown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                select(cal.id);
              }
            }}>
            <span>
              <strong>{cal.name}</strong>
              {#if cal.sub}<small>{cal.sub}</small>{/if}
            </span>
            {#if cal.id === selectedId}<b aria-hidden="true">✓</b>{/if}
          </button>
        {:else}
          <small class="calendar-picker-empty">No matching calendars.</small>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
.calendar-picker {
  position: relative;
  display: grid;
  gap: 4px;
  width: 100%;
  min-width: 0;
}
.calendar-picker-label {
  color: var(--ink-faint);
  font-size: 9px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.calendar-picker-trigger {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--canvas);
  color: var(--ink);
  font-size: 11px;
  text-align: left;
  cursor: pointer;
}
.calendar-picker-trigger:focus-visible {
  border-color: var(--accent-soft);
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.1);
  outline: none;
}
.calendar-picker-value {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.calendar-picker-chevron {
  flex: none;
  color: var(--ink-faint);
  font-size: 10px;
}
.calendar-picker-menu {
  position: absolute;
  left: 0;
  right: auto;
  top: calc(100% + 6px);
  z-index: 12;
  min-width: 100%;
  width: max-content;
  max-width: min(300px, calc(100vw - 24px));
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  box-shadow: var(--shadow-lg);
}
.calendar-picker-search {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 8px 9px;
  border-bottom: 1px solid var(--line);
  color: var(--ink-faint);
}
.calendar-picker-search input {
  flex: 1;
  min-width: 0;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--ink);
  font-size: 11px;
}
.calendar-picker-options {
  max-height: 180px;
  overflow-y: auto;
  padding: 4px;
}
.calendar-picker-options button {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  width: 100%;
  padding: 8px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-soft);
  text-align: left;
  cursor: pointer;
}
.calendar-picker-options button:hover,
.calendar-picker-options button.selected {
  background: var(--surface-muted);
  color: var(--ink);
}
.calendar-picker-options strong,
.calendar-picker-options small {
  display: block;
}
.calendar-picker-options strong {
  font-size: 11px;
}
.calendar-picker-options small {
  margin-top: 2px;
  color: var(--ink-faint);
  font-size: 9px;
}
.calendar-picker-options button > b {
  color: var(--accent);
}
.calendar-picker-empty {
  display: block;
  padding: 10px 8px;
  color: var(--ink-faint);
  font-size: 10px;
}
</style>
