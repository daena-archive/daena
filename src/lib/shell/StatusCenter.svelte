<script lang="ts">
import { Activity, AlertTriangle, Check, ChevronRight, LoaderCircle, X } from "@lucide/svelte";

export type StatusCenterTone = "neutral" | "success" | "busy" | "warning" | "danger";
export type StatusCenterItem = {
  id: string;
  label: string;
  detail: string;
  tone: StatusCenterTone;
  actionLabel?: string;
  onAction?: () => void;
};

let {
  open = $bindable(false),
  summary,
  tone = "neutral",
  items = [],
  onOpenChange,
}: {
  open?: boolean;
  summary: string;
  tone?: StatusCenterTone;
  items?: StatusCenterItem[];
  onOpenChange?: (open: boolean) => void;
} = $props();

function setOpen(next: boolean) {
  open = next;
  onOpenChange?.(next);
}
</script>

<div class="status-center">
  <button
    type="button"
    class={`status-trigger ${tone}`}
    aria-expanded={open}
    aria-haspopup="dialog"
    title="Project status"
    onclick={() => setOpen(!open)}>
    {#if tone === "busy"}<LoaderCircle
        class="spin"
        size={14}
        aria-hidden="true" />{:else if tone === "danger" || tone === "warning"}<AlertTriangle
        size={14}
        aria-hidden="true" />{:else if tone === "success"}<Check size={14} aria-hidden="true" />{:else}<Activity
        size={14}
        aria-hidden="true" />{/if}
    <span>{summary}</span>
  </button>
  {#if open}
    <button class="status-backdrop" aria-label="Close project status" onclick={() => setOpen(false)}></button>
    <div class="status-popover" role="dialog" aria-modal="false" aria-labelledby="status-center-title">
      <header>
        <div><span>PROJECT STATUS</span><strong id="status-center-title">What Daena is doing</strong></div>
        <button aria-label="Close project status" onclick={() => setOpen(false)}><X size={15} /></button>
      </header>
      <div class="status-items" aria-live="polite">
        {#each items as item (item.id)}
          <article class={`status-item ${item.tone}`}>
            <span class="item-mark" aria-hidden="true"
              >{#if item.tone === "busy"}<LoaderCircle
                  class="spin"
                  size={14} />{:else if item.tone === "danger" || item.tone === "warning"}<AlertTriangle
                  size={14} />{:else}<Check size={14} />{/if}</span>
            <div>
              <strong>{item.label}</strong>
              <p>{item.detail}</p>
            </div>
            {#if item.actionLabel && item.onAction}<button class="item-action" onclick={item.onAction}
                >{item.actionLabel}<ChevronRight size={13} /></button
              >{/if}
          </article>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
.status-center {
  position: relative;
}
.status-trigger {
  display: flex;
  align-items: center;
  gap: 6px;
  max-width: 190px;
  padding: 7px 9px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--surface);
  color: var(--ink-soft);
  font-size: 10px;
  cursor: pointer;
}
.status-trigger span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.status-trigger.busy {
  color: var(--accent);
}
.status-trigger.warning {
  border-color: var(--theme-warning-border);
  color: var(--theme-warning-text, var(--accent));
}
.status-trigger.danger {
  border-color: var(--danger-line);
  color: var(--danger);
}
.status-trigger.success {
  color: var(--success);
}
.status-trigger:focus-visible,
.status-popover button:focus-visible {
  outline: 3px solid color-mix(in srgb, var(--accent) 30%, transparent);
  outline-offset: 2px;
}
.status-backdrop {
  position: fixed;
  inset: 0;
  z-index: 19;
  border: 0;
  background: transparent;
}
.status-popover {
  position: absolute;
  z-index: 20;
  top: calc(100% + 10px);
  right: 0;
  width: min(430px, calc(100vw - 24px));
  overflow: hidden;
  border: 1px solid var(--line-strong);
  border-radius: 13px;
  background: var(--surface);
  box-shadow: var(--shadow-lg);
}
.status-popover header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 14px 15px;
  border-bottom: 1px solid var(--line);
  background: var(--surface-subtle);
}
.status-popover header div {
  display: grid;
  gap: 3px;
}
.status-popover header span {
  color: var(--accent);
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.12em;
}
.status-popover header strong {
  color: var(--ink);
  font: 600 15px var(--font-display);
}
.status-popover header button {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--ink-soft);
  cursor: pointer;
}
.status-items {
  display: grid;
  max-height: min(480px, calc(100vh - 120px));
  overflow: auto;
}
.status-item {
  display: grid;
  grid-template-columns: 28px minmax(0, 1fr) auto;
  gap: 9px;
  align-items: start;
  padding: 13px 15px;
}
.status-item + .status-item {
  border-top: 1px solid var(--line-soft);
}
.item-mark {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border-radius: 8px;
  background: var(--surface-warm);
  color: var(--ink-muted);
}
.status-item.success .item-mark {
  background: var(--theme-success-bg, var(--accent-bg));
  color: var(--success);
}
.status-item.busy .item-mark,
.status-item.warning .item-mark {
  background: var(--theme-warning-bg, var(--surface-warm));
  color: var(--theme-warning-text, var(--accent));
}
.status-item.danger .item-mark {
  background: var(--danger-bg);
  color: var(--danger);
}
.status-item strong {
  display: block;
  color: var(--ink);
  font-size: 12px;
}
.status-item p {
  margin: 3px 0 0;
  color: var(--ink-soft);
  font-size: 11px;
  line-height: 1.4;
}
.item-action {
  display: flex;
  align-items: center;
  gap: 2px;
  align-self: center;
  padding: 6px 4px;
  border: 0;
  background: transparent;
  color: var(--accent);
  font-size: 10px;
  font-weight: 700;
  cursor: pointer;
}
.spin {
  animation: status-spin 0.9s linear infinite;
}
@keyframes status-spin {
  to {
    transform: rotate(360deg);
  }
}
@media (prefers-reduced-motion: reduce) {
  .spin {
    animation: none;
  }
}
@media (max-width: 760px) {
  .status-trigger span {
    display: none;
  }
  .status-popover {
    position: fixed;
    top: 66px;
    right: 12px;
    left: 12px;
    width: auto;
  }
}
</style>
