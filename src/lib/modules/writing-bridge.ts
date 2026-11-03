/**
 * Shell side of the daena.writing webview bridge (see docs/plans/writing-isolation.md).
 *
 * Pure logic only: no Tauri or Svelte imports, so the route wires these helpers to
 * IPC and state, and scripts/writing-bridge.test.mjs exercises them directly.
 */

export const WRITING_PLUGIN_ID = "daena.writing";
export const WRITING_VIEW_ID = "writing-workspace";
export const WRITING_STATE_EVENT = "writing-state";
export const WRITING_SHORTCUTS = ["quick-open", "create", "history-back", "history-forward"] as const;
export type WritingBridgeShortcut = (typeof WRITING_SHORTCUTS)[number];
export const WRITING_MODES = ["read", "edit"] as const;
export type WritingBridgeMode = (typeof WRITING_MODES)[number];

const MAX_ID_LENGTH = 128;
const MAX_HREF_LENGTH = 4096;
/** Draft text up to this many UTF-8 bytes arrives inline in a state event (events are capped at 256 KiB). */
export const WRITING_DRAFT_INLINE_LIMIT_BYTES = 128 * 1024;

export function isWritingShortcut(value: unknown): value is WritingBridgeShortcut {
  return typeof value === "string" && (WRITING_SHORTCUTS as readonly string[]).includes(value);
}

/** Text the webview hands over for one entity: inline, or a reference to a chunked transfer. */
export type WritingBridgeDraft =
  | { entityId: string; baseRevision: string; text: string }
  | { entityId: string; baseRevision: string; transfer: string };

/** Messages the webview publishes as `daena.writing/state@1`, after the host forwards them. */
export type WritingStateEvent =
  | { kind: "ready"; entityId: string | null }
  | { kind: "draft"; draft: WritingBridgeDraft }
  | { kind: "flush-reply"; requestId: string; draft: WritingBridgeDraft | null }
  | { kind: "open-link"; href: string }
  | { kind: "open-entity"; entityId: string }
  | { kind: "fullscreen"; enabled: boolean }
  | { kind: "request-rename"; entityId: string }
  | { kind: "request-archive"; entityId: string }
  | { kind: "shortcut"; shortcut: WritingBridgeShortcut };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function idOf(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 && value.length <= MAX_ID_LENGTH ? value : null;
}

function utf8Length(text: string): number {
  return new TextEncoder().encode(text).byteLength;
}

function parseDraft(value: unknown): WritingBridgeDraft | null {
  if (!isRecord(value)) return null;
  const entityId = idOf(value.entityId);
  if (entityId === null || typeof value.baseRevision !== "string") return null;
  const inline = typeof value.text === "string";
  // Exactly one of `text` and `transfer`.
  if (inline === (value.transfer !== undefined)) return null;
  if (inline) {
    const text = value.text as string;
    if (utf8Length(text) > WRITING_DRAFT_INLINE_LIMIT_BYTES) return null;
    return { entityId, baseRevision: value.baseRevision, text };
  }
  const transfer = idOf(value.transfer);
  return transfer === null ? null : { entityId, baseRevision: value.baseRevision, transfer };
}

/** Entity types the Writing webview renders. Other types in a Writing tab keep the shell editor. */
export const WRITING_WEBVIEW_ENTITY_TYPES: readonly string[] = [
  "daena.writing:manuscript",
  "daena.writing:reference-page",
];

export function writingWebviewHandles(entityType: string | null | undefined): boolean {
  return entityType != null && WRITING_WEBVIEW_ENTITY_TYPES.includes(entityType);
}

/**
 * Validates a forwarded `writing-state` payload. Unknown kinds and malformed
 * fields return null so the caller ignores them.
 */
export function parseWritingStateEvent(payload: unknown): WritingStateEvent | null {
  if (!isRecord(payload)) return null;
  switch (payload.kind) {
    case "ready": {
      const entityId = payload.entityId ?? null;
      if (entityId !== null && idOf(entityId) === null) return null;
      return { kind: "ready", entityId: entityId === null ? null : (entityId as string) };
    }
    case "draft": {
      const draft = parseDraft(payload.draft);
      return draft === null ? null : { kind: "draft", draft };
    }
    case "flush-reply": {
      const requestId = idOf(payload.requestId);
      if (requestId === null) return null;
      if (payload.draft === null) return { kind: "flush-reply", requestId, draft: null };
      const draft = parseDraft(payload.draft);
      return draft === null ? null : { kind: "flush-reply", requestId, draft };
    }
    case "open-link":
      return typeof payload.href === "string" && payload.href.length <= MAX_HREF_LENGTH
        ? { kind: "open-link", href: payload.href }
        : null;
    case "open-entity":
    case "request-rename":
    case "request-archive": {
      const entityId = idOf(payload.entityId);
      return entityId === null
        ? null
        : { kind: payload.kind as "open-entity" | "request-rename" | "request-archive", entityId };
    }
    case "fullscreen":
      return typeof payload.enabled === "boolean" ? { kind: "fullscreen", enabled: payload.enabled } : null;
    case "shortcut":
      return isWritingShortcut(payload.shortcut) ? { kind: "shortcut", shortcut: payload.shortcut } : null;
    default:
      return null;
  }
}

/** Messages the shell posts into the webview with `plugin_webview_post`. */
export type WritingHostMessage =
  | {
      type: "writing.load";
      entityId: string;
      revision: string;
      editable: boolean;
      placeholder: string;
      mode: WritingBridgeMode;
    }
  | { type: "writing.flush"; requestId: string }
  | { type: "writing.saved"; entityId: string; revision: string }
  | { type: "writing.conflict"; entityId: string; message: string }
  | { type: "writing.editable"; editable: boolean }
  | { type: "writing.ai-state"; enabled: boolean };

/**
 * Builds the `writing.load` message for a selection. Slice 1 shows the document read-only, so `editable`
 * stays false until the shell takes drafts back from the webview.
 */
export function writingLoadMessage(
  entityId: string,
  revision: string,
  options: { editable?: boolean; placeholder?: string; mode?: WritingBridgeMode } = {},
): WritingHostMessage {
  return {
    type: "writing.load",
    entityId,
    revision,
    editable: options.editable ?? false,
    placeholder: options.placeholder ?? "",
    mode: options.mode ?? "read",
  };
}
