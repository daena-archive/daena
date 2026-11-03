/**
 * Pure wire helpers for the isolated `daena.writing` webview. The shell is the
 * only writer: the webview hosts the editor and hands text to the shell, which
 * saves it. Everything here is node-testable and free of DOM or Tauri
 * dependencies; the shell side is `src/lib/modules/writing-bridge.ts`.
 */

export const WRITING_PLUGIN_ID = "daena.writing";
export const WRITING_STATE_EVENT_TYPE = "daena.writing/state@1";

export const WRITING_MODES = ["read", "edit"] as const;
export type WritingMode = (typeof WRITING_MODES)[number];
export const DEFAULT_WRITING_MODE: WritingMode = "read";

/** Shell shortcuts the webview forwards, because key events inside it never reach the main webview. */
export const WRITING_SHORTCUTS = ["quick-open", "create", "history-back", "history-forward"] as const;
export type WritingShortcut = (typeof WRITING_SHORTCUTS)[number];

/** Longest id or request id either side accepts. */
export const MAX_WRITING_ID_LENGTH = 128;
/** Draft text up to this many UTF-8 bytes travels inline in a state event (events are capped at 256 KiB). */
export const WRITING_DRAFT_INLINE_LIMIT_BYTES = 128 * 1024;
const MAX_HREF_LENGTH = 4096;

export function isWritingMode(value: unknown): value is WritingMode {
  return typeof value === "string" && (WRITING_MODES as readonly string[]).includes(value);
}

