/**
 * Shell side of the daena.language webview bridge (see the wave 2 protocol contract).
 *
 * Pure logic only: no Tauri or Svelte imports, so the route wires these helpers to
 * IPC and state, and scripts/language-bridge.test.mjs exercises them directly.
 */

import { isTabShortcut, type TabShortcut } from "../navigation/tab-shortcuts.ts";
import { PLUGIN_LEAVE_PENDING_TIMEOUT_MS, PluginLeaveTracker, type PluginLeaveTimers } from "./plugin-leave.ts";

export const LANGUAGE_PLUGIN_ID = "daena.language";
export const LANGUAGE_VIEW_ID = "language-workspace";
export const LANGUAGE_STATE_EVENT = "language-state";
export const LANGUAGE_PANES = ["overview", "lexicon", "sounds", "writing", "grammar", "forms", "samples"] as const;
export type LanguageBridgePane = (typeof LANGUAGE_PANES)[number];

/** A can-leave request with no `leave-pending` after this long means the webview is gone. */
export const LANGUAGE_LEAVE_PENDING_TIMEOUT_MS = PLUGIN_LEAVE_PENDING_TIMEOUT_MS;

const MAX_ID_LENGTH = 128;

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
  | { kind: "open-entity"; entityId: string; newTab?: boolean }
  | { kind: "shortcut"; shortcut: TabShortcut };

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
      if (entityId === null) return null;
      // Only a boolean is kept. Anything else is treated as absent, and a bad id is still rejected.
      return typeof payload.newTab === "boolean"
        ? { kind: "open-entity", entityId, newTab: payload.newTab }
        : { kind: "open-entity", entityId };
    }
    case "shortcut":
      return isTabShortcut(payload.shortcut) ? { kind: "shortcut", shortcut: payload.shortcut } : null;
    default:
      return null;
  }
}

export type LanguageLeaveTimers = PluginLeaveTimers;

/** Tracks can-leave requests sent to the Language webview; see `PluginLeaveTracker`. */
export const LanguageLeaveTracker = PluginLeaveTracker;
export type LanguageLeaveTracker = PluginLeaveTracker;

export { openableLinkHref } from "./plugin-links.ts";
