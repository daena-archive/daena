<script lang="ts">
import { onMount } from "svelte";
import type { ModuleContext, ModuleView } from "../../packages/module-api/src/index";

let { view, context, className = "" }: { view: ModuleView; context: ModuleContext; className?: string } = $props();

let container = $state<HTMLElement | null>(null);
let error = $state("");

onMount(() => {
  if (!container) return;
  try {
    const cleanup = view.mount(container, context);
    return () => cleanup?.();
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
});
</script>

<div bind:this={container} class="module-mount {className}">
  {#if error}<p class="module-mount-error">{error}</p>{/if}
</div>

<style>
.module-mount {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  overflow: auto;
}
.module-mount-error {
  color: var(--danger);
  font-size: 12px;
}
</style>
