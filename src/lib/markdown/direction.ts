// Mirrors the first-strong-character detection used by tiptap-text-direction, which the editor
// applies to every paragraph and heading. A stored `dir` that equals the detected direction is
// redundant: it is re-derived whenever the text is shown or edited, so it is not persisted.
const RTL_CHARS = "\u0591-\u07ff\ufb1d-\ufdfd\ufe70-\ufefc";
const LTR_CHARS =
  "A-Za-z\u00c0-\u00d6\u00d8-\u00f6\u00f8-\u02b8\u0300-\u0590\u0800-\u1fff\u200e\u2c00-\ufb1c\ufe00-\ufe6f\ufefd-\uffff";
const RTL_START = new RegExp(`^[^${LTR_CHARS}]*[${RTL_CHARS}]`);
const LTR_START = new RegExp(`^[^${RTL_CHARS}]*[${LTR_CHARS}]`);

export type TextDirectionValue = "ltr" | "rtl";

export function detectTextDirection(text: string): TextDirectionValue | null {
  if (!text) return null;
  if (RTL_START.test(text)) return "rtl";
  if (LTR_START.test(text)) return "ltr";
  return null;
}

/** The `dir` worth persisting: an explicit override of what the text itself implies. */
export function persistedDirection(dir: string, text: string): TextDirectionValue | "" {
  if (dir !== "ltr" && dir !== "rtl") return "";
  return detectTextDirection(text) === dir ? "" : dir;
}
