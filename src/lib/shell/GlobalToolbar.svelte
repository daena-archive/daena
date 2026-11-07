<script lang="ts">
import type { Snippet } from "svelte";
import { ArrowLeft, ArrowRight, Search } from "@lucide/svelte";

let {
  ready,
  tabs,
  quickOpenShortcut,
  navigationBusy,
  canGoBack,
  canGoForward,
  onQuickOpen,
  onBack,
  onForward,
  status,
}: {
  ready: boolean;
  tabs?: Snippet;
  quickOpenShortcut: string;
  navigationBusy: boolean;
  canGoBack: boolean;
  canGoForward: boolean;
  onQuickOpen: () => void;
  onBack: () => void;
  onForward: () => void;
  status?: Snippet;
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
      {#if tabs}{@render tabs()}{/if}
    {/if}
  </div>
  <div class="top-actions">
    {#if ready}
      <button
        class="global-search"
        type="button"
        aria-label="Quick Open"
        title={`Quick Open · ${quickOpenShortcut}`}
        aria-haspopup="dialog"
        onclick={onQuickOpen}>
        <Search size={14} strokeWidth={1.8} aria-hidden="true" /><kbd>{quickOpenShortcut}</kbd>
      </button>
      {#if status}{@render status()}{:else}<span class="sync-badge" title="Your work is stored locally"
          ><span></span> Local</span
        >{/if}
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
  min-height: var(--shell-topbar-height, 44px);
  padding: 0 20px;
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
  flex: 1;
  gap: 12px;
}
.history-actions {
  flex: 0 0 auto;
  gap: 3px;
}
.history-actions button {
  display: grid;
  width: 30px;
  height: 30px;
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
.top-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}
.global-search {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 6px;
  height: 28px;
  padding: 0 8px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-faint);
  cursor: pointer;
  font-size: 12px;
  transition:
    border-color 0.16s ease,
    box-shadow 0.16s ease;
}
.global-search:hover,
.global-search:focus-visible {
  border-color: var(--accent-soft);
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
  background: var(--success);
}
.startup-topbar {
  display: none;
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
  .topbar-leading,
  .top-actions {
    width: 100%;
  }
  .topbar-leading {
    flex: none;
  }
  .sync-badge {
    display: none;
  }
}
</style>
