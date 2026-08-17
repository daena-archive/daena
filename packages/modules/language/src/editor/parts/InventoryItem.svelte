<script lang="ts">
import EditorButton from "./EditorButton.svelte";

let {
  title,
  index,
  total,
  referenced = false,
  locked = false,
  onmove,
  onremove,
  children,
}: {
  title: string;
  index: number;
  total: number;
  referenced?: boolean;
  locked?: boolean;
  onmove?: (delta: number) => void;
  onremove?: () => void;
  children?: import("svelte").Snippet;
} = $props();
</script>

<article class="grammar-inventory-item" role="listitem">
  <div class="grammar-inventory-toolbar">
    <strong>{title}</strong>
    {#if referenced}<span>Referenced by agreement</span>{/if}
    {#if !locked}
      <EditorButton secondary ariaLabel={`Move ${title} up`} disabled={index === 0} onclick={() => onmove?.(-1)}>
        Up
      </EditorButton>
      <EditorButton
        secondary
        ariaLabel={`Move ${title} down`}
        disabled={index === total - 1}
        onclick={() => onmove?.(1)}>
        Down
      </EditorButton>
      <EditorButton secondary danger ariaLabel={`Remove ${title}`} onclick={() => onremove?.()}>Remove</EditorButton>
    {/if}
  </div>
  {@render children?.()}
</article>

<style>
.grammar-inventory-item {
  display: grid;
  gap: 10px;
  min-width: 0;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface);
}
.grammar-inventory-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}
</style>
