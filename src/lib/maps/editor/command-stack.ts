import { UNDO_STACK_SIZE } from "../native-vector/types.ts";
import type { MapCommand } from "./commands.ts";
import { applyCommand } from "./commands.ts";
import { cloneDocument, documentByteSize, documentHash, type MapDocument } from "./model.ts";

const DEFAULT_BYTE_BUDGET = 8 * 1024 * 1024;

export type CommandStackEntry = {
  command: MapCommand;
  inverse: MapCommand;
  bytes: number;
};

export type CommandStackSnapshot = {
  document: MapDocument;
  canUndo: boolean;
  canRedo: boolean;
  undoLabel: string | null;
  redoLabel: string | null;
  dirty: boolean;
  baselineHash: string;
  currentHash: string;
};

export class CommandStack {
  #document: MapDocument;
  #baselineHash: string;
  #undo: CommandStackEntry[] = [];
  #redo: CommandStackEntry[] = [];
  #maxEntries: number;
  #byteBudget: number;
  #listener: ((snapshot: CommandStackSnapshot) => void) | null = null;

  constructor(document: MapDocument, options?: { maxEntries?: number; byteBudget?: number }) {
    this.#document = cloneDocument(document);
    this.#baselineHash = documentHash(this.#document);
    this.#maxEntries = options?.maxEntries ?? UNDO_STACK_SIZE;
    this.#byteBudget = options?.byteBudget ?? DEFAULT_BYTE_BUDGET;
  }

  get document(): MapDocument {
    return this.#document;
  }

  setBaseline(document: MapDocument) {
    this.#document = cloneDocument(document);
    this.#baselineHash = documentHash(this.#document);
    this.#undo = [];
    this.#redo = [];
    this.#emit();
  }

  replaceDocument(document: MapDocument) {
    this.#document = cloneDocument(document);
    this.#emit();
  }

  onChange(listener: ((snapshot: CommandStackSnapshot) => void) | null) {
    this.#listener = listener;
  }

  snapshot(): CommandStackSnapshot {
    const currentHash = documentHash(this.#document);
    return {
      document: cloneDocument(this.#document),
      canUndo: this.#undo.length > 0,
      canRedo: this.#redo.length > 0,
      undoLabel: this.#undo.at(-1)?.command.label ?? null,
      redoLabel: this.#redo.at(-1)?.command.label ?? null,
      dirty: currentHash !== this.#baselineHash,
      baselineHash: this.#baselineHash,
      currentHash,
    };
  }

  isDirty(): boolean {
    return documentHash(this.#document) !== this.#baselineHash;
  }

  canUndo(): boolean {
    return this.#undo.length > 0;
  }

  canRedo(): boolean {
    return this.#redo.length > 0;
  }

  apply(command: MapCommand): MapDocument {
    const before = cloneDocument(this.#document);
    if (command.coalesceKey) {
      const last = this.#undo.at(-1);
      if (last && last.command.coalesceKey === command.coalesceKey) {
        const coalesced = command.apply(before);
        // Keep the original inverse so undo restores the pre-coalesce state.
        last.command = command;
        last.bytes = documentByteSize(coalesced);
        this.#document = coalesced;
        this.#redo = [];
        this.#trim();
        this.#emit();
        return this.#document;
      }
    }
    const after = applyCommand(before, command);
    const inverse = command.invert(before);
    this.#document = after;
    this.#undo.push({ command, inverse, bytes: documentByteSize(after) });
    this.#redo = [];
    this.#trim();
    this.#emit();
    return this.#document;
  }

  undo(): MapDocument | null {
    const entry = this.#undo.pop();
    if (!entry) return null;
    const before = cloneDocument(this.#document);
    this.#document = applyCommand(before, entry.inverse);
    this.#redo.push(entry);
    this.#emit();
    return this.#document;
  }

  redo(): MapDocument | null {
    const entry = this.#redo.pop();
    if (!entry) return null;
    const before = cloneDocument(this.#document);
    this.#document = applyCommand(before, entry.command);
    this.#undo.push(entry);
    this.#emit();
    return this.#document;
  }

  #trim() {
    while (this.#undo.length > this.#maxEntries) this.#undo.shift();
    let total = this.#undo.reduce((sum, entry) => sum + entry.bytes, 0);
    while (this.#undo.length > 1 && total > this.#byteBudget) {
      const removed = this.#undo.shift();
      if (!removed) break;
      total -= removed.bytes;
    }
  }

  #emit() {
    this.#listener?.(this.snapshot());
  }
}
