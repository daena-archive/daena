<script lang="ts">
import type { Snippet } from "svelte";
import { ArrowLeftRight, History, Link, MapPin, Paperclip, UserRound, X } from "@lucide/svelte";
import { trapModalTab } from "./modalFocus";
import type { EntityFieldsTab, EntityFieldsTabId } from "./entityFields";

let {
  title,
  subtitle = null,
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
  subtitle?: string | null;
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
const totalCount = $derived(tabs.reduce((total, tab) => total + (tab.count ?? 0), 0));

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

const tabIcons = {
  relationships: Link,
  profile: UserRound,
  assets: Paperclip,
  maps: MapPin,
  backlinks: History,
} as const;

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    onClose();
    return;
  }
  trapModalTab(event, dialogEl);
}

function focusTab(index: number) {
  const buttons = tabListEl?.querySelectorAll<HTMLButtonElement>("[role='tab']") ?? [];
  const tab = tabs[index];
  if (!tab) return;
  activeTab = tab.id;
  buttons[index]?.focus();
}

function onTabKeydown(event: KeyboardEvent) {
  const buttons = tabListEl?.querySelectorAll<HTMLButtonElement>("[role='tab']") ?? [];
  if (buttons.length === 0) return;
  const index = Array.from(buttons).findIndex((button) => button === event.target);
  if (index < 0) return;
  if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
    event.preventDefault();
    const next =
      event.key === "ArrowRight" ? (index + 1) % buttons.length : (index - 1 + buttons.length) % buttons.length;
    focusTab(next);
  } else if (event.key === "Home") {
    event.preventDefault();
    focusTab(0);
  } else if (event.key === "End") {
    event.preventDefault();
    focusTab(buttons.length - 1);
  }
}
</script>

