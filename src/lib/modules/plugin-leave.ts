/**
 * Shell-side tracking of "may this plugin webview be torn down?" requests. Pure
 * logic with no Tauri or Svelte imports; the Language and Writing bridges both
 * use it.
 */

/** A can-leave request with no `leave-pending` after this long means the webview is gone. */
export const PLUGIN_LEAVE_PENDING_TIMEOUT_MS = 1500;

export interface PluginLeaveTimers {
  set(callback: () => void, ms: number): unknown;
  clear(handle: unknown): void;
}

const browserTimers: PluginLeaveTimers = {
  set: (callback, ms) => setTimeout(callback, ms),
  clear: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
};

interface LeaveEntry {
  phase: "awaiting-pending" | "pending";
  resolve: (allowed: boolean) => void;
  timer: unknown;
}

/**
 * Tracks can-leave requests sent to a plugin webview.
 *
 * - `begin` starts a request. If no `leave-pending` arrives within the timeout the
 *   webview is treated as gone and the request resolves true.
 * - After `leave-pending`, the request waits for `leave-reply` with no timeout.
 * - `releaseAll` (the webview re-sent `ready`, or unmounted) resolves every open
 *   request true.
 */
export class PluginLeaveTracker {
  readonly #timers: PluginLeaveTimers;
  readonly #timeoutMs: number;
  readonly #entries = new Map<string, LeaveEntry>();

  constructor(options: { timers?: PluginLeaveTimers; timeoutMs?: number } = {}) {
    this.#timers = options.timers ?? browserTimers;
    this.#timeoutMs = options.timeoutMs ?? PLUGIN_LEAVE_PENDING_TIMEOUT_MS;
  }

  get outstanding(): number {
    return this.#entries.size;
  }

  begin(requestId: string): Promise<boolean> {
    this.#settle(requestId, true);
    return new Promise<boolean>((resolve) => {
      const timer = this.#timers.set(() => this.#settle(requestId, true), this.#timeoutMs);
      this.#entries.set(requestId, { phase: "awaiting-pending", resolve, timer });
    });
  }

  pending(requestId: string): void {
    const entry = this.#entries.get(requestId);
    if (!entry || entry.phase === "pending") return;
    this.#timers.clear(entry.timer);
    entry.timer = null;
    entry.phase = "pending";
  }

  reply(requestId: string, allowed: boolean): void {
    this.#settle(requestId, allowed);
  }

  releaseAll(): void {
    for (const requestId of [...this.#entries.keys()]) this.#settle(requestId, true);
  }

  #settle(requestId: string, allowed: boolean): void {
    const entry = this.#entries.get(requestId);
    if (!entry) return;
    this.#entries.delete(requestId);
    this.#timers.clear(entry.timer);
    entry.resolve(allowed);
  }
}
