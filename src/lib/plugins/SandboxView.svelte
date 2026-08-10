<script lang="ts">
import { onMount, tick } from "svelte";
import { project } from "$lib/project/client";

let {
  pluginId,
  viewId,
  title,
  mapEntityId,
  linkId,
}: { pluginId: string; viewId?: string; title: string; mapEntityId?: string; linkId?: string } = $props();

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
    viewportWidth: Math.max(1, window.innerWidth),
    viewportHeight: Math.max(1, window.innerHeight),
  };
}

function scheduleResize() {
  if (!hostMounted || resizeFrame !== null) return;
  resizeFrame = window.requestAnimationFrame(() => {
    resizeFrame = null;
    const bounds = currentBounds();
    if (bounds)
      void project.resizePluginWebview(pluginId, bounds).catch((cause) => {
        error = cause instanceof Error ? cause.message : String(cause);
      });
  });
}

onMount(() => {
  let alive = true;
  const observer = new ResizeObserver(scheduleResize);
  if (container) observer.observe(container);
  const topbar = document.querySelector<HTMLElement>(".topbar");
  if (topbar) observer.observe(topbar);
  window.addEventListener("resize", scheduleResize);
  window.visualViewport?.addEventListener("resize", scheduleResize);

  const mount = async () => {
    await tick();
    await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
    if (!alive) return;
    // Close any leftover child before mounting so mapEntityId / linkId query
    // params are applied on a fresh document load rather than a reused view.
    await project.unmountPluginWebview(pluginId).catch(() => undefined);
    await project.closePluginWebview(pluginId).catch(() => undefined);
    if (!alive) return;
    const bounds = currentBounds();
    if (!bounds) throw new Error("sandbox view container is unavailable");
    await project.mountPluginWebview(pluginId, viewId, bounds, mapEntityId, linkId);
    if (alive) {
      hostMounted = true;
      scheduleResize();
    }
  };
  void mount().catch((cause) => {
    if (alive) error = cause instanceof Error ? cause.message : String(cause);
  });

  return () => {
    alive = false;
    hostMounted = false;
    observer.disconnect();
    window.removeEventListener("resize", scheduleResize);
    window.visualViewport?.removeEventListener("resize", scheduleResize);
    if (resizeFrame !== null) window.cancelAnimationFrame(resizeFrame);
    void project.unmountPluginWebview(pluginId);
    void project.closePluginWebview(pluginId);
  };
});
</script>

<section class="sandbox-view-shell" aria-label={title}>
  <div bind:this={container} class="sandbox-view-container" aria-busy={!hostMounted}>
    {#if error}<p class="sandbox-view-error">{error}</p>{:else if !hostMounted}<p class="sandbox-view-loading">
        Loading plugin view…
      </p>{/if}
  </div>
</section>

<style>
.sandbox-view-shell {
  display: flex;
  min-height: 0;
  height: auto;
  flex: 1 1 auto;
  flex-direction: column;
}
.sandbox-view-container {
  position: relative;
  min-height: 0;
  flex: 1;
  background: var(--surface);
}
.sandbox-view-loading,
.sandbox-view-error {
  margin: 24px 40px;
  color: var(--ink-soft);
  font-size: 12px;
}
.sandbox-view-error {
  color: #a14f42;
}
@media (max-width: 760px) {
  .sandbox-view-loading,
  .sandbox-view-error {
    margin: 20px 17px;
  }
}
</style>
