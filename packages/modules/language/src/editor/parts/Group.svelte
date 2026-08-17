<script lang="ts">
let { stack = true, children }: { stack?: boolean; children?: import("svelte").Snippet } = $props();
</script>

{#if stack}
  <section class="language-group grammar-choice-stack">{@render children?.()}</section>
{:else}
  <section class="language-group grammar-choice-editor">{@render children?.()}</section>
{/if}

<style>
.grammar-choice-editor,
.grammar-choice-stack {
  display: grid;
  gap: 12px;
  min-width: 0;
}
</style>