<div class="modal-backdrop" role="presentation" onclick={onClose} tabindex="-1">
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="entity-fields-title"
    aria-describedby={subtitle ? "entity-fields-subtitle" : undefined}
    tabindex="-1"
    bind:this={dialogEl}
    onclick={(event) => event.stopPropagation()}
    onkeydown={onKeydown}>
    <div class="heading">
      <div class="heading-copy">
        <span class="panel-kicker">{kicker}</span>
        <strong id="entity-fields-title">{title}</strong>
        {#if subtitle}
          <p id="entity-fields-subtitle" class="subtitle">{subtitle}</p>
        {/if}
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
          {@const Icon = tabIcons[tab.id] ?? ArrowLeftRight}
          {@const selected = currentTab?.id === tab.id}
          <button
            type="button"
            role="tab"
            id="entity-fields-tab-{tab.id}"
            aria-selected={selected}
            aria-controls="entity-fields-panel-{tab.id}"
            tabindex={selected ? 0 : -1}
            class="tab"
            class:selected
            onclick={() => (activeTab = tab.id)}>
            <Icon size={13} strokeWidth={1.8} aria-hidden="true" />
            <span>{tab.label}</span>
          </button>
        {/each}
      </div>
    {/if}
    <div
      class="body"
      role="tabpanel"
      id="entity-fields-panel-{currentTab?.id}"
      aria-labelledby="entity-fields-tab-{currentTab?.id}"
      tabindex="0">
      {#if currentTab}
        {@const panel = panels[currentTab.id]}
        {#if panel}
          {@render panel()}
        {/if}
      {/if}
    </div>
    <div class="footer">
      <p class="save-hint">
        <span class="save-dot" aria-hidden="true"></span>
        Changes save automatically{#if totalCount > 0}
          · {totalCount} linked{/if}
      </p>
      <div class="footer-actions">
        <span class="esc-hint" aria-hidden="true"><kbd>Esc</kbd> to close</span>
        <button type="button" class="done" onclick={onClose}>Done</button>
      </div>
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
  width: min(720px, 100%);
  max-height: min(90vh, 920px);
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 16px;
  background: var(--surface);
  box-shadow: var(--shadow-lg, 0 24px 70px rgba(38, 42, 33, 0.22));
}
.heading {
  display: flex;
  flex: none;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 18px 20px 8px;
}
.heading-copy {
  min-width: 0;
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
  overflow: hidden;
  margin-top: 4px;
  color: var(--ink);
  font: 500 22px/1.2 var(--font-display);
  text-overflow: ellipsis;
  white-space: nowrap;
}
.subtitle {
  margin: 4px 0 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.4;
}
.close {
  display: grid;
  width: 34px;
  height: 34px;
  flex: none;
  place-items: center;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--surface);
  color: var(--ink-soft);
  cursor: pointer;
}
.close:hover {
  background: var(--surface-muted);
  color: var(--ink);
}
.tabs {
  display: flex;
  flex: none;
  gap: 4px;
  padding: 0 20px;
  overflow-x: auto;
  border-bottom: 1px solid var(--line);
  background: var(--surface);
  scrollbar-width: thin;
}
.tab {
  display: inline-flex;
  flex: none;
  align-items: center;
  gap: 7px;
  min-height: 44px;
  margin-bottom: -1px;
  padding: 0 10px;
  border: 0;
  border-bottom: 2px solid transparent;
  border-radius: 0;
  background: transparent;
  color: var(--ink-soft);
  cursor: pointer;
  font: 600 12px/1 var(--font-body);
  white-space: nowrap;
}
.tab:hover {
  border-bottom-color: var(--line-strong, var(--line));
  color: var(--ink);
}
.tab.selected {
  border-bottom-color: var(--accent-dark);
  background: transparent;
  color: var(--ink);
}
.tab.selected :global(svg) {
  color: var(--accent-dark);
}
.tab :global(svg) {
  flex: none;
}
.body {
  min-height: 0;
  flex: 1 1 auto;
  overflow: auto;
  padding: 0 20px 16px;
  overscroll-behavior: contain;
}
.body:focus-visible {
  outline-offset: -3px;
}
.body :global(.inspector-group:first-child) {
  border-top: 0;
}
.body :global(.inspector-group:last-child) {
  border-bottom: 0;
}
/* Profile tab renders a short inline empty state; center it so sparse tabs
   read as intentional instead of leaving a void. */
#entity-fields-panel-profile :global(.inspector-copy) {
  margin-top: 6px;
  font-size: 12px;
  text-align: center;
}
#entity-fields-panel-profile :global(.inspector-preview) {
  text-align: center;
}
#entity-fields-panel-profile :global(.inspector-open) {
  display: block;
  width: auto;
  min-width: 220px;
  margin: 12px auto 4px;
}
.footer {
  display: flex;
  flex: none;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 20px;
  border-top: 1px solid var(--line);
  background: var(--surface);
}
.save-hint {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  color: var(--ink-faint);
  font-size: 11px;
}
.save-dot {
  width: 7px;
  height: 7px;
  flex: none;
  border-radius: 999px;
  background: var(--success, #4c8a4c);
}
.footer-actions {
  display: inline-flex;
  align-items: center;
  gap: 10px;
}
.esc-hint {
  color: var(--ink-faint);
  font-size: 11px;
}
.esc-hint kbd {
  padding: 1px 6px;
  border: 1px solid var(--line);
  border-bottom-width: 2px;
  border-radius: 5px;
  background: var(--canvas);
  font: 600 10px var(--font-body);
}
.done {
  min-height: 34px;
  padding: 0 16px;
  border: 1px solid var(--accent-dark);
  border-radius: 8px;
  background: var(--accent-dark);
  color: var(--on-accent);
  cursor: pointer;
  font-size: 12px;
  font-weight: 700;
}
.done:hover {
  filter: brightness(1.05);
}
@media (max-width: 640px) {
  .modal-backdrop {
    place-items: end center;
    padding: 0;
  }
  .dialog {
    width: 100%;
    max-height: 94vh;
    border-radius: 16px 16px 0 0;
  }
  .heading,
  .tabs,
  .body,
  .footer {
    padding-right: 16px;
    padding-left: 16px;
  }
  .esc-hint {
    display: none;
  }
}
@media (prefers-reduced-motion: reduce) {
  .tab,
  .close,
  .done {
    transition: none;
  }
}
@media (forced-colors: active) {
  .dialog,
  .close,
  .done {
    border: 1px solid ButtonBorder;
  }
  .tab.selected {
    border-bottom-color: Highlight;
    forced-color-adjust: none;
  }
}
</style>