export function isWritingShortcut(value: unknown): value is WritingShortcut {
  return typeof value === "string" && (WRITING_SHORTCUTS as readonly string[]).includes(value);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function boundedId(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 && value.length <= MAX_WRITING_ID_LENGTH ? value : null;
}

function nonEmpty(value: string | null): string | null {
  return value && value.length > 0 ? value : null;
}

function utf8Length(text: string): number {
  return new TextEncoder().encode(text).byteLength;
}

export interface WritingMountParams {
  projectId: string | null;
  /** Entity the shell had selected when it mounted the webview; null when none is selected. */
  entityId: string | null;
}

/** Reads the mount parameters SandboxView appends to the plugin document URL. */
export function parseWritingMountParams(search: string): WritingMountParams {
  const params = new URLSearchParams(search);
  return {
    projectId: nonEmpty(params.get("project")),
    entityId: nonEmpty(params.get("entityId")),
  };
}

/**
 * Text the webview hands to the shell for one entity: inline for ordinary documents, or a reference to a
 * chunked transfer for text too large for one event. `baseRevision` is the document revision the text was
 * edited from, so the shell can tell a stale draft from a current one.
 */
export type WritingDraft =
  | { entityId: string; baseRevision: string; text: string }
  | { entityId: string; baseRevision: string; transfer: string };

/** Validates a draft. Text over the inline limit and malformed fields return null. */
export function parseWritingDraft(value: unknown): WritingDraft | null {
  if (!isRecord(value)) return null;
  const entityId = boundedId(value.entityId);
  if (entityId === null || typeof value.baseRevision !== "string") return null;
  const inline = typeof value.text === "string";
  // Exactly one of `text` and `transfer`.
  if (inline === (value.transfer !== undefined)) return null;
  if (inline) {
    const text = value.text as string;
    if (utf8Length(text) > WRITING_DRAFT_INLINE_LIMIT_BYTES) return null;
    return { entityId, baseRevision: value.baseRevision, text };
  }
  const transfer = boundedId(value.transfer);
  return transfer === null ? null : { entityId, baseRevision: value.baseRevision, transfer };
}

/** Messages the shell posts into the webview (`plugin_webview_post`, capped at 64 KiB). */
export type WritingHostMessage =
  | {
      type: "writing.load";
      entityId: string;
      revision: string;
      editable: boolean;
      placeholder: string;
      mode: WritingMode;
    }
  | { type: "writing.flush"; requestId: string }
  | { type: "writing.saved"; entityId: string; revision: string }
  | { type: "writing.conflict"; entityId: string; message: string }
  | { type: "writing.editable"; editable: boolean }
  | { type: "writing.ai-state"; enabled: boolean };

const MAX_PLACEHOLDER_LENGTH = 200;
const MAX_CONFLICT_MESSAGE_LENGTH = 2000;

/**
 * Validates a host message. `revision` is required (an entity with no document yet has the empty revision);
 * `editable`, `placeholder` and `mode` fall back to a read-only, read-mode view when the shell omits them.
 */
export function parseWritingHostMessage(value: unknown): WritingHostMessage | null {
  if (!isRecord(value)) return null;
  switch (value.type) {
    case "writing.load": {
      const entityId = boundedId(value.entityId);
      if (entityId === null || typeof value.revision !== "string") return null;
      if (value.editable !== undefined && typeof value.editable !== "boolean") return null;
      if (value.placeholder !== undefined && typeof value.placeholder !== "string") return null;
      if (value.mode !== undefined && !isWritingMode(value.mode)) return null;
      return {
        type: "writing.load",
        entityId,
        revision: value.revision,
        editable: value.editable ?? false,
        placeholder: (value.placeholder ?? "").slice(0, MAX_PLACEHOLDER_LENGTH),
        mode: value.mode ?? DEFAULT_WRITING_MODE,
      };
    }
    case "writing.flush": {
      const requestId = boundedId(value.requestId);
      return requestId === null ? null : { type: "writing.flush", requestId };
    }
    case "writing.saved": {
      const entityId = boundedId(value.entityId);
      if (entityId === null || typeof value.revision !== "string") return null;
      return { type: "writing.saved", entityId, revision: value.revision };
    }
    case "writing.conflict": {
      const entityId = boundedId(value.entityId);
      if (entityId === null) return null;
      if (value.message !== undefined && typeof value.message !== "string") return null;
      return {
        type: "writing.conflict",
        entityId,
        message: (value.message ?? "").slice(0, MAX_CONFLICT_MESSAGE_LENGTH),
      };
    }
    case "writing.editable":
      return typeof value.editable === "boolean" ? { type: "writing.editable", editable: value.editable } : null;
    case "writing.ai-state":
      return typeof value.enabled === "boolean" ? { type: "writing.ai-state", enabled: value.enabled } : null;
    default:
      return null;
  }
}

/** Payloads the webview publishes back to the shell as `daena.writing/state@1`. */
export type WritingStatePayload =
  | { kind: "ready"; entityId: string | null }
  | { kind: "draft"; draft: WritingDraft }
  /** Answer to `writing.flush`; `draft` is null when the webview holds no unsaved text. */
  | { kind: "flush-reply"; requestId: string; draft: WritingDraft | null }
  | { kind: "open-link"; href: string }
  | { kind: "open-entity"; entityId: string }
  | { kind: "fullscreen"; enabled: boolean }
  | { kind: "request-rename"; entityId: string }
  | { kind: "request-archive"; entityId: string }
  | { kind: "shortcut"; shortcut: WritingShortcut };

/** Validates a state payload; the shell bridge applies the same rules to what it receives. */
export function parseWritingStatePayload(value: unknown): WritingStatePayload | null {
  if (!isRecord(value)) return null;
  switch (value.kind) {
    case "ready": {
      const entityId = value.entityId ?? null;
      if (entityId !== null && boundedId(entityId) === null) return null;
      return { kind: "ready", entityId: entityId as string | null };
    }
    case "draft": {
      const draft = parseWritingDraft(value.draft);
      return draft === null ? null : { kind: "draft", draft };
    }
    case "flush-reply": {
      const requestId = boundedId(value.requestId);
      if (requestId === null) return null;
      if (value.draft === null) return { kind: "flush-reply", requestId, draft: null };
      const draft = parseWritingDraft(value.draft);
      return draft === null ? null : { kind: "flush-reply", requestId, draft };
    }
    case "open-link":
      return typeof value.href === "string" && value.href.length <= MAX_HREF_LENGTH
        ? { kind: "open-link", href: value.href }
        : null;
    case "open-entity":
    case "request-rename":
    case "request-archive": {
      const entityId = boundedId(value.entityId);
      return entityId === null
        ? null
        : { kind: value.kind as "open-entity" | "request-rename" | "request-archive", entityId };
    }
    case "fullscreen":
      return typeof value.enabled === "boolean" ? { kind: "fullscreen", enabled: value.enabled } : null;
    case "shortcut":
      return isWritingShortcut(value.shortcut) ? { kind: "shortcut", shortcut: value.shortcut } : null;
    default:
      return null;
  }
}

/** What the workspace needs from its host; the plugin entry wires it to the host message channel. */
export interface WritingWorkspaceBridge {
  /** Subscribes to host messages. Returns an unsubscribe function. */
  onHostMessage(handler: (message: WritingHostMessage) => void): () => void;
  /** Publishes state to the shell. */
  publish(payload: WritingStatePayload): void;
}

export interface WritingRpcPublisher {
  call<T>(method: string, payload: unknown): Promise<T>;
}

/**
 * Builds the function that publishes Writing state to the shell. Publishing
 * never throws into the caller: errors are reported to `onError` instead.
 */
export function createWritingStatePublisher(
  rpc: WritingRpcPublisher,
  onError: (cause: unknown) => void = () => undefined,
): (payload: WritingStatePayload) => Promise<void> {
  return async (payload) => {
    try {
      await rpc.call("event.publish", { type: WRITING_STATE_EVENT_TYPE, payload });
    } catch (cause) {
      onError(cause);
    }
  };
}

/**
 * Picks Writing's effective manifest from `schema.list`. The bundled
 * manifest.json carries unqualified entity type ids, which never match stored
 * entities, so it is only a fallback for hosts that do not list Writing.
 */
export function pickWritingManifest<T extends { id: string }>(manifests: readonly T[], fallback: T): T {
  return manifests.find((manifest) => manifest.id === WRITING_PLUGIN_ID) ?? fallback;
}
