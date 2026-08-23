<script lang="ts">
import { onMount } from "svelte";
import { CalendarRange, Languages, Network } from "@lucide/svelte";
import type { ModuleContext, ModuleView } from "../../packages/module-api/src/index";
import type { ProjectionKind } from "$lib/modules/projections";
import WorkspaceTopbar from "$lib/layout/WorkspaceTopbar.svelte";

let {
  title,
  subtitle,
  kind,
  view,
  context,
  onClose,
}: {
  title: string;
  subtitle: string;
  kind: ProjectionKind;
  view: ModuleView;
  context: ModuleContext;
  onClose: () => void;
} = $props();

let container = $state<HTMLElement | null>(null);
let error = $state("");
const topbarIcon = $derived(kind === "graph" ? Network : kind === "timeline" ? CalendarRange : Languages);

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
  <WorkspaceTopbar {title} {subtitle} icon={topbarIcon} onBack={onClose} />
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
.projection-view-container {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  overflow: auto;
  padding: 24px 40px 40px;
  background: var(--canvas);
}
.projection-view-error {
  color: #a14f42;
  font-size: 12px;
}
@media (max-width: 760px) {
  .projection-view-container {
    padding: 20px 17px 28px;
  }
}
</style>
