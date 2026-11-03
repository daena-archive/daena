/**
 * Shell side of the daena.language webview bridge (see the wave 2 protocol contract).
 *
 * Pure logic only: no Tauri or Svelte imports, so the route wires these helpers to
 * IPC and state, and scripts/language-bridge.test.mjs exercises them directly.
 */

export const LANGUAGE_PLUGIN_ID = "daena.language";
export const LANGUAGE_VIEW_ID = "language-workspace";
export const LANGUAGE_STATE_EVENT = "language-state";
export const LANGUAGE_PANES = ["overview", "lexicon", "sounds", "writing", "grammar", "forms", "samples"] as const;
export type LanguageBridgePane = (typeof LANGUAGE_PANES)[number];

/** A can-leave request with no `leave-pending` after this long means the webview is gone. */
export const LANGUAGE_LEAVE_PENDING_TIMEOUT_MS = 1500;

const MAX_ID_LENGTH = 128;
const MAX_HREF_LENGTH = 4096;
const OPENABLE_PROTOCOLS: ReadonlySet<string> = new Set(["http:", "https:", "mailto:"]);

export function isLanguagePane(value: unknown): value is LanguageBridgePane {
  return typeof value === "string" && (LANGUAGE_PANES as readonly string[]).includes(value);
}

/** Messages the webview publishes as `daena.language/state@1`, after the host forwards them. */
export type LanguageStateEvent =
  | { kind: "ready"; pane: LanguageBridgePane | null; entityId: string | null }
  | { kind: "pane"; pane: LanguageBridgePane }
  | { kind: "leave-pending"; requestId: string }
  | { kind: "leave-reply"; requestId: string; allowed: boolean }
  | { kind: "entity-changed"; entityId: string; deleted: boolean }
  | { kind: "open-link"; href: string }
  | { kind: "open-entity"; entityId: string };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function idOf(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 && value.length <= MAX_ID_LENGTH ? value : null;
}

/**
 * Validates a forwarded `language-state` payload. Unknown kinds and malformed
 * fields return null so the caller ignores them.
 */
export function parseLanguageStateEvent(payload: unknown): LanguageStateEvent | null {
  if (!isRecord(payload)) return null;
  switch (payload.kind) {
    case "ready": {
      const entityId = payload.entityId ?? null;
      if (entityId !== null && idOf(entityId) === null) return null;
      return {
        kind: "ready",
        pane: isLanguagePane(payload.pane) ? payload.pane : null,
        entityId: entityId === null ? null : (entityId as string),
      };
    }
    case "pane":
      return isLanguagePane(payload.pane) ? { kind: "pane", pane: payload.pane } : null;
    case "leave-pending": {
      const requestId = idOf(payload.requestId);
      return requestId === null ? null : { kind: "leave-pending", requestId };
    }
    case "leave-reply": {
      const requestId = idOf(payload.requestId);
      if (requestId === null || typeof payload.allowed !== "boolean") return null;
      return { kind: "leave-reply", requestId, allowed: payload.allowed };
    }
    case "entity-changed": {
      const entityId = idOf(payload.entityId);
      if (entityId === null || typeof payload.deleted !== "boolean") return null;
      return { kind: "entity-changed", entityId, deleted: payload.deleted };
    }
    case "open-link":
      return typeof payload.href === "string" ? { kind: "open-link", href: payload.href } : null;
    case "open-entity": {
      const entityId = idOf(payload.entityId);
      return entityId === null ? null : { kind: "open-entity", entityId };
    }
    default:
      return null;
  }
}

/**
 * Returns the normalised href when the shell may open it outside the app
 * (http, https, mailto only), otherwise null.
 */
export function openableLinkHref(href: unknown): string | null {
  if (typeof href !== "string") return null;
  const trimmed = href.trim();
  if (trimmed.length === 0 || trimmed.length > MAX_HREF_LENGTH) return null;
  let url: URL;
  try {
    url = new URL(trimmed);
  } catch {
    return null;
  }
  return OPENABLE_PROTOCOLS.has(url.protocol) ? url.href : null;
}

export interface LanguageLeaveTimers {
  set(callback: () => void, ms: number): unknown;
  clear(handle: unknown): void;
}

const browserTimers: LanguageLeaveTimers = {
  set: (callback, ms) => setTimeout(callback, ms),
  clear: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
};

interface LeaveEntry {
  phase: "awaiting-pending" | "pending";
  resolve: (allowed: boolean) => void;
  timer: unknown;
}

/**
 * Tracks can-leave requests sent to the Language webview.
 *
 * - `begin` starts a request. If no `leave-pending` arrives within the timeout the
 *   webview is treated as gone and the request resolves true.
 * - After `leave-pending`, the request waits for `leave-reply` with no timeout.
 * - `releaseAll` (the webview re-sent `ready`, or unmounted) resolves every open
 *   request true.
 */
export class LanguageLeaveTracker {
  readonly #timers: LanguageLeaveTimers;
  readonly #timeoutMs: number;
  readonly #entries = new Map<string, LeaveEntry>();

  constructor(options: { timers?: LanguageLeaveTimers; timeoutMs?: number } = {}) {
    this.#timers = options.timers ?? browserTimers;
    this.#timeoutMs = options.timeoutMs ?? LANGUAGE_LEAVE_PENDING_TIMEOUT_MS;
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
