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

const step = $derived(result ? 3 : arming && startPicked ? 2 : arming ? 1 : 0);
</script>

{#snippet pick()}
  <ol class="steps" aria-label="Route steps">
    <li class:done={step > 1} class:active={step === 1}>Start</li>
    <li class:done={step > 2} class:active={step === 2}>End</li>
    <li class:done={step > 3} class:active={step === 3}>Choose</li>
  </ol>
  <p class="section-note">{hint()}</p>
  <div class="actions">
    <button type="button" class="primary" disabled={disabled || searching} onclick={onarm}>
      {arming ? (startPicked ? "Waiting for end…" : "Waiting for start…") : "Pick start and end"}
    </button>
    <button type="button" disabled={disabled || (!arming && !searching && !result && !error)} onclick={oncancel}>
      Cancel
    </button>
  </div>
  {#if error}
    <p class="section-note route-error">{error}</p>
  {/if}
{/snippet}

{#snippet suggestions()}
  {#if result && result.suggestions.length === 0}
    <p class="empty-note">No land path between those points. Try different endpoints.</p>
  {:else if result}
    <ul class="route-results">
      {#each result.suggestions as suggestion (suggestion.id)}
        <li class:active={selectedId === suggestion.id}>
          <button type="button" class="route-candidate" onclick={() => onselect(suggestion)}>
            <strong>{suggestion.label}</strong>
            <span>{km(suggestion.lengthM)} · {suggestion.climbM.toLocaleString("en-US")} m climb</span>
            <span>{suggestion.tradeoff}</span>
            {#each suggestion.reasons.slice(1) as reason}
              <span>{reason}</span>
            {/each}
          </button>
          {#if selectedId === suggestion.id}
            <button type="button" class="primary" {disabled} onclick={() => onaccept(suggestion)}
              >Accept as road</button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
{/snippet}

{#if variant === "studio"}
  <section class="route-suggest studio" aria-label="Suggested routes">
    <div class="block">
      <span class="kicker">Path</span>
      {@render pick()}
    </div>
    {#if result}
      <div class="block">
        <span class="kicker">Suggestions</span>
        {@render suggestions()}
      </div>
    {/if}
  </section>
{:else}
  <details class="route-suggest map-section-group" open={arming || searching || Boolean(result) || Boolean(error)}>
    <summary>
      <ChevronRight size={14} strokeWidth={1.8} aria-hidden="true" />
      <strong>Suggested routes</strong>
      {#if result}
        <span class="section-count">{result.suggestionCount}</span>
      {/if}
    </summary>
    <div class="section-body">
      {@render pick()}
      {@render suggestions()}
    </div>
  </details>
{/if}

<style>
.route-suggest.studio {
  display: grid;
  gap: 10px;
}
.block {
  display: grid;
  gap: 8px;
  padding: 10px;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 10px;
  background: rgb(255 255 255 / 3%);
}
.kicker {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #d5ab6c;
}
.section-body {
  display: grid;
  gap: 8px;
}
.steps {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 4px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.steps li {
  min-height: 26px;
  padding: 4px 6px;
  border-radius: 7px;
  border: 1px solid rgb(255 255 255 / 10%);
  background: #0f1a16;
  color: #aebdb1;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  text-align: center;
}
.steps li.active {
  border-color: #d5ab6c;
  color: #1b2822;
  background: #d5ab6c;
}
.steps li.done {
  border-color: rgb(213 171 108 / 45%);
  color: #edf2ec;
}
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}
.actions button,
.primary {
  min-height: 26px;
  padding: 4px 8px;
  border: 1px solid var(--theme-neutral-border-strong, #405047);
  border-radius: 7px;
  background: #0f1a16;
  color: #edf2ec;
  font-size: 11px;
  cursor: pointer;
}
.primary {
  border-color: #d5ab6c;
  background: #d5ab6c;
  color: #1b2822;
}
.actions button:disabled,
.primary:disabled {
  opacity: 0.45;
  cursor: default;
}
.section-note,
.empty-note {
  margin: 0;
  color: var(--theme-neutral-text-muted, #aebdb1);
  font-size: 11px;
  line-height: 1.45;
}
.empty-note {
  padding: 10px;
  border: 1px dashed rgb(255 255 255 / 14%);
  border-radius: 8px;
}
.route-error {
  color: var(--danger, #c45c48);
}
.route-results {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 6px;
}
.route-results li {
  display: grid;
  gap: 6px;
  padding: 4px;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 8px;
  background: rgb(0 0 0 / 16%);
}
.route-results li.active {
  border-color: #d5ab6c;
  background: rgb(213 171 108 / 16%);
}
.route-candidate {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
  text-align: left;
  padding: 4px 6px;
  border: 0;
  background: none;
  color: inherit;
  cursor: pointer;
}
.route-candidate span {
  font-size: 11px;
  color: #aebdb1;
}
</style>
