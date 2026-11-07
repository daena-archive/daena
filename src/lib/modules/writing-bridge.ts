/**
 * Shell side of the daena.writing webview bridge (see docs/plans/writing-isolation.md).
 *
 * Pure logic only: no Tauri or Svelte imports, so the route wires these helpers to
 * IPC and state, and scripts/writing-bridge.test.mjs exercises them directly.
 */

import { isTabShortcut, type TabShortcut } from "../navigation/tab-shortcuts.ts";

export const WRITING_PLUGIN_ID = "daena.writing";
export const WRITING_VIEW_ID = "writing-workspace";
export const WRITING_STATE_EVENT = "writing-state";
export const WRITING_SHORTCUTS = ["quick-open", "create", "history-back", "history-forward"] as const;
export type WritingBridgeShortcut = (typeof WRITING_SHORTCUTS)[number];
export const WRITING_MODES = ["read", "edit"] as const;
export type WritingBridgeMode = (typeof WRITING_MODES)[number];
export const WRITING_CONFLICT_ACTIONS = ["reload", "overwrite", "recovery"] as const;
export type WritingBridgeConflictAction = (typeof WRITING_CONFLICT_ACTIONS)[number];
/** Host messages are capped at 64 KiB (`MAX_HOST_MESSAGE_BYTES` in plugin_webview.rs); this leaves some margin. */
export const WRITING_HOST_MESSAGE_BUDGET_BYTES = 60 * 1024;
/** How long the shell waits for a flush reply before it carries on without the webview's text. */
export const WRITING_FLUSH_TIMEOUT_MS = 1500;

const MAX_ID_LENGTH = 128;
const MAX_HREF_LENGTH = 4096;
/** Draft text up to this many UTF-8 bytes arrives inline in a state event (events are capped at 256 KiB). */
export const WRITING_DRAFT_INLINE_LIMIT_BYTES = 128 * 1024;

export function isWritingShortcut(value: unknown): value is WritingBridgeShortcut {
  return typeof value === "string" && (WRITING_SHORTCUTS as readonly string[]).includes(value);
}

export function isWritingConflictAction(value: unknown): value is WritingBridgeConflictAction {
  return typeof value === "string" && (WRITING_CONFLICT_ACTIONS as readonly string[]).includes(value);
}

/**
 * Text the webview hands over for one entity: inline, or a reference to a chunked transfer. `token` is the
 * value the shell issued in the `writing.load` the text was edited under.
 */
export type WritingBridgeDraft =
  { entityId: string; token: string; text: string } | { entityId: string; token: string; transfer: string };

/** Messages the webview publishes as `daena.writing/state@1`, after the host forwards them. */
export type WritingStateEvent =
  | { kind: "ready"; entityId: string | null }
  | { kind: "draft"; draft: WritingBridgeDraft }
  | { kind: "flush-reply"; requestId: string; draft: WritingBridgeDraft | null }
  | { kind: "conflict-action"; entityId: string; token: string; action: WritingBridgeConflictAction }
  | { kind: "open-link"; href: string }
  | { kind: "open-entity"; entityId: string; newTab?: boolean }
  | { kind: "fullscreen"; entityId: string; on: boolean }
  | { kind: "request-rename"; entityId: string }
  | { kind: "request-archive"; entityId: string }
  | { kind: "shortcut"; shortcut: WritingBridgeShortcut | TabShortcut };

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
  const token = idOf(value.token);
  if (entityId === null || token === null) return null;
  const inline = typeof value.text === "string";
  // Exactly one of `text` and `transfer`.
  if (inline === (value.transfer !== undefined)) return null;
  if (inline) {
    const text = value.text as string;
    if (utf8Length(text) > WRITING_DRAFT_INLINE_LIMIT_BYTES) return null;
    return { entityId, token, text };
  }
  const transfer = idOf(value.transfer);
  return transfer === null ? null : { entityId, token, transfer };
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
    case "conflict-action": {
      const entityId = idOf(payload.entityId);
      const token = idOf(payload.token);
      if (entityId === null || token === null || !isWritingConflictAction(payload.action)) return null;
      return { kind: "conflict-action", entityId, token, action: payload.action };
    }
    case "open-link":
      return typeof payload.href === "string" && payload.href.length <= MAX_HREF_LENGTH
        ? { kind: "open-link", href: payload.href }
        : null;
    case "open-entity": {
      const entityId = idOf(payload.entityId);
      if (entityId === null) return null;
      // Only a boolean is kept. Anything else is treated as absent, and a bad id is still rejected.
      return typeof payload.newTab === "boolean"
        ? { kind: "open-entity", entityId, newTab: payload.newTab }
        : { kind: "open-entity", entityId };
    }
    case "request-rename":
    case "request-archive": {
      const entityId = idOf(payload.entityId);
      return entityId === null ? null : { kind: payload.kind as "request-rename" | "request-archive", entityId };
    }
    case "fullscreen": {
      const entityId = idOf(payload.entityId);
      return entityId !== null && typeof payload.on === "boolean"
        ? { kind: "fullscreen", entityId, on: payload.on }
        : null;
    }
    case "shortcut": {
      const shortcut = payload.shortcut;
      if (isWritingShortcut(shortcut) || isTabShortcut(shortcut)) return { kind: "shortcut", shortcut };
      return null;
    }
    default:
      return null;
  }
}

