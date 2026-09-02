<script lang="ts">
import type { Snippet } from "svelte";
import type { CollectionViewMode } from "$lib/modules/workspace";

interface Props {
  kicker: string;
  count: number;
  label: string;
  viewMode: CollectionViewMode;
  groupedAriaLabel?: string;
  controls: Snippet;
  children: Snippet;
  footer?: Snippet;
  hidden?: boolean;
  allowOverflow?: boolean;
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
  groupedAriaLabel = "Grouped by type",
  controls,
  children,
  footer,
  hidden = false,
  allowOverflow = false,
  element = $bindable(null),
  listElement = $bindable(null),
  onViewModeChange,
  onScroll,
}: Props = $props();
</script>

<aside
  bind:this={element}
  class:hidden
  class:allow-overflow={allowOverflow}
  class="collection-panel panel-surface"
  aria-label={`${label} collection`}>
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
        aria-label={groupedAriaLabel}
        onclick={() => onViewModeChange("grouped")}>⊟</button>
    </div>
  </div>

  {@render controls()}
  <div class="collection-list" class:allow-overflow={allowOverflow} bind:this={listElement} onscroll={onScroll}>
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
/* Empty-state create menus extend past the pane; don't clip them. */
.collection-panel.allow-overflow,
.collection-panel:has(:global(.entity-empty)) {
  overflow: visible;
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
  display: grid;
  min-height: 0;
  flex: 1;
  align-content: start;
  gap: 8px;
  overflow-y: auto;
  padding: 4px 12px 14px;
}
.collection-list.allow-overflow,
.collection-list:has(:global(.entity-empty)) {
  overflow: visible;
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
