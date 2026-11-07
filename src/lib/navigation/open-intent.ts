export type OpenIntent = "current" | "new-tab";

/** Middle-click, or the platform modifier. Ctrl alone on macOS is a secondary click. */
export function openIntent(
  event:
    Pick<MouseEvent, "button" | "metaKey" | "ctrlKey"> | Pick<KeyboardEvent, "metaKey" | "ctrlKey"> | null | undefined,
  isMac: boolean,
): OpenIntent {
  if (event && "button" in event && event.button === 1) return "new-tab";
  if (event && (isMac ? event.metaKey : event.ctrlKey)) return "new-tab";
  return "current";
}
