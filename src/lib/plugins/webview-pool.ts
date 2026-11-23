/**
 * The shell's record of the embedded plugin webviews it keeps alive (see docs/plans/keep-alive-webviews.md).
 *
 * Pure and immutable, like `navigation/tabs.ts`: every function returns a new pool, or the same pool when nothing
 * changed, so a `$state` holding it does not invalidate for a no-op. The host stays the authority on which webviews
 * exist. This record decides which instances the shell asks the host to keep (`retainSet`) and which it unmounts
 * (`pruneTabs`, `prunePlugins`, `evicted`).
 */

export interface WebviewInstanceRef {
  pluginId: string;
  instanceId: string;
}

export interface WebviewPoolEntry extends WebviewInstanceRef {
  /** What the document was loaded with; see `webviewMountKey`. A different key means a new document. */
  mountKey: string;
  /** The instance reported ready since its document was last created. */
  ready: boolean;
  /** Milliseconds, supplied by the caller. */
  lastUsedAt: number;
}

export type WebviewPool = readonly WebviewPoolEntry[];

/** Embedded webviews kept alive at once, including the one on screen. */
export const WEBVIEW_POOL_LIMIT = 3;

export const emptyWebviewPool: WebviewPool = [];

export function sameWebviewInstance(a: WebviewInstanceRef, b: WebviewInstanceRef): boolean {
  return a.instanceId === b.instanceId && a.pluginId === b.pluginId;
}

/**
 * Everything the document reads once at load: the plugin, the view and every mount param. The host builds its own
 * key from the same values; this one only tells the shell when a pooled entry stopped matching.
 */
export function webviewMountKey(
  pluginId: string,
  params: {
    viewId?: string;
    mapEntityId?: string;
    linkId?: string;
    entityId?: string;
    moduleState?: string;
  } = {},
): string {
  return JSON.stringify([
    pluginId,
    params.viewId ?? null,
    params.mapEntityId ?? null,
    params.linkId ?? null,
    params.entityId ?? null,
    params.moduleState ?? null,
  ]);
}

export function findWebviewEntry(pool: WebviewPool, ref: WebviewInstanceRef): WebviewPoolEntry | undefined {
  return pool.find((entry) => sameWebviewInstance(entry, ref));
}

export function webviewRefs(pool: WebviewPool): WebviewInstanceRef[] {
  return pool.map(({ pluginId, instanceId }) => ({ pluginId, instanceId }));
}

/**
 * Records that `ref` was mounted or shown at `now`. An unknown instance is added, not ready. A known one keeps its
 * `ready` flag when its mount key is unchanged and loses it when the key changed, because that is a new document.
 */
export function touch(pool: WebviewPool, ref: WebviewInstanceRef, mountKey: string, now: number): WebviewPool {
  const existing = findWebviewEntry(pool, ref);
  if (!existing) {
    return [...pool, { pluginId: ref.pluginId, instanceId: ref.instanceId, mountKey, ready: false, lastUsedAt: now }];
  }
  const keyChanged = existing.mountKey !== mountKey;
  if (!keyChanged && existing.lastUsedAt === now) return pool;
  return pool.map((entry) =>
    entry === existing ? { ...entry, mountKey, ready: keyChanged ? false : entry.ready, lastUsedAt: now } : entry,
  );
}

/** Sets the `ready` flag of a known instance. An unknown instance is left out, not added. */
export function markReady(pool: WebviewPool, ref: WebviewInstanceRef, ready: boolean): WebviewPool {
  const existing = findWebviewEntry(pool, ref);
  if (!existing || existing.ready === ready) return pool;
  return pool.map((entry) => (entry === existing ? { ...entry, ready } : entry));
}

/**
 * The instances the host keeps through the next mount: the shown one first (always, even when the pool does not
 * know it yet), then the most recently used others, `limit` in all. Ties go to the entry added later.
 */
export function retainSet(pool: WebviewPool, shown: WebviewInstanceRef, limit: number): WebviewInstanceRef[] {
  const others = pool
    .map((entry, index) => ({ entry, index }))
    .filter(({ entry }) => !sameWebviewInstance(entry, shown))
    .sort((a, b) => b.entry.lastUsedAt - a.entry.lastUsedAt || b.index - a.index)
    .slice(0, Math.max(0, Math.floor(limit) - 1))
    .map(({ entry }) => ({ pluginId: entry.pluginId, instanceId: entry.instanceId }));
  return [{ pluginId: shown.pluginId, instanceId: shown.instanceId }, ...others];
}

/** Entries the host closes when only `retain` survives. */
export function evicted(pool: WebviewPool, retain: readonly WebviewInstanceRef[]): WebviewInstanceRef[] {
  return webviewRefs(pool.filter((entry) => !retain.some((kept) => sameWebviewInstance(entry, kept))));
}

/** Drops the given instances. */
export function removeWebviews(pool: WebviewPool, refs: readonly WebviewInstanceRef[]): WebviewPool {
  if (refs.length === 0) return pool;
  const next = pool.filter((entry) => !refs.some((ref) => sameWebviewInstance(entry, ref)));
  return next.length === pool.length ? pool : next;
}

/** Keeps only instances whose tab is still open. The instance id is the tab id. */
export function pruneTabs(pool: WebviewPool, liveTabIds: ReadonlySet<string>): WebviewPool {
  const next = pool.filter((entry) => liveTabIds.has(entry.instanceId));
  return next.length === pool.length ? pool : next;
}

/** Keeps only instances of plugins that are still enabled. */
export function prunePlugins(pool: WebviewPool, enabledPluginIds: ReadonlySet<string>): WebviewPool {
  const next = pool.filter((entry) => enabledPluginIds.has(entry.pluginId));
  return next.length === pool.length ? pool : next;
}
