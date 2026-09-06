<script lang="ts">
import { ChevronRight } from "@lucide/svelte";
import type { RouteSuggestion, RouteSuggestResult } from "$lib/project/types";

let {
  variant = "editor",
  disabled = false,
  searching = false,
  error = "",
  arming = false,
  startPicked = false,
  result = null,
  selectedId = null,
  onarm,
  onselect,
  onaccept,
  oncancel,
}: {
  variant?: "editor" | "studio";
  disabled?: boolean;
  searching?: boolean;
  error?: string;
  arming?: boolean;
  startPicked?: boolean;
  result?: RouteSuggestResult | null;
  selectedId?: number | null;
  onarm: () => void;
  onselect: (suggestion: RouteSuggestion) => void;
  onaccept: (suggestion: RouteSuggestion) => void;
  oncancel: () => void;
} = $props();

const selected = $derived(result?.suggestions.find((item) => item.id === selectedId) ?? null);

function km(metres: number) {
  return metres >= 10_000 ? `${Math.round(metres / 1_000)} km` : `${metres.toLocaleString("en-US")} m`;
}

function hint() {
  if (searching) return "Finding land routes…";
  if (arming && startPicked) return "Start set. Click the end on land.";
  if (arming) return "Click the start, then the end. Ocean is blocked.";
  if (result)
    return `${result.suggestionCount} suggestion${result.suggestionCount === 1 ? "" : "s"}. Accept writes a road.`;
  return "Two clicks on land. Suggestions stay disposable until you accept one.";
}
</script>

<details
  class="route-suggest"
  class:studio={variant === "studio"}
  class:map-section-group={variant !== "studio"}
  open={arming || searching || Boolean(result) || Boolean(error)}>
  <summary>
    {#if variant !== "studio"}
      <ChevronRight size={14} strokeWidth={1.8} aria-hidden="true" />
    {/if}
    <strong>Suggested routes</strong>
    {#if result}
      <span class="section-count">{result.suggestionCount}</span>
    {/if}
  </summary>
  <div class="section-body">
    <p class="section-note">{hint()}</p>
    <div class="quick-add-row">
      <button type="button" class="primary-button small" disabled={disabled || searching} onclick={onarm}>
        {arming ? (startPicked ? "Waiting for end…" : "Waiting for start…") : "Pick start and end"}
      </button>
      <button
        type="button"
        class="quiet-button small"
        disabled={disabled || (!arming && !searching && !result && !error)}
        onclick={oncancel}>Cancel</button>
    </div>
    {#if error}
      <p class="section-note route-error">{error}</p>
    {/if}
    {#if result && result.suggestions.length === 0}
      <p class="empty-note">No land path between those points.</p>
    {:else if result}
      <ul class="route-results">
        {#each result.suggestions as suggestion (suggestion.id)}
          <li>
            <button
              type="button"
              class="route-candidate"
              class:selected={selectedId === suggestion.id}
              onclick={() => onselect(suggestion)}>
              <strong>{suggestion.label}</strong>
              <span>{km(suggestion.lengthM)} · {suggestion.climbM.toLocaleString("en-US")} m climb</span>
              <span>{suggestion.tradeoff}</span>
              {#each suggestion.reasons.slice(1) as reason}
                <span>{reason}</span>
              {/each}
            </button>
          </li>
        {/each}
      </ul>
      <div class="quick-add-row">
        <button
          type="button"
          class="primary-button small"
          disabled={disabled || !selected}
          onclick={() => selected && onaccept(selected)}>Accept as road</button>
      </div>
    {/if}
  </div>
</details>

<style>
.route-suggest.studio {
  display: grid;
  gap: 0;
}
.route-suggest.studio > summary {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  list-style: none;
  font-size: 12px;
}
.route-suggest.studio > summary::-webkit-details-marker {
  display: none;
}
.route-suggest.studio > summary::before {
  content: "";
  width: 0.4em;
  height: 0.4em;
  border-right: 1.5px solid currentColor;
  border-bottom: 1.5px solid currentColor;
  transform: rotate(-45deg);
  opacity: 0.7;
}
.route-suggest.studio[open] > summary::before {
  transform: rotate(45deg);
}
.route-suggest.studio .section-count {
  margin-left: auto;
  min-width: 20px;
  padding: 2px 6px;
  border-radius: 999px;
  background: color-mix(in srgb, currentColor 14%, transparent);
  font-size: 10px;
  font-weight: 700;
  text-align: center;
}
.route-suggest.studio .section-body {
  display: grid;
  gap: 8px;
  padding-top: 8px;
}
.quick-add-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}
.quiet-button.small,
.primary-button.small {
  min-height: 26px;
  padding: 0 8px;
  border-radius: 7px;
  font-weight: 650;
  font-size: 11px;
  cursor: pointer;
}
.quiet-button.small {
  border: 1px solid var(--line, color-mix(in srgb, currentColor 18%, transparent));
  background: var(--surface, color-mix(in srgb, currentColor 8%, transparent));
  color: inherit;
}
.quiet-button.small:hover:not(:disabled) {
  background: var(--surface-muted, color-mix(in srgb, currentColor 12%, transparent));
}
.primary-button.small {
  border: 1px solid var(--accent, #b4773f);
  background: var(--accent, #b4773f);
  color: var(--on-accent, #fffefa);
}
.quiet-button.small:disabled,
.primary-button.small:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.section-note,
.empty-note {
  margin: 0;
  font-size: 11px;
  line-height: 1.45;
  opacity: 0.82;
}
.empty-note {
  padding: 10px 11px;
  border: 1px dashed var(--line, color-mix(in srgb, currentColor 18%, transparent));
  border-radius: 8px;
  text-align: center;
}
.route-error {
  color: var(--danger, #c45c48);
  opacity: 1;
}
.route-results {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 8px;
}
.route-candidate {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
  text-align: left;
  padding: 7px 8px;
  border: 1px solid var(--line, color-mix(in srgb, currentColor 18%, transparent));
  background: var(--surface, color-mix(in srgb, currentColor 6%, transparent));
  color: inherit;
  border-radius: 8px;
  cursor: pointer;
}
.route-candidate.selected {
  border-color: var(--accent, #e6b03c);
}
.route-candidate span {
  font-size: 11px;
  opacity: 0.82;
}
</style>
