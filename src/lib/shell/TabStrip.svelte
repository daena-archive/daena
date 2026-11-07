<script lang="ts">
import type { Snippet } from "svelte";
import { Plus, X } from "@lucide/svelte";
import type { TabIcon } from "$lib/navigation/tabs";

let {
  tabs,
  panelId,
  onActivate,
  onClose,
  onNew,
  busy = false,
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
  onClose: (id: string) => void;
  onNew: () => void;
  busy?: boolean;
  icon: Snippet<[TabIcon]>;
} = $props();

function tabButtons(list: HTMLElement) {
  return [...list.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
}

function moveTabFocus(event: KeyboardEvent) {
  if (event.altKey || event.metaKey || event.ctrlKey || event.shiftKey) return;
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight" && event.key !== "Home" && event.key !== "End") return;
  const list = event.currentTarget;
  if (!(list instanceof HTMLElement)) return;
  const buttons = tabButtons(list);
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

function onTablistKeydown(event: KeyboardEvent) {
  if (busy) return;
  if (
    (event.key === "Delete" || event.key === "Backspace") &&
    !event.altKey &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.shiftKey
  ) {
    const list = event.currentTarget;
    if (!(list instanceof HTMLElement)) return;
    const buttons = tabButtons(list);
    const index = buttons.findIndex((button) => button === document.activeElement);
    if (index < 0) return;
    event.preventDefault();
    onClose(tabs[index].id);
    return;
  }
  moveTabFocus(event);
}

function selectTab(tab: { id: string; active: boolean }) {
  if (busy || tab.active) return;
  onActivate(tab.id);
}

function closeTab(id: string) {
  if (busy) return;
  onClose(id);
}

function newTab() {
  if (busy) return;
  onNew();
}

function blockMiddleClick(event: MouseEvent) {
  if (event.button === 1) event.preventDefault();
}
</script>

<div class="tab-strip">
  <div role="tablist" aria-label="Open tabs" class="tab-list" tabindex="-1" onkeydown={onTablistKeydown}>
    {#each tabs as tab (tab.id)}
      <div
        role="presentation"
        class="tab-item"
        class:active={tab.active}
        class:pinned={tab.pinned}
        onmousedown={blockMiddleClick}
        onauxclick={(event) => {
          if (event.button === 1) closeTab(tab.id);
        }}>
        <button
          role="tab"
          type="button"
          id="shell-tab-{tab.id}"
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
        {#if !tab.pinned}<button
            type="button"
            class="tab-close"
            tabindex="-1"
            aria-label={`Close ${tab.title}`}
            title="Close tab"
            onclick={() => closeTab(tab.id)}><X size={12} strokeWidth={1.8} aria-hidden="true" /></button
          >{/if}
      </div>
    {/each}
  </div>
  <button type="button" class="tab-new" aria-label="New tab" title="New tab" onclick={newTab}>
    <Plus size={14} strokeWidth={1.8} aria-hidden="true" />
  </button>
</div>

<style>
.tab-strip,
.tab-list {
  display: flex;
  align-items: center;
  gap: 2px;
  min-width: 0;
}
.tab-list {
  overflow: hidden;
}
.tab-item {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  height: 30px;
  flex: 0 1 auto;
  min-width: 72px;
  max-width: 220px;
  padding: 0 4px 0 10px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--ink-soft);
}
.tab-item.pinned {
  flex: none;
  min-width: 0;
  padding: 0 8px;
}
.tab-item:hover:not(.active) {
  background: var(--surface-muted);
  color: var(--ink);
}
.tab-item.active {
  border-color: var(--line);
  background: var(--surface);
  color: var(--ink);
}
.tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  min-width: 0;
  height: 100%;
  padding: 0;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: inherit;
  font: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
.tab-item.active .tab {
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
.tab-close {
  display: grid;
  width: 18px;
  height: 18px;
  flex: 0 0 auto;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: inherit;
  opacity: 0;
  cursor: pointer;
}
.tab-item.active .tab-close,
.tab-item:hover .tab-close,
.tab-close:focus-visible {
  opacity: 1;
}
.tab-close:hover {
  background: var(--surface-muted);
}
.tab-new {
  display: grid;
  width: 28px;
  height: 28px;
  flex: 0 0 auto;
  place-items: center;
  padding: 0;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--ink-soft);
  cursor: pointer;
}
.tab-new:hover {
  border-color: var(--line);
  background: var(--surface-muted);
  color: var(--ink);
}
.tab-new:focus-visible {
  outline: 3px solid var(--focus-ring);
  outline-offset: 2px;
}
</style>
