/**
 * One AI request from the Writing webview, through the broker. Plugin webviews do not receive the shell's
 * `ai-stream:<id>` events, so the run polls `ai.request.poll` until the request ends. Free of DOM and Tauri
 * dependencies; timers are injectable so scripts/writing-plugin-source.test.mjs can drive it.
 *
 * Every request is released when it is over: `ai.request.citations` drops the broker's record of it (a request
 * with citations stays registered until it is read), and its result is ignored here.
 */

import type { AiStreamEvent } from "../../../../src/lib/project/types.ts";
import {
  WRITING_AI_FAILED_MESSAGE,
  applyAiEvents,
  initialAiRunState,
  isTerminalAiPhase,
  type WritingAiRunState,
  type WritingAiStartRequest,
} from "./ai-flow.ts";

/** Poll interval; the broker's event buffer holds the last 64 events, far more than arrive in this time. */
export const WRITING_AI_POLL_MS = 200;
/** Consecutive failed polls before the run gives up (a vanished request fails every poll). */
export const WRITING_AI_MAX_POLL_FAILURES = 5;

export interface WritingAiRunRpc {
  call<T>(method: string, payload: unknown): Promise<T>;
}

/** The run's state plus whether a cancel was sent and is waiting for the request to end. */
export interface WritingAiRunView extends WritingAiRunState {
  cancelPending: boolean;
}

export interface WritingAiRunOptions {
  rpc: WritingAiRunRpc;
  /** Called with every change; not called after `dispose`. */
  onUpdate(view: WritingAiRunView): void;
  pollMs?: number;
  maxPollFailures?: number;
  setTimer?(callback: () => void, ms: number): unknown;
  clearTimer?(handle: unknown): void;
}

export interface WritingAiRun {
  /** Asks the broker to stop the request; the run ends when the terminal event arrives. */
  cancel(): Promise<void>;
  /** Stops polling and cancels and releases a request that is still running. The run goes quiet. */
  dispose(): void;
}

function messageOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

export function startAiRun(options: WritingAiRunOptions, request: WritingAiStartRequest): WritingAiRun {
  const { rpc } = options;
  const pollMs = options.pollMs ?? WRITING_AI_POLL_MS;
  const maxPollFailures = options.maxPollFailures ?? WRITING_AI_MAX_POLL_FAILURES;
  const setTimer = options.setTimer ?? ((callback, ms) => setTimeout(callback, ms));
  const clearTimer = options.clearTimer ?? ((handle) => clearTimeout(handle as ReturnType<typeof setTimeout>));

  let state = initialAiRunState();
  let requestId: string | null = null;
  let cancelPending = false;
  let cancelFailed = false;
  let disposed = false;
  let finished = false;
  let pollFailures = 0;
  let timer: unknown = null;

  const publish = () => {
    if (!disposed) options.onUpdate({ ...state, cancelPending });
  };
  const release = (id: string) => {
    void rpc.call("ai.request.citations", { requestId: id }).catch(() => undefined);
  };
  /** Cancels and releases a request nobody is watching. Failures mean it already ended. */
  const abandon = (id: string) => {
    void rpc
      .call("ai.request.cancel", { requestId: id })
      .catch(() => undefined)
      .then(() => release(id));
  };

  async function sendCancel(id: string) {
    try {
      await rpc.call("ai.request.cancel", { requestId: id });
    } catch (cause) {
      if (disposed || finished) return;
      cancelPending = false;
      cancelFailed = true;
      state = { ...state, error: messageOf(cause) };
      publish();
    }
  }

  function schedule() {
    if (disposed || finished) return;
    timer = setTimer(() => void poll(), pollMs);
  }

  async function finish() {
    finished = true;
    cancelPending = false;
    const id = requestId;
    if (state.phase === "completed" && !state.proposal && id) {
      // Nothing usable came through the events; the broker's result is the proposal.
      try {
        const result = await rpc.call<{ output?: unknown }>("ai.request.result", { requestId: id });
        if (typeof result?.output === "string" && !disposed) state = { ...state, proposal: result.output };
      } catch {
        // The completed event stands.
      }
    }
    // A failed cancel is stale once the request ended on its own.
    if (cancelFailed && (state.phase === "completed" || state.phase === "cancelled")) state = { ...state, error: "" };
    publish();
    if (id) release(id);
  }

  async function poll() {
    timer = null;
    const id = requestId;
    if (disposed || finished || !id) return;
    let events: AiStreamEvent[];
    try {
      events = await rpc.call<AiStreamEvent[]>("ai.request.poll", { requestId: id });
      pollFailures = 0;
    } catch (cause) {
      if (disposed) return;
      pollFailures += 1;
      if (pollFailures < maxPollFailures) {
        schedule();
        return;
      }
      state = { ...state, phase: "failed", progressMessage: "", error: messageOf(cause) || WRITING_AI_FAILED_MESSAGE };
      // Whatever the request is still doing, nobody will see it any more.
      finished = true;
      cancelPending = false;
      publish();
      abandon(id);
      return;
    }
    if (disposed) return;
    state = applyAiEvents(state, Array.isArray(events) ? events : []);
    if (isTerminalAiPhase(state.phase)) {
      await finish();
      return;
    }
    publish();
    schedule();
  }

  async function begin() {
    publish();
    let id: string;
    try {
      id = (await rpc.call<{ requestId: string }>("ai.request.start", request)).requestId;
    } catch (cause) {
      if (disposed) return;
      finished = true;
      state = { ...state, phase: "failed", progressMessage: "", error: messageOf(cause) };
      publish();
      return;
    }
    if (disposed) {
      // Closed while the broker was starting it: stop what was started.
      abandon(id);
      return;
    }
    requestId = id;
    state = { ...state, phase: "running" };
    publish();
    if (cancelPending) await sendCancel(id);
    if (!disposed && !finished) await poll();
  }

  void begin();

  return {
    async cancel() {
      if (disposed || finished || cancelPending) return;
      cancelPending = true;
      cancelFailed = false;
      publish();
      // Before the broker has named the request, `begin` sends the cancel as soon as it can.
      if (requestId) await sendCancel(requestId);
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      if (timer !== null) clearTimer(timer);
      timer = null;
      if (requestId && !finished) abandon(requestId);
    },
  };
}
