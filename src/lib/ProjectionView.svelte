<script lang="ts">
import { onMount } from "svelte";
import type { ModuleContext, ModuleView } from "../../packages/module-api/src/index";

let {
  title,
  view,
  context,
  onClose,
}: { title: string; view: ModuleView; context: ModuleContext; onClose: () => void } = $props();

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

<section class="projection-view-shell" aria-label={title}>
  <header class="projection-view-header">
    <button class="quiet-button" type="button" onclick={onClose}>Back to workspace</button>
    <div>
      <span class="overline">Workspace projection</span>
      <h1>{title}</h1>
    </div>
  </header>
  <div bind:this={container} class="projection-view-container">
    {#if error}<p class="projection-view-error">{error}</p>{/if}
  </div>
</section>

<style>
.projection-view-shell {
  display: flex;
  min-height: 0;
  height: calc(100vh - 58px);
  flex-direction: column;
}
.projection-view-header {
  display: flex;
  align-items: center;
  gap: 16px;
  min-height: 70px;
  padding: 12px 40px;
  border-bottom: 1px solid var(--line);
  background: var(--surface);
}
.projection-view-header > div {
  min-width: 0;
}
.projection-view-header .quiet-button {
  padding: 8px 10px;
  border: 1px solid #ded8cd;
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-soft);
  font: 500 11px/1.2 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  box-shadow: 0 1px 2px rgba(48, 45, 38, 0.05);
  cursor: pointer;
  transition:
    background 0.16s ease,
    border-color 0.16s ease,
    box-shadow 0.16s ease,
    color 0.16s ease,
    transform 0.16s ease;
}
.projection-view-header .quiet-button:hover {
  border-color: #cbbda9;
  background: var(--surface-muted);
  color: var(--ink);
  box-shadow: 0 3px 8px rgba(48, 45, 38, 0.08);
  transform: translateY(-1px);
}
.projection-view-header .quiet-button:active {
  box-shadow: 0 1px 2px rgba(48, 45, 38, 0.05);
  transform: translateY(1px);
}
.projection-view-header .quiet-button:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
.projection-view-header h1 {
  margin: 4px 0 0;
  font: 500 26px/1.05 var(--font-display);
}
.projection-view-container {
  min-height: 0;
  flex: 1;
  overflow: auto;
  padding: 24px 40px 40px;
  background: var(--canvas);
}
.projection-view-error {
  color: #a14f42;
  font-size: 12px;
}
@media (max-width: 760px) {
  .projection-view-header {
    display: block;
    padding: 14px 17px;
  }
  .projection-view-header h1 {
    margin-top: 12px;
    font-size: 24px;
  }
  .projection-view-container {
    padding: 20px 17px 28px;
  }
}
</style>
