<script lang="ts">
import { displayLanguage } from "$lib/i18n";
import { ui } from "$lib/i18n/ui";
import { epochNotice, type LandmassSelection } from "./landmass-selection.ts";

let {
  selection = null,
  canCreate = true,
  canAdd = false,
  busy = false,
  hint = "",
  includeOccupied = $bindable(false),
  showIncludeOccupied = false,
  covered = false,
  oncreate,
  onadd,
  oninvert,
  onclear,
}: {
  selection?: LandmassSelection | null;
  canCreate?: boolean;
  canAdd?: boolean;
  busy?: boolean;
  hint?: string;
  includeOccupied?: boolean;
  showIncludeOccupied?: boolean;
  covered?: boolean;
  oncreate: () => void;
  onadd: () => void;
  oninvert: () => void;
  onclear: () => void;
} = $props();
</script>

{#if hint}
  <p class="note" role="status">{hint}</p>
{/if}
{#if selection}
  <p class="note" role="status">{epochNotice(selection.epochOffsetYears)}</p>
  <p class="note">{$ui("{0} · {1} cells", selection.label, selection.cellCount.toLocaleString($displayLanguage))}</p>
  {#if showIncludeOccupied}
    <label class="include">
      <input type="checkbox" bind:checked={includeOccupied} disabled={busy} /> {$ui("Include existing regions")} </label>
  {/if}
  {#if covered && !includeOccupied}
    <p class="note">{$ui("Existing regions already cover this landmass. Add leftover space, or include existing regions.")}</p>
  {/if}
  <div class="actions">
    <button type="button" disabled={busy || !canCreate} onclick={oncreate}>{$ui("New overlay")}</button>
    <button type="button" disabled={busy || !canAdd} onclick={onadd}>{$ui("Add to overlay")}</button>
    <button type="button" disabled={busy} onclick={oninvert}>{$ui("Invert")}</button>
    <button type="button" disabled={busy} onclick={onclear}>{$ui("Clear")}</button>
  </div>
{/if}

<style>
.note {
  margin: 0;
  color: var(--theme-neutral-text-muted, #aebdb1);
}
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.actions button {
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 6px;
  background: #0f1a16;
  color: #edf2ec;
  padding: 4px 7px;
}
.actions button:disabled {
  opacity: 0.45;
}
.include {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--theme-neutral-text-muted, #aebdb1);
}
</style>
