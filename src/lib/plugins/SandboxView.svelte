<script lang="ts">
  import { onMount } from "svelte";
  import { project } from "$lib/project/client";

  let {
    pluginId,
    viewId,
    title,
    onClose,
  }: { pluginId: string; viewId?: string; title: string; onClose: () => void } = $props();

  let container = $state<HTMLElement | null>(null);
  let hostMounted = $state(false);
  let resizeFrame: number | null = null;
  let error = $state("");

  function currentBounds() {
    const rect = container?.getBoundingClientRect();
    if (!rect) return null;
    return {
      x: Math.max(0, rect.left),
      y: Math.max(0, rect.top),
      width: Math.max(1, rect.width),
      height: Math.max(1, rect.height),
    };
  }

  function scheduleResize() {
    if (!hostMounted || resizeFrame !== null) return;
    resizeFrame = window.requestAnimationFrame(() => {
      resizeFrame = null;
      const bounds = currentBounds();
      if (bounds) void project.resizePluginWebview(pluginId, bounds).catch((cause) => {
        error = cause instanceof Error ? cause.message : String(cause);
      });
    });
  }

  onMount(() => {
    let alive = true;
    const observer = new ResizeObserver(scheduleResize);
    if (container) observer.observe(container);
    window.addEventListener("resize", scheduleResize);

    const mount = async () => {
      const bounds = currentBounds();
      if (!bounds) throw new Error("sandbox view container is unavailable");
      await project.mountPluginWebview(pluginId, viewId, bounds);
      if (alive) hostMounted = true;
    };
    void mount().catch((cause) => {
      if (alive) error = cause instanceof Error ? cause.message : String(cause);
    });

    return () => {
      alive = false;
      hostMounted = false;
      observer.disconnect();
      window.removeEventListener("resize", scheduleResize);
      if (resizeFrame !== null) window.cancelAnimationFrame(resizeFrame);
      void project.unmountPluginWebview(pluginId);
    };
  });
</script>

<section class="sandbox-view-shell" aria-label={title}>
  <header class="sandbox-view-header">
    <button class="quiet-button" type="button" onclick={onClose}>Back to workspace</button>
    <div>
      <span class="overline">Sandboxed plugin</span>
      <h1>{title}</h1>
    </div>
  </header>
  <div bind:this={container} class="sandbox-view-container" aria-busy={!hostMounted}>
    {#if error}<p class="sandbox-view-error">{error}</p>{:else if !hostMounted}<p class="sandbox-view-loading">Loading plugin view…</p>{/if}
  </div>
</section>

<style>
  .sandbox-view-shell { display: flex; min-height: 0; height: calc(100vh - 58px); flex-direction: column; }
  .sandbox-view-header { display: flex; align-items: center; gap: 16px; min-height: 70px; padding: 12px 40px; border-bottom: 1px solid var(--line); background: var(--surface); }
  .sandbox-view-header > div { min-width: 0; }
  .sandbox-view-header h1 { margin: 4px 0 0; font: 500 26px/1.05 var(--font-display); }
  .sandbox-view-container { position: relative; min-height: 0; flex: 1; background: var(--surface); }
  .sandbox-view-loading, .sandbox-view-error { margin: 24px 40px; color: var(--ink-soft); font-size: 12px; }
  .sandbox-view-error { color: #a14f42; }
  @media (max-width: 760px) {
    .sandbox-view-shell { height: calc(100vh - 58px); }
    .sandbox-view-header { display: block; padding: 14px 17px; }
    .sandbox-view-header h1 { margin-top: 12px; font-size: 24px; }
    .sandbox-view-loading, .sandbox-view-error { margin: 20px 17px; }
  }
</style>
