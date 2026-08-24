<script lang="ts">
import type { Snippet } from "svelte";
import type { CollectionViewMode } from "$lib/modules/workspace";

interface Props {
  kicker: string;
  count: number;
  label: string;
  viewMode: CollectionViewMode;
  controls: Snippet;
  children: Snippet;
  footer?: Snippet;
  hidden?: boolean;
  element?: HTMLElement | null;
  listElement?: HTMLDivElement | null;
  onViewModeChange: (mode: CollectionViewMode) => void;
  onScroll: () => void;
}

let {
  kicker,
  count,
  label,
  viewMode,
  controls,
  children,
  footer,
  hidden = false,
  element = $bindable(null),
  listElement = $bindable(null),
  onViewModeChange,
  onScroll,
}: Props = $props();
</script>

<aside bind:this={element} class:hidden class="collection-panel panel-surface" aria-label={`${label} collection`}>
  <div class="panel-heading">
    <div><span class="panel-kicker">{kicker}</span><strong>{count} {label}</strong></div>
    <div class="view-mode-toggle">
      <button
        type="button"
        class:active={viewMode === "flat"}
        aria-label="Flat list"
        onclick={() => onViewModeChange("flat")}>≡</button
      ><button
        type="button"
        class:active={viewMode === "grouped"}
        aria-label="Grouped by type"
        onclick={() => onViewModeChange("grouped")}>⊟</button>
    </div>
  </div>

  {@render controls()}
  <div class="collection-list" bind:this={listElement} onscroll={onScroll}>
    {@render children()}
  </div>
  {#if footer}{@render footer()}{/if}
</aside>

<style>
.collection-panel {
  display: flex;
  min-width: 0;
  min-height: 650px;
  flex-direction: column;
  overflow: clip;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  box-shadow: var(--shadow-sm);
}
.hidden {
  display: none;
}
.panel-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 18px 17px 12px;
}
.panel-heading > div {
  min-width: 0;
}
.panel-heading strong {
  display: block;
  margin-top: 5px;
  font: 500 28px var(--font-display);
}
.view-mode-toggle {
  display: flex;
  gap: 3px;
  padding: 2px;
  border-radius: 7px;
  background: var(--canvas);
}
.view-mode-toggle button {
  display: grid;
  width: 25px;
  height: 23px;
  padding: 0;
  place-items: center;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--ink-faint);
  cursor: pointer;
}
.view-mode-toggle button:hover,
.view-mode-toggle button.active {
  background: var(--surface);
  box-shadow: var(--shadow-sm);
  color: var(--accent);
}
.collection-list {
  min-height: 0;
  flex: 1;
  overflow-y: auto;
}
@media (max-width: 760px) {
  .collection-panel {
    width: 100%;
    min-height: auto;
    border-radius: 11px;
  }
  .collection-list {
    max-height: 320px;
    -webkit-overflow-scrolling: touch;
  }
  .panel-heading strong {
    font-size: 24px;
  }
}
</style>
