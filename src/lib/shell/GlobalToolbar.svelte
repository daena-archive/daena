<script lang="ts">
import { ArrowLeft, ArrowRight, Search } from "@lucide/svelte";

let {
  ready,
  breadcrumbs,
  quickOpenShortcut,
  navigationBusy,
  canGoBack,
  canGoForward,
  onQuickOpen,
  onBack,
  onForward,
}: {
  ready: boolean;
  breadcrumbs: string[];
  quickOpenShortcut: string;
  navigationBusy: boolean;
  canGoBack: boolean;
  canGoForward: boolean;
  onQuickOpen: () => void;
  onBack: () => void;
  onForward: () => void;
} = $props();
</script>

<header class:startup-topbar={!ready} class="topbar">
  <div class="topbar-leading">
    {#if ready}
      <div class="history-actions" aria-label="Navigation history" aria-busy={navigationBusy}>
        <button
          type="button"
          aria-label="Go back"
          title="Go back · Alt+Left"
          disabled={navigationBusy || !canGoBack}
          onclick={onBack}><ArrowLeft size={15} strokeWidth={1.8} aria-hidden="true" /></button>
        <button
          type="button"
          aria-label="Go forward"
          title="Go forward · Alt+Right"
          disabled={navigationBusy || !canGoForward}
          onclick={onForward}><ArrowRight size={15} strokeWidth={1.8} aria-hidden="true" /></button>
      </div>
    {/if}
    <div class="breadcrumbs" aria-label="Breadcrumb">
      {#each breadcrumbs as breadcrumb, index}
        {#if index > 0}<i>/</i>{/if}
        {#if index === 1}<strong>{breadcrumb}</strong>{:else}<span>{breadcrumb}</span>{/if}
      {/each}
    </div>
  </div>
  <div class="top-actions">
    {#if ready}
      <button class="global-search" type="button" aria-haspopup="dialog" onclick={onQuickOpen}>
        <span aria-hidden="true"><Search size={14} strokeWidth={1.8} aria-hidden="true" /></span>
        <span class="global-search-label">Quick Open</span><kbd>{quickOpenShortcut}</kbd>
      </button>
      <span class="sync-badge" title="Your work is stored locally"><span></span> Local</span>
    {/if}
  </div>
</header>

<style>
.topbar {
  position: sticky;
  top: 0;
  z-index: 4;
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: space-between;
  min-height: 58px;
  padding: 0 40px;
  border-bottom: 1px solid var(--line);
  background: color-mix(in srgb, var(--surface) 78%, transparent);
  backdrop-filter: blur(14px);
}
.topbar-leading,
.history-actions {
  display: flex;
  min-width: 0;
  align-items: center;
}
.topbar-leading {
  gap: 12px;
}
.history-actions {
  flex: 0 0 auto;
  gap: 3px;
}
.history-actions button {
  display: grid;
  width: 28px;
  height: 28px;
  place-items: center;
  padding: 0;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--ink-soft);
  cursor: pointer;
}
.history-actions button:hover:not(:disabled) {
  border-color: var(--line);
  background: var(--surface-muted);
  color: var(--ink);
}
.history-actions button:disabled {
  color: var(--ink-faint);
  cursor: default;
  opacity: 0.42;
}
.breadcrumbs,
.top-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}
.breadcrumbs {
  min-width: 0;
  color: var(--ink-faint);
  font-size: 12px;
}
.breadcrumbs strong {
  color: var(--ink-soft);
}
.breadcrumbs span:last-child {
  overflow: hidden;
  max-width: 180px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.breadcrumbs i {
  color: var(--ink-faint);
  font-style: normal;
}
.global-search {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 230px;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-faint);
  cursor: pointer;
  font-size: 12px;
  text-align: left;
  transition:
    border-color 0.16s ease,
    box-shadow 0.16s ease;
}
.global-search:hover,
.global-search:focus-visible {
  border-color: var(--accent-soft);
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.1);
}
.global-search-label {
  min-width: 0;
  flex: 1;
  color: var(--ink-soft);
}
.global-search kbd {
  padding: 2px 5px;
  border: 1px solid var(--line);
  border-radius: 5px;
  background: var(--canvas);
  color: var(--ink-faint);
  font: 500 9px var(--font-sans);
}
.sync-badge {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--ink-soft);
  font-size: 10px;
}
.sync-badge span {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #72a97a;
}
.startup-topbar {
  display: none;
}

@media (max-width: 1040px) {
  .topbar {
    padding-inline: 28px;
  }
}

@media (max-width: 760px) {
  .topbar {
    position: relative;
    align-items: stretch;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
    padding: 12px 17px;
  }
  .breadcrumbs,
  .top-actions {
    width: 100%;
  }
  .breadcrumbs span:first-child,
  .sync-badge {
    display: none;
  }
  .global-search {
    flex: 1;
    width: auto;
  }
}
</style>
