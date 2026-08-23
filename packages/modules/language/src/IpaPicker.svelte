<script lang="ts">
import { onMount } from "svelte";
import {
  IPA_SECTION_LABELS,
  IPA_SECTIONS,
  IPA_SYMBOLS,
  searchIpaSymbols,
  type IpaSection,
  type IpaSymbol,
} from "./ipa";

let {
  onselect,
  onclose,
}: {
  onselect: (symbol: string) => void;
  onclose: () => void;
} = $props();

const RECENT_KEY = "daena.language.ipa-recent.v1";
const RECENT_LIMIT = 10;

let query = $state("");
let recent = $state<string[]>([]);
let searchInput: HTMLInputElement | undefined = $state();

const filtered = $derived(searchIpaSymbols(query));
const recentEntries = $derived(
  recent
    .map((symbol) => IPA_SYMBOLS.find((entry) => entry.symbol === symbol))
    .filter((entry): entry is IpaSymbol => !!entry),
);

onMount(() => {
  try {
    const stored = JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]");
    if (Array.isArray(stored))
      recent = stored.filter((item): item is string => typeof item === "string").slice(0, RECENT_LIMIT);
  } catch {
    recent = [];
  }
  searchInput?.focus();
});

function entries(section: IpaSection) {
  return filtered.filter((entry) => entry.section === section);
}

function groups(section: IpaSection) {
  return [...new Set(entries(section).map((entry) => entry.group))];
}

function choose(entry: IpaSymbol) {
  recent = [entry.symbol, ...recent.filter((symbol) => symbol !== entry.symbol)].slice(0, RECENT_LIMIT);
  try {
    localStorage.setItem(RECENT_KEY, JSON.stringify(recent));
  } catch {
    // Recents are a convenience; symbol insertion must still work if storage is unavailable.
  }
  onselect(entry.symbol);
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key !== "Escape") return;
  event.preventDefault();
  onclose();
}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="ipa-backdrop" role="presentation" onclick={onclose}>
  <div
    class="ipa-dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="ipa-picker-title"
    tabindex="-1"
    onclick={(event) => event.stopPropagation()}>
    <header>
      <div>
        <p>Language tools</p>
        <h2 id="ipa-picker-title">IPA picker</h2>
      </div>
      <button type="button" class="ipa-close" onclick={onclose}>Close</button>
    </header>

    <label class="ipa-search">
      <span>Search by symbol or description</span>
      <input
        bind:this={searchInput}
        bind:value={query}
        type="search"
        placeholder="Try postalveolar, velar nasal, long, or ʃ" />
    </label>

    <div class="ipa-scroll">
      {#if !query && recentEntries.length > 0}
        <section class="ipa-section" aria-labelledby="ipa-recent-title">
          <h3 id="ipa-recent-title">Recently used</h3>
          <div class="ipa-symbols compact">
            {#each recentEntries as entry (entry.symbol)}
              <button type="button" class="ipa-symbol" title={entry.name} onclick={() => choose(entry)}>
                <strong>{entry.display ?? entry.symbol}</strong>
                <span>{entry.name}</span>
              </button>
            {/each}
          </div>
        </section>
      {/if}

      {#if filtered.length === 0}
        <p class="ipa-empty" role="status">No IPA symbols match “{query}”.</p>
      {:else}
        {#each IPA_SECTIONS as section (section)}
          {@const sectionEntries = entries(section)}
          {#if sectionEntries.length > 0}
            <section class="ipa-section" aria-labelledby={`ipa-${section}`}>
              <h3 id={`ipa-${section}`}>{IPA_SECTION_LABELS[section]}</h3>
              <div class:vowel-groups={section === "vowels"} class="ipa-groups">
                {#each groups(section) as group (group)}
                  <div class="ipa-group">
                    <h4>{group}</h4>
                    <div class="ipa-symbols">
                      {#each sectionEntries.filter((entry) => entry.group === group) as entry (`${entry.section}-${entry.symbol}`)}
                        <button type="button" class="ipa-symbol" title={entry.name} onclick={() => choose(entry)}>
                          <strong>{entry.display ?? entry.symbol}</strong>
                          <span>{entry.name}</span>
                        </button>
                      {/each}
                    </div>
                  </div>
                {/each}
              </div>
            </section>
          {/if}
        {/each}
      {/if}
    </div>

    <footer>Select as many symbols as you need. They are inserted at the field’s cursor.</footer>
  </div>
</div>

<style>
.ipa-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1200;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(24, 28, 22, 0.54);
  backdrop-filter: blur(3px);
}
.ipa-dialog {
  display: grid;
  grid-template-rows: auto auto minmax(0, 1fr) auto;
  gap: 14px;
  width: min(980px, 100%);
  max-height: min(820px, calc(100vh - 40px));
  padding: 18px;
  border: 1px solid var(--line);
  border-radius: 16px;
  background: var(--surface);
  color: var(--ink);
  box-shadow: 0 24px 70px rgba(20, 22, 18, 0.3);
}
header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}
header p {
  margin: 0 0 3px;
  color: var(--accent);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}
header h2,
.ipa-section h3,
.ipa-group h4 {
  margin: 0;
}
.ipa-close {
  padding: 7px 11px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: transparent;
  color: var(--ink);
  cursor: pointer;
}
.ipa-search {
  display: grid;
  gap: 6px;
  color: var(--ink-soft);
  font-size: 11px;
}
.ipa-search input {
  width: 100%;
  box-sizing: border-box;
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--surface-muted);
  color: var(--ink);
  font: inherit;
}
.ipa-scroll {
  min-height: 0;
  overflow: auto;
  padding-right: 3px;
}
.ipa-section {
  display: grid;
  gap: 10px;
  padding: 14px 0;
  border-top: 1px solid var(--line);
}
.ipa-section:first-child {
  border-top: 0;
  padding-top: 2px;
}
.ipa-section h3 {
  font-size: 15px;
}
.ipa-groups {
  display: grid;
  gap: 12px;
}
.ipa-groups.vowel-groups {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}
.ipa-group {
  display: grid;
  gap: 6px;
}
.ipa-group h4 {
  color: var(--ink-soft);
  font-size: 11px;
  font-weight: 650;
}
.ipa-symbols {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(118px, 1fr));
  gap: 6px;
}
.ipa-symbols.compact {
  grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
}
.ipa-symbol {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 8px;
  align-items: center;
  min-height: 48px;
  padding: 7px 9px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface-muted);
  color: var(--ink);
  text-align: left;
  cursor: pointer;
}
.ipa-symbol:hover,
.ipa-symbol:focus-visible {
  border-color: var(--accent);
  background: var(--surface);
}
.ipa-symbol:focus-visible,
.ipa-close:focus-visible,
.ipa-search input:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
.ipa-symbol strong {
  min-width: 28px;
  font-family: "Noto Sans", "Charis SIL", system-ui, sans-serif;
  font-size: 22px;
  font-weight: 600;
  text-align: center;
}
.ipa-symbol span {
  color: var(--ink-soft);
  font-size: 10px;
  line-height: 1.25;
}
.ipa-empty,
footer {
  margin: 0;
  color: var(--ink-soft);
  font-size: 11px;
}
footer {
  padding-top: 10px;
  border-top: 1px solid var(--line);
}
@media (max-width: 720px) {
  .ipa-backdrop {
    padding: 0;
  }
  .ipa-dialog {
    width: 100%;
    height: 100%;
    max-height: none;
    border-radius: 0;
  }
  .ipa-groups.vowel-groups {
    grid-template-columns: 1fr;
  }
}
</style>
