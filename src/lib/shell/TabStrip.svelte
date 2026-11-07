<script lang="ts">
import type { Snippet } from "svelte";
import type { TabIcon } from "$lib/navigation/tabs";

let {
  tabs,
  panelId,
  onActivate,
  icon,
}: {
  tabs: Array<{
    id: string;
    title: string;
    tooltip: string;
    icon: TabIcon;
    active: boolean;
    pinned: boolean;
  }>;
  panelId: string;
  onActivate: (id: string) => void;
  icon: Snippet<[TabIcon]>;
} = $props();

function moveTabFocus(event: KeyboardEvent) {
  if (event.altKey || event.metaKey || event.ctrlKey || event.shiftKey) return;
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight" && event.key !== "Home" && event.key !== "End") return;
  const list = event.currentTarget;
  if (!(list instanceof HTMLElement)) return;
  const buttons = [...list.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
  const index = buttons.findIndex((button) => button === document.activeElement);
  if (index < 0 || buttons.length === 0) return;
  event.preventDefault();
  const next =
    event.key === "Home"
      ? 0
      : event.key === "End"
        ? buttons.length - 1
        : event.key === "ArrowLeft"
          ? (index - 1 + buttons.length) % buttons.length
          : (index + 1) % buttons.length;
  buttons[next]?.focus();
}

function selectTab(tab: { id: string; active: boolean }) {
  if (tab.active) return;
  onActivate(tab.id);
}
</script>

<div role="tablist" aria-label="Open tabs" class="tab-strip" tabindex="-1" onkeydown={moveTabFocus}>
  {#each tabs as tab (tab.id)}
    <button
      role="tab"
      type="button"
      id="shell-tab-{tab.id}"
      class:active={tab.active}
      class:pinned={tab.pinned}
      class="tab"
      aria-selected={tab.active}
      aria-controls={panelId}
      aria-label={tab.pinned ? tab.title : undefined}
      title={tab.tooltip}
      tabindex={tab.active ? 0 : -1}
      onclick={() => selectTab(tab)}>
      <span class="tab-icon" aria-hidden="true">{@render icon(tab.icon)}</span>
      {#if !tab.pinned}<span class="tab-title">{tab.title}</span>{/if}
    </button>
  {/each}
</div>

<style>
.tab-strip {
  display: flex;
  align-items: center;
  gap: 2px;
  min-width: 0;
  overflow: hidden;
}
.tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 30px;
  min-width: 0;
  max-width: 220px;
  padding: 0 10px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--ink-soft);
  font: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
.tab:hover:not(.active) {
  background: var(--surface-muted);
  color: var(--ink);
}
.tab.active {
  border-color: var(--line);
  background: var(--surface);
  color: var(--ink);
  cursor: default;
}
.tab:focus-visible {
  outline: 3px solid var(--focus-ring);
  outline-offset: 2px;
}

@media (forced-colors: active) {
  .tab:focus-visible {
    outline: 3px solid Highlight;
  }
}
.tab-icon {
  display: grid;
  width: 16px;
  height: 16px;
  flex: 0 0 auto;
  place-items: center;
}
.tab-title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
