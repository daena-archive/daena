<script lang="ts">
import type { Snippet } from "svelte";
import { Plus } from "@lucide/svelte";
import { ENTITY_ACTIONS } from "./vocabulary.ts";

let {
  title,
  message,
  createLabel = "",
  maps = false,
  actions,
  onCreate,
}: {
  title: string;
  message: string;
  createLabel?: string;
  maps?: boolean;
  actions?: Snippet;
  onCreate?: () => void;
} = $props();
</script>

<div class="entity-empty" role="status">
  <span class="empty-mark" aria-hidden="true">✦</span>
  <strong>{title}</strong>
  <p>{message}</p>
  {#if actions}
    {@render actions()}
  {:else if onCreate && !maps}
    <button class="empty-create" type="button" onclick={onCreate}>
      <span style="display:inline-flex;vertical-align:middle" aria-hidden="true"
        ><Plus size={16} strokeWidth={1.8} /></span>
      {createLabel ? `${ENTITY_ACTIONS.new} ${createLabel}` : ENTITY_ACTIONS.new}
    </button>
  {/if}
</div>

<style>
.entity-empty {
  display: grid;
  justify-items: center;
  gap: 8px;
  padding: 28px 18px;
  color: var(--ink-faint);
  text-align: center;
}
.empty-mark {
  display: grid;
  width: 38px;
  height: 38px;
  place-items: center;
  border-radius: 50%;
  background: var(--surface-muted);
  color: var(--accent-dark);
  font-size: 18px;
}
.entity-empty strong {
  color: var(--ink);
  font: 500 16px/1.3 var(--font-display);
}
.entity-empty p {
  max-width: 36ch;
  margin: 0;
  font-size: 11px;
  line-height: 1.5;
}
.empty-create {
  display: inline-flex;
  min-height: var(--touch-target-min, 44px);
  align-items: center;
  gap: 6px;
  margin-top: 4px;
  padding: 0 12px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  cursor: pointer;
  font-size: 12px;
}
.empty-create:hover,
.empty-create:focus-visible {
  border-color: var(--accent);
  outline: 0;
}
</style>
