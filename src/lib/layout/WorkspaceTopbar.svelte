<script lang="ts">
import type { Component, Snippet } from "svelte";
import { ArrowLeft } from "@lucide/svelte";

let {
  title,
  subtitle = "",
  icon: Icon = null,
  backLabel = "Back to workspace",
  actionsLabel = "View actions",
  onBack,
  brandActions,
  actions,
  children,
}: {
  title: string;
  subtitle?: string;
  icon?: Component | null;
  backLabel?: string;
  actionsLabel?: string;
  onBack: () => void;
  brandActions?: Snippet;
  actions?: Snippet;
  children?: Snippet;
} = $props();
const actionContent = $derived(actions ?? children);
</script>

<header class="workspace-topbar">
  <div class="workspace-topbar-brand">
    <button class="workspace-topbar-back" type="button" onclick={onBack} aria-label={backLabel} title={backLabel}>
      <ArrowLeft size={15} strokeWidth={1.8} aria-hidden="true" />
    </button>
    {#if brandActions}
      <div class="workspace-topbar-brand-actions" data-workspace-topbar-brand-actions>
        {@render brandActions()}
      </div>
    {/if}
    {#if Icon}
      <span class="workspace-topbar-mark"><Icon size={16} strokeWidth={1.8} aria-hidden="true" /></span>
    {/if}
    <div class="workspace-topbar-copy">
      <strong>{title}</strong>
      {#if subtitle}<small>{subtitle}</small>{/if}
    </div>
  </div>
  {#if actionContent}
    <div class="workspace-topbar-actions" role="toolbar" aria-label={actionsLabel}>
      {@render actionContent()}
    </div>
  {/if}
</header>

<style>
.workspace-topbar {
  z-index: 20;
  display: grid;
  min-height: 58px;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 16px;
  padding: 10px 18px;
  border-bottom: 1px solid var(--theme-neutral-border, #dde1da);
  background: color-mix(in srgb, var(--surface) 96%, transparent);
  color: var(--theme-neutral-text, #252b26);
  box-shadow: 0 1px 8px rgba(30, 37, 31, 0.03);
  font-family: var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.workspace-topbar-brand,
.workspace-topbar-actions {
  display: flex;
  align-items: center;
  gap: 9px;
}
.workspace-topbar-brand {
  min-width: 0;
}
.workspace-topbar-brand-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 0 0 auto;
}
.workspace-topbar-brand-actions :global(button),
:global([data-workspace-topbar-brand-actions] > button) {
  display: inline-flex;
  width: 34px;
  min-width: 34px;
  min-height: 34px;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: 1px solid var(--theme-neutral-border, #d9ddd6);
  border-radius: 8px;
  background: var(--theme-surface-bg, #fff);
  color: var(--theme-neutral-text-soft, #4d584f);
  cursor: pointer;
}
.workspace-topbar-brand-actions :global(button:hover),
.workspace-topbar-brand-actions :global(button:focus-visible),
:global([data-workspace-topbar-brand-actions] > button:hover),
:global([data-workspace-topbar-brand-actions] > button:focus-visible) {
  border-color: var(--theme-neutral-border-strong, #b9c4ba);
  background: var(--theme-success-bg, #f2f6f2);
  color: var(--theme-success-text, #2f4e35);
  outline: 0;
}
.workspace-topbar-brand-actions :global(button.active),
.workspace-topbar-brand-actions :global(button[aria-pressed="true"]),
:global([data-workspace-topbar-brand-actions] > button.active),
:global([data-workspace-topbar-brand-actions] > button[aria-pressed="true"]) {
  border-color: var(--theme-neutral-border-strong, #b8c9ba);
  background: var(--theme-success-bg, #e4ece4);
  color: var(--theme-success-text, #2f4e35);
}
.workspace-topbar-brand-actions :global(button:disabled),
:global([data-workspace-topbar-brand-actions] > button:disabled) {
  opacity: 0.38;
  cursor: not-allowed;
}
.workspace-topbar-copy {
  display: grid;
  min-width: 0;
  gap: 2px;
}
.workspace-topbar-copy strong,
.workspace-topbar-copy small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.workspace-topbar-copy strong {
  color: var(--theme-neutral-text, #252b26);
  font-size: 12px;
  line-height: 1.2;
}
.workspace-topbar-copy small {
  color: var(--theme-neutral-text-muted, #899088);
  font-size: 9px;
  line-height: 1.2;
}
.workspace-topbar-mark {
  display: grid;
  width: 31px;
  height: 31px;
  flex: 0 0 31px;
  place-items: center;
  border-radius: 8px;
  background: var(--theme-success-bg, #e4ece4);
  color: var(--theme-success-text, #416047);
}
.workspace-topbar-back,
:global(.workspace-topbar-action),
:global([data-workspace-topbar-actions] > button) {
  display: inline-flex;
  min-height: 34px;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border: 1px solid var(--theme-neutral-border, #d9ddd6);
  border-radius: 8px;
  background: var(--theme-surface-bg, #fff);
  color: var(--theme-neutral-text-soft, #4d584f);
  font: 650 11px/1 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  cursor: pointer;
}
.workspace-topbar-back {
  width: 34px;
  flex: 0 0 34px;
  padding: 0;
}
:global(.workspace-topbar-action) {
  padding: 0 10px;
}
:global([data-workspace-topbar-actions] > button) {
  padding: 0 10px;
}
:global(.workspace-topbar-action.icon) {
  width: 34px;
  min-width: 34px;
  padding: 0;
}
:global([data-workspace-topbar-actions] > button.icon-button) {
  width: 34px;
  min-width: 34px;
  padding: 0;
}
.workspace-topbar-back:hover,
.workspace-topbar-back:focus-visible,
:global(.workspace-topbar-action:hover),
:global(.workspace-topbar-action:focus-visible),
:global([data-workspace-topbar-actions] > button:hover),
:global([data-workspace-topbar-actions] > button:focus-visible) {
  border-color: var(--theme-neutral-border-strong, #b9c4ba);
  background: var(--theme-success-bg, #f2f6f2);
  color: var(--theme-success-text, #2f4e35);
  outline: 0;
}
:global(.workspace-topbar-action.active),
:global(.workspace-topbar-action[aria-pressed="true"]),
:global(.workspace-topbar-action.primary),
:global([data-workspace-topbar-actions] > button.active),
:global([data-workspace-topbar-actions] > button[aria-pressed="true"]),
:global([data-workspace-topbar-actions] > button.primary) {
  border-color: var(--theme-neutral-border-strong, #b8c9ba);
  background: var(--theme-success-bg, #e4ece4);
  color: var(--theme-success-text, #2f4e35);
}
:global(.workspace-topbar-action:disabled),
:global([data-workspace-topbar-actions] > button:disabled) {
  opacity: 0.38;
  cursor: not-allowed;
}
.workspace-topbar-actions {
  min-width: 0;
  flex-wrap: wrap;
  justify-content: flex-end;
}
@media (max-width: 900px) {
  .workspace-topbar {
    grid-template-columns: 1fr;
    align-items: start;
    gap: 8px;
  }
  .workspace-topbar-actions {
    padding-left: 43px;
    justify-content: flex-start;
  }
}
@media (max-width: 650px) {
  .workspace-topbar {
    padding: 9px 12px;
  }
  .workspace-topbar-copy small {
    display: none;
  }
  .workspace-topbar-actions {
    padding-left: 0;
  }
}
@media print {
  .workspace-topbar {
    display: none !important;
  }
}
</style>