/** Messages the shell posts into the webview with `plugin_webview_post`. */
export type WritingHostMessage =
  | {
      type: "writing.load";
      entityId: string;
      /** Issued per load; drafts and conflict actions must echo it. Saving does not change it. */
      token: string;
      revision: string;
      editable: boolean;
      placeholder: string;
      mode: WritingBridgeMode;
    }
  | { type: "writing.flush"; requestId: string }
  | { type: "writing.saved"; entityId: string; revision: string }
  | { type: "writing.conflict"; entityId: string; message: string; canOverwrite: boolean }
  | { type: "writing.save-error"; entityId: string; message: string }
  | { type: "writing.editable"; editable: boolean }
  | { type: "writing.ai-state"; enabled: boolean; providerName: string; templates: WritingBridgeAiTemplate[] }
  | { type: "writing.fullscreen"; on: boolean }
  | { type: "writing.title"; entityId: string; title: string };

/** An editor prompt template as the webview receives it: only what the editor menu and the AI dialog read. */
export interface WritingBridgeAiTemplate {
  id: string;
  label: string;
  instruction: string;
  requiresSelection: boolean;
  enabled: boolean;
  kind: "editor";
}

const MAX_AI_TEMPLATES = 64;
const MAX_AI_LABEL_LENGTH = 200;
const MAX_AI_PROVIDER_NAME_LENGTH = 200;

/** The prompt templates the shell merged, in the shape the webview expects. Nothing else of a template travels. */
export interface WritingAiStateSource {
  enabled: boolean;
  providerName: string;
  templates: ReadonlyArray<{
    id: string;
    label: string;
    instruction: string;
    kind: string;
    requiresSelection?: boolean;
    enabled?: boolean;
  }>;
}

function utf8Bytes(value: unknown): number {
  return new TextEncoder().encode(JSON.stringify(value)).byteLength;
}

/**
 * Builds the `writing.ai-state` message: whether AI is on, the provider's name and the editor templates (the git
 * and image ones are the shell's). The message must fit in one host message, and posting must never fail on
 * size, so when the templates do not fit the trailing ones are dropped (and with them the ones past
 * `MAX_AI_TEMPLATES`). The bundled templates come first, so only custom templates at the end of a very large
 * prompt library are ever lost.
 */
export function writingAiStateMessage(
  source: WritingAiStateSource,
  budgetBytes: number = WRITING_HOST_MESSAGE_BUDGET_BYTES,
): Extract<WritingHostMessage, { type: "writing.ai-state" }> {
  const templates: WritingBridgeAiTemplate[] = [];
  for (const template of source.templates) {
    if (template.kind !== "editor") continue;
    const id = idOf(template.id);
    if (id === null || !template.label.trim() || templates.some((existing) => existing.id === id)) continue;
    templates.push({
      id,
      label: template.label.slice(0, MAX_AI_LABEL_LENGTH),
      instruction: template.instruction,
      requiresSelection: template.requiresSelection === true,
      enabled: template.enabled !== false,
      kind: "editor",
    });
  }
  const message: Extract<WritingHostMessage, { type: "writing.ai-state" }> = {
    type: "writing.ai-state",
    enabled: source.enabled,
    providerName: source.providerName.slice(0, MAX_AI_PROVIDER_NAME_LENGTH),
    templates: templates.slice(0, MAX_AI_TEMPLATES),
  };
  while (message.templates.length > 0 && utf8Bytes(message) > budgetBytes) message.templates.pop();
  return message;
}

/**
 * Turns the webview's full screen on or off. The shell only sends `false`; the webview starts its own full screens.
 */
