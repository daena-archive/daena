<script lang="ts">
import { onMount, tick } from "svelte";
import { project } from "$lib/project/client";

let {
  pluginId,
  instanceId,
  viewId,
  title,
  mapEntityId,
  linkId,
  entityId,
  moduleState,
  holdMount = false,
}: {
  pluginId: string;
  instanceId: string;
  viewId?: string;
  title: string;
  mapEntityId?: string;
  linkId?: string;
  entityId?: string;
  moduleState?: Record<string, unknown> | null;
  /**
   * While true, the webview is not created yet. The shell sets it during a tab switch: the active tab (and so
   * `instanceId`) changes before the target location is restored, and a view that is about to be replaced must
   * not create a webview for the wrong tab.
   */
  holdMount?: boolean;
} = $props();

let releaseWaiters: Array<() => void> = [];

$effect(() => {
  if (holdMount) return;
  const waiters = releaseWaiters;
  releaseWaiters = [];
  for (const release of waiters) release();
});

function mountReleased(): Promise<void> {
  if (!holdMount) return Promise.resolve();
  return new Promise((resolve) => releaseWaiters.push(resolve));
}

let container = $state<HTMLElement | null>(null);
let hostMounted = $state(false);
let resizeFrame: number | null = null;
// Rounded bounds last sent to the host, so a re-sync that finds nothing moved skips the IPC call.
let lastBoundsKey: string | null = null;
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

function boundsKey(bounds: NonNullable<ReturnType<typeof currentBounds>>) {
  return [bounds.x, bounds.y, bounds.width, bounds.height, bounds.viewportWidth, bounds.viewportHeight]
    .map((value) => Math.round(value))
    .join(",");
}

function scheduleResize() {
  if (!hostMounted || resizeFrame !== null) return;
  resizeFrame = window.requestAnimationFrame(() => {
    resizeFrame = null;
    const bounds = currentBounds();
    if (!bounds) return;
    const key = boundsKey(bounds);
    if (key === lastBoundsKey) return;
    lastBoundsKey = key;
    void project.resizePluginWebview(pluginId, instanceId, bounds).catch((cause) => {
      lastBoundsKey = null;
      error = cause instanceof Error ? cause.message : String(cause);
    });
  });
}

/**
 * The container moves when anything above it changes size, not only when it resizes itself.
 * Observe the container, each ancestor up to document.body, and the preceding siblings of both.
 */
function observePositionSources(observer: ResizeObserver) {
  for (let node: HTMLElement | null = container; node && node !== document.body; node = node.parentElement) {
    observer.observe(node);
    for (let sibling = node.previousElementSibling; sibling; sibling = sibling.previousElementSibling)
      observer.observe(sibling);
  }
}

onMount(() => {
  // Captured for teardown. A `{#key}` swap creates the next view before this
  // cleanup runs. Closing every webview of this plugin would close that newer instance.
  const mountedPluginId = pluginId;
  const mountedInstanceId = instanceId;
  let alive = true;
  const observer = new ResizeObserver(scheduleResize);
  observePositionSources(observer);
  window.addEventListener("resize", scheduleResize);
  window.visualViewport?.addEventListener("resize", scheduleResize);
  // Scroll events do not bubble, so the capture listener on window sees scrolling in any ancestor.
  window.addEventListener("scroll", scheduleResize, { capture: true, passive: true });
  // Fonts that finish loading after mount can change the height of the content above the container.
  void document.fonts?.ready.then(() => {
    if (alive) scheduleResize();
  });

  const mount = async () => {
    await tick();
    await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
    await mountReleased();
    if (!alive) return;
    lastBoundsKey = null;
    // Drop this instance's previous child so the mount query params
    // (mapEntityId, linkId, entityId, state) load in a fresh document.
    // The host mount closes every other embedded plugin webview.
    await project.unmountPluginWebview(mountedPluginId, mountedInstanceId).catch(() => undefined);
    if (!alive) return;
    const bounds = currentBounds();
    if (!bounds) throw new Error("sandbox view container is unavailable");
    const serialisedState = moduleState ? JSON.stringify(moduleState) : undefined;
    await project.mountPluginWebview(
      mountedPluginId,
      mountedInstanceId,
      viewId,
      bounds,
      mapEntityId,
      linkId,
      entityId,
      serialisedState,
    );
    if (alive) {
      hostMounted = true;
      lastBoundsKey = boundsKey(bounds);
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
    window.removeEventListener("scroll", scheduleResize, { capture: true });
    if (resizeFrame !== null) window.cancelAnimationFrame(resizeFrame);
    void project.unmountPluginWebview(mountedPluginId, mountedInstanceId);
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
  color: var(--danger);
}
@media (max-width: 760px) {
  .sandbox-view-loading,
  .sandbox-view-error {
    margin: 20px 17px;
  }
}
</style>
