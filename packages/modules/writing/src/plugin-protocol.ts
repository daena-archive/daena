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

/** What the user chose in the conflict banner; the shell runs the matching action it already has. */
export const WRITING_CONFLICT_ACTIONS = ["reload", "overwrite", "recovery"] as const;
export type WritingConflictAction = (typeof WRITING_CONFLICT_ACTIONS)[number];

/** Longest id, token or request id either side accepts. */
export const MAX_WRITING_ID_LENGTH = 128;
/** Draft text up to this many UTF-8 bytes travels inline in a state event (events are capped at 256 KiB). */
export const WRITING_DRAFT_INLINE_LIMIT_BYTES = 128 * 1024;
const MAX_HREF_LENGTH = 4096;
const MAX_TITLE_LENGTH = 512;

export function isWritingMode(value: unknown): value is WritingMode {
  return typeof value === "string" && (WRITING_MODES as readonly string[]).includes(value);
}

export function isWritingShortcut(value: unknown): value is WritingShortcut {
  return typeof value === "string" && (WRITING_SHORTCUTS as readonly string[]).includes(value);
}

export function isWritingConflictAction(value: unknown): value is WritingConflictAction {
  return typeof value === "string" && (WRITING_CONFLICT_ACTIONS as readonly string[]).includes(value);
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
 * chunked transfer (`text.transfer.*`) for text too large for one event. `token` is the opaque value the
 * shell sent in the `writing.load` the text was edited under; the shell drops a draft whose entity or token
 * is no longer current. Saving does not change the token.
 */
export type WritingDraft =
  { entityId: string; token: string; text: string } | { entityId: string; token: string; transfer: string };

/** Validates a draft. Text over the inline limit and malformed fields return null. */
export function parseWritingDraft(value: unknown): WritingDraft | null {
  if (!isRecord(value)) return null;
  const entityId = boundedId(value.entityId);
  const token = boundedId(value.token);
  if (entityId === null || token === null) return null;
  const inline = typeof value.text === "string";
  // Exactly one of `text` and `transfer`.
  if (inline === (value.transfer !== undefined)) return null;
  if (inline) {
    const text = value.text as string;
    if (utf8Length(text) > WRITING_DRAFT_INLINE_LIMIT_BYTES) return null;
    return { entityId, token, text };
  }
  const transfer = boundedId(value.transfer);
  return transfer === null ? null : { entityId, token, transfer };
}

/** Messages the shell posts into the webview (`plugin_webview_post`, capped at 64 KiB). */
export type WritingHostMessage =
  | {
      type: "writing.load";
      entityId: string;
      /** Opaque value the shell issues per load; the webview echoes it on every draft. */
      token: string;
      revision: string;
      editable: boolean;
      placeholder: string;
      mode: WritingMode;
    }
  | { type: "writing.flush"; requestId: string }
  | { type: "writing.saved"; entityId: string; revision: string }
  | { type: "writing.conflict"; entityId: string; message: string; canOverwrite: boolean }
  | { type: "writing.save-error"; entityId: string; message: string }
  | { type: "writing.editable"; editable: boolean }
  | { type: "writing.ai-state"; enabled: boolean }
  /** The shell turns the webview's full screen off (selection change, navigation, Escape in the shell). */
  | { type: "writing.fullscreen"; on: boolean }
  /** The entry was renamed; the header updates without a reload, so the editor keeps its text and focus. */
  | { type: "writing.title"; entityId: string; title: string };

const MAX_PLACEHOLDER_LENGTH = 200;
const MAX_CONFLICT_MESSAGE_LENGTH = 2000;

/**
 * Validates a host message. `token` and `revision` are required (an entity with no document yet has the empty
 * revision); `editable`, `placeholder` and `mode` fall back to a read-only, read-mode view when the shell
 * omits them.
 */
export function parseWritingHostMessage(value: unknown): WritingHostMessage | null {
  if (!isRecord(value)) return null;
  switch (value.type) {
    case "writing.load": {
      const entityId = boundedId(value.entityId);
      const token = boundedId(value.token);
      if (entityId === null || token === null || typeof value.revision !== "string") return null;
      if (value.editable !== undefined && typeof value.editable !== "boolean") return null;
      if (value.placeholder !== undefined && typeof value.placeholder !== "string") return null;
      if (value.mode !== undefined && !isWritingMode(value.mode)) return null;
      return {
        type: "writing.load",
        entityId,
        token,
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
      if (value.canOverwrite !== undefined && typeof value.canOverwrite !== "boolean") return null;
      return {
        type: "writing.conflict",
        entityId,
        message: (value.message ?? "").slice(0, MAX_CONFLICT_MESSAGE_LENGTH),
        canOverwrite: value.canOverwrite ?? true,
      };
    }
    case "writing.save-error": {
      const entityId = boundedId(value.entityId);
      if (entityId === null || typeof value.message !== "string") return null;
      return { type: "writing.save-error", entityId, message: value.message.slice(0, MAX_CONFLICT_MESSAGE_LENGTH) };
    }
    case "writing.editable":
      return typeof value.editable === "boolean" ? { type: "writing.editable", editable: value.editable } : null;
    case "writing.ai-state":
      return typeof value.enabled === "boolean" ? { type: "writing.ai-state", enabled: value.enabled } : null;
    case "writing.fullscreen":
      return typeof value.on === "boolean" ? { type: "writing.fullscreen", on: value.on } : null;
    case "writing.title": {
      const entityId = boundedId(value.entityId);
      if (entityId === null || typeof value.title !== "string") return null;
      return { type: "writing.title", entityId, title: value.title.slice(0, MAX_TITLE_LENGTH) };
    }
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
  /** The user's choice in the conflict banner; the shell checks entity and token like it does for a draft. */
  | { kind: "conflict-action"; entityId: string; token: string; action: WritingConflictAction }
  | { kind: "open-link"; href: string }
  | { kind: "open-entity"; entityId: string }
  /** The webview's own full screen; `on` is the state after the change, and `entityId` is the entry it is for. */
  | { kind: "fullscreen"; entityId: string; on: boolean }
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
    case "conflict-action": {
      const entityId = boundedId(value.entityId);
      const token = boundedId(value.token);
      if (entityId === null || token === null || !isWritingConflictAction(value.action)) return null;
      return { kind: "conflict-action", entityId, token, action: value.action };
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
    case "fullscreen": {
      const entityId = boundedId(value.entityId);
      return entityId !== null && typeof value.on === "boolean" ? { kind: "fullscreen", entityId, on: value.on } : null;
    }
    case "shortcut":
      return isWritingShortcut(value.shortcut) ? { kind: "shortcut", shortcut: value.shortcut } : null;
    default:
      return null;
  }
}

/** A key press as the shortcut mapping reads it. A KeyboardEvent has all of these fields. */
export interface WritingKeyPress {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  repeat: boolean;
  /** True when the key went to a text field or the editor. Alt+arrows move by word there, so history ignores them. */
  editable?: boolean;
}

/** Whether a key event's target takes text, where Alt+arrows move the caret by word. */
export function isEditableKeyTarget(target: EventTarget | null): boolean {
  return (
    typeof Element !== "undefined" &&
    target instanceof Element &&
    target.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"])') !== null
  );
}

/**
 * The shell shortcut a key press in the webview stands for. It applies the same rules as the shell's window
 * listeners (`handleHistoryKey` and `handleQuickOpenKey` in src/routes/+page.svelte), so a key does the same thing
 * in both: Alt+ArrowLeft and Alt+ArrowRight alone go back and forward unless the caret is in a text field, Cmd/Ctrl+K
 * opens quick open and Cmd/Ctrl+N creates. Null for every other key.
 */
export function writingShortcutForKey(press: WritingKeyPress): WritingShortcut | null {
  if (press.altKey) {
    if (press.metaKey || press.ctrlKey || press.shiftKey || press.editable) return null;
    if (press.key === "ArrowLeft") return "history-back";
    if (press.key === "ArrowRight") return "history-forward";
    return null;
  }
  if (!(press.metaKey || press.ctrlKey) || press.shiftKey || press.repeat) return null;
  const key = press.key.toLowerCase();
  if (key === "k") return "quick-open";
  if (key === "n") return "create";
  return null;
}

/** What the workspace needs from its host; the plugin entry wires it to the host message channel and the broker. */
export interface WritingWorkspaceBridge {
  /** Subscribes to host messages. Returns an unsubscribe function. */
  onHostMessage(handler: (message: WritingHostMessage) => void): () => void;
  /** Publishes state to the shell, in call order. Resolves false when the payload did not reach it. */
  publish(payload: WritingStatePayload): Promise<boolean>;
  /** Hands text too large for one event to the shell through `text.transfer.*`; resolves to the transfer id. */
  transferText(text: string): Promise<string>;
}

export interface WritingRpcPublisher {
  call<T>(method: string, payload: unknown): Promise<T>;
}

/**
 * Builds the function that publishes Writing state to the shell. Publishing
 * never throws into the caller: errors are reported to `onError` and the
 * result is false instead. Payloads go out one at a time, in call order, so a
 * flush reply can never overtake the draft published before it.
 */
export function createWritingStatePublisher(
  rpc: WritingRpcPublisher,
  onError: (cause: unknown) => void = () => undefined,
): (payload: WritingStatePayload) => Promise<boolean> {
  let tail: Promise<boolean> = Promise.resolve(true);
  return (payload) => {
    const sent = tail.then(async () => {
      try {
        await rpc.call("event.publish", { type: WRITING_STATE_EVENT_TYPE, payload });
        return true;
      } catch (cause) {
        onError(cause);
        return false;
      }
    });
    tail = sent;
    return sent;
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