export function writingFullscreenMessage(on: boolean): WritingHostMessage {
  return { type: "writing.fullscreen", on };
}

/** The header title of an entry the shell has renamed. */
export function writingTitleMessage(entityId: string, title: string): WritingHostMessage {
  return { type: "writing.title", entityId, title };
}

/** Builds the `writing.load` message for a selection. */
export function writingLoadMessage(
  entityId: string,
  token: string,
  revision: string,
  options: { editable?: boolean; placeholder?: string; mode?: WritingBridgeMode } = {},
): WritingHostMessage {
  return {
    type: "writing.load",
    entityId,
    token,
    revision,
    editable: options.editable ?? false,
    placeholder: options.placeholder ?? "",
    mode: options.mode ?? "read",
  };
}

/** The load the shell last posted: the only one whose drafts and conflict actions it accepts. */
export interface WritingLoad {
  entityId: string;
  token: string;
}

/**
 * True when a draft or conflict action belongs to the entity the shell has selected and to the load it last
 * posted. A draft from before a reload, or for another entry, fails this and is dropped.
 */
export function writingMessageCurrent(
  message: { entityId: string; token: string },
  selectedEntityId: string | null | undefined,
  load: WritingLoad | null,
): boolean {
  return (
    load !== null &&
    selectedEntityId != null &&
    message.entityId === selectedEntityId &&
    load.entityId === selectedEntityId &&
    message.token === load.token
  );
}

/** What the conflict banner in the webview shows, from the shell's `documentConflict`. */
export function writingConflictMessage(
  entityId: string,
  conflict: { diagnostics: readonly string[] },
): Extract<WritingHostMessage, { type: "writing.conflict" }> {
  const attention = conflict.diagnostics.length > 0;
  return {
    type: "writing.conflict",
    entityId,
    message: attention
      ? conflict.diagnostics[0]
      : "Your unsaved document and details are preserved. Choose how to reconcile them before saving.",
    // A source that needs attention cannot be overwritten, as in the shell's own conflict actions.
    canOverwrite: !attention,
  };
}

export interface WritingFlushBrokerOptions {
  /** Delivers a message to the webview; resolves false when it could not be delivered. */
  post(message: WritingHostMessage): Promise<boolean>;
  newRequestId(): string;
  timeoutMs?: number;
  onTimeout?(requestId: string): void;
  setTimer?(callback: () => void, ms: number): unknown;
  clearTimer?(handle: unknown): void;
}

export interface WritingFlushBroker {
  /**
   * Asks the webview for its unsent text. Resolves to the draft, or null when it has none, could not be
   * reached, did not answer in time, or went away.
   */
  request(): Promise<WritingBridgeDraft | null>;
  /** Routes a `flush-reply` to its request. False for an unknown or already answered request id. */
  reply(requestId: string, draft: WritingBridgeDraft | null): boolean;
  /** Settles every waiting request with null; the webview is gone. */
  releaseAll(): void;
}

export function createWritingFlushBroker(options: WritingFlushBrokerOptions): WritingFlushBroker {
  const timeoutMs = options.timeoutMs ?? WRITING_FLUSH_TIMEOUT_MS;
  const setTimer = options.setTimer ?? ((callback, ms) => setTimeout(callback, ms));
  const clearTimer = options.clearTimer ?? ((handle) => clearTimeout(handle as ReturnType<typeof setTimeout>));
  const waiting = new Map<string, { settle: (draft: WritingBridgeDraft | null) => void; timer: unknown }>();
  return {
    request() {
      const requestId = options.newRequestId();
      return new Promise<WritingBridgeDraft | null>((resolve) => {
        const settle = (draft: WritingBridgeDraft | null) => {
          const entry = waiting.get(requestId);
          if (!entry) return;
          waiting.delete(requestId);
          clearTimer(entry.timer);
          resolve(draft);
        };
        const timer = setTimer(() => {
          if (!waiting.has(requestId)) return;
          options.onTimeout?.(requestId);
          settle(null);
        }, timeoutMs);
        waiting.set(requestId, { settle, timer });
        void options.post({ type: "writing.flush", requestId }).then((sent) => {
          if (!sent) settle(null);
        });
      });
    },
    reply(requestId, draft) {
      const entry = waiting.get(requestId);
      if (!entry) return false;
      entry.settle(draft);
      return true;
    },
    releaseAll() {
      for (const entry of [...waiting.values()]) entry.settle(null);
    },
  };
}
