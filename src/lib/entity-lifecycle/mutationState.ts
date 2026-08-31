import { MUTATION_STATUS, MUTATION_STATUS_MESSAGES } from "./vocabulary.ts";

export type MutationPhase = "idle" | "saving" | "saved" | "conflict" | "failed";

export type MutationSnapshot = {
  phase: MutationPhase;
  message: string;
  detail: string;
};

export type MutationClassifier = (error: unknown) => {
  conflict: boolean;
  message: string;
};

export type MutationSnapshotStore = {
  get: () => MutationSnapshot;
  set: (next: MutationSnapshot) => void;
};

const idle: MutationSnapshot = { phase: "idle", message: "", detail: "" };

function defaultClassifier(error: unknown): { conflict: boolean; message: string } {
  const message =
    error instanceof Error
      ? error.message
      : error &&
          typeof error === "object" &&
          "message" in error &&
          typeof (error as { message: unknown }).message === "string"
        ? (error as { message: string }).message
        : String(error ?? MUTATION_STATUS_MESSAGES.failedTitle);
  const conflict =
    /revision[- ]conflict/i.test(message) ||
    (error !== null &&
      typeof error === "object" &&
      "code" in error &&
      String((error as { code: unknown }).code).includes("revision"));
  return { conflict, message: message || MUTATION_STATUS_MESSAGES.failedTitle };
}

function memoryStore(initial: MutationSnapshot = { ...idle }): MutationSnapshotStore {
  let snapshot = initial;
  return {
    get: () => snapshot,
    set: (next) => {
      snapshot = next;
    },
  };
}

/**
 * Shared mutation-status controller for first-party entity lifecycle actions.
 * Callers still perform ModuleContext / project mutations; this only owns UI state.
 * Pass a Svelte $state-backed store from the shell so updates invalidate the UI.
 */
export function createMutationController(
  store: MutationSnapshotStore = memoryStore(),
  classify: MutationClassifier = defaultClassifier,
) {
  return {
    get snapshot() {
      return store.get();
    },
    get phase() {
      return store.get().phase;
    },
    get busy() {
      return store.get().phase === "saving";
    },
    begin(detail = "") {
      store.set({ phase: "saving", message: MUTATION_STATUS.saving, detail });
    },
    succeed(detail = "") {
      store.set({ phase: "saved", message: MUTATION_STATUS.saved, detail });
    },
    conflict(detail = "") {
      store.set({
        phase: "conflict",
        message: MUTATION_STATUS_MESSAGES.conflictTitle,
        detail: detail || MUTATION_STATUS_MESSAGES.conflictBody,
      });
    },
    fail(message: string, detail = "") {
      store.set({
        phase: "failed",
        message: message || MUTATION_STATUS_MESSAGES.failedTitle,
        detail,
      });
    },
    reset() {
      store.set({ ...idle });
    },
    async run<T>(
      operation: () => Promise<T>,
      detail = "",
    ): Promise<{ ok: true; value: T } | { ok: false; error: unknown }> {
      this.begin(detail);
      try {
        const value = await operation();
        this.succeed(detail);
        return { ok: true, value };
      } catch (error) {
        const classified = classify(error);
        if (classified.conflict) this.conflict(classified.message);
        else this.fail(classified.message);
        return { ok: false, error };
      }
    },
  };
}

export type MutationController = ReturnType<typeof createMutationController>;
