/** Live dirty-check for the schema overlay editor (set while ModuleSchemaPanel is mounted). */
type DirtyCheck = () => boolean;
/** Async discard confirmation (must not use window.confirm — silent no-op on macOS WKWebView). */
type DiscardPrompt = () => Promise<boolean>;

let dirtyCheck: DirtyCheck | null = null;
let discardPrompt: DiscardPrompt | null = null;

export function setSchemaEditorDirtyCheck(check: DirtyCheck | null) {
  dirtyCheck = check;
}

export function setSchemaEditorDiscardPrompt(prompt: DiscardPrompt | null) {
  discardPrompt = prompt;
}

export function isSchemaEditorDirty(): boolean {
  return dirtyCheck?.() ?? false;
}

/**
 * Returns false if the editor is dirty and the user cancels discard.
 * Uses the registered in-app prompt — never window.confirm.
 */
export async function allowLeaveSchemaEditor(): Promise<boolean> {
  if (!dirtyCheck?.()) return true;
  if (!discardPrompt) {
    // Editor is dirty but no UI prompt is mounted — block leave rather than silently discard.
    return false;
  }
  return discardPrompt();
}
