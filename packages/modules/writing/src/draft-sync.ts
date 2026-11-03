/**
 * Turns editor changes into drafts for the shell. The webview never saves: it coalesces changes, publishes
 * the latest text after the user pauses, and hands over whatever is unsent when the shell asks to flush.
 * Free of DOM and Tauri dependencies so scripts/writing-plugin-source.test.mjs can drive it with fake timers.
 */

import {
  WRITING_DRAFT_INLINE_LIMIT_BYTES,
  type WritingDraft,
  type WritingStatePayload,
} from "./plugin-protocol.ts";

/** Idle time before a change becomes a draft. The shell autosaves about 900 ms after its last change. */
export const WRITING_DRAFT_IDLE_MS = 500;
/** Wait before a draft that could not be published is tried again. */
export const WRITING_DRAFT_RETRY_MS = 2000;
/**
 * Largest JSON encoding of an inline draft's text. Quotes, newlines and control characters grow when
 * encoded, and events are capped at 256 KiB, so text near the byte limit can still need a transfer.
 */
export const WRITING_DRAFT_INLINE_JSON_LIMIT_BYTES = 192 * 1024;

const encoder = new TextEncoder();

/** True when `text` is too big to ride in a state event and must go through a chunked transfer. */
export function draftNeedsTransfer(text: string): boolean {
  return (
    encoder.encode(text).byteLength > WRITING_DRAFT_INLINE_LIMIT_BYTES ||
    encoder.encode(JSON.stringify(text)).byteLength > WRITING_DRAFT_INLINE_JSON_LIMIT_BYTES
  );
}

export interface DraftSyncOptions {
  /** Publishes in call order; resolves false when the payload did not reach the shell. */
  publish(payload: WritingStatePayload): Promise<boolean>;
  /** Sends text through `text.transfer.*` and resolves to the transfer id. Rejects when the hand-off fails. */
  transfer(text: string): Promise<string>;
  onError?(cause: unknown): void;
  /** Called with `true` while text is waiting to be published and `false` once none is. */
  onPending?(pending: boolean): void;
  idleMs?: number;
  retryMs?: number;
  setTimer?(callback: () => void, ms: number): unknown;
  clearTimer?(handle: unknown): void;
}

export interface DraftSync {
  /** Drops anything unsent. Called when the shell loads an entry: it chose to replace what the editor held. */
  reset(): void;
  /** Records the editor's latest text for the load it was edited under and (re)starts the idle timer. */
  change(entityId: string, token: string, text: string): void;
  /** Publishes the unsent text now, if there is any. */
  publishNow(): Promise<void>;
  /** Answers a `writing.flush`: the unsent text as a draft, or null when everything was already published. */
  flush(requestId: string): Promise<void>;
  /** True while text is waiting to be published. */
  hasUnsent(): boolean;
  /** Cancels timers; call when the view goes away. */
  dispose(): void;
}

interface Unsent {
  entityId: string;
  token: string;
  text: string;
}

export function createDraftSync(options: DraftSyncOptions): DraftSync {
  const idleMs = options.idleMs ?? WRITING_DRAFT_IDLE_MS;
  const retryMs = options.retryMs ?? WRITING_DRAFT_RETRY_MS;
  const setTimer = options.setTimer ?? ((callback, ms) => setTimeout(callback, ms));
  const clearTimer = options.clearTimer ?? ((handle) => clearTimeout(handle as ReturnType<typeof setTimeout>));
  let unsent: Unsent | null = null;
  // Bumped by reset(), so text that failed to send is not restored over a newer load.
  let epoch = 0;
  let timer: unknown = null;
  let reportedPending = false;
  // Sends and flush replies run one after another, so a reply cannot overtake a draft still being handed over.
  let queue: Promise<unknown> = Promise.resolve();
  const enqueue = <T>(task: () => Promise<T>): Promise<T> => {
    const run = queue.then(task, task);
    queue = run;
    return run;
  };

  const report = () => {
    const pending = unsent !== null;
    if (pending === reportedPending) return;
    reportedPending = pending;
    options.onPending?.(pending);
  };
  const cancelTimer = () => {
    if (timer === null) return;
    clearTimer(timer);
    timer = null;
  };
  const arm = (ms: number) => {
    cancelTimer();
    timer = setTimer(() => {
      timer = null;
      void publishNow();
    }, ms);
  };

  /** Builds the draft for `taken`; large text goes through a transfer first. */
  async function draftFor(taken: Unsent): Promise<WritingDraft> {
    if (!draftNeedsTransfer(taken.text)) return { entityId: taken.entityId, token: taken.token, text: taken.text };
    return { entityId: taken.entityId, token: taken.token, transfer: await options.transfer(taken.text) };
  }

  /**
   * Takes the unsent text and sends it. When it cannot be sent it becomes unsent again, unless the user
   * has typed something newer in the meantime, which supersedes it.
   */
  async function send(deliver: (draft: WritingDraft) => Promise<boolean>): Promise<boolean> {
    const taken = unsent;
    if (taken === null) return true;
    const takenEpoch = epoch;
    unsent = null;
    cancelTimer();
    report();
    let delivered = false;
    try {
      delivered = await deliver(await draftFor(taken));
    } catch (cause) {
      options.onError?.(cause);
    }
    if (!delivered && unsent === null && epoch === takenEpoch) {
      unsent = taken;
      report();
      arm(retryMs);
    }
    return delivered;
  }

  function reset() {
    cancelTimer();
    unsent = null;
    epoch += 1;
    report();
  }

  function publishNow(): Promise<void> {
    return enqueue(async () => {
      await send((draft) => options.publish({ kind: "draft", draft }));
    });
  }

  return {
    reset,
    change(entityId, token, text) {
      unsent = { entityId, token, text };
      report();
      arm(idleMs);
    },
    publishNow,
    flush(requestId) {
      return enqueue(async () => {
        if (unsent === null) {
          await options.publish({ kind: "flush-reply", requestId, draft: null });
          return;
        }
        // Unlike an idle draft, a failed hand-off is not answered: the shell times out rather than being told
        // there is nothing to save.
        await send((draft) => options.publish({ kind: "flush-reply", requestId, draft }));
      });
    },
    hasUnsent: () => unsent !== null,
    dispose: reset,
  };
}
