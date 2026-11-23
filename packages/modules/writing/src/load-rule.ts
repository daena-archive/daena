/**
 * When a `writing.load` may replace the editor, and when it must leave it alone. Pure, so
 * scripts/writing-plugin-source.test.mjs can drive every case.
 *
 * The shell posts a load each time it shows an entry: a selection, a reload from disk, and a return to a tab
 * whose webview was kept alive. A load that names the entity and revision the editor already shows changes
 * nothing about the text, so the editor keeps its undo history and selection. Anything else replaces it, except
 * that text the shell has not received is never discarded by a load.
 */

/** What the editor is showing: its entity and the revision it last knew to be saved (or loaded). */
export interface WritingShownEntry {
  entityId: string;
  shownRevision: string;
}

export interface WritingLoadRuleInput {
  /** The editor on screen, or null when there is none (nothing loaded yet, or the last load failed). */
  current: WritingShownEntry | null;
  /** True while the editor holds text the shell does not have: unpublished, or held back by an earlier load. */
  unsent: boolean;
  /** The entity and revision the incoming `writing.load` names. */
  incoming: { entityId: string; revision: string };
}

export type WritingLoadDecision =
  /** Build a new editor, as for any new entry. Only chosen when no unsent text can be lost. */
  | { action: "replace" }
  /**
   * Same entity and revision: keep the editor, its undo history and its selection. The new load token is adopted.
   * With `republish`, the unsent text is published again under that token.
   */
  | { action: "keep"; republish: boolean }
  /**
   * A different entity or revision while text is unsent. The shell's flush should have prevented this. The editor
   * is kept and keeps its old token (a draft under the new one would overwrite a revision the editor never saw),
   * and the failure is surfaced through the save-error path.
   */
  | { action: "hold" };

export function decideWritingLoad(input: WritingLoadRuleInput): WritingLoadDecision {
  const { current, unsent, incoming } = input;
  if (current === null) return { action: "replace" };
  if (current.entityId === incoming.entityId && current.shownRevision === incoming.revision) {
    return { action: "keep", republish: unsent };
  }
  return unsent ? { action: "hold" } : { action: "replace" };
}

/**
 * The revision the editor knows after `writing.saved`. The shell's saved revision describes the text it saved; it
 * describes the editor only when nothing newer is waiting to be sent, so with unsent text the old value stays and
 * the next load for this entity replaces the editor rather than keeping text the revision does not match.
 */
export function revisionAfterSaved(
  current: WritingShownEntry | null,
  saved: { entityId: string; revision: string },
  unsent: boolean,
): string | null {
  if (current === null || current.entityId !== saved.entityId || unsent) return null;
  return saved.revision;
}

/** Shown when a load named another entity or revision while the editor held text the shell had not received. */
export const WRITING_HELD_TEXT_MESSAGE =
  "This entry changed while you had unsaved changes. Your text is still here but was not saved. Copy it, then close this tab to continue.";
