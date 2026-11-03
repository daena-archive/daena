/**
 * Pure wire helpers for the isolated `daena.language` webview. The shell side
 * of this contract is documented in the Language protocol notes; everything
 * here is node-testable and free of DOM or Tauri dependencies.
 */

export const LANGUAGE_PLUGIN_ID = "daena.language";
export const LANGUAGE_STATE_EVENT_TYPE = "daena.language/state@1";

export const LANGUAGE_PANES = ["overview", "lexicon", "sounds", "writing", "grammar", "forms", "samples"] as const;
export type LanguagePane = (typeof LANGUAGE_PANES)[number];
export const DEFAULT_LANGUAGE_PANE: LanguagePane = "overview";

export function isLanguagePane(value: unknown): value is LanguagePane {
  return typeof value === "string" && (LANGUAGE_PANES as readonly string[]).includes(value);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function nonEmpty(value: string | null): string | null {
  return value && value.length > 0 ? value : null;
}

function parseModuleState(raw: string | null): Record<string, unknown> | null {
  if (!raw) return null;
  try {
    const parsed: unknown = JSON.parse(raw);
    return isRecord(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

export interface LanguageMountParams {
  projectId: string | null;
  /** Language entity the shell selected; null when none is selected. */
  entityId: string | null;
  /** Pane to show first; falls back to Overview for missing or unknown values. */
  pane: LanguagePane;
  /** Parsed `state` JSON exactly as the shell sent it, or null. */
  moduleState: Record<string, unknown> | null;
}

/** Reads the mount parameters SandboxView appends to the plugin document URL. */
export function parseLanguageMountParams(search: string): LanguageMountParams {
  const params = new URLSearchParams(search);
  const moduleState = parseModuleState(params.get("state"));
  const statePane = moduleState?.pane;
  return {
    projectId: nonEmpty(params.get("project")),
    entityId: nonEmpty(params.get("entityId")),
    pane: isLanguagePane(statePane) ? statePane : DEFAULT_LANGUAGE_PANE,
    moduleState,
  };
}

/** Messages the shell posts into the webview (`plugin_webview_post`). */
export type LanguageHostMessage =
  | { type: "language.pane"; pane: LanguagePane }
  | { type: "language.can-leave"; requestId: string };

export function parseLanguageHostMessage(value: unknown): LanguageHostMessage | null {
  if (!isRecord(value)) return null;
  if (value.type === "language.pane" && isLanguagePane(value.pane)) {
    return { type: "language.pane", pane: value.pane };
  }
  if (value.type === "language.can-leave" && typeof value.requestId === "string" && value.requestId.length > 0) {
    return { type: "language.can-leave", requestId: value.requestId };
  }
  return null;
}

/** Payloads the webview publishes back to the shell as `daena.language/state@1`. */
export type LanguageStatePayload =
  | { kind: "ready"; pane: LanguagePane; entityId: string | null }
  | { kind: "pane"; pane: LanguagePane }
  | { kind: "leave-pending"; requestId: string }
  | { kind: "leave-reply"; requestId: string; allowed: boolean }
  | { kind: "entity-changed"; entityId: string; deleted: boolean }
  | { kind: "open-link"; href: string };

export type LeaveStatePayload = Extract<LanguageStatePayload, { kind: "leave-pending" | "leave-reply" }>;

/**
 * What the workspace needs from its host. The plugin entry wires these to the
 * host message channel and the state publisher; the workspace never touches
 * the transport itself. Pane changes are reported through the ModuleContext's
 * `onModuleStateChange`, so they are not part of this bridge.
 */
export interface LanguageWorkspaceBridge {
  /** Subscribes to `language.pane` requests. Returns an unsubscribe function. */
  onPaneRequest(handler: (pane: LanguagePane) => void): () => void;
  /** Subscribes to `language.can-leave` requests. Returns an unsubscribe function. */
  onLeaveRequest(handler: (requestId: string) => void): () => void;
  /** Publishes leave progress to the shell. */
  reportLeave(message: LeaveStatePayload): void;
}

/** Maps the workspace's opaque module state to a `pane` payload, or null when it names no pane. */
export function paneStatePayload(state: Record<string, unknown> | null | undefined): LanguageStatePayload | null {
  return state && isLanguagePane(state.pane) ? { kind: "pane", pane: state.pane } : null;
}

export interface LanguageRpcPublisher {
  call<T>(method: string, payload: unknown): Promise<T>;
}

/**
 * Builds the function that publishes Language state to the shell. Publishing
 * never throws into the caller: a failed publish must not fail the edit that
 * triggered it, so errors are reported to `onError` instead.
 */
export function createLanguageStatePublisher(
  rpc: LanguageRpcPublisher,
  onError: (cause: unknown) => void = () => undefined,
): (payload: LanguageStatePayload) => Promise<void> {
  return async (payload) => {
    try {
      await rpc.call("event.publish", { type: LANGUAGE_STATE_EVENT_TYPE, payload });
    } catch (cause) {
      onError(cause);
    }
  };
}

/**
 * Picks Language's effective manifest from `schema.list`. The bundled
 * manifest.json carries unqualified entity type ids, which never match stored
 * entities, so it is only a fallback for hosts that do not list Language.
 */
export function pickLanguageManifest<T extends { id: string }>(manifests: readonly T[], fallback: T): T {
  return manifests.find((manifest) => manifest.id === LANGUAGE_PLUGIN_ID) ?? fallback;
}
