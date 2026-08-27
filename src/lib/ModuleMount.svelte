<script lang="ts">
import type { ModuleContext, ModuleView } from "../../packages/module-api/src/index";

let { view, context, className = "" }: { view: ModuleView; context: ModuleContext; className?: string } = $props();

let container = $state<HTMLElement | null>(null);
let error = $state("");

$effect(() => {
  void view;
  void context;
  const target = container;
  if (!target || !view) return;
  error = "";
  let cleanup: (() => void) | void;
  try {
    cleanup = view.mount(target, context);
  } catch (cause) {
    error = cause instanceof Error ? cause.message : String(cause);
  }
  return () => cleanup?.();
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
