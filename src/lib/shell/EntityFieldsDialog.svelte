<script lang="ts">
import type { Snippet } from "svelte";
import { X } from "@lucide/svelte";
import { trapModalTab } from "./modalFocus";
import type { EntityFieldsTab, EntityFieldsTabId } from "./entityFields";

let {
  title,
  kicker = "FIELDS",
  tabs,
  activeTab = $bindable(),
  relationships,
  profile,
  assets,
  maps,
  backlinks,
  onClose,
}: {
  title: string;
  kicker?: string;
  tabs: EntityFieldsTab[];
  activeTab: EntityFieldsTabId;
  relationships?: Snippet;
  profile?: Snippet;
  assets?: Snippet;
  maps?: Snippet;
  backlinks?: Snippet;
  onClose: () => void;
} = $props();

let dialogEl = $state<HTMLElement | null>(null);
let tabListEl = $state<HTMLElement | null>(null);
let lastFocused: Element | null = null;
const currentTab = $derived(tabs.find((tab) => tab.id === activeTab) ?? tabs[0]);

$effect(() => {
  const el = dialogEl;
  if (!el) return;
  if (!lastFocused) lastFocused = document.activeElement;
  const frame = window.requestAnimationFrame(() => el.focus());
  return () => {
    window.cancelAnimationFrame(frame);
    const focused = lastFocused;
    lastFocused = null;
    if (focused instanceof HTMLElement && focused.isConnected) focused.focus();
  };
});

const panels: Record<EntityFieldsTabId, Snippet | undefined> = $derived({
  relationships,
  profile,
  assets,
  maps,
  backlinks,
});

function onKeydown(event: KeyboardEvent) {
  trapModalTab(event, dialogEl);
}

function onTabKeydown(event: KeyboardEvent) {
  if (event.key !== "ArrowRight" && event.key !== "ArrowLeft") return;
  const buttons = tabListEl?.querySelectorAll<HTMLButtonElement>("[role='tab']") ?? [];
  if (buttons.length === 0) return;
  const index = Array.from(buttons).findIndex((button) => button === event.target);
  if (index < 0) return;
  event.preventDefault();
  const next =
    event.key === "ArrowRight" ? (index + 1) % buttons.length : (index - 1 + buttons.length) % buttons.length;
  const tab = tabs[next];
  if (!tab) return;
  activeTab = tab.id;
  buttons[next]?.focus();
}
</script>

<div class="modal-backdrop" role="presentation" onclick={onClose} tabindex="-1">
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="entity-fields-title"
    tabindex="-1"
    bind:this={dialogEl}
    onclick={(event) => event.stopPropagation()}
    onkeydown={onKeydown}>
    <div class="heading">
      <div>
        <span class="panel-kicker">{kicker}</span>
        <strong id="entity-fields-title">{title}</strong>
      </div>
      <button type="button" class="close" aria-label="Close fields" onclick={onClose}
        ><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
    </div>
    {#if tabs.length > 0}
      <div
        class="tabs"
        role="tablist"
        aria-label="Field sections"
        tabindex="-1"
        bind:this={tabListEl}
        onkeydown={onTabKeydown}>
        {#each tabs as tab (tab.id)}
          <button
            type="button"
            role="tab"
            id="entity-fields-tab-{tab.id}"
            aria-selected={currentTab?.id === tab.id}
            aria-controls="entity-fields-panel-{tab.id}"
            tabindex={currentTab?.id === tab.id ? 0 : -1}
            onclick={() => (activeTab = tab.id)}
            >{tab.label}{#if tab.count !== undefined}<em>{tab.count}</em>{/if}</button>
        {/each}
      </div>
    {/if}
    <div
      class="body"
      role="tabpanel"
      id="entity-fields-panel-{currentTab?.id}"
      aria-labelledby="entity-fields-tab-{currentTab?.id}">
      {#if currentTab}
        {@const panel = panels[currentTab.id]}
        {#if panel}
          {@render panel()}
        {/if}
      {/if}
    </div>
  </div>
</div>

<style>
.modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 72;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(28, 26, 22, 0.42);
}
.dialog {
  display: flex;
  width: min(780px, 100%);
  max-height: min(88vh, 900px);
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
  box-shadow: var(--shadow-sm);
}
.heading {
  display: flex;
  flex: none;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 16px 18px 10px;
}
.panel-kicker {
  display: block;
  color: var(--accent);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.16em;
}
.heading strong {
  display: block;
  margin-top: 4px;
  color: var(--ink);
  font: 500 20px/1.2 var(--font-display);
}
.close {
  display: grid;
  width: 32px;
  height: 32px;
  flex: none;
  place-items: center;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-soft);
  cursor: pointer;
}
.tabs {
  display: flex;
  flex: none;
  gap: 4px;
  margin: 0 18px 10px;
  padding: 4px;
  overflow-x: auto;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--canvas);
}
.tabs button {
  display: inline-flex;
  flex: 1 0 auto;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 32px;
  padding: 0 10px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--ink-soft);
  cursor: pointer;
  font: 600 11px/1 var(--font-body);
  white-space: nowrap;
}
.tabs button[aria-selected="true"] {
  border-color: var(--theme-warning-border, #d3c0a9);
  background: var(--surface);
  box-shadow: 0 1px 4px rgba(38, 42, 33, 0.08);
  color: var(--accent-dark);
}
.tabs em {
  min-width: 16px;
  padding: 1px 5px;
  border-radius: 999px;
  background: var(--surface-muted);
  color: var(--ink-faint);
  font-style: normal;
  font-size: 9px;
}
.body {
  min-height: 0;
  flex: 1 1 auto;
  overflow: auto;
  padding: 0 0 12px;
}
.body :global(.inspector-group:first-child) {
  border-top: 1px solid var(--line);
}
.body :global(.inspector-group:last-child) {
  border-bottom: 0;
}
</style>
