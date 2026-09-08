<script lang="ts">
import type { Snippet } from "svelte";
import type { EntityFieldsTab, EntityFieldsTabId } from "./entityFields";

let {
  typeLabel,
  entityName,
  chips = [],
  aiEnabled = false,
  aiBusy = false,
  onFillAi,
  onOpenTab,
  notice,
  details,
}: {
  typeLabel: string;
  entityName: string;
  chips?: EntityFieldsTab[];
  aiEnabled?: boolean;
  aiBusy?: boolean;
  onFillAi?: () => void;
  onOpenTab: (tab?: EntityFieldsTabId) => void;
  notice?: Snippet;
  details?: Snippet;
} = $props();
</script>

<div class="inspector-heading">
  <div>
    <span class="panel-kicker">INSPECTOR</span>
    <strong>{typeLabel}</strong>
    <p class="entity-name">{entityName}</p>
  </div>
  {#if aiEnabled && onFillAi}
    <div class="heading-actions">
      <button class="ai-action" type="button" onclick={onFillAi} disabled={aiBusy}
        ><span aria-hidden="true">✦</span>{aiBusy ? "Finding…" : "Fill with AI"}</button>
    </div>
  {/if}
</div>

{#if notice}
  {@render notice()}
{/if}

{#if details}
  {@render details()}
{/if}

<div class="overview-more">
  {#if chips.length}
    <div class="chips" role="group" aria-label="More fields">
      {#each chips as chip (chip.id)}
        <button type="button" class="chip" onclick={() => onOpenTab(chip.id)}>
          <span>{chip.label}</span>
          {#if chip.count !== undefined}<em>{chip.count}</em>{/if}
        </button>
      {/each}
    </div>
  {/if}

  <button class="edit-fields" type="button" onclick={() => onOpenTab()}>Edit fields</button>
</div>

<style>
.inspector-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
  padding: 18px 17px 12px;
  border-bottom: 1px solid var(--line);
}
.inspector-heading strong {
  display: block;
  margin-top: 7px;
  font: 500 20px var(--font-display);
}
.entity-name {
  margin: 4px 0 0;
  overflow: hidden;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.35;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.heading-actions {
  display: flex;
  flex: none;
  flex-direction: column;
  align-items: flex-end;
  gap: 5px;
}
.ai-action {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 6px;
  border: 1px solid var(--theme-warning-border, #d9b98f);
  border-radius: 5px;
  background: var(--warning-bg);
  color: var(--accent);
  font-size: 9px;
  font-weight: 700;
  cursor: pointer;
}
.ai-action:disabled {
  opacity: 0.65;
  cursor: wait;
}
.ai-action span {
  font-size: 10px;
}
.overview-more {
  display: grid;
  gap: 12px;
  padding: 14px 17px 18px;
}
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 28px;
  padding: 0 8px 0 10px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--surface);
  color: var(--ink-soft);
  cursor: pointer;
  font-size: 10px;
  font-weight: 700;
}
.chip:hover {
  background: var(--surface-muted);
  color: var(--ink);
}
.chip em {
  min-width: 16px;
  padding: 1px 5px;
  border-radius: 999px;
  background: var(--surface-muted);
  color: var(--ink-faint);
  font-style: normal;
  font-size: 9px;
  text-align: center;
}
.edit-fields {
  min-height: 34px;
  border: 1px solid var(--accent-dark);
  border-radius: 8px;
  background: var(--accent-dark);
  color: var(--on-accent);
  cursor: pointer;
  font-size: 12px;
  font-weight: 700;
}
.edit-fields:hover {
  filter: brightness(1.05);
}
</style>
